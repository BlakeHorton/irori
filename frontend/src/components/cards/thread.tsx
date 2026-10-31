import { useState, type ReactNode } from "react"

import { Badge } from "@/components/ui/badge"
import { AttachmentList } from "@/components/attachments"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import {
  cards,
  EditState,
  type AgentRef,
  type CardMessage,
  type CardThreadItem,
} from "@/lib/cards"
import { parseTime, plainBlock, plainLine, relativeTime } from "@/lib/irori"
import { errorText, talk } from "@/lib/talk"
import {
  isRevision,
  opensInFull,
  threadLabel,
  ThreadKind,
  visible,
} from "@/lib/thread"
import { cn } from "@/lib/utils"
import { Face } from "./faces"
import { RichLine, RichText } from "@/components/common/rich-text"

// Thread is a card's thread, oldest first: comments, messages about the
// card ("A -> B") and one muted line per thing that happened, each with
// who wrote it and, once the person has a second device, where. The
// newest page shows first; "Show earlier" opens the rest. A message the
// card cut, doesn't carry the text of yet, or that has files opens in full
// in place (cardId names the card). composer renders under the list,
// newest last: the card's comment box, or a blocker's answer form.
export function Thread({
  cardId,
  items,
  now,
  earlier = 0,
  historyTrimmed,
  empty = "Nothing yet.",
  compact,
  inBlocker,
  composer,
}: {
  cardId?: string
  items: CardThreadItem[]
  now: Date
  // earlier counts entries older than the ones sent (BoardService.Card
  // sends the newest MaxThreadView).
  earlier?: number
  // historyTrimmed: the card's oldest activity lines are no longer kept.
  historyTrimmed?: boolean
  empty?: ReactNode
  compact?: boolean
  // inBlocker: the items sit under their blocker, so they don't name it.
  inBlocker?: boolean
  composer?: ReactNode
}) {
  const [showAll, setShowAll] = useState(false)
  const shown = visible(items, showAll)
  const hidden = items.length - shown.length
  return (
    <div className={cn("grid text-[12.5px]", compact ? "gap-1" : "gap-2")}>
      {hidden > 0 ? (
        <Button
          size="xs"
          variant="ghost"
          className="justify-self-start text-muted-foreground"
          onClick={() => setShowAll(true)}
        >
          Show {hidden} earlier
        </Button>
      ) : (
        (earlier > 0 || historyTrimmed) &&
        items.length > 0 && (
          <p className="text-xs text-muted-foreground">
            {earlier > 0
              ? `${earlier} earlier ${earlier === 1 ? "entry isn't" : "entries aren't"} shown here.`
              : "Earlier history not kept."}
          </p>
        )
      )}
      {shown.length ? (
        shown.map((it) => (
          <ThreadRow
            key={it.id}
            cardId={cardId}
            item={it}
            now={now}
            inBlocker={inBlocker}
          />
        ))
      ) : (
        <p className="text-muted-foreground">{empty}</p>
      )}
      {composer}
    </div>
  )
}

function ThreadRow({
  cardId,
  item,
  now,
  inBlocker,
}: {
  cardId?: string
  item: CardThreadItem
  now: Date
  inBlocker?: boolean
}) {
  const [full, setFull] = useState<CardMessage | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // released: the person released a dismissed message here; its text
  // joins the card once the inbox lets it through.
  const [released, setReleased] = useState(false)
  // edit: the person's comment no agent read yet (card #105). Modify holds
  // it back from every agent until Save (the new text goes out) or Cancel
  // (it goes as it was). shown is what the person just did, until the
  // card's next load changes the comment.
  const seen = `${item.edit ?? ""}\n${item.text}`
  const [shown, setShown] = useState<{ edit: string; seen: string } | null>(
    null
  )
  const [draft, setDraft] = useState<string | null>(null)
  const edit = shown && shown.seen === seen ? shown.edit : (item.edit ?? "")
  const at = parseTime(item.at)
  const time = (
    <time
      className="text-muted-foreground tabular-nums"
      dateTime={at?.toISOString()}
      title={at?.toLocaleString()}
    >
      {relativeTime(at, now)}
    </time>
  )
  // A revision of its blocker: what the item asks now, under the reply it
  // answers; the current options to pick sit below the thread.
  if (isRevision(item))
    return (
      <div className="grid grid-cols-[56px_minmax(0,1fr)] gap-2">
        {time}
        <span className="flex items-start gap-1.5 text-ink-2">
          <ActorFace who={item.who} />
          <span className="min-w-0">
            <Badge
              variant="warning"
              className="mr-1.5 h-4 px-1.5 align-middle text-[10.5px]"
            >
              Revised
            </Badge>
            <RichLine text={item.text} />
            {item.device && <DeviceTag device={item.device} />}
          </span>
        </span>
      </div>
    )
  // An activity line (card #117): whoever made the change shows their
  // face, and the line stays dim so it reads as a log, not a comment.
  if (item.kind === ThreadKind.Event)
    return (
      <div className="grid grid-cols-[56px_minmax(0,1fr)] gap-2">
        {time}
        <span className="flex items-start gap-1.5 text-muted-foreground">
          <ActorFace who={item.who} />
          <span className="min-w-0">
            <RichLine text={item.text} />
            {item.device && <DeviceTag device={item.device} />}
          </span>
        </span>
      </div>
    )
  const l = threadLabel(item, inBlocker)
  const body = plainBlock(item.text)
  const open = async () => {
    if (!cardId || busy) return
    setBusy(true)
    setError(null)
    try {
      setFull(await cards.message(cardId, item.id))
    } catch (err) {
      setError(errorText(err))
    } finally {
      setBusy(false)
    }
  }
  const editAction = async (
    run: () => Promise<void>,
    after: string,
    keepDraft = false
  ) => {
    if (!cardId || busy) return
    setBusy(true)
    setError(null)
    try {
      await run()
      setShown({ edit: after, seen })
      if (!keepDraft) setDraft(null)
    } catch (err) {
      setError(errorText(err))
    } finally {
      setBusy(false)
    }
  }
  const modify = () => {
    if (!cardId) return
    setDraft(item.text)
    void editAction(
      () => cards.modifyComment(cardId, item.id),
      EditState.Editing,
      true
    )
  }
  const cancelEdit = () =>
    cardId &&
    void editAction(() => cards.cancelComment(cardId, item.id), EditState.Can)
  const saveEdit = () => {
    if (!cardId) return
    const text = draft ?? item.text
    if (!text.trim()) {
      setError("Write a comment first.")
      return
    }
    void editAction(
      () => cards.saveComment(cardId, item.id, text),
      EditState.Can
    )
  }
  const release = async () => {
    if (!item.deliveryId || busy) return
    setBusy(true)
    setError(null)
    try {
      await talk.release(item.deliveryId)
      setReleased(true)
    } catch (err) {
      setError(errorText(err))
    } finally {
      setBusy(false)
    }
  }
  return (
    <div className="grid grid-cols-[56px_minmax(0,1fr)] gap-2">
      {time}
      <div className="grid min-w-0 gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-1.5">
          <Face who={item.who} noDot />
          <span className="font-medium text-ink">{plainLine(l.author)}</span>
          {item.to && l.arrow && (
            <>
              <span className="text-muted-foreground" aria-hidden>
                {"->"}
              </span>
              <span className="sr-only">to</span>
              <Face who={item.to} noDot />
              <span className="font-medium text-ink">{plainLine(l.arrow)}</span>
            </>
          )}
          {l.device && <DeviceTag device={l.device} />}
          {l.badges.map((b) => (
            <Badge
              key={b.label}
              variant={b.tone}
              className="h-4 px-1.5 text-[10.5px]"
            >
              {b.label}
            </Badge>
          ))}
        </div>
        {full ? (
          <>
            <RichText
              text={full.text}
              mentions={item.mentions}
              className="text-ink-2"
            />
            {full.files.length > 0 && (
              <ul className="grid gap-0.5 text-[11px] text-muted-foreground">
                {full.files.map((f, i) => (
                  <li key={i} className="font-mono break-all">
                    {plainLine(f)}
                  </li>
                ))}
              </ul>
            )}
          </>
        ) : item.held && released ? (
          <p className="text-muted-foreground">
            Released. Its text joins the card once it is on its way.
          </p>
        ) : item.held && item.dismissed ? (
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <p className="text-muted-foreground">
              Held, then dismissed: it never reached{" "}
              {l.arrow ? plainLine(l.arrow) : "its recipient"}, and its text
              isn't on the card.
            </p>
            {item.deliveryId && (
              <Button
                size="xs"
                variant="outline"
                disabled={busy}
                onClick={() => void release()}
              >
                Release
              </Button>
            )}
          </div>
        ) : item.held ? (
          <p className="text-muted-foreground">
            Not let through the wake policy yet, so its text isn't on the card.
          </p>
        ) : edit === EditState.Editing ? (
          <form
            className="grid gap-1.5"
            onSubmit={(e) => {
              e.preventDefault()
              saveEdit()
            }}
          >
            <Textarea
              value={draft ?? item.text}
              onChange={(e) => setDraft(e.target.value)}
              aria-label="Edit your comment"
              className="min-h-16 text-[12.5px]"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault()
                  saveEdit()
                }
                if (e.key === "Escape") {
                  e.preventDefault()
                  cancelEdit()
                }
              }}
            />
            <div className="flex flex-wrap items-center gap-2">
              <Button size="xs" type="submit" disabled={busy}>
                Save
              </Button>
              <Button
                size="xs"
                variant="ghost"
                type="button"
                disabled={busy}
                onClick={cancelEdit}
              >
                Cancel
              </Button>
              <span className="text-xs text-muted-foreground">
                Held back: no agent gets it until you save or cancel.
              </span>
            </div>
          </form>
        ) : (
          body && (
            <RichText
              text={body}
              mentions={item.mentions}
              className="text-ink-2"
            />
          )
        )}
        {!!item.missing?.length && (
          <div className="grid gap-0.5 text-ink-2">
            <span className="text-xs text-muted-foreground">Still needed:</span>
            <ul className="ml-4 list-disc">
              {item.missing.map((m, i) => (
                <li key={i}>
                  <RichLine text={m} />
                </li>
              ))}
            </ul>
          </div>
        )}
        <AttachmentList attachments={item.attachments ?? []} />
        {cardId && edit === EditState.Can && (
          <Button
            size="xs"
            variant="ghost"
            className="justify-self-start text-muted-foreground"
            disabled={busy}
            title="No agent has read it yet: take it back to edit it"
            onClick={modify}
          >
            Modify
          </Button>
        )}
        {!full && cardId && opensInFull(item) && (
          <Button
            size="xs"
            variant="ghost"
            className="justify-self-start text-muted-foreground"
            disabled={busy}
            onClick={() => void open()}
          >
            {busy ? "Opening…" : "Show the whole message"}
          </Button>
        )}
        {error && <p className="text-xs text-destructive">{error}</p>}
      </div>
    </div>
  )
}

// DeviceTag names the device an entry was written on.
function DeviceTag({ device }: { device: string }) {
  return (
    <Badge
      variant="slate"
      className="ml-1.5 h-4 px-1.5 align-middle font-mono text-[10.5px]"
      title={`Written on ${plainLine(device)}`}
    >
      {plainLine(device)}
    </Badge>
  )
}

// ActorFace is who made an activity line's change (card #117), dimmed to
// match the line; Irori's own lines have none.
function ActorFace({ who }: { who?: AgentRef | null }) {
  if (!who?.key || who.status === "irori") return null
  return <Face who={who} noDot className="-my-0.5 shrink-0 opacity-75" />
}

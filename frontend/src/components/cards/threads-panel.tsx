import { useState } from "react"
import { ChevronDownIcon, ChevronRightIcon, XIcon } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { RichText } from "@/components/common/rich-text"
import { CommentBox } from "./card-detail"
import { Face } from "./faces"
import { Thread } from "./thread"
import {
  displayName,
  threads as threadsApi,
  useCard,
  type ThreadSummary,
} from "@/lib/cards"
import { parseTime, plainLine, relativeTime } from "@/lib/irori"
import { errorText } from "@/lib/talk"

// ThreadsPanel is the Threads panel beside the Board (card #116, layout A):
// the person's conversations with the room, open ones on top (the one
// they're in unfolded), resolved ones folded under "Resolved". A thread
// isn't a card: agents on the cards it names answer it, or after 45s an
// idle agent does; the person replies here and resolves it when done.
export function ThreadsPanel({
  threads,
  open,
  onOpen,
  onHide,
  now,
}: {
  threads: ThreadSummary[]
  // The thread unfolded ("" none).
  open: string
  onOpen: (id: string) => void
  onHide: () => void
  now: Date
}) {
  const [showResolved, setShowResolved] = useState(false)
  const live = threads.filter((t) => t.state !== "resolved")
  const resolved = threads.filter((t) => t.state === "resolved")
  return (
    <aside
      className="grid min-w-0 content-start gap-3 overflow-hidden p-3 glass-panel"
      aria-label="Threads"
    >
      <div className="flex items-center justify-between">
        <h2 className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
          Threads{live.length > 0 && ` · ${live.length} open`}
        </h2>
        <Button
          size="icon-xs"
          variant="ghost"
          aria-label="Hide threads"
          title="Hide threads"
          onClick={onHide}
        >
          <XIcon />
        </Button>
      </div>
      {live.length === 0 && (
        <p className="text-[12.5px] text-muted-foreground">
          No open threads. Ask about anything in the box above: an agent on the
          cards you name answers, or an idle one does.
        </p>
      )}
      {live.map((t) => (
        <ThreadRow
          key={t.id}
          t={t}
          open={open === t.id}
          onToggle={() => onOpen(open === t.id ? "" : t.id)}
          now={now}
        />
      ))}
      {resolved.length > 0 && (
        <div className="grid gap-2 border-t border-border pt-3">
          <button
            type="button"
            className="flex items-center gap-1 justify-self-start text-xs text-muted-foreground hover:text-ink"
            onClick={() => setShowResolved((v) => !v)}
            aria-expanded={showResolved}
          >
            {showResolved ? (
              <ChevronDownIcon className="size-3.5" />
            ) : (
              <ChevronRightIcon className="size-3.5" />
            )}
            Resolved ({resolved.length})
          </button>
          {showResolved &&
            resolved.map((t) => (
              <ThreadRow
                key={t.id}
                t={t}
                open={open === t.id}
                onToggle={() => onOpen(open === t.id ? "" : t.id)}
                now={now}
              />
            ))}
        </div>
      )}
    </aside>
  )
}

// stateBadges label a thread's state on its title line (#128).
const stateBadges: Record<
  string,
  { label: string; variant: "info" | "success" | "warning" | "slate" }
> = {
  waiting: { label: "Waiting", variant: "warning" },
  handed: { label: "Answering", variant: "info" },
  answered: { label: "Answered", variant: "success" },
  resolved: { label: "Resolved", variant: "slate" },
}

// stateLine says where a folded thread stands.
function stateLine(t: ThreadSummary): string {
  switch (t.state) {
    case "answered":
      return t.lastBy && t.lastText
        ? `${displayName(t.lastBy)}: ${plainLine(t.lastText)}`
        : "Answered"
    case "handed":
      return t.told.length
        ? `${displayName(t.told[t.told.length - 1])} is answering`
        : "An idle agent is answering"
    case "resolved":
      return "Resolved"
    default:
      return t.told.length
        ? `Waiting for ${t.told.map(displayName).join(", ")}; an idle agent answers after 45s`
        : "Waiting for a free agent"
  }
}

function ThreadRow({
  t,
  open,
  onToggle,
  now,
}: {
  t: ThreadSummary
  open: boolean
  onToggle: () => void
  now: Date
}) {
  const at = parseTime(t.updatedAt)
  const waiting = t.state === "waiting" || t.state === "handed"
  const badge = stateBadges[t.state]
  return (
    <section
      className={
        open
          ? "grid min-w-0 gap-2 border border-primary bg-background/40 px-3 py-2.5"
          : "grid min-w-0 gap-1 border border-border bg-background/40 px-3 py-2.5"
      }
    >
      <button
        type="button"
        className="grid min-w-0 gap-0.5 text-left"
        onClick={onToggle}
        aria-expanded={open}
      >
        <span className="flex min-w-0 items-center gap-2">
          {t.short && (
            <span className="shrink-0 text-[11px] text-muted-foreground tabular-nums">
              #{plainLine(t.short)}
            </span>
          )}
          <span className="min-w-0 flex-1 truncate text-[13px] font-medium text-ink">
            {plainLine(t.title)}
          </span>
          {badge && (
            <Badge
              variant={badge.variant}
              className="h-4 shrink-0 px-1.5 text-[10.5px]"
            >
              {badge.label}
            </Badge>
          )}
          <time
            className="shrink-0 text-[11px] text-muted-foreground tabular-nums"
            dateTime={at?.toISOString()}
          >
            {relativeTime(at, now)}
          </time>
        </span>
        {!open && (
          <span
            className={
              waiting
                ? "line-clamp-2 text-xs text-warning"
                : "line-clamp-2 text-xs text-muted-foreground"
            }
          >
            {stateLine(t)}
          </span>
        )}
      </button>
      {open && <ThreadView t={t} now={now} />}
    </section>
  )
}

// ThreadView is an unfolded thread: what the person asked, the answers
// (the card's own thread, events left out), the reply box and Resolve.
function ThreadView({ t, now }: { t: ThreadSummary; now: Date }) {
  const { detail } = useCard(t.id)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const resolved = t.state === "resolved"
  const items = (detail?.thread ?? []).filter((it) => it.kind !== "event")
  const resolve = async () => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await threadsApi.resolve(t.id)
    } catch (err) {
      setError(errorText(err))
    } finally {
      setBusy(false)
    }
  }
  return (
    <div className="grid min-w-0 gap-2">
      <div className="grid min-w-0 gap-1 text-[12.5px]">
        <RichText text={t.text} className="break-words text-ink-2" />
        {t.told.length > 0 && (
          <span className="flex flex-wrap items-center gap-1 text-[11px] text-muted-foreground">
            {t.state === "waiting" ? "Asked first:" : "Told:"}
            {t.told.map((w) => (
              <Badge
                key={w.key}
                variant="slate"
                className="h-4 gap-1 px-1 text-[10.5px]"
              >
                <Face who={w} noDot />
                {displayName(w)}
              </Badge>
            ))}
          </span>
        )}
      </div>
      <Thread
        cardId={t.id}
        items={items}
        now={now}
        compact
        empty={
          t.state === "waiting"
            ? "No answer yet. If nobody answers in 45s, an idle agent picks it up."
            : "No answer yet."
        }
        composer={resolved ? undefined : <CommentBox cardId={t.id} reply />}
      />
      {!resolved && (
        <div className="flex items-center gap-2">
          <Button
            size="xs"
            variant="outline"
            disabled={busy}
            onClick={() => void resolve()}
            title="Done with it: the thread closes"
          >
            Resolve
          </Button>
          {error && <span className="text-xs text-destructive">{error}</span>}
        </div>
      )}
    </div>
  )
}

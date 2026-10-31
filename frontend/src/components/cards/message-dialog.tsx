import { useEffect, useState, type FormEvent } from "react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { MentionTextarea } from "@/components/common/mention-textarea"
import { parseTime, plainBlock, plainLine, relativeTime } from "@/lib/irori"
import { notDelivered, type Mention } from "@/lib/mentions"
import {
  errorText,
  holdText,
  MaxText,
  State,
  talk,
  type Delivery,
} from "@/lib/talk"

// MessageDialog is one message a notification's Open named (there is no
// inbox in the window): a message Irori held, with Release and Dismiss, or
// one an agent sent the person, in full, with Reply and Mark read. A reply
// can @mention other agents (card #77): each gets its own copy, woken. When
// one wasn't delivered, the dialog stays open to say so.
export function MessageDialog({
  id,
  name,
  now,
  onClose,
}: {
  id: string
  name: (key: string) => string
  now: Date
  onClose: () => void
}) {
  const [d, setD] = useState<Delivery | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [reply, setReply] = useState("")
  const [mentions, setMentions] = useState<Mention[]>([])
  const [missed, setMissed] = useState("")
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    let live = true
    talk.get(id).then(
      (x) => live && setD(x),
      (err) => live && setError(errorText(err))
    )
    return () => {
      live = false
    }
  }, [id])

  // act runs fn and closes, unless fn's answer says a mention wasn't
  // delivered: then the reply has gone, and the dialog says who missed it.
  const act = async (fn: () => Promise<unknown>) => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      const res = await fn()
      const line = Array.isArray(res) ? notDelivered(res) : ""
      if (line) {
        setReply("")
        setMentions([])
        setMissed(`Replied. ${line}`)
        return
      }
      onClose()
    } catch (err) {
      setError(errorText(err))
    } finally {
      setBusy(false)
    }
  }

  const submit = (e: FormEvent) => {
    e.preventDefault()
    if (!reply.trim()) {
      setError("Write a reply first.")
      return
    }
    void act(() => talk.reply(id, reply, mentions))
  }

  const from = d
    ? plainLine(d.message.from.name) || name(d.message.from.key)
    : ""
  const to = !d
    ? ""
    : d.to.startsWith("user:")
      ? "you"
      : d.to
        ? name(d.to)
        : `repo ${plainLine(d.repoKey ?? "")}`
  const held = d?.state === State.Held
  const forYou =
    !!d &&
    d.to.startsWith("user:") &&
    (d.state === State.Queued ||
      d.state === State.Delivered ||
      d.state === State.Engaged ||
      d.state === State.Deferred)
  const sent = parseTime(d?.message.sentAt)
  // An agent's message whose recipient had ended came to the person instead
  // (the inbox's recipient-gone fallback); a reply goes to its sender.
  const gone = d?.for?.startsWith("recipient gone") ? d.for : ""

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {!d
              ? error
                ? "Couldn't open the message"
                : "Opening the message…"
              : held
                ? `${from} sent ${to} a message Irori held`
                : `${from} sent ${to} a message`}
          </DialogTitle>
          {d && (
            <DialogDescription>
              {held && plainLine(holdText(d))}{" "}
              <time
                dateTime={sent?.toISOString()}
                title={sent?.toLocaleString()}
              >
                {relativeTime(sent, now)}
              </time>
              {!held && !forYou && ` · ${d.state}`}
            </DialogDescription>
          )}
        </DialogHeader>
        {gone && (
          <p className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
            <Badge variant="warning" className="h-4 px-1.5 text-[10.5px]">
              Recipient gone
            </Badge>
            {plainLine(gone)}
          </p>
        )}
        {d && (
          <div className="max-h-[50vh] overflow-y-auto border border-border bg-background/60 px-3 py-2.5 text-[13.5px] whitespace-pre-wrap text-ink">
            {plainBlock(d.message.text)}
          </div>
        )}
        {forYou && !missed && (
          <form id="message-reply" onSubmit={submit}>
            <MentionTextarea
              value={reply}
              mentions={mentions}
              maxLength={MaxText}
              onChange={(v, ms) => {
                setReply(v)
                setMentions(ms)
              }}
              placeholder={`Reply to ${from}`}
              aria-label={`Your reply to ${from}`}
              autoSize
              className="max-h-48"
              disabled={busy}
            />
          </form>
        )}
        {missed && <p className="text-xs text-destructive">{missed}</p>}
        {error && <p className="text-xs text-destructive">{error}</p>}
        {d && (held || forYou) && !missed && (
          <DialogFooter>
            {held ? (
              <>
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() => void act(() => talk.dismiss(id))}
                >
                  Dismiss
                </Button>
                <Button
                  disabled={busy}
                  onClick={() => void act(() => talk.release(id))}
                >
                  Release
                </Button>
              </>
            ) : (
              <>
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() => void act(() => talk.markRead(id))}
                >
                  Mark read
                </Button>
                <Button type="submit" form="message-reply" disabled={busy}>
                  {busy ? "Sending…" : "Reply"}
                </Button>
              </>
            )}
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  )
}

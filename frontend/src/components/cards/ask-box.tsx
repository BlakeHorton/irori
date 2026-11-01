import { useState, type FormEvent } from "react"

import { MentionTextarea } from "@/components/common/mention-textarea"
import { Button } from "@/components/ui/button"
import { useBoards } from "@/lib/boards"
import { MaxComment, threads } from "@/lib/cards"
import { useDraft, useMentionsDraft } from "@/lib/drafts"
import { composing } from "@/lib/mentions"
import { errorText } from "@/lib/talk"

// AskBox is the box over the grid that starts a thread (cards #116, #134):
// a question or a conversation with the room, not a card. The agents on
// the cards it names (#12) and the agents it @mentions hear it first,
// woken; if none answers within 45s, one idle agent gets it. It shows in
// the Threads panel, where the person replies and resolves it. Enter
// sends, Shift+Enter is a new line. On a shared board's page it starts the
// thread on that board: only that board's agents and people see it, the
// other people's agents hear it through their own devices, and its page
// lists it in its own Threads panel. A board thread takes no mentions (a
// board log carries none): name a card instead. No files on a thread yet.
// New cards come from the header's "+ New card".
export function AskBox({
  onThread,
  board: onPage = "",
}: {
  // onThread gets the new thread, for the Threads panel to open.
  onThread: (id: string) => void
  board?: string
}) {
  const draft = onPage ? `ask/board/${onPage}` : "ask/room"
  const [text, setText] = useDraft(draft)
  const [mentions, setMentions] = useMentionsDraft(draft)
  const { boards } = useBoards()
  // The page's board, while it is still a ready board of ours.
  const onBoard = boards.some(
    (b) => b.boardId === onPage && b.ready && !b.leaving
  )
    ? onPage
    : ""
  const [busy, setBusy] = useState(false)
  const [line, setLine] = useState<{ text: string; error?: boolean } | null>(
    null
  )

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    if (busy || !text.trim()) return
    if (onPage && !onBoard) {
      setLine({ text: "This board isn't ready here yet.", error: true })
      return
    }
    setBusy(true)
    setLine(null)
    try {
      const res = await threads.start(text, onBoard ? [] : mentions, onBoard)
      // On a board the other people's agents are told by their own
      // devices: count every agent the thread's round names.
      const n = onBoard
        ? (res.task.ask?.told ?? 0)
        : (res.told.to?.length ?? 0)
      setLine({
        text:
          n === 0
            ? "Sent. No agent is free yet: the first to go idle answers it."
            : `Sent to ${n === 1 ? "1 agent" : `${n} agents`}. If nobody answers in 45s, an idle agent picks it up.`,
      })
      setText("")
      setMentions([])
      onThread(res.task.id)
    } catch (err) {
      setLine({ text: errorText(err), error: true })
    } finally {
      setBusy(false)
    }
  }

  return (
    <form className="grid gap-1" onSubmit={(e) => void submit(e)}>
      <div className="flex items-start gap-2 p-1.5 glass-panel">
        <MentionTextarea
          autoSize
          rows={1}
          value={text}
          mentions={onBoard ? [] : mentions}
          disabledMentions={
            onBoard
              ? "Mentions work in threads on your own board; name a card (#12) here"
              : undefined
          }
          maxLength={MaxComment}
          onChange={(v, ms) => {
            setText(v)
            setMentions(ms)
            setLine(null)
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && !composing(e)) {
              e.preventDefault()
              e.currentTarget.form?.requestSubmit()
            }
          }}
          placeholder={
            onBoard
              ? "Ask about anything… (# a card)"
              : "Ask about anything… (# a card, @ an agent)"
          }
          aria-label="Start a thread"
          className="max-h-40 min-h-9 flex-1 border-transparent bg-transparent shadow-none focus-visible:border-transparent"
          // readOnly, not disabled: disabling the focused box would blur it,
          // and the next message would need a click back in.
          readOnly={busy}
          aria-busy={busy}
        />
        {/* The button rides the box's first line (h-9, its min height)
            and stays up top as it grows. */}
        <div className="flex h-9 shrink-0 items-center gap-2">
          <Button
            type="submit"
            size="sm"
            className="shrink-0"
            disabled={busy || !text.trim()}
          >
            {busy ? "Sending…" : "Send"}
          </Button>
        </div>
      </div>
      {line && (
        <p
          className={
            line.error
              ? "px-1 text-xs text-destructive"
              : "px-1 text-xs text-muted-foreground"
          }
          role={line.error ? "alert" : undefined}
        >
          {line.text}
        </p>
      )}
    </form>
  )
}

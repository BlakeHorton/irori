import { useEffect, useState, type FormEvent } from "react"

import {
  AttachButton,
  PendingFiles,
  useFileAttach,
} from "@/components/attachments"

import { ChoiceSelect } from "@/components/common/choice-select"
import { MentionTextarea } from "@/components/common/mention-textarea"
import { Button } from "@/components/ui/button"
import { useBoards } from "@/lib/boards"
import {
  cards,
  MaxComment,
  threads,
  type AskRepo,
} from "@/lib/cards"
import { useDraft, useMentionsDraft } from "@/lib/drafts"
import { askingLine, composing, notDelivered } from "@/lib/mentions"
import { errorText } from "@/lib/talk"

// AskBox is the person talking to the room (docs/design/board.md "Asks"):
// one line over the grid. What they type goes to every free agent, one of
// them takes it, and it shows first in the grid as an ask until it becomes
// a card or closes. It is the card comment box lifted out: Enter sends,
// Shift+Enter is a new line. It always wakes them: only idle agents hear
// an ask, and an idle agent gets no next turn until someone types in its
// window. The repo chip keeps it to one repo's agents. Files go with it: pick them with the clip, drop them
// on the box, or paste a screenshot (card #25); pictures show as
// thumbnails before they go, other files an icon for their kind (#73). The window stays on the
// grid. On a shared board's page it goes on that board (v0.5 §9, cards
// #59, #96): every member's free agents hear it, the other people's through
// their own devices. There is no board picker (card #102): to ask on
// another board, go to that board. A board's agents carry no repo, and a
// board ask takes no files yet, so the repo chip and the clip step aside.
// A thread (card #116) takes files like an ask (card #124).
// An @mention (card #77) narrows it: only the agents it names hear it,
// woken, in any repo, so the repo chip steps aside too. Mentions work on
// your own board only.
export function AskBox({
  onAsked,
  onThread,
  board: onPage = "",
}: {
  onAsked: (id: string) => void
  // On your own board the box starts a thread (card #116): onThread gets
  // it, for the Threads panel to open.
  onThread?: (id: string) => void
  board?: string
}) {
  const [text, setText] = useDraft("ask/room")
  const [mentions, setMentions] = useMentionsDraft("ask/room")
  const [repo, setRepo] = useState("")
  const [repos, setRepos] = useState<AskRepo[]>([])
  const { boards } = useBoards()
  // The page's board, while it is still a ready board of ours.
  const onBoard = boards.some(
    (b) => b.boardId === onPage && b.ready && !b.leaving
  )
    ? onPage
    : ""
  // A board ask takes no mentions in v1.
  const targeted = onBoard ? [] : mentions
  // Your own board's box starts a thread, not an ask (card #116); a shared
  // board's page keeps the ask.
  const thread = !onBoard && !!onThread
  const [busy, setBusy] = useState(false)
  // A thread takes files as an ask does (card #124).
  const attach = useFileAttach(busy)
  const files = attach.files
  const [line, setLine] = useState<{ text: string; error?: boolean } | null>(
    null
  )

  useEffect(() => {
    let live = true
    cards
      .askRepos()
      .then((rs) => live && setRepos(rs))
      .catch(() => {})
    return () => {
      live = false
    }
  }, [])

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    if (busy || !text.trim()) return
    if (onBoard && files.length) {
      setLine({
        text: "Files don't go on a board ask yet: remove them, or ask on My board.",
        error: true,
      })
      return
    }
    setBusy(true)
    setLine(null)
    if (thread) {
      try {
        const res = await threads.start(text, mentions, files)
        const n = res.told.to?.length ?? 0
        setLine({
          text:
            n === 0
              ? "Sent. No agent is free yet: the first to go idle answers it."
              : `Sent to ${n === 1 ? "1 agent" : `${n} agents`}. If nobody answers in 45s, an idle agent picks it up.`,
        })
        setText("")
        setMentions([])
        attach.clear()
        onThread?.(res.task.id)
      } catch (err) {
        setLine({ text: errorText(err), error: true })
      } finally {
        setBusy(false)
      }
      return
    }
    try {
      const res = onBoard
        ? await cards.ask(text, "", true, [], onBoard)
        : await cards.ask(text, targeted.length ? "" : repo, true, files, "", targeted)
      // On a board the other people's agents are told by their own
      // devices: count every agent the ask's round names.
      const n = onBoard ? (res.task.ask?.told ?? 0) : (res.told.to?.length ?? 0)
      const missed = notDelivered(res.mentioned)
      setLine({
        text: targeted.length
          ? n === 0
            ? `Nobody you mentioned could hear it; it stays first on the board. ${missed}`.trim()
            : `${n === 1 ? "Woke the agent you mentioned." : `Woke ${n} agents you mentioned; one of them takes it.`} ${missed}`.trim()
          : n === 0
            ? "Nobody is free to hear it yet; it stays first on the board."
            : n === 1
              ? "Woke 1 agent."
              : `Woke ${n} agents; one of them takes it.`,
        error: !!missed && n === 0,
      })
      setText("")
      setMentions([])
      attach.clear()
      onAsked(res.task.id)
    } catch (err) {
      setLine({ text: errorText(err), error: true })
    } finally {
      setBusy(false)
    }
  }

  return (
    <form
      className="grid gap-1"
      onSubmit={(e) => void submit(e)}
      {...attach.drop}
    >
      <div className="flex items-start gap-2 p-1.5 glass-panel">
        <MentionTextarea
          autoSize
          rows={1}
          value={text}
          mentions={mentions}
          disabledMentions={
            onBoard ? "Mentions work on your own board's asks" : undefined
          }
          maxLength={MaxComment}
          onChange={(v, ms) => {
            setText(v)
            setMentions(ms)
            setLine(null)
          }}
          onPaste={(e) => {
            if (attach.onPaste(e)) setLine(null)
          }}
          onKeyDown={(e) => {
            if (
              e.key === "Enter" &&
              !e.shiftKey &&
              !composing(e)
            ) {
              e.preventDefault()
              e.currentTarget.form?.requestSubmit()
            }
          }}
          placeholder={
            thread
              ? "Ask about anything…"
              : "Ask the room…"
          }
          aria-label={
            thread
              ? "Start a thread"
              : targeted.length
                ? askingLine(targeted)
                : "Ask every free agent"
          }
          className="max-h-40 min-h-9 flex-1 border-transparent bg-transparent shadow-none focus-visible:border-transparent"
          // readOnly, not disabled: disabling the focused box would blur it,
          // and the next ask would need a click back in.
          readOnly={busy}
          aria-busy={busy}
        />
        {/* The controls ride the box's first line (h-9, its min height)
            whatever their own heights, and stay up top as it grows. */}
        <div className="flex h-9 shrink-0 items-center gap-2">
          {!thread && targeted.length > 0 && (
            <span
              className="max-w-56 truncate text-xs text-muted-foreground"
              title="Only the agents you mention hear it, in any repo"
            >
              {askingLine(targeted)}
            </span>
          )}
          {!thread && !onBoard && !targeted.length && repos.length > 0 && (
            <ChoiceSelect
              label="Which agents hear it"
              size="sm"
              value={repo}
              onValueChange={setRepo}
              disabled={busy}
              className="w-auto shrink-0"
              choices={[
                { value: "", label: "Any repo" },
                ...repos.map((r) => ({ value: r.repoKey, label: r.label })),
              ]}
            />
          )}
          {attach.input}
          <AttachButton attach={attach} disabled={busy || !!onBoard} />
          <Button
            type="submit"
            size="sm"
            className="shrink-0"
            disabled={busy || !text.trim()}
          >
            {busy ? (thread ? "Sending…" : "Asking…") : thread ? "Send" : "Ask"}
          </Button>
        </div>
      </div>
      <PendingFiles
        attach={attach}
        disabled={busy}
        label={thread ? "Files with the thread" : "Files with the ask"}
      />
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

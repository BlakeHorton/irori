// BoardPage is one board as its own page (card #96): the board's cards
// (List or Canvas, with Needs a decision and Closed) on the left, and on
// the right who is on it, Invite someone (owners), and the board's
// settings. Rename, Leave and Forget sit by its name.

import { useState, type FormEvent } from "react"
import { ArrowLeftIcon } from "lucide-react"

import { AskBox } from "@/components/cards/ask-box"
import { CardGrid, DoneGrid, shownCards } from "@/components/cards/card-grid"
import { BoardCanvas, ViewSwitch, type BoardView } from "@/components/canvas/switch"
import { ConfirmButton } from "@/components/common/confirm-button"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  boards as actions,
  MaxBoardName,
  useBoards,
  useInvite,
  type BoardSummary,
} from "@/lib/boards"
import type { CardSummary, DoneState } from "@/lib/cards"
import { plainLine } from "@/lib/irori"
import { go } from "@/lib/nav"
import { errorText } from "@/lib/talk"
import { cn } from "@/lib/utils"

import { BoardSettingsPanel, InviteHow, InvitePanel, useHomeOff } from "./parts"

type Tab = "all" | "decide" | "done"

export function BoardPage({
  boardId,
  cards,
  done,
  tab,
  onTab,
  view,
  onView,
  selected,
  onOpen,
  now,
}: {
  boardId: string
  cards: CardSummary[]
  done: DoneState
  tab: Tab
  onTab: (t: Tab) => void
  view: BoardView
  onView: (v: BoardView) => void
  selected: string
  onOpen: (id: string) => void
  now: Date
}) {
  const { boards, reload } = useBoards()
  const b = boards.find((x) => x.boardId === boardId)
  if (!b)
    return (
      <div className="grid gap-2">
        <BackToBoards />
        <p className="text-sm text-muted-foreground">
          This board isn't here any more: you may have left or forgotten it.
        </p>
      </div>
    )
  const own = cards.filter((c) => c.board === boardId)
  const decide = own.filter((c) => c.openBlockers > 0).length
  const { shown, context } = shownCards(
    own,
    tab === "decide" ? "decide" : tab === "done" ? "done" : "all"
  )
  const closed = { ...done, cards: done.cards.filter((c) => c.board === boardId) }
  return (
    <div className="grid gap-3.5">
      <BoardHead board={b} onChanged={reload} />
      <div className="grid items-start gap-4 lg:grid-cols-[minmax(0,1fr)_340px]">
        <div className="grid min-w-0 gap-3.5">
          <AskBox board={boardId} onAsked={() => onTab("all")} />
          <div className="flex flex-wrap items-center gap-1" role="group" aria-label="Filter cards">
            {(
              [
                ["all", "All"],
                ["decide", "Needs a decision"],
                ["done", "Closed"],
              ] as const
            ).map(([id, label]) => (
              <Button
                key={id}
                size="sm"
                aria-pressed={tab === id}
                variant={tab === id ? "default" : "ghost"}
                onClick={() => onTab(id)}
              >
                {label}
                {id === "decide" && decide > 0 && (
                  <span className="text-destructive tabular-nums">{decide}</span>
                )}
              </Button>
            ))}
            <span className="ml-auto">
              <ViewSwitch view={view} onView={onView} />
            </span>
          </div>
          {tab === "done" ? (
            <DoneGrid done={closed} selected={selected} onSelect={onOpen} now={now} />
          ) : view === "canvas" ? (
            <BoardCanvas cards={shown} onOpen={onOpen} />
          ) : shown.length ? (
            <CardGrid cards={shown} context={context} selected={selected} onSelect={onOpen} />
          ) : (
            <p className="px-1 text-[12.5px] text-muted-foreground">
              {tab === "decide"
                ? "Nothing on this board waits on you."
                : "No cards on this board yet. Ask the room above, with this board picked, or share a card to it from its page."}
            </p>
          )}
        </div>
        <PeoplePanel board={b} onChanged={reload} />
      </div>
    </div>
  )
}

function BackToBoards() {
  return (
    <Button variant="ghost" size="sm" className="w-fit px-1.5" onClick={() => go({ kind: "boards" })}>
      <ArrowLeftIcon aria-hidden />
      Boards
    </Button>
  )
}

// BoardHead is the board's name and tag, its state, and the actions on
// the board itself: Rename (owners), Leave, Forget here.
function BoardHead({ board: b, onChanged }: { board: BoardSummary; onChanged: () => Promise<void> }) {
  const [renaming, setRenaming] = useState(false)
  const [name, setName] = useState(b.name)
  const [error, setError] = useState<string | null>(null)
  const rename = async (e: FormEvent) => {
    e.preventDefault()
    setError(null)
    try {
      await actions.rename(b.boardId, name)
      setRenaming(false)
      await onChanged()
    } catch (err) {
      setError(errorText(err))
    }
  }
  const ok = b.ready && !b.warnings?.length
  return (
    <div className="grid gap-1.5">
      <BackToBoards />
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        {renaming ? (
          <form className="flex gap-1.5" onSubmit={rename}>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={MaxBoardName}
              aria-label="Board name"
              className="h-8 w-64"
              autoFocus
            />
            <Button type="submit" size="sm" className="h-8" disabled={!name.trim()}>
              Save
            </Button>
            <Button type="button" variant="ghost" size="sm" className="h-8" onClick={() => setRenaming(false)}>
              Cancel
            </Button>
          </form>
        ) : (
          <h1 className="flex items-baseline gap-2 text-lg font-semibold text-ink">
            <span className={cn("size-2 self-center rounded-full", ok ? "bg-success" : "bg-warning")} aria-hidden />
            {plainLine(b.name) || "(unnamed)"}
            <span className="font-mono text-sm font-normal text-muted-foreground">{plainLine(b.tag)}#</span>
          </h1>
        )}
        <span className="text-xs text-muted-foreground">
          {b.leaving
            ? "Leaving…"
            : b.ready
              ? `${b.cards === 1 ? "1 card" : `${b.cards} cards`} · visible to ${b.people.map(plainLine).join(", ")}`
              : "Its cards are still arriving from your other device"}
        </span>
        <span className="ml-auto flex items-center gap-1.5">
          {b.owner && !renaming && (
            <Button variant="ghost" size="sm" onClick={() => setRenaming(true)}>
              Rename
            </Button>
          )}
          {b.leaving ? (
            <ConfirmButton
              label="Forget now"
              variant="destructive"
              confirmLabel="Forget it now"
              question={
                <>
                  Forget <b>{plainLine(b.name)}</b> on every one of your devices now? The others may not hear
                  that you left until one of their devices next reaches yours, which may be never.
                </>
              }
              onConfirm={async () => {
                await actions.forgetNow(b.boardId)
                go({ kind: "boards" })
              }}
            />
          ) : (
            <ConfirmButton
              label="Leave"
              variant="destructive"
              confirmLabel="Leave the board"
              question={
                b.people.length > 1 ? (
                  <>
                    Leave <b>{plainLine(b.name)}</b>? It goes from every one of your devices once another
                    member's Irori has your leave, and nothing you write there counts after it. Coming back
                    needs a new invite.
                  </>
                ) : (
                  <>
                    Leave <b>{plainLine(b.name)}</b>? It goes from every one of your devices, with its cards.
                    Coming back needs a new board.
                  </>
                )
              }
              onConfirm={async () => {
                await actions.leave(b.boardId)
                await onChanged()
              }}
            />
          )}
          <ConfirmButton
            label="Forget here"
            confirmLabel="Forget on this device"
            question={
              <>
                Forget <b>{plainLine(b.name)}</b> on this device only? Its file goes from here; your other
                devices keep it.
              </>
            }
            onConfirm={async () => {
              await actions.forget(b.boardId)
              go({ kind: "boards" })
            }}
          />
        </span>
      </div>
      {b.tagClash && (
        <p className="text-xs text-destructive">
          Another board is also {plainLine(b.tag)}#: its refs need the card's id.
        </p>
      )}
      {b.warnings?.map((w, i) => (
        <p key={i} className="text-xs text-warning">
          {plainLine(w)}
        </p>
      ))}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  )
}

// PeoplePanel is who is on the board and how to get more people on it:
// Invite someone (an owner, once the board is ready) with how an invite
// works, else why there is no Invite; then the members and the board's
// settings. An open invite takes the panel over until it ends.
function PeoplePanel({ board: b, onChanged }: { board: BoardSummary; onChanged: () => Promise<void> }) {
  const { invite, setInvite } = useInvite(true)
  const homeOff = useHomeOff(true)
  const [error, setError] = useState<string | null>(null)
  const inviting = !!invite && invite.state !== ""
  return (
    <aside className="glass-panel grid gap-3 p-4 text-[13px]" aria-label="People on this board">
      <h2 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">People</h2>
      {inviting ? (
        <InvitePanel
          s={invite}
          onClose={() => {
            setInvite(null)
            void onChanged()
          }}
        />
      ) : (
        <>
          {!b.owner ? (
            <p className="text-xs text-muted-foreground">
              Only an owner can invite people to this board: ask one of them.
            </p>
          ) : !b.ready ? (
            <p className="text-xs text-muted-foreground">
              You can invite people once its cards have arrived from your other device.
            </p>
          ) : (
            !b.leaving && (
              <div className="grid gap-2">
                <Button
                  className="w-fit"
                  onClick={async () => {
                    setError(null)
                    try {
                      setInvite(await actions.invite(b.boardId))
                    } catch (err) {
                      setError(errorText(err))
                    }
                  }}
                >
                  Invite someone
                </Button>
                <InviteHow homeOff={homeOff} />
              </div>
            )
          )}
          {error && <p className="text-xs text-destructive">{error}</p>}
          {b.ready && !b.leaving && (
            <div className="border-t border-border pt-3">
              <BoardSettingsPanel boardId={b.boardId} onChanged={onChanged} />
            </div>
          )}
        </>
      )}
    </aside>
  )
}

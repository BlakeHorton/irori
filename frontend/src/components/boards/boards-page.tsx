// BoardsPage is where boards live (card #96): each board a project (say
// ww-multiplayer) you share with people you invite. A board opens as its
// own page; here you see them all, make a new one, or join one with an
// invite someone sent. Everything is private: only invited people see a
// board.

import { useBoards, useInvite, type BoardSummary } from "@/lib/boards"
import type { CardSummary } from "@/lib/cards"
import { plainLine } from "@/lib/irori"
import { go } from "@/lib/nav"
import { cn } from "@/lib/utils"

import { InvitePanel, JoinInvite, NewBoard } from "./parts"

export function BoardsPage({ cards }: { cards: CardSummary[] }) {
  const { boards, error, reload } = useBoards()
  const { invite, setInvite } = useInvite(true)
  const inviting = !!invite && invite.state !== ""
  return (
    <div className="grid gap-4">
      <div className="grid gap-1">
        <h1 className="text-lg font-semibold text-ink">Boards</h1>
        <p className="max-w-2xl text-[13px] text-muted-foreground">
          A board is a project you share: its cards, the people on it and
          their agents. Make one per project; anyone can be on as many as
          they like. Boards are private: only the people you invite see them.
        </p>
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
      {/* An open invite sits over the list, never in its place (card
          #103): your boards stay one click away while it waits. */}
      {inviting && (
        <div className="glass-panel max-w-xl p-4">
          <InvitePanel
            s={invite}
            onClose={() => {
              setInvite(null)
              void reload()
            }}
          />
        </div>
      )}
      {boards.length > 0 ? (
        <ul
          className="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(280px,1fr))]"
          aria-label="Your boards"
        >
          {boards.map((b) => (
            <BoardTile key={b.boardId} board={b} cards={cards} />
          ))}
        </ul>
      ) : (
        <p className="text-[13px] text-muted-foreground">
          No boards yet. Make one for a project below, then invite people
          to it; or join one with an invite link someone sent you.
        </p>
      )}
      <div className="grid max-w-xl gap-3">
        <NewBoard taken={boards.map((b) => b.tag)} onMade={reload} />
        {/* One invite at a time: joining waits until this one ends. */}
        {!inviting && <JoinInvite onJoined={setInvite} />}
      </div>
    </div>
  )
}

// BoardTile is one board on the Boards page: name, tag, who is on it, how
// many cards wait on you; it opens the board's page.
function BoardTile({ board: b, cards }: { board: BoardSummary; cards: CardSummary[] }) {
  const mine = cards.filter((c) => c.board === b.boardId)
  const decide = mine.filter((c) => c.openBlockers > 0).length
  const ok = b.ready && !b.warnings?.length
  return (
    <li>
      <button
        type="button"
        className="glass-panel grid w-full gap-1.5 p-3 text-left transition-colors hover:border-primary/60 focus-visible:outline-2 focus-visible:outline-primary"
        onClick={() => go({ kind: "board", id: b.boardId })}
      >
        <span className="flex items-center gap-2">
          <span className={cn("size-2 shrink-0 rounded-full", ok ? "bg-success" : "bg-warning")} aria-hidden />
          <span className="min-w-0 truncate text-sm font-medium text-ink">
            {plainLine(b.name) || "(unnamed)"}
          </span>
          <span className="font-mono text-xs text-muted-foreground">{plainLine(b.tag)}#</span>
          {b.owner && (
            <span className="ml-auto rounded bg-muted px-1 text-[11px] text-muted-foreground">owner</span>
          )}
        </span>
        <span className="text-xs text-muted-foreground">
          {b.leaving
            ? "Leaving…"
            : b.ready
              ? `${b.cards === 1 ? "1 card" : `${b.cards} cards`} · ${b.people.map(plainLine).join(", ")}`
              : "Arriving from your other device"}
        </span>
        {decide > 0 && (
          <span className="text-xs text-destructive">
            {decide === 1 ? "1 card needs" : `${decide} cards need`} a decision
          </span>
        )}
        {b.warnings?.slice(0, 1).map((w, i) => (
          <span key={i} className="text-xs text-warning">
            {plainLine(w)}
          </span>
        ))}
      </button>
    </li>
  )
}

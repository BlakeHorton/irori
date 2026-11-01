// BoardView is one board's cards as a page (card #161): the header box,
// the filter tabs and device chips, the Threads panel, Pinned, the grid
// or canvas, and Closed. Home is BoardView over your own cards; a shared
// board's page is BoardView over that board's, beside its People panel.
// Each feature lands here once and shows on both.
//
// The filter and view live in App (they survive a card's page and a
// notification's Open); the cards are every live card, and BoardView
// keeps the scope's; Closed comes in already scoped (useDoneCards).

import type { ReactNode } from "react"

import {
  BoardBar,
  CardGrid,
  DeviceBar,
  DoneGrid,
  FilterBar,
  asFilter,
  shownCards,
  type Filter,
} from "@/components/cards/card-grid"
import { SearchBox } from "@/components/cards/search-box"
import { ThreadsPanel } from "@/components/cards/threads-panel"
import {
  BoardCanvas,
  ViewSwitch,
  type BoardView as View,
} from "@/components/canvas/switch"
import { Badge } from "@/components/ui/badge"
import { TaskState } from "@/lib/models"
import { Button } from "@/components/ui/button"
import {
  cards as cardsApi,
  deviceChips,
  type CardSummary,
  type DoneState,
  type ThreadSummary,
} from "@/lib/cards"

// ScopeOwn is Home's scope: your own cards. Any other scope is a board id.
export const ScopeOwn = "own"

// inScope says whether card c belongs on scope's page. Home (card #199)
// is your private cards and every shared board's card one of your agents
// holds or watches (ours).
export function inScope(c: CardSummary, scope: string): boolean {
  return scope === ScopeOwn
    ? c.scope !== "board" || !!c.ours
    : c.board === scope
}

// homeBoards is Home's board row (card #199): each board with a card on
// Home, in the order they first appear.
export function homeBoards(
  cards: CardSummary[]
): { boardId: string; name: string; tag: string }[] {
  const out: { boardId: string; name: string; tag: string }[] = []
  for (const c of cards) {
    if (c.scope !== "board" || !c.board || out.some((b) => b.boardId === c.board))
      continue
    out.push({ boardId: c.board, name: c.scopeLabel, tag: c.tag ?? "" })
  }
  return out
}

// activeFilter is filter as the page can show it: a device chip whose
// device has no cards left, or Mine fall back to All. A board filter and
// Private are Home's (boards: its board row, card #199): kept while that
// board has cards there; elsewhere they fall back to All.
export function activeFilter(
  filter: Filter,
  chips: { id: string }[],
  boards?: { boardId: string }[]
): Filter {
  if (
    filter.startsWith("device:") &&
    !chips.some((d) => filter === `device:${d.id}`)
  )
    return "all"
  if (filter === "private") return boards ? filter : "all"
  if (filter.startsWith("board:"))
    return boards?.some((b) => filter === `board:${b.boardId}`) ? filter : "all"
  if (filter === "mine") return "all"
  return filter
}

// Home remembers its last board pick per viewer (card #199): All, Private
// or one board, kept in this window's localStorage under homeFilterKey.
// The other tabs (Needs a decision, Closed) are not remembered.
export const homeFilterKey = "irori.homeFilter"

// rememberedFilter says whether f is a pick Home keeps.
export function rememberedFilter(f: Filter): boolean {
  return f === "all" || f === "private" || f.startsWith("board:")
}

// savedHomeFilter is the pick Home starts on from what was saved: a
// remembered pick, else All.
export function savedHomeFilter(saved: string | null): Filter {
  if (!saved) return "all"
  const f = asFilter(saved)
  return rememberedFilter(f) ? f : "all"
}

export type ThreadsProps = {
  list: ThreadSummary[]
  open: string
  onOpen: (id: string) => void
  hidden: boolean
  onHide: (hidden: boolean) => void
  // why says why a waiting thread has no agent yet (card #172).
  why?: string
}

export function BoardView({
  scope,
  cards,
  done,
  filter,
  onFilter,
  view,
  onView,
  query,
  onQuery,
  selected,
  onOpen,
  now,
  header,
  notice,
  empty,
  nothingDecide,
  threads,
  aside,
}: {
  scope: string
  cards: CardSummary[]
  done: DoneState
  filter: Filter
  onFilter: (f: Filter) => void
  view: View
  onView: (v: View) => void
  // query is the search box's words (card #171), kept in App like the view.
  query: string
  onQuery: (q: string) => void
  selected: string
  onOpen: (id: string) => void
  now: Date
  // header is the box above the board (Ask about anything…).
  header?: ReactNode
  // notice sits under the filters (Home: a board's decisions waiting).
  notice?: ReactNode
  // empty is what an empty board shows.
  empty: ReactNode
  // nothingDecide is Needs a decision with nothing in it.
  nothingDecide?: string
  threads?: ThreadsProps
  // aside is a right column under the Threads panel (a board's People).
  aside?: ReactNode
}) {
  const mine = cards.filter((c) => inScope(c, scope))
  const chips = deviceChips(mine)
  // Home's board row (card #199); a board page has none.
  const rowBoards = scope === ScopeOwn ? homeBoards(mine) : undefined
  const active = activeFilter(filter, chips, rowBoards)
  const { shown, context } = shownCards(mine, active, query)
  // The Pinned group: pinned cards in their order, each pinned parent with
  // its subtasks (as BoardService.Cards lists them).
  const pinnedGroup = shown.filter(
    (c) =>
      c.pinned ||
      (c.parent && shown.some((p) => p.id === c.parent?.id && p.pinned))
  )
  const inPinned = new Set(pinnedGroup.map((c) => c.id))
  const decide = mine.filter((c) => c.openBlockers > 0).length
  const review = mine.filter((c) => c.state === TaskState.InReview).length

  const openThreads =
    threads?.list.filter((t) => t.state !== "resolved").length ?? 0
  const showThreads =
    !!threads && !threads.hidden && (threads.list.length > 0 || !!threads.open)
  // The app is exactly the window's height (card #170): beside the Board
  // the Threads column scrolls on its own; without it the Board does.
  const panel = showThreads && threads && (
    <ThreadsPanel
      threads={threads.list}
      open={threads.open}
      onOpen={threads.onOpen}
      onHide={() => threads.onHide(true)}
      now={now}
      why={threads.why}
    />
  )
  const withThreads = (main: ReactNode) =>
    aside ? (
      <div className="flex min-h-0 flex-1 flex-col gap-4 lg:flex-row">
        <div className={boardColumn}>{main}</div>
        <div className="-mr-1 grid min-h-0 shrink-0 content-start gap-3.5 overflow-y-auto pr-1 pb-4 lg:w-[360px]">
          {panel}
          {aside}
        </div>
      </div>
    ) : showThreads && threads ? (
      <div className="flex min-h-0 flex-1 gap-4">
        <div className={boardColumn}>{main}</div>
        <div className="-mr-1 min-h-0 w-[380px] shrink-0 overflow-y-auto pr-1 pb-4">
          <ThreadsPanel
            threads={threads.list}
            open={threads.open}
            onOpen={threads.onOpen}
            onHide={() => threads.onHide(true)}
            now={now}
            why={threads.why}
          />
        </div>
      </div>
    ) : (
      <div className={boardColumn}>{main}</div>
    )

  if (mine.length === 0 && filter !== "done")
    return (
      <>
        {header}
        {withThreads(
          <>
            <FilterBar
              filter={filter}
              onFilter={onFilter}
              decide={0}
              only={["all", "done"]}
            />
            {notice}
            {empty}
          </>
        )}
      </>
    )

  return (
    <>
      {header}
      {withThreads(
        <>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
            <FilterBar
              filter={active}
              onFilter={onFilter}
              decide={decide}
              review={review}
              only={["all", "decide", "review", "done"]}
            />
            <DeviceBar filter={active} onFilter={onFilter} devices={chips} />
            {rowBoards && (
              <BoardBar
                filter={active}
                onFilter={onFilter}
                boards={rowBoards}
                withPrivate
              />
            )}
            <span className="ml-auto flex items-center gap-2">
              {threads && !showThreads && threads.list.length > 0 && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => threads.onHide(false)}
                  title="Show the Threads panel"
                >
                  Threads
                  {openThreads > 0 && (
                    <Badge variant="slate" className="tabular-nums">
                      {openThreads}
                    </Badge>
                  )}
                </Button>
              )}
              <SearchBox query={query} onQuery={onQuery} />
              <ViewSwitch view={view} onView={onView} />
            </span>
          </div>
          {notice}
          {active === "done" ? (
            <DoneGrid
              done={done}
              byAge={scope}
              query={query}
              selected={selected}
              onSelect={onOpen}
              now={now}
            />
          ) : view === "canvas" ? (
            <BoardCanvas cards={shown} onOpen={onOpen} />
          ) : shown.length ? (
            <>
              {pinnedGroup.length > 0 && (
                <>
                  <h2 className="px-1 text-xs font-medium tracking-wide text-muted-foreground uppercase">
                    Pinned
                  </h2>
                  <CardGrid
                    label="Pinned cards"
                    cards={pinnedGroup}
                    context={context}
                    selected={selected}
                    onSelect={onOpen}
                    onMove={(id, before) => void cardsApi.movePin(id, before)}
                  />
                  <h2 className="px-1 pt-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
                    Everything else
                  </h2>
                </>
              )}
              <CardGrid
                cards={shown.filter((c) => !inPinned.has(c.id))}
                context={context}
                selected={selected}
                onSelect={onOpen}
              />
            </>
          ) : (
            <p className="px-1 text-[12.5px] text-muted-foreground">
              {query.trim()
                ? "No cards match."
                : active === "decide" && nothingDecide
                  ? nothingDecide
                  : "Nothing here with this filter."}
            </p>
          )}
        </>
      )}
    </>
  )
}

// The Board's column (card #170): it scrolls by itself under the box.
// The 4px gutter keeps a selected card's ring clear of the clip.
const boardColumn =
  "-mx-1 grid min-h-0 min-w-0 flex-1 content-start gap-3.5 overflow-y-auto px-1 pb-10"

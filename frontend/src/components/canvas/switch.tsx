// The Board's view switch (card #93): List (the grid) or Canvas (the board
// as a flow chart you can pan, zoom and arrange). The canvas is its own
// lazy chunk, so React Flow only loads when Canvas is picked. A card's spot
// is kept in coord (cards.place), so every device shows the same layout.

import { lazy, Suspense, useEffect, useMemo, useState } from "react"

import { Button } from "@/components/ui/button"
import { cards as cardsApi, type CardDetail, type CardSummary } from "@/lib/cards"
import { previewFlag } from "@/lib/preview"

import { autoLayout, toGraph, withPlaced } from "./data"

export type BoardView = "list" | "canvas"

const views: { id: BoardView; label: string }[] = [
  { id: "list", label: "List" },
  { id: "canvas", label: "Canvas" },
]

const Canvas = lazy(() => import("./reactflow"))

// The picked view is a per-window convenience, kept in this browser.
const viewKey = "irori.board.view"

function asView(v: string | null): BoardView {
  return views.find((x) => x.id === v)?.id ?? "list"
}

// useBoardView is the picked view: ?view=<id> in the preview, else the one
// last picked here, else List.
export function useBoardView(): [BoardView, (v: BoardView) => void] {
  const [view, setView] = useState<BoardView>(() => {
    const q = previewFlag("view")
    if (q) return asView(q)
    try {
      return asView(localStorage.getItem(viewKey))
    } catch {
      return "list"
    }
  })
  const pick = (v: BoardView) => {
    setView(v)
    try {
      localStorage.setItem(viewKey, v)
    } catch {
      // not kept
    }
  }
  return [view, pick]
}

export function ViewSwitch({
  view,
  onView,
}: {
  view: BoardView
  onView: (v: BoardView) => void
}) {
  return (
    <div className="flex flex-wrap items-center gap-1" role="group" aria-label="Board view">
      {views.map((v) => (
        <Button
          key={v.id}
          size="sm"
          aria-pressed={view === v.id}
          variant={view === v.id ? "secondary" : "ghost"}
          onClick={() => onView(v.id)}
        >
          {v.label}
        </Button>
      ))}
    </div>
  )
}

// useBlockerDetails loads the page data of every card with open or
// answered blockers, so each blocker is its own node with its question.
// It reloads when one of those cards changes.
function useBlockerDetails(list: CardSummary[]): Map<string, CardDetail> {
  const want = list.filter((c) => c.openBlockers > 0 || c.waiting > 0)
  const key = want.map((c) => `${c.id}@${c.updatedAt}`).join(",")
  const [got, setGot] = useState<Map<string, CardDetail>>(new Map())
  useEffect(() => {
    let live = true
    const ids = key ? key.split(",").map((k) => k.split("@")[0]) : []
    Promise.all(ids.map((id) => cardsApi.detail(id).catch(() => null))).then((ds) => {
      if (!live) return
      const m = new Map<string, CardDetail>()
      for (const d of ds) if (d) m.set(d.id, d)
      setGot(m)
    })
    return () => {
      live = false
    }
  }, [key])
  return got
}

// BoardCanvas is the canvas over the cards the grid shows.
export function BoardCanvas({
  cards,
  onOpen,
}: {
  cards: CardSummary[]
  onOpen: (id: string) => void
}) {
  // The grid hands a new array on every render (its clock ticks): build
  // the graph again only when a card's content changed, or a drag in
  // progress would be reset under the pointer.
  const sig = JSON.stringify(cards)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const stable = useMemo(() => cards, [sig])
  const details = useBlockerDetails(stable)
  const graph = useMemo(() => toGraph(stable, details), [stable, details])
  const auto = useMemo(() => autoLayout(graph), [graph])
  const layout = useMemo(() => withPlaced(graph, auto), [graph, auto])
  return (
    <div className="glass-panel relative h-[calc(100vh-220px)] min-h-[480px] overflow-hidden">
      <Suspense
        fallback={<p className="p-4 text-sm text-muted-foreground">Loading the canvas…</p>}
      >
        <Canvas
          graph={graph}
          layout={layout}
          auto={auto}
          onOpen={onOpen}
          onPlace={(id, x, y) => void cardsApi.place(id, x, y)}
        />
      </Suspense>
    </div>
  )
}

// Canvas showcase (card #80): the Board's view switch, List | React Flow |
// Excalidraw | pele, in the browser preview only (bun run preview:look).
// Each canvas is its own lazy chunk in canvas/<lib>/index.tsx, so a
// library's weight only loads when its tab is picked and the app's own
// build never pulls one in until a canvas ships for real.

import {
  lazy,
  Suspense,
  useMemo,
  useState,
  type ComponentType,
  type LazyExoticComponent,
} from "react"

import { Button } from "@/components/ui/button"
import { inHost } from "@/lib/irori"
import { previewFlag } from "@/lib/preview"

import {
  autoLayout,
  toGraph,
  type CanvasLib,
  type CanvasProps,
} from "./data"
import type { CardSummary } from "@/lib/cards"

export type BoardView = "list" | CanvasLib

const views: { id: BoardView; label: string }[] = [
  { id: "list", label: "List" },
  { id: "reactflow", label: "React Flow" },
  { id: "excalidraw", label: "Excalidraw" },
  { id: "pele", label: "pele" },
]

const canvases: Record<CanvasLib, LazyExoticComponent<ComponentType<CanvasProps>>> = {
  reactflow: lazy(() => import("./reactflow")),
  excalidraw: lazy(() => import("./excalidraw")),
  pele: lazy(() => import("./pele")),
}

// canvasPreview says the switch shows: never inside the app.
export function canvasPreview(): boolean {
  return !inHost()
}

const viewKey = "irori.canvas.view"

function asView(v: string | null): BoardView {
  return views.find((x) => x.id === v)?.id ?? "list"
}

// useBoardView is the picked view: ?view=<id> in the preview, else the one
// last picked in this browser, else List.
export function useBoardView(): [BoardView, (v: BoardView) => void] {
  const [view, setView] = useState<BoardView>(() => {
    if (!canvasPreview()) return "list"
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
  if (!canvasPreview()) return null
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

// BoardCanvas is the picked canvas over the cards the grid shows.
export function BoardCanvas({
  lib,
  cards,
  onOpen,
}: {
  lib: CanvasLib
  cards: CardSummary[]
  onOpen: (id: string) => void
}) {
  const graph = useMemo(() => toGraph(cards), [cards])
  const layout = useMemo(() => autoLayout(graph), [graph])
  const Canvas = canvases[lib]
  return (
    <div className="glass-panel relative h-[calc(100vh-220px)] min-h-[480px] overflow-hidden">
      <Suspense
        fallback={<p className="p-4 text-sm text-muted-foreground">Loading the canvas…</p>}
      >
        <Canvas cards={cards} graph={graph} layout={layout} onOpen={onOpen} />
      </Suspense>
    </div>
  )
}

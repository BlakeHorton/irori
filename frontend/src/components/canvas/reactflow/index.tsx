// Canvas showcase (card #80): the React Flow (@xyflow/react) prototype. A stub until its agent
// fills it in; it reads the shared graph from ../data.ts and nothing else.

import type { CanvasProps } from "../data"

export default function Canvas({ graph, layout }: CanvasProps) {
  const cards = graph.nodes.filter((n) => n.kind === "card").length
  const groups = Object.keys(layout.groups).length
  return (
    <div className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
      React Flow (@xyflow/react) canvas goes here: {cards} cards in {groups} groups, {graph.edges.length} edges.
    </div>
  )
}

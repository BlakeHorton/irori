// The Board's canvas on React Flow (@xyflow/react; picked on card #80,
// shipped on #93). Nodes are React components, so a card here is the
// grid's own card body (StatePill, Faces, ProgressBar). Pan, zoom, drag,
// minimap and controls come built in. A dragged card keeps its spot in
// coord (onPlace) and its blockers move with it; each group is a box drawn
// behind its cards, sized to them as they move. Agents can be moved for a
// look, but their spots aren't kept.

import "@xyflow/react/dist/style.css"
import "./flow.css"

import { useCallback, useMemo, useState } from "react"
import {
  Background,
  BackgroundVariant,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow,
  applyNodeChanges,
  type Edge,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react"

import { Button } from "@/components/ui/button"

import {
  followers,
  groupRects,
  type AgentNode,
  type BlockerNode,
  type CanvasProps,
  type CardNode,
  type Graph,
  type GraphEdge,
  type GroupNode,
  type Layout,
  type Point,
} from "../data"
import { AgentBody, BlockerBody, CardBody, openTarget, toneVar } from "../node"

type CardData = { n: CardNode }
type BlockerData = { n: BlockerNode }
type AgentData = { n: AgentNode }
type GroupData = { n: GroupNode }

const hidden = "!size-1.5 !min-h-0 !min-w-0 !border-0 !bg-transparent"

function CardNodeView({ data, selected }: NodeProps<Node<CardData>>) {
  return (
    <>
      <Handle type="target" position={Position.Top} id="top" className={hidden} />
      <Handle type="target" position={Position.Left} id="left" className={hidden} />
      <Handle type="target" position={Position.Right} id="right" className={hidden} />
      <CardBody n={data.n} selected={selected} />
      <Handle type="source" position={Position.Bottom} id="bottom" className={hidden} />
    </>
  )
}

function BlockerNodeView({ data }: NodeProps<Node<BlockerData>>) {
  return (
    <>
      <BlockerBody n={data.n} />
      <Handle type="source" position={Position.Left} id="left" className={hidden} />
    </>
  )
}

function AgentNodeView({ data }: NodeProps<Node<AgentData>>) {
  return (
    <>
      <AgentBody n={data.n} />
      <Handle type="source" position={Position.Right} id="right" className={hidden} />
    </>
  )
}

function GroupNodeView({ data }: NodeProps<Node<GroupData>>) {
  return (
    <div className="h-full w-full border border-dashed border-border/70 bg-foreground/[0.02]">
      <span className="absolute top-2 left-3 text-xs font-medium tracking-wide text-muted-foreground uppercase">
        {data.n.label}
      </span>
    </div>
  )
}

// Defined once, outside the component: React Flow warns and remounts
// every node when nodeTypes is a new object each render.
const nodeTypes = {
  card: CardNodeView,
  blocker: BlockerNodeView,
  agent: AgentNodeView,
  group: GroupNodeView,
}

// toNodes is every card, blocker and agent at its spot (groups are drawn
// from these on each render, see groupNodes).
function toNodes(graph: Graph, layout: Layout): Node[] {
  const out: Node[] = []
  for (const n of graph.nodes) {
    if (n.kind === "group") continue
    const p = layout.pos[n.id]
    const s = layout.size[n.id]
    if (!p || !s) continue
    out.push({
      id: n.id,
      type: n.kind,
      position: p,
      style: { width: s.w, height: s.h },
      data: { n },
      // A blocker follows its card.
      draggable: n.kind !== "blocker",
    })
  }
  return out
}

// groupNodes is a box behind each group's cards, where they sit now.
function groupNodes(graph: Graph, layout: Layout, nodes: Node[]): Node[] {
  const pos: Record<string, Point> = {}
  for (const n of nodes) pos[n.id] = n.position
  const rects = groupRects(graph, pos, layout)
  const out: Node[] = []
  for (const g of graph.nodes) {
    if (g.kind !== "group") continue
    const r = rects[g.id]
    if (!r) continue
    out.push({
      id: g.id,
      type: "group",
      position: { x: r.x, y: r.y },
      style: { width: r.w, height: r.h },
      // Rebuilt each render, so never measured by React Flow (which hides
      // a node until it is): give it its size.
      width: r.w,
      height: r.h,
      measured: { width: r.w, height: r.h },
      data: { n: g },
      draggable: false,
      selectable: false,
      focusable: false,
      zIndex: -1,
    })
  }
  return out
}

const edgeStyle: Record<GraphEdge["kind"], { stroke: string; dash?: string; animated?: boolean }> = {
  subtask: { stroke: "var(--muted-foreground)" },
  blocker: { stroke: "var(--destructive)", dash: "4 4" },
  holder: { stroke: "var(--primary)", animated: true },
  watcher: { stroke: "var(--border)", dash: "2 4" },
}

function toEdges(graph: Graph): Edge[] {
  return graph.edges.map((e) => {
    const s = edgeStyle[e.kind]
    const handles =
      e.kind === "subtask"
        ? { sourceHandle: "bottom", targetHandle: "left" }
        : e.kind === "blocker"
          ? { sourceHandle: "left", targetHandle: "right" }
          : { sourceHandle: "right", targetHandle: "left" }
    return {
      id: e.id,
      source: e.source,
      target: e.target,
      ...handles,
      type: e.kind === "subtask" ? "smoothstep" : "default",
      animated: s.animated,
      style: { stroke: s.stroke, strokeDasharray: s.dash, strokeWidth: 1.5 },
      markerEnd: { type: MarkerType.ArrowClosed, color: s.stroke, width: 16, height: 16 },
    }
  })
}

export default function Canvas({ graph, layout, auto, onOpen, onPlace }: CanvasProps) {
  const start = useMemo(() => toNodes(graph, layout), [graph, layout])
  const [nodes, setNodes] = useState<Node[]>(start)
  const [seed, setSeed] = useState(start)
  if (seed !== start) {
    // The cards (or a kept spot) changed: start from the new layout
    // (render-time reset, the React-recommended way to derive state).
    setSeed(start)
    setNodes(start)
  }

  // A card dragged carries its blockers; dropped, its spot is kept.
  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      setNodes((ns) => {
        const next = applyNodeChanges(changes, ns)
        const cardMoved = changes.some(
          (c) => c.type === "position" && c.position && next.find((n) => n.id === c.id)?.type === "card"
        )
        if (!cardMoved) return next
        const pos: Record<string, Point> = {}
        for (const n of next) pos[n.id] = n.position
        const follow = followers(graph, pos)
        return next.map((n) =>
          n.type === "blocker" && follow[n.id] ? { ...n, position: follow[n.id] } : n
        )
      })
      for (const c of changes) {
        if (c.type !== "position" || c.dragging !== false || !c.position) continue
        const n = graph.nodes.find((x) => x.id === c.id)
        if (n?.kind === "card") onPlace(n.id, c.position.x, c.position.y)
      }
    },
    [graph, onPlace]
  )

  // Agent lines cross the whole board, so they stay faint until you hover
  // an agent or a card it is on.
  const [hover, setHover] = useState<string | null>(null)
  const base = useMemo(() => toEdges(graph), [graph])
  const edges = useMemo(
    () =>
      base.map((e) => {
        const agentLine = e.source.startsWith("agent:")
        const lit = hover !== null && (e.source === hover || e.target === hover)
        if (!agentLine && !lit) return e
        return {
          ...e,
          animated: lit && e.animated,
          zIndex: lit ? 10 : 0,
          style: { ...e.style, opacity: lit ? 1 : 0.1 },
        }
      }),
    [base, hover]
  )
  const all = useMemo(
    () => [...groupNodes(graph, layout, nodes), ...nodes],
    [graph, layout, nodes]
  )
  const placed = graph.nodes.some((n) => n.kind === "card" && n.card.pos)

  return (
    <div className="irori-flow h-full w-full">
      <ReactFlow
        nodes={all}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onNodeMouseEnter={(_, n) => setHover(n.id)}
        onNodeMouseLeave={() => setHover(null)}
        onNodeClick={(_, node) => {
          const n = (node.data as { n: { kind: string; id: string; card?: string } }).n
          const id = openTarget(n)
          if (id) onOpen(id)
        }}
        colorMode="dark"
        fitView
        fitViewOptions={{ padding: 0.12 }}
        minZoom={0.15}
        maxZoom={1.75}
        nodesConnectable={false}
        elevateNodesOnSelect={false}
      >
        <Background variant={BackgroundVariant.Dots} gap={22} size={1} color="var(--border)" />
        <MiniMap
          pannable
          zoomable
          nodeColor={(n) => {
            const d = (n.data as { n?: CardNode | GroupNode }).n
            if (d?.kind === "card") return toneVar[d.tone]
            if (n.type === "blocker") return "var(--destructive)"
            if (d?.kind === "group") return "transparent"
            return "var(--primary)"
          }}
          maskColor="color-mix(in oklch, var(--background) 70%, transparent)"
          style={{ background: "var(--card)" }}
        />
        <Controls showInteractive={false} />
      </ReactFlow>
      {placed && (
        <Button
          size="sm"
          variant="secondary"
          className="absolute top-3 right-3 z-10"
          title="Put every card back where Irori lays it out"
          onClick={() => {
            for (const n of graph.nodes) {
              const p = auto.pos[n.id]
              if (n.kind === "card" && n.card.pos && p) onPlace(n.id, p.x, p.y)
            }
          }}
        >
          Tidy up
        </Button>
      )}
    </div>
  )
}

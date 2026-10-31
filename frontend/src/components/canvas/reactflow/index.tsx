// Canvas showcase (card #80): the React Flow (@xyflow/react) prototype.
// Nodes are React components, so a card here is the grid's own card body
// (StatePill, Faces, ProgressBar). Groups are React Flow parent nodes;
// pan, zoom, drag, minimap and controls come built in. Dragged positions
// are kept per browser (data.ts), stored absolute like the other canvases.

import "@xyflow/react/dist/style.css"
import "./flow.css"

import { useCallback, useEffect, useMemo, useState } from "react"
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
  clearPositions,
  loadPositions,
  savePositions,
  withKept,
  type AgentNode,
  type BlockerNode,
  type CanvasProps,
  type CardNode,
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

// toNodes lays the graph out as React Flow nodes. Groups come first (a
// parent must precede its children); members carry parentId and positions
// relative to their group, as React Flow subflows want.
function toNodes(graph: CanvasProps["graph"], layout: Layout): Node[] {
  const out: Node[] = []
  for (const g of graph.nodes) {
    if (g.kind !== "group") continue
    const r = layout.groups[g.id]
    if (!r) continue
    out.push({
      id: g.id,
      type: "group",
      position: { x: r.x, y: r.y },
      style: { width: r.w, height: r.h },
      data: { n: g },
      selectable: true,
      zIndex: -1,
    })
  }
  for (const n of graph.nodes) {
    if (n.kind === "group") continue
    const p = layout.pos[n.id]
    const s = layout.size[n.id]
    if (!p || !s) continue
    const group = n.kind === "agent" ? null : n.group
    const r = group ? layout.groups[group] : undefined
    out.push({
      id: n.id,
      type: n.kind,
      position: r ? { x: p.x - r.x, y: p.y - r.y } : p,
      parentId: r ? group! : undefined,
      style: { width: s.w, height: s.h },
      data: { n },
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

function toEdges(graph: CanvasProps["graph"]): Edge[] {
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

// absolute is every node's top-left in canvas space: a member's position
// plus its group's, so the stored shape matches the other canvases.
function absolute(nodes: Node[]): Record<string, Point> {
  const byId = new Map(nodes.map((n) => [n.id, n]))
  const out: Record<string, Point> = {}
  for (const n of nodes) {
    const parent = n.parentId ? byId.get(n.parentId) : undefined
    out[n.id] = parent
      ? { x: n.position.x + parent.position.x, y: n.position.y + parent.position.y }
      : n.position
  }
  return out
}

export default function Canvas({ graph, layout, onOpen }: CanvasProps) {
  const start = useMemo(
    () => toNodes(graph, withKept(layout, loadPositions("reactflow"))),
    [graph, layout]
  )
  const [nodes, setNodes] = useState<Node[]>(start)
  const [seed, setSeed] = useState(start)
  if (seed !== start) {
    // The cards changed: start again from the new layout (render-time
    // reset, the React-recommended way to derive state from props).
    setSeed(start)
    setNodes(start)
  }
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

  // Save once a drag ends (a position change with dragging false). The
  // effect reads the applied nodes, so the store sees the dropped spot.
  const [dropped, setDropped] = useState(0)
  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setNodes((ns) => applyNodeChanges(changes, ns))
    if (changes.some((c) => c.type === "position" && c.dragging === false))
      setDropped((d) => d + 1)
  }, [])
  useEffect(() => {
    if (dropped) savePositions("reactflow", absolute(nodes))
  }, [dropped]) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="irori-flow h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onNodeMouseEnter={(_, n) => setHover(n.id)}
        onNodeMouseLeave={() => setHover(null)}
        onNodeClick={(_, node) => {
          const n = (node.data as { n: { kind: string; id: string } }).n
          const id = openTarget(n as never)
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
            const d = (n.data as { n?: CardNode }).n
            if (d?.kind === "card") return toneVar[d.tone]
            if (d?.kind === "blocker") return "var(--destructive)"
            if (d?.kind === "group") return "transparent"
            return "var(--primary)"
          }}
          maskColor="color-mix(in oklch, var(--background) 70%, transparent)"
          style={{ background: "var(--card)" }}
        />
        <Controls showInteractive={false} />
      </ReactFlow>
      <Button
        size="sm"
        variant="secondary"
        className="absolute top-3 right-3 z-10"
        onClick={() => {
          clearPositions("reactflow")
          const fresh = toNodes(graph, layout)
          setNodes(fresh)
        }}
      >
        Reset layout
      </Button>
    </div>
  )
}

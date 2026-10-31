// Canvas showcase (card #80): the pele prototype. pele (pele.run, MIT)
// renders Mermaid text to a static SVG, so this canvas writes the board as
// a Mermaid flowchart: a subgraph per group, a box per card, blocker and
// agent, edges for subtasks, blockers and holders. pele lays it out itself
// (our layout and kept positions don't apply) and nodes can't be dragged;
// it brings its own pan and zoom. A click on a node opens its card.

import { useEffect, useMemo, useRef } from "react"
import { mount, register, type Mounted } from "pele/core"
import flowchart from "pele/diagrams/flowchart"

import type { CanvasProps, CardNode, Graph } from "../data"
import { stepsText } from "../node"

register(flowchart)

// Mermaid ids must be plain: card ids are ULIDs (start with a digit),
// group and agent ids carry ":" and "/". Each node gets n<index>.
function ids(graph: Graph) {
  const toSafe = new Map<string, string>()
  const toReal = new Map<string, string>()
  graph.nodes.forEach((n, i) => {
    const safe = `${n.kind[0]}${i}`
    toSafe.set(n.id, safe)
    toReal.set(safe, n.id)
  })
  return { toSafe, toReal }
}

// text is a label line made safe for a quoted Mermaid label.
function text(s: string, max = 48): string {
  const t = s.length > max ? `${s.slice(0, max - 1)}…` : s
  return t
    .replace(/&/g, "#amp;")
    .replace(/"/g, "#quot;")
    .replace(/</g, "#lt;")
    .replace(/>/g, "#gt;")
    .replace(/[[\]{}()|]/g, " ")
}

function cardLabel(n: CardNode): string {
  const who = n.faces
    .filter((f) => f.role === "working")
    .map((f) => (f.person ? "you" : f.name))
    .join(", ")
  const foot = [stepsText(n.progress), who].filter(Boolean).join(" · ")
  return [`${n.card.tag ?? ""}#${n.short}`, text(n.label), foot && text(foot)]
    .filter(Boolean)
    .join("<br/>")
}

// toMermaid writes the board as a left-to-right flowchart.
export function toMermaid(graph: Graph): {
  src: string
  toReal: Map<string, string>
} {
  const { toSafe, toReal } = ids(graph)
  const lines = ["flowchart LR"]
  const byGroup = new Map<string, string[]>()
  for (const n of graph.nodes) {
    if (n.kind === "group" || n.kind === "agent") continue
    const id = toSafe.get(n.id)!
    const def =
      n.kind === "card"
        ? `${id}["${cardLabel(n)}"]:::${n.tone}`
        : `${id}{{"${n.state === "open" ? "Needs you" : "Waiting on agent"}<br/>${text(n.label, 56)}"}}:::${n.state === "open" ? "needs" : "waits"}`
    byGroup.set(n.group, [...(byGroup.get(n.group) ?? []), def])
  }
  for (const n of graph.nodes) {
    if (n.kind !== "agent") continue
    lines.push(
      `  ${toSafe.get(n.id)}(["${text(n.person ? "you" : n.label)}"]):::${n.person ? "person" : n.live ? "agent" : "gone"}`
    )
  }
  for (const g of graph.nodes) {
    if (g.kind !== "group") continue
    const body = byGroup.get(g.id)
    if (!body?.length) continue
    lines.push(
      `  subgraph ${toSafe.get(g.id)}["${text(g.label)}"]`,
      "    direction TB"
    )
    for (const d of body) lines.push(`    ${d}`)
    lines.push("  end")
  }
  for (const e of graph.edges) {
    const a = toSafe.get(e.source)
    const b = toSafe.get(e.target)
    if (!a || !b) continue
    const arrow =
      e.kind === "subtask"
        ? "-->"
        : e.kind === "blocker"
          ? "-.->"
          : e.kind === "holder"
            ? "-->"
            : "-.-"
    lines.push(`  ${a} ${arrow} ${b}`)
  }
  // Irori's status colours (pele passes these through as inline styles).
  lines.push(
    "  classDef info stroke:#60a5fa,stroke-width:2px",
    "  classDef destructive stroke:#f87171,stroke-width:2px",
    "  classDef success stroke:#4ade80,stroke-width:2px",
    "  classDef slate stroke:#71717a",
    "  classDef needs fill:#3f1d1d,stroke:#f87171,color:#fecaca",
    "  classDef waits fill:#3b2f12,stroke:#fbbf24,color:#fde68a",
    "  classDef agent fill:#172554,stroke:#60a5fa,color:#bfdbfe",
    "  classDef person fill:#2e1065,stroke:#a78bfa,color:#ddd6fe",
    "  classDef gone fill:#27272a,stroke:#52525b,color:#a1a1aa"
  )
  return { src: lines.join("\n"), toReal }
}

export default function Canvas({ graph, onOpen }: CanvasProps) {
  const host = useRef<HTMLDivElement>(null)
  const mounted = useRef<Mounted | null>(null)
  const { src, toReal } = useMemo(() => toMermaid(graph), [graph])

  useEffect(() => {
    const el = host.current
    if (!el) return
    mounted.current = mount(el, src, {
      zoom: { always: true, maxScale: 3, controls: true },
      onError: (e) => console.warn("pele:", e),
    })
    // pele can't zoom out past fitting the width, so a tall board would
    // open cut off: narrow the box to the board's shape so it fits by height.
    const vb = el.querySelector("svg")?.viewBox.baseVal
    const outer = el.parentElement
    if (vb?.height && outer) {
      const w = Math.min(
        outer.clientWidth,
        (outer.clientHeight * vb.width) / vb.height
      )
      el.style.width = `${Math.floor(w)}px`
      mounted.current.refresh()
    }
    return () => {
      mounted.current?.destroy()
      mounted.current = null
    }
  }, [src])

  // A click on a node opens its card (a blocker opens the card it blocks).
  const onClick = (ev: React.MouseEvent) => {
    const node = (ev.target as Element).closest(".pele-node[data-id]")
    const safe = node?.getAttribute("data-id")
    const real = safe ? toReal.get(safe) : undefined
    if (!real) return
    const n = graph.nodes.find((x) => x.id === real)
    if (n?.kind === "card") onOpen(n.id)
    else if (n?.kind === "blocker") onOpen(n.card)
  }

  return (
    <div className="flex h-full w-full justify-center">
      <div
        ref={host}
        onClick={onClick}
        className="irori-pele h-full w-full overflow-hidden [&_.pele-node]:cursor-pointer"
        style={
          {
            "--pele-bg": "transparent",
            "--pele-fg": "var(--foreground)",
            "--pele-muted": "var(--muted-foreground)",
            "--pele-line": "var(--muted-foreground)",
            "--pele-surface": "var(--card)",
            "--pele-surface-alt": "var(--secondary)",
            "--pele-border": "var(--border)",
            "--pele-accent": "var(--primary)",
            "--pele-font": "var(--font-sans)",
            "--pele-radius": "0px",
          } as React.CSSProperties
        }
      />
    </div>
  )
}

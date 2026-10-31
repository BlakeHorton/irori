// Canvas showcase (card #80): the Excalidraw (@excalidraw/excalidraw)
// prototype. Excalidraw is a whiteboard: a node can't hold React, so each
// card is its own hand-drawn rectangle with a text label, each group a
// dashed box, and holder, subtask and blocker lines are bound arrows that
// follow when a card moves. Everything stays editable: draw, write notes,
// move anything. The link on a card opens it in Irori.

import "@excalidraw/excalidraw/index.css"

import { useEffect, useMemo, useRef } from "react"
import { convertToExcalidrawElements, Excalidraw } from "@excalidraw/excalidraw"
import type { ExcalidrawElementSkeleton } from "@excalidraw/excalidraw/data/transform"
import type { OrderedExcalidrawElement } from "@excalidraw/excalidraw/element/types"

import {
  loadPositions,
  savePositions,
  withKept,
  type CanvasProps,
  type CardNode,
  type GraphEdge,
  type Point,
} from "../data"
import { stepsText } from "../node"

// Fonts from this build's own copy, not Excalidraw's CDN fallback: Irori
// talks to nothing third-party (architecture.md §7.1). In the dev preview
// Vite serves node_modules as is; a shipped build would copy the fonts.
declare global {
  interface Window {
    EXCALIDRAW_ASSET_PATH?: string | string[]
  }
}
window.EXCALIDRAW_ASSET_PATH = "/node_modules/@excalidraw/excalidraw/dist/prod/"

// Excalidraw's dark theme inverts the canvas (invert + hue-rotate), so
// colours are given as on its light theme: its own open-color palette.
const tone: Record<CardNode["tone"], { stroke: string; fill: string }> = {
  destructive: { stroke: "#e03131", fill: "#fff5f5" },
  success: { stroke: "#2f9e44", fill: "#ebfbee" },
  slate: { stroke: "#868e96", fill: "#f8f9fa" },
  info: { stroke: "#1971c2", fill: "#e7f5ff" },
}
const line: Record<GraphEdge["kind"], { color: string; style: "solid" | "dashed" | "dotted" }> = {
  subtask: { color: "#495057", style: "solid" },
  blocker: { color: "#e03131", style: "dashed" },
  holder: { color: "#1971c2", style: "solid" },
  watcher: { color: "#adb5bd", style: "dotted" },
}

const ink = "#1e1e1e" // text: near-black on the light theme, so near-white here

const linkPrefix = "irori://card/"

// cardText is what a card says on the whiteboard: ref and state, title,
// then steps and who is on it.
function cardText(n: CardNode): string {
  const who = n.faces.filter((f) => f.role === "working").map((f) => (f.person ? "you" : f.name))
  const state = n.openBlockers
    ? `${n.openBlockers} blocker${n.openBlockers === 1 ? "" : "s"}`
    : n.card.working.length
      ? "in progress"
      : n.state === "submitted"
        ? "open"
        : n.state.replace(/_/g, " ")
  const title = n.label.length > 60 ? `${n.label.slice(0, 58)}…` : n.label
  const foot = [stepsText(n.progress), who.join(", ")].filter(Boolean).join(" · ")
  return [`${n.card.tag ?? ""}#${n.short} · ${state}`, title, foot].filter(Boolean).join("\n")
}

function toSkeleton({ graph, layout }: Pick<CanvasProps, "graph" | "layout">): ExcalidrawElementSkeleton[] {
  const els: ExcalidrawElementSkeleton[] = []
  for (const n of graph.nodes) {
    if (n.kind === "group") continue
    const p = layout.pos[n.id]
    const s = layout.size[n.id]
    if (!p || !s) continue
    const common = {
      id: n.id,
      x: p.x,
      y: p.y,
      width: s.w,
      height: s.h,
      roughness: 1,
      strokeWidth: 1,
      customData: { nodeId: n.id, kind: n.kind },
    }
    if (n.kind === "card") {
      const t = tone[n.tone]
      els.push({
        ...common,
        type: "rectangle",
        strokeColor: t.stroke,
        backgroundColor: t.fill,
        fillStyle: "solid",
        roundness: { type: 3 },
        link: `${linkPrefix}${n.id}`,
        label: { text: cardText(n), fontSize: 14, textAlign: "left", verticalAlign: "top", strokeColor: ink },
      })
    } else if (n.kind === "blocker") {
      const person = n.state === "open"
      els.push({
        ...common,
        type: "rectangle",
        strokeColor: person ? "#e03131" : "#f08c00",
        backgroundColor: person ? "#fff5f5" : "#fff9db",
        fillStyle: "solid",
        strokeStyle: "dashed",
        link: `${linkPrefix}${n.card}`,
        label: {
          text: `${person ? "Needs you" : "Waiting on agent"}: ${n.label.length > 70 ? `${n.label.slice(0, 68)}…` : n.label}`,
          fontSize: 12,
          strokeColor: ink,
        },
      })
    } else {
      els.push({
        ...common,
        type: "ellipse",
        strokeColor: n.person ? "#7048e8" : "#1971c2",
        backgroundColor: n.person ? "#f3f0ff" : "#e7f5ff",
        fillStyle: "solid",
        opacity: n.live || n.person ? 100 : 50,
        label: { text: n.person ? "you" : n.label, fontSize: 13, strokeColor: ink },
      })
    }
  }
  for (const e of graph.edges) {
    const l = line[e.kind]
    const [a, as] = [layout.pos[e.source], layout.size[e.source]]
    const [b, bs] = [layout.pos[e.target], layout.size[e.target]]
    if (!a || !as || !b || !bs) continue
    // Subtasks run down from the parent's bottom-left into the child's
    // left side; everything else runs side to side.
    const from =
      e.kind === "subtask"
        ? { x: a.x + 16, y: a.y + as.h }
        : e.kind === "blocker"
          ? { x: a.x, y: a.y + as.h / 2 }
          : { x: a.x + as.w, y: a.y + as.h / 2 }
    const to =
      e.kind === "blocker" ? { x: b.x + bs.w, y: b.y + bs.h / 2 } : { x: b.x, y: b.y + bs.h / 2 }
    const pts: [number, number][] =
      e.kind === "subtask"
        ? [[0, 0], [0, to.y - from.y], [to.x - from.x, to.y - from.y]]
        : [[0, 0], [to.x - from.x, to.y - from.y]]
    els.push({
      type: "arrow",
      x: from.x,
      y: from.y,
      width: Math.abs(to.x - from.x),
      height: Math.abs(to.y - from.y),
      points: pts as never,
      strokeColor: l.color,
      strokeStyle: l.style,
      strokeWidth: 1,
      roughness: 1,
      opacity: e.kind === "holder" || e.kind === "watcher" ? 30 : 100,
      start: { id: e.source },
      end: { id: e.target },
      endArrowhead: "arrow",
    })
  }
  // Groups: a dashed box behind the cards with its name above (frames
  // would clip what is dragged out of them, and place their name oddly).
  const groups: ExcalidrawElementSkeleton[] = []
  for (const g of graph.nodes) {
    if (g.kind !== "group") continue
    const r = layout.groups[g.id]
    if (!r) continue
    groups.push(
      {
        type: "rectangle",
        id: g.id,
        x: r.x,
        y: r.y,
        width: r.w,
        height: r.h,
        strokeColor: "#adb5bd",
        strokeStyle: "dashed",
        strokeWidth: 1,
        roughness: 0,
        backgroundColor: "transparent",
        locked: true,
      },
      {
        type: "text",
        x: r.x + 10,
        y: r.y + 8,
        text: g.label,
        fontSize: 14,
        strokeColor: "#868e96",
        locked: true,
      }
    )
  }
  return [...groups, ...els]
}

export default function Canvas({ graph, layout, onOpen }: CanvasProps) {
  const elements = useMemo(
    () => convertToExcalidrawElements(toSkeleton({ graph, layout: withKept(layout, loadPositions("excalidraw")) })),
    [graph, layout]
  )
  const saveTimer = useRef<number | undefined>(undefined)
  const last = useRef("")
  useEffect(() => () => window.clearTimeout(saveTimer.current), [])

  // Keep where nodes were dragged: the top-left of each element that
  // stands for a graph node, once the board settles for half a second.
  const onChange = (els: readonly OrderedExcalidrawElement[]) => {
    window.clearTimeout(saveTimer.current)
    saveTimer.current = window.setTimeout(() => {
      const pos: Record<string, Point> = {}
      for (const el of els) {
        const id = (el.customData as { nodeId?: string } | undefined)?.nodeId
        if (id && !el.isDeleted) pos[id] = { x: Math.round(el.x), y: Math.round(el.y) }
      }
      const key = JSON.stringify(pos)
      if (key !== last.current) {
        last.current = key
        savePositions("excalidraw", pos)
      }
    }, 500)
  }

  return (
    <div className="irori-excalidraw h-full w-full">
      <Excalidraw
        key={elements.length}
        initialData={{
          elements,
          appState: { viewBackgroundColor: "#ffffff", currentItemFontFamily: 5 },
          scrollToContent: true,
        }}
        theme="dark"
        excalidrawAPI={(api) => {
          // Fit the whole board once it has mounted.
          requestAnimationFrame(() =>
            api.scrollToContent(undefined, { fitToContent: true, animate: false })
          )
        }}
        onChange={onChange}
        onLinkOpen={(el, ev) => {
          const link = el.link ?? ""
          if (!link.startsWith(linkPrefix)) return
          ev.preventDefault()
          onOpen(link.slice(linkPrefix.length))
        }}
        UIOptions={{
          canvasActions: {
            loadScene: false,
            saveToActiveFile: false,
            export: false,
            saveAsImage: false,
            toggleTheme: false,
            changeViewBackgroundColor: false,
          },
          tools: { image: false },
        }}
      />
    </div>
  )
}

// Agent avatars (card #407): open-bot-avatar faces, an SVG silhouette with
// eyes. Until an agent makes its own, it wears a default picked from the
// package's starters and gallery by its name, so a name keeps its face on
// every device. Avatar definitions from elsewhere (a friend's board) are
// only ever used after validateAvatar; never raw SVG.
//
// The gallery shapes are copied from open-bot-avatar's gallery/ (MIT,
// Copyright (c) 2026 Dominik Scholz).

import {
  examples,
  renderAvatarSvg,
  validateAvatar,
  type AvatarDefinition,
  type AvatarMotion,
} from "open-bot-avatar"

import gallery from "@/lib/avatar-gallery.json"

// DEFAULTS are the faces an agent can wear before it makes its own.
// Append only: a default is its name's hash over this list.
export const DEFAULTS: AvatarDefinition[] = [
  ...examples,
  ...(gallery as unknown[]).flatMap((g) => {
    const r = validateAvatar(g)
    return r.ok ? [r.value] : []
  }),
]

function fnv1a(s: string): number {
  let h = 0x811c9dc5
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i)
    h = Math.imul(h, 0x01000193)
  }
  return h >>> 0
}

// defaultAvatar is the face an agent name wears until it makes its own.
export function defaultAvatar(name: string): AvatarDefinition {
  return DEFAULTS[fnv1a(name) % DEFAULTS.length]
}

// faceOf is the face an agent wears: its own (validated again here, never
// trusted as is), else the default for its name.
const ownCache = new Map<string, AvatarDefinition | null>()
export function faceOf(who: {
  name?: string
  key: string
  avatar?: string | null
}): AvatarDefinition {
  const raw = who.avatar ?? ""
  if (raw) {
    let own = ownCache.get(raw)
    if (own === undefined) {
      try {
        const r = validateAvatar(JSON.parse(raw))
        own = r.ok ? r.value : null
      } catch {
        own = null
      }
      ownCache.set(raw, own)
    }
    if (own) return own
  }
  return defaultAvatar(who.name || who.key)
}

// moodOf is an agent's mood from its status: thinking while busy.
export function moodOf(status: string): AvatarMotion {
  return status === "busy" ? "thinking" : "idle"
}

// Look is where a face looks (card #407): at you while a card waits on
// you, down while it waits on something else.
export type Look = "you" | "down" | undefined

// gazeOf is the look direction for an agent's state: looking at you, down
// while waiting, up and to the side while it works; none otherwise.
export function gazeOf(
  status: string,
  look: Look
): { x: number; y: number } | undefined {
  if (look === "you") return { x: 0, y: 0 }
  if (look === "down") return { x: -0.2, y: 0.9 }
  if (status === "busy") return { x: 0.6, y: -0.55 }
  return undefined
}

// hexOf resolves a CSS colour (oklch, a var(...)) to #rrggbb, which the
// renderer needs to shade the body. "" when it can't.
const hexCache = new Map<string, string>()
let probe: CanvasRenderingContext2D | null = null
export function hexOf(css: string): string {
  if (!css || typeof document === "undefined") return ""
  const key = `${document.documentElement.className}|${css}`
  const hit = hexCache.get(key)
  if (hit !== undefined) return hit
  const el = document.createElement("span")
  el.style.color = css
  el.style.display = "none"
  document.body.appendChild(el)
  const resolved = getComputedStyle(el).color
  el.remove()
  probe ??= document.createElement("canvas").getContext("2d", {
    willReadFrequently: true,
  })
  let hex = ""
  if (probe) {
    probe.clearRect(0, 0, 1, 1)
    probe.fillStyle = "#000"
    probe.fillStyle = resolved
    probe.fillRect(0, 0, 1, 1)
    const [r, g, b] = probe.getImageData(0, 0, 1, 1).data
    hex = "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")
  }
  hexCache.set(key, hex)
  return hex
}

// stillSvg is one face as an SVG string, cached: a still face never
// changes for the same inputs.
const svgCache = new Map<string, string>()
export function stillSvg(
  def: AvatarDefinition,
  color: string,
  mood: AvatarMotion,
  label: string,
  gaze?: { x: number; y: number }
): string {
  const key = `${def.id}|${def.body.path}|${color}|${mood}|${label}|${gaze?.x},${gaze?.y}`
  let svg = svgCache.get(key)
  if (!svg) {
    svg = renderAvatarSvg(
      { ...def, name: label || def.name },
      { color: color || undefined, motion: mood, time: 0.6, gaze }
    )
    svgCache.set(key, svg)
  }
  return svg
}

// nameKey is the key a face is kept under: the agent's name, trimmed and
// lower case (coord::avatar::name_key).
export function nameKey(name: string): string {
  return name.trim().toLowerCase()
}

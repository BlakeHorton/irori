import { useEffect, useMemo, useRef, useState } from "react"
import { mountAvatar, type MountedAvatar } from "open-bot-avatar/dom"
import { displayName, type AgentRef } from "@/lib/cards"
import {
  faceOf,
  gazeOf,
  hexOf,
  moodOf,
  stillSvg,
  type Look,
} from "@/lib/avatars"

// faceTint is the colour a face wears for its state (card #407): amber
// while it isn't answering, else the given tint.
function faceTint(who: AgentRef, tint: string): string {
  return who.status === "unresponsive" ? "var(--color-warning)" : tint
}

// BotFace is an agent's open-bot-avatar face as one still SVG (card #407):
// cheap enough for every face on the board. Its colour is the agent's
// device colour, or a friend's person colour on a shared board (#385). It
// looks up and away while working, at you while its card waits on you,
// and down while it waits on something else.
export function BotFace({
  who,
  tint,
  look,
  className,
}: {
  who: AgentRef
  tint: string
  look?: Look
  className?: string
}) {
  const name = displayName(who)
  const color = faceTint(who, tint)
  const svg = useMemo(
    () =>
      stillSvg(
        faceOf(who),
        hexOf(color),
        moodOf(who.status),
        "",
        gazeOf(who.status, look)
      ),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [who.name, who.key, who.status, who.avatar, color, look]
  )
  return (
    <span
      className={className}
      aria-label={name}
      role="img"
      // The renderer re-serializes a validated path and escapes the label.
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  )
}

// HAPPY_MS is how long a face smiles after its agent finishes work.
const HAPPY_MS = 2600

// LiveFace is the same face mounted live (card #407): it idles, thinks
// while the agent works, smiles for a moment when it goes from working to
// idle, and follows the pointer. For the hover card, the agents strip and
// headers; reduced motion is the system's.
export function LiveFace({
  who,
  tint,
  size = 56,
  lookAt,
}: {
  who: AgentRef
  tint: string
  size?: number
  lookAt?: { x: number; y: number }
}) {
  const host = useRef<HTMLDivElement>(null)
  const live = useRef<MountedAvatar | null>(null)
  const def = faceOf(who)
  const color = hexOf(faceTint(who, tint)) || undefined
  const [happy, setHappy] = useState(false)
  const was = useRef(who.status)
  useEffect(() => {
    const before = was.current
    was.current = who.status
    if (before === "busy" && who.status === "idle") {
      setHappy(true)
      const t = window.setTimeout(() => setHappy(false), HAPPY_MS)
      return () => window.clearTimeout(t)
    }
  }, [who.status])
  const mood = happy ? "happy" : moodOf(who.status)
  useEffect(() => {
    if (!host.current) return
    live.current = mountAvatar(host.current, {
      definition: def,
      color,
      mood,
      followPointer: !lookAt,
      lookAt,
      label: displayName(who),
    })
    return () => {
      live.current?.destroy()
      live.current = null
    }
    // One mount per face; changes go through update below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [def.id, def.body.path])
  useEffect(() => {
    live.current?.update({ color, mood, lookAt })
  }, [color, mood, lookAt])
  return (
    <div
      ref={host}
      className="shrink-0"
      style={{ width: size, aspectRatio: "1" }}
    />
  )
}

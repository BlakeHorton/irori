import { minidenticon } from "minidenticons"
import { Avatar, AvatarBadge, AvatarFallback } from "@/components/ui/avatar"
import { deviceVar, displayName, type AgentRef } from "@/lib/cards"
import { cn } from "@/lib/utils"

// Face is one agent (or the person) on a card (card #79): a 5x5 pixel
// identicon drawn from the agent's key, so an agent keeps its face for
// as long as it lives. A working face wears its device's colour once there
// are two devices (card #22), so the colour says which device it is on;
// with one device it is blue. A watcher is grey, the person is a violet
// "you". A live working agent gets the green dot; a gone one, or one on
// an offline device, is faded; a contested lease rings it amber
// (docs/design/v0.4-devices.md §9).
export function Face({
  who,
  watching,
  noDot,
  className,
}: {
  who: AgentRef
  watching?: boolean
  noDot?: boolean
  className?: string
}) {
  const person = who.status === "person"
  const gone = who.status === "gone"
  const tint = !person && !watching ? deviceVar(who.color) : ""
  const name = displayName(who)
  const title = [
    name,
    who.where,
    who.device && `on ${who.device}`,
    gone && "gone",
    who.offline && "offline",
    who.conflict && `lease conflict on ${who.conflict}`,
  ]
    .filter(Boolean)
    .join(" · ")
  return (
    <Avatar
      size="sm"
      className={cn(
        "size-[22px] rounded-none after:rounded-none after:border-0",
        (gone || who.offline) && "opacity-55",
        who.conflict && "ring-2 ring-warning",
        className
      )}
      title={title}
    >
      {person ? (
        <AvatarFallback className="rounded-none bg-person-surface font-sans text-[10px] font-semibold text-person">
          you
        </AvatarFallback>
      ) : (
        <span
          className={cn(
            "flex size-full items-center justify-center",
            watching ? "text-muted-foreground/70" : "text-primary"
          )}
          style={tint ? { color: tint } : undefined}
        >
          <Identicon seed={who.key} />
        </span>
      )}
      {!watching && !noDot && who.live && !who.offline && !person && (
        <AvatarBadge className="-right-0.5 -bottom-0.5 bg-success" />
      )}
    </Avatar>
  )
}

// fnv1a is a 32-bit FNV-1a hash: minidenticons' own hash leaves near
// keys (sample-wren) with three or four lit squares.
function fnv1a(str: string): number {
  let h = 0x811c9dc5
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i)
    h = Math.imul(h, 0x01000193)
  }
  return h >>> 0
}

// cells are the lit squares of a seed's identicon, read from
// minidenticons' SVG but hashed with fnv1a, so they don't match
// minidenticon(key) on its own hash. A pattern too sparse
// to read as a face (under 8 squares) is drawn again from a salted seed.
const cellCache = new Map<string, [number, number][]>()
function cells(seed: string): [number, number][] {
  let c = cellCache.get(seed)
  if (!c) {
    for (let salt = 0; salt < 8; salt++) {
      const svg = minidenticon(salt ? `${seed}#${salt}` : seed, 0, 0, fnv1a)
      c = [...svg.matchAll(/x="(\d+)" y="(\d+)"/g)].map(
        (m) => [Number(m[1]), Number(m[2])] as [number, number]
      )
      if (c.length >= 8) break
    }
    cellCache.set(seed, c!)
  }
  return c!
}

// Identicon draws the 5x5 pattern in currentColor at 4px a square, so it
// stays crisp at 20px.
function Identicon({ seed }: { seed: string }) {
  return (
    <svg
      viewBox="0 0 5 5"
      className="size-5"
      fill="currentColor"
      shapeRendering="crispEdges"
      aria-hidden
    >
      {cells(seed).map(([x, y]) => (
        <rect key={`${x}-${y}`} x={x} y={y} width={1} height={1} />
      ))}
    </svg>
  )
}

// Faces is a row of faces.
export function Faces({
  who,
  watching,
}: {
  who: AgentRef[]
  watching?: boolean
}) {
  return (
    <span className="flex items-center gap-1">
      {who.map((w) => (
        <Face key={w.key} who={w} watching={watching} />
      ))}
    </span>
  )
}

// TalkBubble shows agents talking to each other on a card (card #22):
// their faces, in their device colours, with a speech bubble whose dots
// bounce in turn; with reduced motion the dots stand still.
export function TalkBubble({ who }: { who: AgentRef[] }) {
  if (who.length < 2) return null
  const names = who.map(displayName).join(", ")
  return (
    <span
      className="flex items-center gap-1"
      title={`Talking on this card: ${names}`}
      aria-label={`${names} are talking on this card`}
    >
      <span className="flex items-center gap-0.5">
        {who.map((w) => (
          <Face key={w.key} who={w} noDot />
        ))}
      </span>
      <span
        aria-hidden
        className="relative flex h-4 items-center gap-[3px] rounded-full bg-secondary px-1.5 ring-1 ring-border"
      >
        {[0, 1, 2].map((i) => (
          <span
            key={i}
            className="size-[3px] rounded-full bg-foreground motion-safe:animate-bounce"
            style={{
              animationDelay: `${i * 150}ms`,
              animationDuration: "900ms",
            }}
          />
        ))}
      </span>
    </span>
  )
}

// LiveDot is the green "live" dot in Who's on it.
export function LiveDot() {
  return (
    <span
      className="inline-block size-[7px] rounded-full bg-success"
      aria-hidden
    />
  )
}

import type { CSSProperties } from "react"
import { UserIcon } from "lucide-react"
import { AgentHover } from "@/components/cards/agent-hover"
import { BotFace } from "@/components/cards/bot-face"
import type { Look } from "@/lib/avatars"
import { Avatar, AvatarBadge } from "@/components/ui/avatar"
import { deviceVar, displayName, type AgentRef } from "@/lib/cards"
import { cn } from "@/lib/utils"

// Face is one agent (or the person) on a card (card #79): an
// open-bot-avatar face (card #407), its own or the default for its name,
// so an agent keeps its face on every device. A working face wears its device's colour once there
// are two devices (card #22), so the colour says which device it is on;
// with one device it is blue. A watcher is grey; the person is a user
// icon in their device's colour (card #108). A live working agent gets the green dot; a gone one, or one on
// an offline device, is faded; a contested lease rings it amber
// (docs/design/v0.4-devices.md §9). Hovering an agent's face shows its
// hover card (card #104): name, device, and its cards; noHover leaves it
// out where the face already sits in a popup. inline is the small face
// beside a name in a thread row (card #121): about the name's height, in
// the text's colour (grey for the person or a gone agent), and in its
// device colour only while the face or its row (group/row) is hovered.
export function Face({
  who,
  watching,
  noDot,
  noHover,
  inline,
  look,
  className,
}: {
  who: AgentRef
  watching?: boolean
  noDot?: boolean
  noHover?: boolean
  inline?: boolean
  // Where the face looks (card #407): at you, or down while it waits.
  look?: Look
  className?: string
}) {
  const person = who.status === "person"
  const gone = who.status === "gone"
  const tint = !watching ? deviceVar(who.color) : ""
  const name = displayName(who)
  const title = [
    name,
    who.where,
    who.device && `on ${who.device}`,
    gone && "gone",
    who.status === "unresponsive" && "isn't answering",
    who.offline && "offline",
    who.conflict && `lease conflict on ${who.conflict}`,
  ]
    .filter(Boolean)
    .join(" · ")
  if (inline) {
    const face = (
      <span
        className={cn(
          "inline-flex size-3.5 shrink-0 items-center justify-center",
          person || gone ? "text-muted-foreground" : "text-ink",
          "transition-colors group-hover/row:text-(--face-tint) hover:text-(--face-tint)",
          (gone || who.offline) && "opacity-55",
          className
        )}
        style={
          { "--face-tint": tint || "var(--color-primary)" } as CSSProperties
        }
        title={noHover || person ? title : undefined}
        aria-label={title}
        role="img"
      >
        {person ? (
          <UserIcon className="size-full" strokeWidth={2.5} aria-hidden />
        ) : (
          <BotFace
            who={who}
            tint={tint || "var(--color-primary)"}
            className="flex size-full [&>svg]:size-full"
          />
        )}
      </span>
    )
    return noHover || person ? face : <AgentHover who={who} trigger={face} />
  }
  const face = (
    <Avatar
      size="sm"
      className={cn(
        // A dim square outline frames the person's icon (card #104); an
        // agent's face is its own shape (card #407).
        "size-[26px] rounded-none after:rounded-none after:mix-blend-normal dark:after:mix-blend-normal",
        person
          ? "after:border after:border-muted-foreground/35"
          : "after:border-0",
        (gone || who.offline) && "opacity-55",
        who.conflict && "ring-2 ring-warning",
        className
      )}
      title={noHover || person ? title : undefined}
      aria-label={title}
    >
      {person ? (
        // The person is a plain user icon (card #179), not a pixel face, so
        // they never read as one more agent; in the colour of the device
        // they wrote from (card #108), blue with one device.
        <span
          className={cn(
            "flex size-full items-center justify-center",
            watching ? "text-muted-foreground/70" : "text-primary"
          )}
          style={tint ? { color: tint } : undefined}
        >
          <UserIcon className="size-5" strokeWidth={2.25} aria-hidden />
        </span>
      ) : (
        // An open-bot-avatar face (card #407): a still SVG in the agent's
        // colour, grey while it only watches; thinking while it works.
        <BotFace
          who={who}
          tint={
            watching
              ? "var(--muted-foreground)"
              : tint || "var(--color-primary)"
          }
          look={watching ? undefined : look}
          className={cn(
            "flex size-full items-center justify-center [&>svg]:size-full",
            (gone || who.offline) && "grayscale"
          )}
        />
      )}
      {!watching && !noDot && who.live && !who.offline && !person && (
        <AvatarBadge className="-right-0.5 -bottom-0.5 bg-success" />
      )}
    </Avatar>
  )
  return noHover ? face : <AgentHover who={who} trigger={face} />
}

// FACES_MAX is how many faces a tile's working or watching row shows
// before "+N" (card #222).
export const FACES_MAX = 5

// Faces is a row of faces. With max, at most max of them show, the rest
// as "+N" whose tooltip names them (card #222: a card's twelve watchers
// ran past its edge).
export function Faces({
  who,
  watching,
  max,
  look,
}: {
  who: AgentRef[]
  watching?: boolean
  max?: number
  look?: Look
}) {
  const over = max !== undefined && who.length > max
  const shown = over ? who.slice(0, Math.max(max - 1, 1)) : who
  const rest = who.slice(shown.length)
  return (
    <span className="flex min-w-0 items-center gap-1">
      {shown.map((w) => (
        <Face key={w.key} who={w} watching={watching} look={look} />
      ))}
      {rest.length > 0 && (
        <span
          className="shrink-0 text-[11px] text-muted-foreground tabular-nums"
          title={rest.map(displayName).join(", ")}
          aria-label={`and ${rest.length} more: ${rest.map(displayName).join(", ")}`}
        >
          +{rest.length}
        </span>
      )}
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

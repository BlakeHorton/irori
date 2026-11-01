import { createContext, useContext, type ReactElement } from "react"
import { PreviewCard } from "@base-ui/react/preview-card"

import { cardRef } from "@/components/cards/bits"
import {
  deviceVar,
  displayName,
  type AgentRef,
  type CardSummary,
} from "@/lib/cards"
import { plainLine } from "@/lib/irori"

// AgentCards is what an agent's hover card (card #104) and a #ref in text
// read: the live cards and the threads, and how to open each (card #144).
// App provides it; outside it (the preview of a single component, a test)
// a face has no hover card and a #ref stays text.
export const AgentCards = createContext<{
  cards: CardSummary[]
  onOpen: (id: string) => void
  threads?: readonly { id: string; short: string; title: string }[]
  onOpenThread?: (id: string) => void
} | null>(null)

// MaxRows is how many cards each list shows before "and N more".
const MaxRows = 6

// AgentHover wraps an agent's face (trigger) in a hover card: its name,
// its device, whether it's live, and the cards it holds, is offered, and
// watches, each one click from its page. Its grids are grid-cols-1
// (minmax(0,1fr)) so a long name, path or title truncates inside the
// card's width instead of widening the column past it (card #148).
export function AgentHover({
  who,
  trigger,
}: {
  who: AgentRef
  trigger: ReactElement
}) {
  const ctx = useContext(AgentCards)
  if (!ctx || who.status === "person") return trigger
  return (
    <PreviewCard.Root>
      <PreviewCard.Trigger render={trigger} delay={250} closeDelay={150} />
      <PreviewCard.Portal>
        <PreviewCard.Positioner sideOffset={6} className="isolate z-50">
          <PreviewCard.Popup
            className="z-50 flex w-72 origin-(--transform-origin) flex-col gap-2.5 rounded-none bg-popover/90! p-3 text-sm text-popover-foreground glass-panel outline-hidden duration-100 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95"
            // The popup is portalled, but React still bubbles its clicks to
            // the face's ancestors (a card tile opens on click).
            onClick={(e) => e.stopPropagation()}
          >
            <AgentSummary who={who} cards={ctx.cards} onOpen={ctx.onOpen} />
          </PreviewCard.Popup>
        </PreviewCard.Positioner>
      </PreviewCard.Portal>
    </PreviewCard.Root>
  )
}

function AgentSummary({
  who,
  cards,
  onOpen,
}: {
  who: AgentRef
  cards: CardSummary[]
  onOpen: (id: string) => void
}) {
  const gone = who.status === "gone"
  const holding: CardSummary[] = []
  const offered: CardSummary[] = []
  const watching: CardSummary[] = []
  for (const c of cards) {
    const w = c.working.find((r) => r.key === who.key)
    if (w) (w.assigned ? offered : holding).push(c)
    else if (c.watching.some((r) => r.key === who.key)) watching.push(c)
  }
  const tint = deviceVar(who.color)
  const device = who.device || who.where
  const state = gone
    ? "gone"
    : who.offline
      ? "device offline"
      : !who.live
        ? "not running"
        : who.status === "unresponsive"
          ? "isn't answering"
          : who.status === "busy"
          ? "working"
          : "idle"
  return (
    <>
      <div className="grid grid-cols-1 gap-0.5">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate font-medium">{displayName(who)}</span>
          <span
            className={
              gone || who.offline || !who.live
                ? "shrink-0 text-xs text-muted-foreground"
                : who.status === "unresponsive"
                ? "shrink-0 text-xs text-warning"
                : "shrink-0 text-xs text-success"
            }
          >
            {state}
          </span>
        </div>
        {device && (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <span
              className="inline-block size-2 shrink-0 bg-primary"
              style={tint ? { background: tint } : undefined}
              aria-hidden
            />
            <span className="truncate">{plainLine(device)}</span>
          </div>
        )}
        {who.where && who.device && who.where !== who.device && (
          <div className="truncate text-xs text-muted-foreground">
            {plainLine(who.where)}
          </div>
        )}
      </div>
      <CardList label="Holding" cards={holding} onOpen={onOpen} />
      <CardList label="Offered" cards={offered} onOpen={onOpen} />
      <CardList label="Watching" cards={watching} onOpen={onOpen} />
      {!holding.length && !offered.length && !watching.length && (
        <p className="text-xs text-muted-foreground">No open cards.</p>
      )}
    </>
  )
}

function CardList({
  label,
  cards,
  onOpen,
}: {
  label: string
  cards: CardSummary[]
  onOpen: (id: string) => void
}) {
  if (!cards.length) return null
  const more = cards.length - MaxRows
  return (
    <div className="grid grid-cols-1 gap-1">
      <div className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
        {label} · {cards.length}
      </div>
      <ul className="grid grid-cols-1 gap-0.5">
        {cards.slice(0, MaxRows).map((c) => (
          <li key={c.id}>
            <button
              type="button"
              className="flex w-full items-baseline gap-1.5 px-1 py-0.5 text-left hover:bg-secondary"
              onClick={() => onOpen(c.id)}
            >
              <span className="shrink-0 font-mono text-xs text-muted-foreground">
                {cardRef(c)}
              </span>
              <span className="truncate">{plainLine(c.title)}</span>
            </button>
          </li>
        ))}
      </ul>
      {more > 0 && (
        <p className="px-1 text-xs text-muted-foreground">and {more} more</p>
      )}
    </div>
  )
}

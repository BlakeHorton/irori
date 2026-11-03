// A card's thread (BoardService.Card's thread): comments, messages about
// the card and what happened, oldest first. These helpers are pure, so
// the thread view and its tests share them.

import {
  BlockerState,
  displayName,
  type AgentRef,
  type CardBlocker,
  type CardThreadItem,
} from "./cards"

import { createContext } from "react"

import { parseTime } from "./irori"

// FocusComment is the comment a notification opened the card at (card
// #177): the thread that holds it shows it, scrolled into view.
export const FocusComment = createContext("")

// threadEntryId is a thread entry's element id, for scrolling to it.
export function threadEntryId(id: string): string {
  return `thread-entry-${id}`
}

// ThreadPage is how many of the newest entries show before "Show earlier".
export const ThreadPage = 50

// ThreadKind mirrors the Go constants on CardThreadItem.kind.
export const ThreadKind = {
  Comment: "comment",
  Message: "message",
  Event: "event",
} as const

export type ThreadBadge = {
  label: string
  tone: "warning" | "slate" | "success"
}

// ThreadLabel is an entry's header: who wrote it, who a message went to,
// the device it was written on ("" for this one) and its badges.
export type ThreadLabel = {
  author: string
  arrow?: string
  device: string
  badges: ThreadBadge[]
  // Under the person's comment: who still has to hear it and why it waits
  // ("garnet is busy: it hears this when its current turn ends"), then who
  // read it (card #159).
  waiting: string[]
}

// threadLabel is the header line for one entry: "A -> B" for a message,
// with "recipient gone" when Irori left it on the card for an ended
// recipient, "nobody has heard this yet" on the person's comment that no
// agent was around to hear (until an agent writes on the card), "held" until the wake policy lets it through (its text isn't
// on the card), "held · dismissed" once the person dismissed it, the
// count of files that stayed on the message. The person's comment's
// pending lines go in waiting, shown under it, not as badges. An
// answer or reply on a blocker in the card's own thread (its item is no
// longer kept) says which item; inBlocker leaves that off, as the entry
// already sits under its item.
export function threadLabel(
  item: CardThreadItem,
  inBlocker = false
): ThreadLabel {
  const badges: ThreadBadge[] = []
  if (item.blockerId && item.kind !== ThreadKind.Event && !inBlocker)
    badges.push({ label: `on ${item.blockerId}`, tone: "slate" })
  if (item.gone) badges.push({ label: "recipient gone", tone: "warning" })
  if (item.unheard)
    badges.push({ label: "nobody has heard this yet", tone: "warning" })
  if (item.held)
    badges.push({
      label: item.dismissed ? "held · dismissed" : "held",
      tone: "slate",
    })
  if (item.edited) badges.push({ label: "edited", tone: "slate" })
  if (item.files)
    badges.push({
      label: item.files === 1 ? "1 file" : `${item.files} files`,
      tone: "slate",
    })
  const to: AgentRef | null | undefined = item.to
  return {
    author: displayName(item.who),
    arrow: to ? displayName(to) : undefined,
    device: item.device ?? "",
    badges,
    waiting: item.pending ?? [],
  }
}

export { BlockerState }

// visible is the entries to show: all of them, or the newest n.
export function visible<T>(items: T[], showAll: boolean, n = ThreadPage): T[] {
  if (showAll || items.length <= n) return items
  return items.slice(items.length - n)
}

// opensInFull reports whether a message entry has more than the card
// shows: its text was cut (it ends pointing at msg:<id>), it hasn't been
// let through the wake policy (no text yet), or it has files.
// BoardService.Message opens the rest for the person.
export function opensInFull(item: CardThreadItem): boolean {
  if (item.kind !== ThreadKind.Message) return false
  return (
    !!item.held ||
    !!item.files ||
    (item.text ?? "").endsWith(`(full: msg:${item.id})`)
  )
}

export type BlockerTone = "amber" | "grey" | "green" | "closed"

// blockerTone is how an item looks: amber while it waits on the person,
// grey once answered (never a check: it isn't resolved), green with the
// check when resolved, and the closed look. "" is an open item from an
// older app.
export function blockerTone(state: string): BlockerTone {
  switch (state) {
    case BlockerState.Answered:
      return "grey"
    case BlockerState.Resolved:
      return "green"
    case BlockerState.Closed:
      return "closed"
  }
  return "amber"
}

// needsPerson reports whether the item waits on the person: open, or
// replied (the agent asked back).
export function needsPerson(state: string): boolean {
  return blockerTone(state) === "amber"
}

// answerable reports whether the person can still answer the item:
// open, answered (to add to their answer) or replied.
export function answerable(state: string): boolean {
  return state !== BlockerState.Resolved && state !== BlockerState.Closed
}

// blockerChip is an item's state chip: "Sent to X" (" · session ended"
// once X has gone) while answered, "Replied" while back with the person,
// "Revised" while open after an agent revised it (card #50: what it asks
// and its options changed under the person's reply), none otherwise (an
// open item is amber, a resolved one has its check).
export function blockerChip(b: CardBlocker): ThreadBadge | null {
  if (b.state === BlockerState.Answered) {
    const to = b.sentTo ?? b.raisedBy
    return {
      label: `Sent to ${displayName(to)}${to.live ? "" : " · session ended"}`,
      tone: "slate",
    }
  }
  if (b.state === BlockerState.Replied)
    return { label: "Replied", tone: "warning" }
  // A done check (card #112): the agent says it's done.
  if (b.doneCheck && (b.state || BlockerState.Open) === BlockerState.Open)
    return { label: "Ready for you", tone: "success" }
  if (
    (b.state || BlockerState.Open) === BlockerState.Open &&
    (b.thread ?? []).some(isRevision)
  )
    return { label: "Revised", tone: "warning" }
  return null
}

// isRevision reports whether a blocker thread entry is an agent revising
// the item: a new question, context or options (or none).
export function isRevision(item: CardThreadItem): boolean {
  return item.kind === ThreadKind.Event && !!item.revised
}

// heardLine says who heard the person's comment (BoardService.Comment):
// who answers it on the card, woken or at their next turn, and the
// watchers, who answer only if it asks them. Nobody: "Nobody has heard
// this yet."
export function heardLine(
  answers: AgentRef[],
  watching: AgentRef[],
  mode: string
): string {
  const names = (rs: AgentRef[]) => {
    const n = rs.map(displayName)
    return n.length > 1
      ? `${n.slice(0, -1).join(", ")} and ${n[n.length - 1]}`
      : n[0]
  }
  const how = (rs: AgentRef[], woken: boolean) =>
    `${names(rs)} ${woken ? (rs.length > 1 ? "are woken" : "is woken") : rs.length > 1 ? "hear it at their next turn" : "hears it at its next turn"}`
  if (!answers.length && !watching.length) return "Nobody has heard this yet."
  const parts: string[] = []
  if (answers.length && mode === "stop")
    parts.push(
      `${names(answers)} ${answers.length > 1 ? "are" : "is"} stopped and will answer here first.`
    )
  else if (answers.length)
    parts.push(`${how(answers, mode === "wake")} and will answer here.`)
  if (watching.length)
    parts.push(
      answers.length
        ? `${how(watching, false)}, watching: they answer only if asked.`
        : `${how(watching, mode === "wake" || mode === "stop")}, watching.`
    )
  return parts.join(" ")
}

// LiveRoster is each agent's live roster state by key (App provides it from
// roster:changed, card #299): a card's AgentRef carries the status of when
// the card was read, the roster the status of now.
export type LiveAgent = { status: string; updatedAt?: string }
export const LiveRoster = createContext<ReadonlyMap<string, LiveAgent>>(new Map())

// StoppedGrace is how long an agent must sit idle before a thread says it
// stopped without replying: between the end of one turn and the wake that
// delivers the person's comment it is idle for a moment.
export const StoppedGrace = 3000

export type WorkingLine = { text: string; tone: "working" | "stopped" }

// workingLine is the live line under a thread (card #299, GitHub #5):
// "oak is working…" while an agent on it (a holder, or the one an ask was
// handed to) is in a turn, by the live roster; "oak stopped without
// replying" once an agent heard the person's latest comment, had a turn
// after it and went idle with nothing on the thread since. None when the
// card is over or nobody is on it.
export function workingLine(
  agents: readonly AgentRef[],
  items: readonly CardThreadItem[],
  live: ReadonlyMap<string, LiveAgent>,
  now: Date,
  over = false
): WorkingLine | null {
  if (over) return null
  const seen = new Set<string>()
  const on = agents.filter((a) => {
    if (a.status === "person" || !a.live || a.offline || seen.has(a.key)) return false
    seen.add(a.key)
    return true
  })
  const status = (a: AgentRef) => live.get(a.key)?.status ?? a.status
  const names = (rs: AgentRef[]) => {
    const n = rs.map(displayName)
    return n.length > 1 ? `${n.slice(0, -1).join(", ")} and ${n[n.length - 1]}` : n[0]
  }
  const busy = on.filter((a) => status(a) === "busy")
  if (busy.length)
    return { text: `${names(busy)} ${busy.length > 1 ? "are" : "is"} working…`, tone: "working" }

  // The person's latest comment, and whether anyone answered it since.
  let p = -1
  for (let i = items.length - 1; i >= 0; i--) {
    if (items[i].kind !== ThreadKind.Event && items[i].who.status === "person") { p = i; break }
  }
  if (p < 0) return null
  const comment = items[p]
  if ((comment.pending ?? []).length) return null // not everyone has heard it yet
  const at = parseTime(comment.at)
  if (!at) return null
  const after = items.slice(p + 1)
  const stopped = on.filter((a) => {
    if (status(a) !== "idle") return false
    if (after.some((it) => it.who.key === a.key)) return false
    const since = parseTime(live.get(a.key)?.updatedAt)
    return !!since && since > at && now.getTime() - since.getTime() >= StoppedGrace
  })
  if (!stopped.length) return null
  return { text: `${names(stopped)} stopped without replying`, tone: "stopped" }
}

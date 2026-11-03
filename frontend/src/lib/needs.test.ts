import { describe, expect, test } from "bun:test"

import { TaskState, withLive, workingNow, type AgentRef, type CardSummary } from "./cards"
import { IDLE_LINE, needsOf, needsYou, rowState, toPerson } from "./needs"
import { State, type Delivery } from "./talk"

const card = (c: Partial<CardSummary>): CardSummary =>
  ({
    id: "c",
    short: "1",
    title: "A card",
    state: TaskState.Working,
    scope: "mine",
    scopeLabel: "My devices",
    openBlockers: 0,
    blockerQuestion: "",
    waiting: 0,
    working: [],
    watching: [],
    talking: [],
    devices: [],
    updatedAt: "2026-11-02T10:00:00Z",
    progress: { done: 0, total: 0 },
    holder: { state: "nobody", who: { key: "", name: "" }, others: 0 },
    ...c,
  }) as CardSummary

const delivery = (d: Partial<Delivery> & { text?: string }): Delivery =>
  ({
    id: "d",
    messageId: "m",
    to: "user:dev",
    state: State.Queued,
    reminded: false,
    createdAt: "2026-11-02T10:00:00Z",
    updatedAt: "2026-11-02T10:00:00Z",
    message: {
      id: "m",
      from: { key: "dev/agent", name: "otter", kind: "claude" },
      to: { kind: "user", value: "user:dev" },
      text: d.text ?? "Hello",
      hop: 0,
      trust: "agent",
      sentAt: "2026-11-02T10:00:00Z",
    },
    ...d,
  }) as Delivery

// Card #305: a held card whose holder sits idle with nothing named.
describe("idle with no reason given", () => {
  const at = (iso: string) => Date.parse(iso)
  const holder = (w: Record<string, unknown> = {}) => ({
    key: "dev/s1",
    name: "orca",
    status: "idle",
    live: true,
    where: "This PC",
    idleSince: "2026-11-02T10:05:00Z",
    ...w,
  })
  const held = (c: Partial<CardSummary> = {}, w: Record<string, unknown> = {}) =>
    card({
      working: [holder(w)] as CardSummary["working"],
      holder: { state: "idle", who: holder(w), others: 0 } as CardSummary["holder"],
      ...c,
    })

  test("calm Waiting at first, Idle after ten minutes", () => {
    expect(rowState(held(), at("2026-11-02T10:10:00Z")).state).toBe("waiting")
    expect(rowState(held(), at("2026-11-02T10:15:00Z"))).toEqual({
      state: "idle",
      line: `${IDLE_LINE} (orca)`,
    })
  })
  test("counts from the later of the card's change and the holder's idle", () => {
    const c = held({ updatedAt: "2026-11-02T10:12:00Z" })
    expect(rowState(c, at("2026-11-02T10:16:00Z")).state).toBe("waiting")
    expect(rowState(c, at("2026-11-02T10:22:00Z")).state).toBe("idle")
  })
  test("a named wait, a busy, gone or offline holder is not idle-unnamed", () => {
    const late = at("2026-11-02T11:00:00Z")
    expect(rowState(held({ waitingOn: 1, waitOn: "kestrel" }), late)).toEqual({
      state: "waiting",
      line: "Waiting on kestrel",
    })
    expect(rowState(held({ openBlockers: 1, blockerQuestion: "Which?" }), late).state).toBe("needs")
    expect(rowState(held({}, { status: "busy" }), late).state).toBe("working")
    expect(rowState(held({}, { status: "gone", live: false }), late).state).toBe("waiting")
    expect(rowState(held({}, { offline: true }), late).state).toBe("waiting")
    expect(rowState(held({ state: TaskState.InReview }), late).state).toBe("waiting")
  })
})

describe("rowState", () => {
  test("the four states", () => {
    expect(rowState(card({ openBlockers: 1, blockerQuestion: "Steam or relay?" }))).toEqual({
      state: "needs",
      line: "Steam or relay?",
    })
    expect(rowState(card({ ready: true, openBlockers: 1 })).state).toBe("needs")
    expect(rowState(card({ state: TaskState.Completed })).state).toBe("done")
    expect(rowState(card({ state: TaskState.InReview })).line).toBe("In review")
    expect(rowState(card({})).state).toBe("waiting")
  })
  test("Agent progress and the card page agree, by the live roster (card #317)", () => {
    const oak = { key: "d/oak", name: "oak", status: "idle", live: true, where: "This PC" } as AgentRef
    const c = card({
      working: [oak],
      holder: { state: "one", who: { key: "d/oak", name: "oak" }, others: 0 } as CardSummary["holder"],
    })
    const live = new Map([["d/oak", { status: "busy" }]])
    for (const [m, working] of [
      [new Map(), false],
      [live, true],
    ] as const) {
      const now = withLive(c, m)
      expect(rowState(now).state === "working").toBe(working)
      expect(workingNow(now).length > 0).toBe(working)
    }
    // A blocker: Needs you on Home, and no working row on the page.
    const blocked = withLive({ ...c, openBlockers: 1 }, live)
    expect(rowState(blocked).state).toBe("needs")
    expect(workingNow(blocked)).toEqual([])
  })
  test("a closed card never needs you", () => {
    expect(needsYou(card({ state: TaskState.Completed, openBlockers: 1 }))).toBe(false)
  })
})

describe("needsOf", () => {
  test("cards, held and messages to the person, newest first, each once", () => {
    const cs = [
      card({ id: "a", openBlockers: 1, updatedAt: "2026-11-02T10:00:00Z" }),
      card({ id: "b", ready: true, openBlockers: 1, updatedAt: "2026-11-02T10:05:00Z" }),
      card({ id: "c" }),
    ]
    const held = delivery({ id: "h", to: "dev/agent2", state: State.Held, createdAt: "2026-11-02T10:02:00Z" })
    const ds = [
      delivery({ id: "m", createdAt: "2026-11-02T10:03:00Z" }),
      delivery({ id: "x", to: "dev/agent2" }),
      delivery({ id: "r", state: State.Answered }),
      held,
    ]
    const got = needsOf(cs, ds, [held]).map((n) => `${n.kind}:${n.id}`)
    expect(got).toEqual(["check:b", "message:m", "held:h", "decision:a"])
  })
  test("toPerson: open messages to a user key only", () => {
    expect(toPerson(delivery({}))).toBe(true)
    expect(toPerson(delivery({ state: State.Held }))).toBe(false)
    expect(toPerson(delivery({ to: "dev/agent" }))).toBe(false)
  })
})

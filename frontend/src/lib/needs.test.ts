import { describe, expect, test } from "bun:test"

import { TaskState, type CardSummary } from "./cards"
import { needsOf, needsYou, rowState, toPerson } from "./needs"
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

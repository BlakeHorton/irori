import { describe, expect, test } from "bun:test"

import { agentCounts, freeIn, whyWaiting } from "./agent-counts"

const a = (status: string, extra: { deviceState?: string; repoKey?: string } = {}) => ({
  status,
  ...extra,
})

describe("agentCounts (card #172)", () => {
  test("shell counts as working, not free; gone isn't counted", () => {
    const c = agentCounts([
      a("busy"),
      a("shell"),
      a("shell"),
      a("idle"),
      a("unresponsive"),
      a("idle", { deviceState: "offline" }),
      a("gone"),
    ])
    expect(c).toEqual({ total: 6, working: 3, running: 2, free: 1, silent: 1, offline: 1 })
  })

  test("free matches who a thread goes to: idle on an online device", () => {
    const agents = [a("idle"), a("idle", { deviceState: "offline" }), a("shell"), a("unresponsive")]
    expect(agentCounts(agents).free).toBe(freeIn(agents))
    expect(freeIn(agents)).toBe(1)
  })
})

describe("whyWaiting (card #172)", () => {
  test("everyone working, some on their own commands", () => {
    expect(whyWaiting([a("busy"), a("shell"), a("shell")])).toBe(
      "Nobody is free: 3 working (2 waiting on a command they started). The first to go idle answers it."
    )
  })
  test("idle but not answering", () => {
    expect(whyWaiting([a("busy"), a("unresponsive"), a("unresponsive")])).toBe(
      "Nobody is free: 1 working, 2 idle but not answering. The first to go idle answers it."
    )
  })
  test("on an offline device", () => {
    expect(whyWaiting([a("idle", { deviceState: "offline" })])).toBe(
      "Nobody is free: 1 on an offline device. The first to go idle answers it."
    )
  })
  test("idle, but none in the repo", () => {
    expect(whyWaiting([a("idle", { repoKey: "mlg" }), a("idle", { repoKey: "mlg" })], "irori")).toBe(
      "2 agents are idle, but none works in irori."
    )
  })
  test("someone free: no reason", () => {
    expect(whyWaiting([a("idle"), a("busy")])).toBe("")
    expect(whyWaiting([a("idle", { repoKey: "irori" })], "irori")).toBe("")
  })
  test("no agents", () => {
    expect(whyWaiting([a("gone")])).toBe("No agents are running.")
  })
})

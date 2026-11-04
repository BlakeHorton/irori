import { describe, expect, test } from "bun:test"
import { sameAs } from "./talk"

// sameAs lets useLatest and the desks store keep an unchanged answer, so
// App doesn't re-render the window on every board change (0.11.1).
describe("sameAs", () => {
  test("an equal answer matches the last one, a changed one doesn't", () => {
    const key = { current: null as string | null }
    expect(sameAs(key, [{ id: 1, read: false }])).toBe(false)
    expect(sameAs(key, [{ id: 1, read: false }])).toBe(true)
    expect(sameAs(key, [{ id: 1, read: true }])).toBe(false)
    expect(sameAs(key, [{ id: 1, read: true }])).toBe(true)
  })
  test("a value that can't be serialized never matches", () => {
    const key = { current: null as string | null }
    const loop: Record<string, unknown> = {}
    loop.self = loop
    expect(sameAs(key, loop)).toBe(false)
    expect(sameAs(key, loop)).toBe(false)
    expect(sameAs(key, undefined)).toBe(false)
    expect(sameAs(key, undefined)).toBe(false)
  })
})

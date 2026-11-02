import { describe, expect, test } from "bun:test"

import {
  InviteState,
  InviteStatus,
  boards,
  boardsNow,
  inviteOpen,
  reloadBoards,
  subscribeBoards,
  suggestTag,
  validTag,
} from "./boards"

// settled waits for the reloads a change started (the sample's list is a
// resolved promise, so a few turns of the queue are enough).
async function settled() {
  for (let i = 0; i < 5; i++) await Promise.resolve()
}

describe("the shared boards list (card #233)", () => {
  // The sample runs outside the host; bun has no window.
  const g = globalThis as unknown as { window?: unknown }
  g.window ??= { location: { search: "" } }

  test("every reader sees one list, read again when a board moves", async () => {
    let a = 0
    let b = 0
    const offA = subscribeBoards(() => a++)
    const offB = subscribeBoards(() => b++)
    await settled()
    const before = boardsNow().boards.map((x) => x.tag)
    expect(before).toContain("mlg")
    // A board arriving (here the sample's New board; in the app the
    // host's boards:changed) reaches both readers without a reload call.
    const made = await boards.create("Wind", "wind")
    await settled()
    const after = boardsNow().boards
    expect(after.some((x) => x.boardId === made.boardId)).toBe(true)
    expect(a).toBeGreaterThan(1)
    expect(b).toBeGreaterThan(1)
    // A page opening later (a new reader) gets the same list, not its own.
    let c = 0
    const offC = subscribeBoards(() => c++)
    await settled()
    expect(c).toBeGreaterThan(0)
    expect(boardsNow().boards.map((x) => x.boardId)).toEqual(after.map((x) => x.boardId))
    offA()
    offB()
    offC()
    await boards.leave(made.boardId)
  })

  test("an older read that finishes last doesn't win", async () => {
    const off = subscribeBoards(() => {})
    await settled()
    const first = reloadBoards()
    const made = await boards.create("Late", "late")
    await Promise.all([first, reloadBoards()])
    expect(boardsNow().boards.some((x) => x.boardId === made.boardId)).toBe(true)
    off()
    await boards.leave(made.boardId)
  })
})

describe("board tags (identity.ValidBoardTag)", () => {
  test("a lowercase letter, then up to 15 letters, digits or '-'", () => {
    for (const t of ["mlg", "a", "site-2", "abcdefghijklmnop"]) expect(validTag(t)).toBe(true)
    for (const t of ["", "MLG", "1mlg", "-x", "has space", "abcdefghijklmnopq", "m#1"]) expect(validTag(t)).toBe(false)
  })
  test("New board suggests one from the name's first word", () => {
    expect(suggestTag("MLG")).toBe("mlg")
    expect(suggestTag("Website refresh")).toBe("website")
    expect(suggestTag("2026 plans")).toBe("")
    expect(suggestTag("")).toBe("")
  })
})

describe("invites", () => {
  test("inviteOpen is true only while an invite is in progress", () => {
    expect(inviteOpen(null)).toBe(false)
    expect(inviteOpen(InviteStatus({}))).toBe(false)
    for (const st of [InviteState.Waiting, InviteState.Words, InviteState.Joining])
      expect(inviteOpen(InviteStatus({ state: st }))).toBe(true)
    for (const st of [InviteState.Done, InviteState.Failed])
      expect(inviteOpen(InviteStatus({ state: st }))).toBe(false)
  })

})

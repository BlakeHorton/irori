import { describe, expect, test } from "bun:test"

import { AgentRef } from "../gen/models"
import {
  addMention,
  askingLine,
  exactUnique,
  filterCandidates,
  hasMention,
  insertMention,
  keepMentions,
  MaxMentions,
  menuKey,
  moveHighlight,
  notDelivered,
  settle,
  splitMentions,
  tokenAt,
  type MentionCandidate,
} from "./mentions"

function cand(
  key: string,
  name: string,
  opts: { status?: string; device?: string } = {}
): MentionCandidate {
  return {
    who: AgentRef({
      key,
      name,
      status: opts.status ?? "idle",
      live: opts.status !== "gone",
      where: opts.device ?? "This PC",
      device: opts.device,
    }),
    cwd: "",
    repo: "",
  }
}

const oak = cand("agent:oak", "oak")
const oakMac = cand("agent:oak-mac", "oak", { device: "Mac" })
const oaktree = cand("agent:oaktree", "oak tree")
const fern = cand("agent:fern", "fern")
const cedar = cand("agent:cedar", "cedar", { status: "gone" })
const boak = cand("agent:boak", "boak")
const all = [fern, cedar, oakMac, boak, oaktree, oak]
const names = all.map((c) => c.who.name)
const keys = (cs: MentionCandidate[]) => cs.map((c) => c.who.key)

describe("tokenAt", () => {
  test("@ at the start, after whitespace or ( starts a token", () => {
    expect(tokenAt("@oa", 3)).toEqual({ start: 0, query: "oa" })
    expect(tokenAt("hi @", 4)).toEqual({ start: 3, query: "" })
    expect(tokenAt("hi\n@f", 5)).toEqual({ start: 3, query: "f" })
    expect(tokenAt("(@f", 3)).toEqual({ start: 1, query: "f" })
  })

  test("a@b is not a token", () => {
    expect(tokenAt("mail a@b", 8)).toBeNull()
  })

  test("the query runs to the caret", () => {
    expect(tokenAt("@fern and more", 3)).toEqual({ start: 0, query: "fe" })
  })

  test("a space ends it, unless a name still holds it", () => {
    expect(tokenAt("@oak hello", 10, names)).toBeNull()
    expect(tokenAt("@oak ", 5, ["oak"])).toBeNull()
    expect(tokenAt("@oak t", 6, names)).toEqual({ start: 0, query: "oak t" })
    expect(tokenAt("@oak\n", 5, names)).toBeNull()
  })
})

describe("filterCandidates", () => {
  test("names holding the query, ignoring case", () => {
    expect(keys(filterCandidates(all, "OAK"))).toEqual([
      "agent:oak",
      "agent:oaktree",
      "agent:oak-mac",
      "agent:boak",
    ])
  })

  test("live first, this device's before others', gone last", () => {
    expect(keys(filterCandidates(all, ""))).toEqual([
      "agent:boak",
      "agent:fern",
      "agent:oak",
      "agent:oaktree",
      "agent:oak-mac",
      "agent:cedar",
    ])
  })

  test("at most 20 rows", () => {
    const many = Array.from({ length: 30 }, (_, i) => cand(`k${i}`, `a${i}`))
    expect(filterCandidates(many, "a")).toHaveLength(20)
  })
})

describe("moveHighlight", () => {
  test("Tab enters at the first row, then steps and wraps", () => {
    let i = -1
    const seen: number[] = []
    for (let n = 0; n < 4; n++) seen.push((i = moveHighlight(i, 3, 1)))
    expect(seen).toEqual([0, 1, 2, 0])
  })

  test("Shift+Tab goes back, wrapping; from none it is the last row", () => {
    expect(moveHighlight(-1, 3, -1)).toBe(2)
    expect(moveHighlight(0, 3, -1)).toBe(2)
    expect(moveHighlight(2, 3, -1)).toBe(1)
  })

  test("no rows, no highlight", () => {
    expect(moveHighlight(0, 0, 1)).toBe(-1)
  })
})

describe("menuKey (the Enter guard)", () => {
  const k = (key: string, shiftKey = false, isComposing = false) => ({
    key,
    shiftKey,
    isComposing,
  })

  test("open with rows: Tab, arrows, Enter and Esc are the picker's", () => {
    expect(menuKey(k("Tab"), true, 2)).toBe("next")
    expect(menuKey(k("Tab", true), true, 2)).toBe("prev")
    expect(menuKey(k("ArrowDown"), true, 2)).toBe("next")
    expect(menuKey(k("ArrowUp"), true, 2)).toBe("prev")
    expect(menuKey(k("Enter"), true, 2)).toBe("pick")
    expect(menuKey(k("Escape"), true, 2)).toBe("close")
  })

  test("closed, Enter is the site's (it sends)", () => {
    expect(menuKey(k("Enter"), false, 2)).toBeNull()
    expect(menuKey(k("Tab"), false, 2)).toBeNull()
  })

  test("no matches: Enter sends and Tab moves focus; Esc still closes", () => {
    expect(menuKey(k("Enter"), true, 0)).toBeNull()
    expect(menuKey(k("Tab"), true, 0)).toBeNull()
    expect(menuKey(k("Escape"), true, 0)).toBe("close")
  })

  test("Shift+Enter stays a new line; an IME gets every key", () => {
    expect(menuKey(k("Enter", true), true, 2)).toBeNull()
    expect(menuKey(k("Enter", false, true), true, 2)).toBeNull()
    expect(menuKey(k("Escape", false, true), true, 2)).toBeNull()
  })
})

describe("insertMention", () => {
  test("puts @name and a space in place of the query", () => {
    expect(insertMention("hi @oa", { start: 3, query: "oa" }, 6, "oak")).toEqual({
      text: "hi @oak ",
      caret: 8,
    })
  })

  test("keeps the text after the caret, without doubling a space", () => {
    expect(insertMention("@f there", { start: 0, query: "f" }, 2, "fern")).toEqual({
      text: "@fern there",
      caret: 6,
    })
    expect(insertMention("@fx", { start: 0, query: "f" }, 2, "fern")).toEqual({
      text: "@fern x",
      caret: 6,
    })
  })
})

describe("token tracking on edit", () => {
  const ms = [
    { key: "agent:oak", name: "oak" },
    { key: "agent:fern", name: "fern" },
  ]

  test("removing @name drops its key", () => {
    expect(keepMentions(ms, "@oak and @fern")).toBe(ms)
    expect(keepMentions(ms, "@oak and ")).toEqual([ms[0]])
    expect(keepMentions(ms, "@oa and @fern")).toEqual([ms[1]])
    expect(keepMentions(ms, "")).toEqual([])
  })

  test("the name must stand whole, case as inserted", () => {
    expect(hasMention("@oakx", "oak")).toBe(false)
    expect(hasMention("@Oak", "oak")).toBe(false)
    expect(hasMention("(@oak).", "oak")).toBe(true)
    expect(hasMention("x@oak", "oak")).toBe(false)
    expect(hasMention("@oak tree", "oak tree")).toBe(true)
  })

  test("one entry per key, at most ten", () => {
    expect(addMention(ms, { key: "agent:oak", name: "oak" })).toBe(ms)
    let many: { key: string; name: string }[] = []
    for (let i = 0; i < 12; i++) many = addMention(many, { key: `k${i}`, name: `n${i}` })
    expect(many).toHaveLength(MaxMentions)
  })
})

describe("exact names", () => {
  test("one live agent named exactly that, ignoring case", () => {
    expect(exactUnique("FERN", all)?.who.key).toBe("agent:fern")
    // two live oaks: ambiguous
    expect(exactUnique("oak", all)).toBeNull()
    // gone agents never match by name
    expect(exactUnique("cedar", all)).toBeNull()
    expect(exactUnique("", all)).toBeNull()
  })

  test("settle records the exact name at the caret, as typed", () => {
    expect(settle("hi @Fern", 8, [], all)).toEqual([{ key: "agent:fern", name: "Fern" }])
    expect(settle("hi @fer", 7, [], all)).toEqual([])
    const ms = [{ key: "agent:fern", name: "fern" }]
    expect(settle("@fern", 5, ms, all)).toBe(ms)
  })
})

describe("splitMentions", () => {
  const ms = [
    { name: "oak", id: 1 },
    { name: "oak tree", id: 2 },
  ]

  test("cuts around each named @name, longest first", () => {
    expect(splitMentions("ask @oak tree and @oak.", ms)).toEqual([
      { kind: "text", text: "ask " },
      { kind: "mention", text: "@oak tree", mention: ms[1] },
      { kind: "text", text: " and " },
      { kind: "mention", text: "@oak", mention: ms[0] },
      { kind: "text", text: "." },
    ])
  })

  test("an @word no mention names stays text", () => {
    expect(splitMentions("hi @fern, a@oak", ms)).toEqual([{ kind: "text", text: "hi @fern, a@oak" }])
    expect(splitMentions("plain", [])).toEqual([{ kind: "text", text: "plain" }])
  })
})

describe("lines", () => {
  test("not delivered names each miss", () => {
    expect(
      notDelivered([
        { name: "oak", state: "woke" },
        { name: "fern", state: "gone" },
        { name: "cedr", state: "unknown" },
      ])
    ).toBe("Not delivered: fern is gone; no agent named cedr.")
    expect(notDelivered([{ name: "oak", state: "woke" }])).toBe("")
    expect(notDelivered(undefined)).toBe("")
  })

  test("asking line", () => {
    expect(askingLine([{ key: "a", name: "oak" }])).toBe("Asking @oak only")
    expect(
      askingLine([
        { key: "a", name: "oak" },
        { key: "b", name: "fern" },
        { key: "c", name: "elm" },
      ])
    ).toBe("Asking @oak, @fern and @elm only")
  })
})

test("@oak inside a recorded @oak tree doesn't keep oak", () => {
  const ms = [
    { key: "agent:oak", name: "oak" },
    { key: "agent:oaktree", name: "oak tree" },
  ]
  expect(keepMentions(ms, "@oak tree hi")).toEqual([ms[1]])
  expect(keepMentions(ms, "@oak tree and @oak")).toBe(ms)
})

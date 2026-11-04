import { describe, expect, test } from "bun:test"

import { effortLabel, modelLabel } from "./agents"

describe("Settings › Agents labels", () => {
  test("the default is marked, the rest named", () => {
    expect(modelLabel("sonnet", "sonnet")).toBe("Sonnet (default)")
    expect(modelLabel("opus", "sonnet")).toBe("Opus")
    expect(effortLabel("xhigh", "medium")).toBe("Extra high")
    expect(effortLabel("medium", "medium")).toBe("Medium (default)")
  })
  test("an unknown value shows as itself", () => {
    expect(modelLabel("fable", "sonnet")).toBe("fable")
  })
})

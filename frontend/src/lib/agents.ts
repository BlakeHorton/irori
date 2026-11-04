// Settings › Agents (card #448, docs/design/spawned-agents.md §10, sa-6):
// a board's spawned-agent switch and the model and effort its agents start
// with. The app only writes them; no tool can. Outside the app the preview
// keeps a sample in memory so the pane can be looked at.

import { useCallback, useEffect, useState } from "react"

import type { AgentsChange, AgentsSettings } from "../gen/bindings"
import { AgentsService } from "./host"
import { inWails } from "./irori"
import { previewFlag } from "./preview"

export type { AgentsSettings }

// sampleAgents is the preview's private board: chosen and on, or with
// ?spawn=first on by default and not yet chosen, so Home shows the
// first-run notice.
export const sampleAgents: AgentsSettings = {
  boardId: "",
  on: true,
  chosen: previewFlag("spawn") !== "first",
  model: "sonnet",
  effort: "medium",
  models: ["sonnet", "opus", "haiku"],
  efforts: ["low", "medium", "high", "xhigh", "max"],
  defaultModel: "sonnet",
  defaultEffort: "medium",
}

const modelNames: Record<string, string> = {
  sonnet: "Sonnet",
  opus: "Opus",
  haiku: "Haiku",
}

const effortNames: Record<string, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  xhigh: "Extra high",
  max: "Max",
}

// choiceLabel names a model or effort, marking the default.
export function choiceLabel(
  v: string,
  names: Record<string, string>,
  def: string,
): string {
  const n = names[v] ?? v
  return v === def ? `${n} (default)` : n
}

export const modelLabel = (v: string, def: string) =>
  choiceLabel(v, modelNames, def)
export const effortLabel = (v: string, def: string) =>
  choiceLabel(v, effortNames, def)

// useAgents loads a board's settings while open and saves a change,
// keeping the last good settings when a save fails.
export function useAgents(open: boolean, boardId = "") {
  const [s, setS] = useState<AgentsSettings | null>(() =>
    inWails() ? null : { ...sampleAgents, boardId },
  )
  const [error, setError] = useState("")
  useEffect(() => {
    if (!open || !inWails()) return
    let live = true
    AgentsService.Settings(boardId).then(
      (v) => live && (setS(v), setError("")),
      (e: Error) => live && setError(e.message),
    )
    return () => {
      live = false
    }
  }, [open, boardId])
  const set = useCallback(
    (c: Omit<AgentsChange, "boardId">) => {
      const change: AgentsChange = {
        boardId,
        on: c.on ?? null,
        model: c.model ?? null,
        effort: c.effort ?? null,
      }
      if (!inWails()) {
        setS((x) =>
          x
            ? {
                ...x,
                on: c.on ?? x.on,
                chosen: x.chosen || c.on != null,
                model: c.model ?? x.model,
                effort: c.effort ?? x.effort,
              }
            : x,
        )
        return
      }
      AgentsService.Set(change).then(
        (v) => (setS(v), setError("")),
        (e: Error) => setError(e.message),
      )
    },
    [boardId],
  )
  return { settings: s, error, set }
}

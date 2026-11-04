// AgentsPane is Settings › Agents (card #448, docs/design/spawned-agents.md
// §10, mockup sa-6), as much of it as part 4 needs: your own board's
// switch for agents Irori starts per card, and the model and effort they
// start with. Only you set these, here; no agent's tool can. The rest of
// sa-6 (permissions, caps, limits, cost) arrives with parts 5, 6 and 12.

import { useId } from "react"

import { ChoiceSelect } from "@/components/common/choice-select"
import { PermissionsSection } from "@/components/settings/agent-permissions"
import { Switch } from "@/components/ui/switch"
import { effortLabel, modelLabel, useAgents } from "@/lib/agents"

export function AgentsPane({ open }: { open: boolean }) {
  const { settings: s, error, set } = useAgents(open)
  const id = useId()
  if (!s) {
    return (
      <p className="text-muted-foreground">
        {error ? `Could not load: ${error}` : "Loading…"}
      </p>
    )
  }
  return (
    <div className="grid gap-4">
      <div className="flex items-start justify-between gap-3">
        <span className="flex flex-col">
          <label htmlFor={id} className="cursor-pointer text-ink">
            Start an agent for each card
          </label>
          <span id={`${id}-hint`} className="text-xs text-muted-foreground">
            On your own board, Irori starts a fresh agent for every card you
            post instead of waiting for a free one, and ends it when the card
            is done.
          </span>
        </span>
        <Switch
          id={id}
          checked={s.on}
          aria-describedby={`${id}-hint`}
          onCheckedChange={(v) => set({ on: v })}
          className="mt-0.5"
        />
      </div>
      <section className="grid gap-2">
        <h3 className="text-sm font-medium text-ink">
          What a new agent starts with
        </h3>
        <p className="text-[12.5px] text-muted-foreground">
          Every agent Irori starts uses these. One already working keeps what
          it started with.
        </p>
        <div className="grid gap-2 sm:grid-cols-2">
          <ChoiceSelect
            label="Model"
            showLabel
            size="sm"
            value={s.model}
            choices={s.models.map((m) => ({
              value: m,
              label: modelLabel(m, s.defaultModel),
            }))}
            onValueChange={(v) => set({ model: v })}
            className="text-[13px]"
          />
          <ChoiceSelect
            label="Effort"
            showLabel
            size="sm"
            value={s.effort}
            choices={s.efforts.map((e) => ({
              value: e,
              label: effortLabel(e, s.defaultEffort),
            }))}
            onValueChange={(v) => set({ effort: v })}
            className="text-[13px]"
          />
        </div>
      </section>
      <PermissionsSection open={open} />
      {error && <p className="text-[12.5px] text-destructive">{error}</p>}
    </div>
  )
}

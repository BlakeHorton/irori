# Stop and Pause: design

*2026-11-01, card #207, from Stephen's #206: "Irori only wakes idle agents or queues for the next turn; a busy agent keeps going until it reads you. I need a hard stop." Builds on main @ ca36d22: the delivery modes (`inbox::mode`), the attach stream (`api/src/attach.rs`, `api/src/stream.rs`), the mod (`mod/hooks/register.tsx`) and `task.archived` as the model for a person-only card flag.*

Two things for the person, from the app:

- **Stop:** cut a busy agent's running turn now, and make what you wrote its next prompt. It must answer you before anything else.
- **Pause a card:** stop every holder of a card, and keep them from doing anything but talk to you (Irori's own tools only) until you resume it.

Both work across devices: a Stop from the PC reaches an agent on the Mac.

## 1. Stop is a third delivery mode

Today a delivery has two modes, `wake` and `next_turn` (`inbox/src/lib.rs:263`). Stop is a third, `stop`. It is a wake that must not wait.

- **Sending.** The same `send_with` path as any person message (`inbox/src/messages.rs`). It writes `message.sent` and `delivery.state` rows to the sender's log, the rows replicate to the recipient's device, and that device's `host_hold` decides the hold (`inbox/src/mesh.rs`). Nothing new crosses the mesh: the mode is a field of the rows that already travel. **Only the person may send `stop`**: an agent's send (the `send` tool) with mode `stop` is refused.
- **Holding.** A person's message is not held by the sender policy today. Stop is also not held by Pause all (`policy.paused`), since the person sending it is the one who paused. A stop to an agent that isn't reachable waits, like any wake, until it attaches.
- **Planning a wake** (`inbox/src/wake.rs` `next_wake`). A stop item **jumps the gate**: one outstanding wake normally holds the next back until its turn ends (`WakeRec::gates`). A stop goes out even then, alone in its own wake (other waiting items ride the next one). Its prompt starts with a fixed header, so the agent can't mistake it for an ordinary message:
  > `STOP from <person>: your running turn was cut. Answer <person> first and do nothing else until they say so.`
  The text follows, and then the usual reply instructions.
- **The attach event.** `deliver` gains a field `stop: true` (omitted when false, like every attach field). Only a mod of 0.5.0 or newer gets it (`STOP_SINCE`, the same check as `APPEND_SINCE`). An older mod gets the same wake as a normal `deliver`: it then reads it after its turn, as today, which is the old behaviour, not a failure.
- **The mod** (`register.tsx`). It remembers the running main-loop turn's id (set at `turn.start` with no `agentId`, cleared at `turn.complete`). On a `deliver` with `stop`:
  1. If a main turn runs, `$.turn.abort({ turnId })`: the turn and its running tools are cancelled, with no interruption marker.
  2. `$.prompt.submit({ text })`: the session is idle now, so the stop's prompt starts the next turn at once.
  3. Report delivered and engaged as for any wake. The aborted turn's own wake (if one ran) is reported `turn-ended`, so the Core's gate opens.

  If no turn runs, step 1 is skipped: a stop to an idle agent is a wake.

## 2. Pause is a card flag plus stops

- **The row.** `task.paused {paused: bool, note?}`, replicated, private log only, like `task.archived`: only the person writes it (`actor == user`, the owner device decides as for archive), newest row wins. Folded onto `Task.paused`. Shared-board cards: not in this card (a person's pause over another person's agents needs decision 26's autonomy rules; the button is hidden there).
- **Pausing** writes the row, then sends each holder (holders, co-holders) a `stop` delivery with `pause: "<ref>"` and the text:
  > `<person> paused #N "<title>". Your tools are off for this card except Irori's own: answer <person> on the card (board_comment) and wait.`
  plus the person's note if they wrote one.
- **Resuming** writes `paused: false`, then sends each holder a `wake` with `resume: "<ref>"`: `<person> resumed #N: carry on.` A person comment that says so is not parsed: Resume is the button.
- **The mod's gate.** The mod keeps the set of cards it is paused on.
  - A `deliver` with `pause` adds the ref; one with `resume` removes it.
  - At `hello` (a restart, a reattach) it rebuilds the set from `GET /v1/board`: the cards it holds with `paused` set.
  - A `tool.call` hook on every tool: while the set is not empty, any tool that is not `mcp__irori__*` gets `{ result: "<person> paused #N: answer them on the card and wait." }` without running.
- **The window** shows the card as **Paused** (a badge, and the header's Pause button reads Resume).

## 3. The window

- **Stop on a comment and a thread reply:** the CommentBox's wake choice gets a third option, **Stop and read**. The comment is written as today; its notices to the holders go with mode `stop` instead of `wake`.
- **Stop on a message Reply:** the Reply form gets **Stop and read** beside Send.
- **Stop on an agent:** on its hover card (`AgentHover`) and in Who's on it: **Stop**, which opens a one-line box (empty is fine: then the text is "Stop and wait for me").
- **Pause / Resume** on the card header (`PageHeader` extra, next to Pin).

## 4. What isn't in this card

- Stopping a subagent's turn on its own: an abort cuts the main loop, which stops its subagents too.
- Pausing shared-board cards (§2).
- A paused state blocking Irori's own tools: the agent must still answer on the card.

## 5. Tests

- inbox: a stop jumps a running wake's gate; an agent's stop is refused; Pause all doesn't hold a stop; a stop crosses devices (`inbox/tests/mesh.rs`).
- coord: `task.paused` folds, only the person, newest wins, private log only.
- attach: `deliver` carries `stop`, `pause`, `resume` for a 0.5.0 mod, none for 0.4.x.
- mod: unit tests for the gate and the stop sequence (`mod/unit`).
- agent-check: a `stop` scenario: a session busy on a long task is stopped and answers the stop's question first.

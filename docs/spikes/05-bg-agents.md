# Spike 05: a fresh background agent per card with `claude --bg`

*2026-11-03, card #439 (from Stephen's thread #438). PC: Windows 11, Claude Code 2.1.317 (Lead-Lemmings). Mac: macOS 26, 128 GB / 18 cores, Claude Code 2.1.318 (caribou). Probe cards #440 (PC), #441 and #442 (Mac). No product code; every spawned agent was stopped and removed.*

**Question:** can Irori start a fresh Claude agent for one card, have it take and finish that card, then track and clean it up, so agents don't sit on long context? And what does that cost?

**Answer: yes, for cards that need little context.** Every step works with commands Claude Code already has. Four things have to be built around them: trust, permissions, hand-out, and cleanup.

## What ran

1. `claude --bg -n <name> [--model M --effort E --permission-mode P] "<brief>"`, where the brief says: board_join the probe card, comment your name and cwd, set it completed, do nothing else.
2. Watched the card, `claude agents --json` and `claude logs <id>`.
3. `claude stop <id>`, then `claude rm <id>`.
4. Several at once: three on the PC, six on the Mac (#442).

## Results

| Question | PC | Mac |
|---|---|---|
| Start and id | Returns in ~1.0 s: `backgrounded · <id> · <name>` plus attach/logs/stop hints | Returns in 1.4 s; the session is up at 2.9 s. The first `--bg` also starts a transient `claude daemon` that keeps one warm spare |
| Id mapping | The short id is the first 8 characters of the session id, and Irori's agent key is `<device>/<session id>`, so they map directly. `-n/--name` sets both the Irori name and the `claude agents` name | same |
| Attaches to Irori, takes the card | Yes: the irori plugin loads, and the agent joined #440, commented and completed it in ~12 s of its own turn | Yes: the plugin loads through `CLAUDE_CODE_PLUGIN_DIRS` (no `--plugin-dir`), and its `irori attach --session <id>` child starts with it. #441 was done 12 s after launch |
| Several at once | 3 started within 3 s, all done in under 20 s | 6 launched at once all commented 8 to 14 s after launch |
| Memory | ~490 MB working set per agent (two `claude` processes each); back to baseline after stop and rm | ~470 to 620 MB RSS per idle agent, plus ~50 MB of children (irori attach 12 MB, user MCP servers). Six came to 3.3 GB |
| Tokens and cost | $0.44 on Opus/medium (one probe) | 50 to 130k tokens, ~$0.10 per probe on Sonnet/low, mostly the irori tool descriptions |
| Track | `claude agents --json`: id, pid, kind=background, name, status (busy/idle), state (working/done/failed); `--all` keeps finished ones. `claude logs <id>` returns raw terminal output (strip the ANSI codes) | same |
| End | `claude stop` 0.47 s: the session and its children end, and Irori drops it from the roster within seconds. `claude rm` removes it from the list | `stop` 0.5 s, `rm` 0.6 s |

## Problems the build must handle

1. **Trust.** `--bg` refuses an untrusted folder: "Workspace not trusted. Run `claude` in <dir> once and accept the trust prompt". A plain folder inherits trust from a trusted parent, but **a git repo needs its own trust**. On the PC `~/projects` is trusted and `~/projects/irori` isn't. So Irori must check trust before spawning and say why it can't, or offer a one-time "trust this repo" step that the person confirms. Never trust silently.
2. **Permissions.** With no `--permission-mode`, the agent inherits the caller's mode (bypass on both machines). With `default`, irori MCP calls and a plain `echo` ran, but a real write would prompt and leave the agent waiting with nobody attached. Irori must always pass the mode explicitly, plus an allow-list (`--allowedTools`) or `acceptEdits` for card work. Which one is Stephen's call.
3. **Hand-out after finishing.** A finished agent stays listed as idle/done and Irori treats it as free: the PC probe had an offer within 2 minutes. A spawned agent must be kept out of hand-out (marked "spawned for #N") and stopped as soon as its card is completed or canceled.
4. **Stale holder.** After stop and rm, the finished card still showed "held by" the removed agent until Stephen marked it Done (#441). Irori should treat a removed spawned agent as gone.
5. **Worktrees (not tested).** `-w` needs a trusted repo, and none could be trusted without touching a live one, so worktree creation and `rm`'s "and its worktree when that is safe" are still unverified. This is the first thing for the build's first slice to check, in a repo the person trusts.
6. **Windows windows.** No console window appeared, but the spawns ran from a console shell. Irori's GUI spawn must use `CREATE_NO_WINDOW` (as Revive and prwatch do, #310).
7. **Tool descriptions.** The mod warns at every start that the board tool description (5,681 characters) is over the 4,096 the model sees up front. It's a cost and clarity issue for every spawned agent, so trim it.

## Recommendation

**Build it**, for cards that need little context. The shape:

- **Spawn.** On hand-out, for cards the person marks (a board setting or a per-card switch, his call), Irori runs `claude --bg --name <card-agent> --model <m> --effort <e> --permission-mode <mode> [--allowedTools …] [-w <card>] "<brief: board_join #N, the card's objective and done-when>"` in the card's trusted repo, with `CREATE_NO_WINDOW`. It refuses with a reason if the repo isn't trusted.
- **Track.** Map the short id to the session key, and show spawned agents in the roster with Attach (`claude attach <id>`), Logs, Stop and Remove, plus a cap on how many run at once (memory: ~0.5 GB each).
- **Isolate.** A spawned agent takes only its own card and is never offered another.
- **Clean up.** When its card is completed (or canceled), run `claude stop` then `claude rm`, and mark the agent gone so the card shows no live holder.
- **Long-context terminal agents** don't change; they get cards only when the person chooses.

Decisions for Stephen before building: which cards spawn, the default model and effort, the permission mode and allow-list, worktree per card or the repo itself, and the concurrency cap.

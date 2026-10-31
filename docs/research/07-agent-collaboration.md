# Agents working together on one piece of work

*Irori research, 2026-10-31, card #78. Stephen: "maybe we need a separate task for figuring out how agents collaborate more. Right now I think agents pretty much stay 1:1 on tasks. Need to put value on the use-case though." Sources: the Board's cards and threads (#20, #41, #42-#72, #73-#77), the PC lead sessions' transcripts, git log, docs/design/board.md and waits-signals.md, and the irori-* memory notes. Not yet signed off: see Decisions for Stephen.*

## Short version

Agents already work together a lot, but almost all of it runs through one lead agent that relays messages by hand. The Board records one holder per card and treats everyone else as a passive watcher. So the lead carries every hand-off, every "it landed, go" ping and every status question, and each of those is a wake of the session with the biggest context. The two failures Stephen saw on 10-30 (an assigned agent idle for an hour, a promised ping forgotten for an hour) both come from that.

The highest-value fixes are small: an assignment that wakes an idle assignee, a hand-off note that the card shows until it's taken, and "wake me when #49 is done" so nobody has to remember a ping. @mentions (#77) cover the "talk to that agent on this card" half. They don't cover state: who owes what to whom, and when.

## Settled since: who takes a card

Stephen, 10-31, after seeing Lead hold #77-#80 at once: put the task up and see if someone claims it; an agent picks up another card only when it becomes free. This answers part of case G and leans decision 6 toward (a). Card #82 changes Irori's instructions to agents and how an unheld card reaches free agents. Done on main with #82 (decision 40): an unheld card an agent posts goes to one free agent at a time, the one idle longest, and moves on only on events (it gives it back, its turn ends without `board_join`, or it goes away), as asks do since #91; `board_join` refuses an agent already holding a card in progress or owning an open step on one; and the tool guidance says post-then-claim, with `board_assign` only when one agent is clearly the right one (docs/design/board.md, "Card #82").

## 1. Real cases

### A. One lead fans a big card out to several agents

- **#42 shared boards stage 2** (10-30): 18 subtasks (#43-#49, #58-#67, #72) over five agents on two machines (PC lead start-irori-build; Mac agents birch, bramble, fennel, oak). The lead made every card, assigned it, and sent a separate "Hi fennel ... Your card is #46 ... Please start now" DM, since assigning alone doesn't wake (#46 thread). Stephen had added the agents by hand: "I've added 3 more agents alongside birch to help you out. Hopefully you can communicate with all of them" (10-30 18:18).
- **#51 parity audit** (10-30): 5 subtasks; "fennel takes #55 ..., oak takes #56" posted as a comment by the lead.
- **#20 subtasks** (10-29): Stephen "activated 2 more claude sessions "Bob" and "John" please delegate work with them so you can get more parallelism" (10-29 04:22). #30-#32 went to them.
- **#41 Rust rewrite M4** (10-29): ran 2 then 3 agents in sequence; Stephen: "about to hit 4 hours... Really hope we can parallelize going forward" (10-30 01:20). Lesson kept in parallelize-milestones.md.
- **Pause and resume** (10-31 02:06): the lead sent the same "finish your step, push, post a state note, reply paused" DM to five agents one by one (#42 thread); #72 later resumed from those notes.

*Cost to Stephen:* he starts the helpers himself and has to ask where things stand ("how far along are we?", 10-30 20:43). *How often:* every large build since 10-29 (#20, #41, #42, #51).

### B. Hand-offs: a card or part moves to another agent

- **#56, assigned and idle** (10-30): oak sat idle about an hour on #56 because `board_assign` only tells at the assignee's next turn and an idle agent has none. The lead's later DM: "Your next card is #56 (assigned an hour ago, but you weren't woken)". Still true on main: `assign_notice` (crates/coord/src/board/comments.rs) leaves `wake` false.
- **#66, split by DM** (10-30): oak handed s4's window half and s6 to fennel: "#66 is already a subtask of #42, so I can't split it further; this message is the handoff." fennel did both (3cbd557, c108dad), yet the card still lists only oak as holder.
- **#58 then #57 then #48** (10-30 20:53-21:01): the lead chained fennel from the Mac check run to the main-into-boards2 merge ("since you know #57 best") to the five-Core harness, each by DM, each a fresh wake of both sides.
- **#72 release, PC to Mac and back** (10-31): s11 PC publish, s12 Mac release by birch, then the PC's -Finalize. Each hop was a DM and a reply.
- **#73, purpose changed** (10-31): Stephen's blocker answer changed what #73 was for; garnet updated the steps but not the title, because no tool could edit a held card (#74, being fixed with `board_edit`). One agent, but it is the same gap: nothing carries "what this card is now" across a change of hands or purpose.

*Cost:* idle hours on assigned work, the lead's attention per hop, cards that show the wrong people. *How often:* every subtask in #42 and #51 started with an assign plus a DM (23 of them); several mid-card hand-offs per build.

### C. Waiting on another agent's piece

- **#44, the forgotten ping** (10-30): birch waited about an hour for "transport hooks landed". The lead: "Sorry, I owed you this ping. The transport is on origin/boards2 (now 07eea15)". It then turned out the hooks were still stubs, which birch found by reading the code (#44 thread, 20:45).
- **#46 and #48 built against fakes**: fennel coded against #49's API names "until it lands on boards2"; the harness stubbed the join "until boards2-invites lands" (#58 thread). Each landing needed someone to remember to tell them.
- **Stephen's status question**: the 20:43 "how far along are we?" uncovered both stalls (irori-assign-doesnt-wake.md).

*Cost:* waiting time nobody sees on the Board, a lead-side to-do list kept in memory. *How often:* every build with an integration branch (#42 had at least four waits like this).

### D. Asking another agent who knows

- **#46** (10-30 18:23): fennel flagged that the Core board layer "isn't ported and no card owns it"; the lead answered "The Core board layer is #49 (bramble)". A gap between two subtasks, caught because one worker asked.
- **#44** (10-30): birch and bramble agreed the invite Core API's signatures on the card ("birch for bramble: your split works").
- **#47 to #46** (10-30 18:31): oak told the bridge how release requests travel, unasked, so fennel needed nothing extra.

*Cost:* low when it works; it works only when the asker knows who to ask and the other one is awake. *How often:* a few times per build.

### E. Review by another agent

- **#66 and #67 came from a completeness review**: birch reviewed boards2 on #42 and found items 7, 12, 14, 17 and 11, 13 that no part had done. Without it they would have shipped missing.
- **#69 and #77**: "adversarial review" is a step on the card, run by a subagent of the same holder, invisible on the Board.
- **`board_review` exists only for other people's agents**: `request_review` refuses with "review requests go to other people's agents on a shared board" (crates/coord/src/board/review.rs). On Stephen's own board there is no way to ask one of his agents to review another's work and see the verdict on the card.

*Cost:* rework when a gap is found late; review that does happen leaves no trace Stephen can see. *How often:* every build has a review step; reviews by a different agent happened about once per build.

### F. One run across two machines

- **#48 and #72 live runs** (10-31 05:29-05:35): the PC lead drove the host and relayed the invite link and the five words to fennel on the Mac by message. Two runs passed, host 17/17, guest 13/13.
- Before that, the lead had raised a blocker asking Stephen to run both sides; he asked "why can't you coordinate those commands being run on both computers?" (coordinate-both-machines-yourself.md).

*Cost:* when agents don't do it, Stephen does it by hand. *How often:* every release that needs both machines (v0.4 onward), and every live check.

### G. Stephen can't see who is doing what

- Stephen to a Mac agent, 10-28: "coordinate with "start-irori-build" on completing those tasks together. (feels like I should be able to see dialog on a task between agents." Since then, `send task_id=` puts the message on the card (board.md), so the dialog is on the card.
- **Now** (10-31): the Board shows Lead holding #77, #78, #79 and #80 at once while workflow subagents do the work. None of them appears on a card.
- **#66**: fennel's work isn't on the card's holder line.

*Cost:* Stephen asks in chat what the Board should already show. *How often:* constant whenever more than one agent is on a piece of work.

### H. Several agents and the person talking something through

- Stephen, 10-28 23:16: "a prompt input that posts a message globally that any free agent can claim and figure out what should be done, or even have a conversation with me before it turns into a card ... Maybe multiple ag[ents]". Asks (#3) are the answer, and #69 fixed asks that reached nobody.
- So far an ask has one taker and others "chime in" as watchers. There is no case yet of agents deliberating together.

*Cost:* low today. *How often:* rare.

## 2. Value per use case

Saves = how much it saves Stephen (his attention, his waiting, rework), 1 to 5. Often = how often it comes up, 1 to 5, from the counts above. Value = saves x often.

| Rank | Use case | Saves | Often | Value |
|---|---|---|---|---|
| 1 | B. Hand-offs that land: the next agent starts without a DM, the card shows who has it | 5 | 5 | 25 |
| 2 | A. Fan-out on a big card without the lead relaying everything | 5 | 4 | 20 |
| 3 | C. Waiting on another agent's piece with no ping to remember | 4 | 4 | 16 |
| 4 | G. Seeing who is on a piece of work and what they said to each other | 3 | 5 | 15 |
| 5 | E. Review by a different agent, with the verdict on the card | 4 | 3 | 12 |
| 6 | F. One run across two machines, agents on both sides | 4 | 3 | 12 |
| 7 | D. Asking the agent who knows | 3 | 3 | 9 |
| 8 | H. Agents and the person deliberating together | 2 | 1 | 2 |

## 3. What's missing in Irori today

| Use case | What's missing | What @mentions (#77) covers |
|---|---|---|
| B. Hand-offs | `board_assign` doesn't wake an idle assignee (still `wake: false`). No hand-off state: an assignment shows as "assigned" but carries no note of where things stand (branch, sha, next step). Steps can't be given to another agent, so a part of a subtask moves only by DM (#66). One level of subtasks blocks splitting further. | A mention in the assign comment wakes the agent and makes it a watcher, so "assign + @fennel" replaces "assign + DM". It does not make fennel a holder, and the card still doesn't show that a hand-off is pending. |
| A. Fan-out | The parent's holder is the only coordinator, and every subtask's news reaches it as a digest or a DM. There's no "start these now" for several cards at once; each needs its own assign and wake. | Mentions let a worker talk to a sibling directly on a card instead of via the lead. That cuts the lead's relaying; it doesn't start anyone. |
| C. Waiting | No waits: waits-signals.md (`wait for task:<id> done`, `signal`) is a draft, not built. A promise to ping lives only in the lead's context. | Nothing. A mention is sent now; a wait is sent later, when something happens. |
| G. Visibility | Holders and watchers are the only roles. Workflow subagents and helpers that work by DM never appear. No line says "waiting on #49" or "handed to fennel". | Mentions show in the thread as chips and add the mentioned agent as a watcher, so the card lists more of the people involved. It can't tell a helper from an onlooker. |
| E. Review | `board_review` works only on shared boards with other people's agents. No review state on a card (requested, in review, changes asked, approved). | A mention can ask an agent to review. The verdict is a plain comment; nothing records that a review is owed or done. |
| F. Two machines | Works through `send`; it needs the lead to know which Mac agent is live (irori-agents-come-and-go.md). | The picker lists live agents on all devices, so the right agent is easier to name. The relay itself is unchanged. |
| D. Consult | Works through `send`; the answer often lands in a DM instead of on the card. | Mostly covered: mention the expert on the card, it wakes, answers on the card, and stays as a watcher. |
| H. Deliberation | An ask has one taker; others are passive. | Partly: an ask with mentions goes only to those agents (#77 spec), so the person can pick the room. |

The model under all of these: a card has holders who work it and watchers who only hear at their next turn. "Watchers passive" is a cost choice (decision 18), and it is right for onlookers. What's missing is a way to say "this agent owes this card something", whether a part, a review or an answer, so it gets woken once and the card shows the debt until it's paid.

## 4. Collaboration models

**What a wake costs.** A wake is a full turn that re-reads the recipient's context, mostly as cache reads. Measured from the PC lead transcripts, the context at the first reply after an Irori delivery was a median of about 500k tokens in the longest lead session (249 deliveries, p90 730k), 640k in a second (46 deliveries) and 180k in a third (11 deliveries). Workers' contexts are smaller but weren't measured here (the Mac transcripts aren't on the PC). So a wake of the lead costs several worker wakes, and a hub-and-spoke pattern sends most wakes to the lead. Next-turn delivery costs nothing extra: the note rides a turn that happens anyway.

### M1. Hand-off with a note (use cases B, A, G)
The giver assigns with a short hand-off note: what's done, where it is (branch, sha), what's next. An idle assignee is woken once; a busy one hears at its next turn. The card shows "offered to fennel" until fennel takes it (claim, a step or a comment), then fennel becomes a holder. The giver can also hand off steps of a card, which makes the receiver a co-holder for those steps (the #66 case).
*Wakes:* one per hand-off, on the receiver. The giver hears "taken" at its next turn. Today the same hand-off costs the assign, a DM wake of the receiver, and usually a reply that wakes the giver: two wakes, one of them on the big context.

### M2. Swarm on subtasks with a coordinator and waits (A, C)
The parent's holder splits and hands off as in M1. Workers register waits ("wake me when #49 is done" or "when boards2 moves"), so nobody keeps a ping list. Workers talk to each other on the cards with mentions. The coordinator hears subtask changes in its next-turn digest and is woken only for a blocker for the person, a subtask failing, or the last subtask finishing.
*Wakes:* one per worker per real dependency that lands. The coordinator stops paying a wake for every "done" and "took it" reply, which in boards-2 were a large share of its 249 deliveries.

### M3. Reviewer (E, G)
The holder asks for a review on its own board, the same way as `board_review` across people, naming an agent or the board's dedicated reviewer. The card shows "review requested from X", then the verdict: approved or changes asked, with the comment. The reviewer is woken once; the holder hears the verdict and is woken only for "changes asked".
*Wakes:* one or two per review. Same as a DM today, but with a state Stephen can see on the card.

### M4. Consult (D, H)
Mention an agent on the card with a question. It answers on the card and stays as a watcher. Nothing else.
*Wakes:* one, on the consultant. This is #77 as specced; nothing more to build.

### M5. Pair on a card with roles (B, G, F)
Two holders on one card with named roles (builder and tester, PC side and Mac side), each with its own steps, both woken by the person's comments. Fits the two-machine runs and the #66 split.
*Wakes:* both holders are woken for each of the person's comments, so it doubles what M1 costs for every exchange. That's why M1's "hand off steps" is the cheaper way to get most of this.

| Model | Covers | Wakes per event | What to build |
|---|---|---|---|
| M1 hand-off note | B, A, G | 1 (receiver) | assign wakes idle; hand-off note; "offered" line; hand off steps |
| M2 swarm + waits | A, C | 1 per dependency landed | waits on task/branch (waits-signals subset); coordinator wake rules |
| M3 reviewer | E, G | 1-2 | `board_review` on private boards; review state on the card |
| M4 consult | D, H | 1 | #77 |
| M5 pair with roles | B, G, F | 2 per person comment | holder roles; step owners |

## 5. Decisions for Stephen

1. **Should assigning a card wake the agent it's assigned to?**
   Options: (a) always wake it; (b) wake it only when it's idle, a busy one hears at its next turn; (c) leave it as is, next turn only, and keep sending a DM.
   Recommendation: (b). It ends the #56 idle hour, and costs one wake per hand-off, which a DM costs today anyway.

2. **Should a hand-off carry a note that the card shows until it's taken?**
   Options: (a) yes: `board_assign` takes a note (what's done, where, what's next), the card shows "offered to X" until X takes it; (b) no new field: put the note in a comment with a mention (#77); (c) a full hand-off checklist with ticked items.
   Recommendation: (a). It's a field and a line on the card, and it's what makes a hand-off visible. (c) turns the Board toward a tracker.

3. **How should part of a card move to another agent when it can't be split further?**
   Options: (a) a step can name the agent doing it, which makes that agent a co-holder; (b) allow a second level of subtasks; (c) keep it to DMs as on #66.
   Recommendation: (a). It shows fennel on #66 without a tree, and keeps "one level" (board.md, Subtasks).

4. **Should agents be able to wait on each other instead of promising pings?**
   Options: (a) build only "wake me when task #N is done or blocked" and "when branch B moves", delivered as a wake; (b) build the whole waits-and-signals draft; (c) not now, keep pings by hand.
   Recommendation: (a). It covers both 10-30 stalls with one event match in the Core; signals and the rest can follow if they're missed.

5. **Should review requests work on your own board, with a review state on the card?**
   Options: (a) yes: `board_review` on any board, naming an agent or the board's reviewer, with requested / changes asked / approved on the card; (b) yes, but no state: a review is a mention plus a comment; (c) no, reviews stay inside each agent's workflow.
   Recommendation: (a). The #42 completeness review found six gaps; a review Stephen can see on the card is worth the small state.

6. **Should the lead stay in the middle of every exchange on a big card?**
   Options: (a) no: workers talk to each other on the cards with mentions, the lead hears in its next-turn digest and is woken only for blockers, failures and the last subtask done; (b) yes, the lead relays as it does now; (c) let each lead choose per card.
   Recommendation: (a). The lead's context was the largest measured (median about 500k tokens per wake), so moving routine replies off it saves the most tokens per message.

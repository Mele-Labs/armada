# Sessions take what Jobs did well, and Jobs go

**Decided 2026-10-15.** His words: *"I have been using session 100% of the time
with great success."* What he keeps from Jobs is the plan, tasks and scope (he
can see what will be worked on and how far along it is), workflows (a structured
order that keeps an agent on the plan), judges (they sometimes caught drift, and
were usually wrong), and the proposer (a quick reading of what kind of work it
is).

What failed in Jobs: they never talked to him while planning, and a Session's
front-loaded questions, spikes and walks are why Sessions work. Checks at every
step bogged his machine down. Drones with restricted tools were slow and could
not finish. The Job page did not carry over.

| Asked | He chose | The cost he took |
|---|---|---|
| How a Session records its plan | A new Armada tool the agent calls when planning ends (scope, task groups, a subagent per group); he approves the plan in the view, and Fleet still ticks progress from what it observes | Reverses *A Session builds a ring* (14 Oct): "no new agent tool". The plan is an agent's statement; observed work still decides progress |
| How firmly a workflow holds a Session | A soft rail: the steps are in the agent's prompt and in the view, Fleet detects the step from what it observes and nudges when one is skipped, nothing blocks | A Session can still wander; the nudge is all that stops it |
| Where a judge's concern goes | A card to him first, with Send to session and Dismiss | He reads every concern; nothing reaches the agent without his press |
| What happens to Autopilot, Commander, Epics, Triggers and the Job Board | Decided one by one when Jobs are deleted | |

**Also settled in the request.** No per-step Checks: the agent runs `armada
check` when it needs to. Subagents have full tools. A Session looks the same on
the Cockpit and on the Sessions page, in the Cockpit's layout of panels around
the thread and composer. The plan replaces the status ring as the measure of
progress. The proposer runs on Session dispatch and fills in the title, the
workflow (none where nothing fits, which is a custom Session as today), the
model by difficulty, and the effort.

## Walk notes, 15 and 16 Oct 2026

**Approved 16 Oct 2026** on the walk `?walk=oneSessionView`, branch
`mock/one-session-view`, which is the reference for the build.

| He said | Settled |
|---|---|
| *"I really dont see a purpose in having a separate sessions page. I think its too much to maintain. We have everything in the cockpit."* | The Sessions page goes. A Session is viewed in the Cockpit only; what the page alone offers moves to the Cockpit first |
| The Cockpit's panels: keep Incoming calls, Flight control, Merge line, Status, the Ledger and the Jobs and Sessions list; Activity, Bays and Autopilot may give way | A selected Session fills the middle with its thread and composer, and its own panels take the three places that give way |
| *"I should be able to collapse any of these panels by clicking their headers"* | Every panel folds to its band. Incoming calls is folded with no calls, opens when one arrives, and lifts over the Ledger when focused |
| The proposal card duplicated the composer's model and effort | No proposal card. The title is the Session's name, model and effort preset the composer's own controls, the workflow is picked above the steps in Status |
| A Start press in Status could be missed | No Start. A Session starts as soon as the proposal lands; the workflow, model and effort stay changeable, and a change applies from the next turn |
| Plan approval in the Plan panel could be missed | Approval is a card in the thread, like a question; the Plan panel carries the plan after it |
| *"These are all tools built on top of agents so we need to ensure that changes being made are notifying agents so they can readjust"* | Every change he makes through a panel reaches the agent. A workflow or plan change is sent at once (at its next tool call mid-turn, waking it when idle). A model or effort change applies from the next turn, which is told of it. A dismissed concern tells it nothing. Each shows as an event row in the thread |
| *"I should be able to quickly and easily suggest changes to the plan. Suggestions and edits should go back to the agent so they can cross examine them"* | Edits and notes on a group or task collect as marks and go to the agent as one message with Send changes. The agent answers, may push back, and records a revised plan that shows what changed. Nothing he marks changes the plan without the agent |
| *"Anything that needs me for this specific session should appear in the thread. That is where most of my focus is going to be."* | A selected Session's calls (judge concerns, plan approval, questions, permission asks, waiting items) are cards in its thread, and are left out of Incoming calls while it is selected. Incoming calls carries the other Sessions'. This replaces the 15 Oct answer "a card to him first" only in where the card sits |
| *"A user should be able to disable the workflow system and always just dispatch plain sessions via settings."* | A setting turns workflows off: the proposer still names the Session and picks its model and effort, but never a workflow; no steps, nudges or step judges; the Judge tile still reads against the plan |
| A horizontal strip cannot hold a long workflow | Steps are drawn vertically |
| The Field should be the repository's floorplan | A treemap of the directories with files as bars; drones fly to the file they touch |

**Why the ring had no pieces.** A Bridge-hosted Session was started without
`CLAUDE_CODE_ENABLE_TODO_TOOLS`, so its agent had no task tools (#2385).

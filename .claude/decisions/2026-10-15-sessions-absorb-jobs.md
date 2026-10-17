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
| *"Lets disable any automatic judge checks for now. I think how a judge checks the work needs some work and most claims have been useless or wayyyyyy too technical"* | No judge runs at a step boundary. The Judge tile still reads on his press. How a judge reads the work is reworked before any runs on its own again |
| On a 14" screen the thread had about 150px. Three layout takes (Flight control and the Field moved or folded) he turned down: *"When I scroll the session thread, the question/plan or whatever is pinned above the composer should seamlessly animate into a collapsed state"* | The calls pinned above the composer shrink with the thread's scroll to one header band, grow back at its end, open over the whole thread on a press, and close back to where he was. The panels stay where they are. Approved 16 Oct on `?walk=pinnedCallsScroll` |
| *"The formatting of a judges claims is awful. Its just a wall of text"* | A concern is a claim: the consequence as its headline, then Expected and Found, two lines each with More; mono only for paths. The judge is asked for one short sentence each. Approved 16 Oct on `?walk=judgeConcernCard` |
| The Sessions page's search and views, as one line of glyphs: *"I dont like this … Change the header to read 'Work'"*, then his drawing of a Filter menu in the band and rows as tiles | The list is Work, with Filter in its band: a search with a clear button inside it, and the scopes Active, Quiet, Ended and All, ruled apart. `/` opens it. The Sessions and Jobs tabs leave the title bar. Approved 16 Oct on `?walk=cockpitSessionsList` |
| *"The field panel everytime i jump in a session should have the entire field in view"*; swap the Ledger and Status; *"the merge line should highlight the PRs that are related to that session"* | The Session's Field has a camera of its own, fitted whole on each open. The Ledger stands above Status. The Merge line rings the open Session's pull requests. Approved 16 Oct on `?walk=sessionViewFeedback` |
| Epics go with Jobs: *"Maybe we split into linked sessions as long as the main one can keep track of them"*; then *"I liked the idea of it being in the ledger as well"* and *"This is a lot of info on one line"* | A Session hands a plan group to a linked Session. Its row stands under that group on the Plan and in a Linked section of the Ledger, two lines each. Every Session started from another nests under it in Work. The parent's agent is told when a child needs him, ends or merges. Approved 16 Oct on `?walk=linkedSessions` |
| *"It would be sick if you could dispatch a session that waits for @Cockpit Map to complete"*; later, the waited-on Session should be told what the waiting one needs and choose to start it on its pull request's branch or when it lands | A Session can wait on another and starts by itself when its condition is met: the other ends (the default), a plan group or task of it is done, its pull request is pushed (starting from that branch) or lands. The waited-on Session is told what is needed and can change the condition and base |
| The After form: *"Chained forms like this are so boring. We live in an era of LLMs"* | No form. He says it in the brief (`after @Cockpit Map lands its PR, …`); `@` finds the Session, and the proposer's reading is one sentence chip whose parts Tab and the arrows change. What the waited-on Session is told is the need read from his words. `⌥A` opens Dispatch with `@` typed. Approved 16 Oct on `?walk=sessionAfterAnother` |
| A horizontal strip cannot hold a long workflow | Steps are drawn vertically |
| The Field should be the repository's floorplan | A treemap of the directories with files as bars; drones fly to the file they touch |

**Why the ring had no pieces.** A Bridge-hosted Session was started without
`CLAUDE_CODE_ENABLE_TODO_TOOLS`, so its agent had no task tools (#2385).

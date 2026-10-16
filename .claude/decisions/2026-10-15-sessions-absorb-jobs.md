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

**Why the ring had no pieces.** A Bridge-hosted Session was started without
`CLAUDE_CODE_ENABLE_TODO_TOOLS`, so its agent had no task tools (#2385).

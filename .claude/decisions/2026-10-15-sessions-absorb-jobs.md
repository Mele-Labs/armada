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
| Short of room again: Progress in place of Status and the Plan, no Ledger, more Flight control keys, and an Artifacts view for *"everything that was created or used during a session"* | The Session view is compact. The Field and Progress share the right column; the Ledger and Incoming calls are not drawn while a Session is open. Flight control takes `1` to `9` and `0`, and its last four open layers over the thread: Artifacts (`8`), Subagents (`9`), Checks (`0`) and Slot, lit only when the slot is stranded, 20 commits behind or in a write clash. Back returns to the thread where it was. This replaces *"the Ledger stands above Status"* and the pinned calls below. Approved 16 Oct on `?walk=compactSessionView` |
| A rail of calls beside the thread: *"nevermind I dont like this. Lets just make the rows in the work panel pulse or something when they are calling me"* | The call rail is turned down. A Session calling him pulses a 2px bar on its row in Work, by opacity alone, and holds still under reduced motion; the row's hover card names the call. The open Session's own row does not pulse |
| The step names in Progress took a line each | The step label is a hover card: the workflow is a line of glyphs and each names itself and its state (`Build · current`) |
| *"too much padding around this panel and the contents in here. i think we can cut it in half"* | The Session's card, its message column and its box are half as padded, in the Session view's sheet and not in the components |
| The pinned, shrinking question panes: *"I am really not liking these new scrolling question panes"*, *"We gotta find a better way to do this"*. Three takes: the composer answers, the thread's last card, a stack in the right column | Take B. What waits on him is the thread's last card, answered where it stands, with the Needs you band in it, and a chip over the composer jumps back to it when the thread is scrolled away. Nothing is pinned or shrinks. The 16 Oct `pinnedCallsScroll` approval is withdrawn. Approved 16 Oct on `?walk=compactCallsInThread` |
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

## Rulings made while deleting Jobs, 17 Oct 2026

Made in the pull requests that deleted the Job code (#2495, #2497, #2498,
#2499, #2500, #2502), where the plan above did not say. The pull requests
chose these. The write-back row was answered by the owner the same day, in the
table below this one.

| Asked | Ruled | The cost |
|---|---|---|
| Autopilot, when a pick's Session replies or goes idle (#2497) | The Session gives its place back and stays open. A Session counts against the cap while it works or has not run yet, and does not while it waits on the person. The spend and Sessions-started caps count Sessions | A Session that replied once no longer holds Autopilot's place, so Autopilot can start another beside it |
| Autopilot, which issues are "already taken" (#2497) | Every pick's issue, from any run, whose Session is still live | A Session that ended frees its issue to be picked again |
| Autopilot, what Stop does (#2497) | It closes a picked Session whose agent never ran, and leaves the others | A Session already working keeps running after Stop |
| Autopilot, issue write-back (#91) | Lost when Jobs went: it was keyed on Jobs, so a landed pick no longer wrote back to the tracker. Answered below | An issue Autopilot finished is not marked on the tracker |
| Pocket, which acts stay (#2498) | Needs you, Running and Done list Sessions from the routes Bridge reads. Dispatch starts a Session, and the phone answers a Session's ask. A stuck Session pushes once | The seven Job acts are lost on the phone: approve, redirect, restart a step, kill, redispatch, approve a review and request changes. So are the Job and Review screens |
| A bay held by an old Job (#2500) | Drawn without a holder | The bay does not say who has it |
| The Job tables | Left in place and unread until one later migration drops them (#2495, #2499, #2502). Wire types stay until #30 | A store carries tables nothing reads until then |
| The acceptance claims of the deleted machinery (#2502) | `auto_release`, `board`, `drone_per_step` and `recovery` are retired. `bug_job`, `drone_per_task`, `landing`, `plan`, `trigger` and `trust` are trimmed to what survives | What a retired claim proved is no longer proved by anything |
| Rehearsals (#2502) | `start_run`, `stop_run` and `undo_run` went with Jobs. They ran a Check or Command in a Job's worktree | There is no rehearsal in a Job's worktree. A Session runs `armada check` itself |
| The Epic workflow (#2499) | Inert once Job dispatch went, and removed with #30 | Epics are Sessions linked to a parent, as decided above |
| The Helm session retention setting (#2499) | `retention.helmSessionDays` went with the Commander. After the Commander left, the only conversation it swept was the Commander's | None |
| What else left with Job dispatch (#2502) | `dispatch_studio_draft`, the retro `agree_lesson` op and the seed warm-up. Copy-on-write cloning stays. Triggers stay as saved definitions, with no firing, holds, repair or added steps | A retro item starts a Session with Start Session |

## Fleet does not comment for him, 17 Oct 2026

His words: *"Fleet should never comment on issues or PRs on my behalf."*

| Asked | He ruled | The cost he took |
|---|---|---|
| Whether issue write-back (#91) is rebuilt for Sessions | No. It is deleted. Fleet never comments on, labels or edits an issue or a pull request unless he presses for it. A review or comment he approves, or types and sends from Bridge, is his own act | An issue Autopilot picked and finished stays as it was on the tracker until he marks it |

## Needs are deleted, 17 Oct 2026

His words: *"I feel like we dont need to keep the need. Armada now directly tells agents when they are touching the same file"*. That is write-clash detection (#2197), which stays.

| Asked | He ruled | The cost he took |
|---|---|---|
| Whether `armada need`, the ledger order behind it and the `needs` status on a pull request are kept beside write-clash detection | No. They are deleted end to end: the CLI, `act_on_need` and `list_needs`, the mod's reporting from a Bash command, the status, and the capability page | Two agents that both mean to take one number are no longer ordered. Fleet tells both when their changes would conflict, and neither is held |

The `ledger_attachments` rows of kind `need` stay in the store, unread, until the one later migration that drops the Job tables. Fleet's single re-run of a hung CI job and the Helm `rerun_failed_checks` act were never needs and stay. `docs/practices/protocol.md`, *Needs are deleted*, has the wire.

## Sessions are called Jobs, 17 Oct 2026

His words: *"I would love to actually rename Sessions to Jobs everywhere."*

| Asked | He ruled | The cost he took |
|---|---|---|
| Whether the concept that absorbed Jobs keeps its name | It is called a Job. The old Job is deleted, so the word is free | Every page written before 17 Oct that says Session is now old vocabulary; decision records and spikes keep it as history |

What changed is the word people and agents read: Bridge's text, Fleet's notices and refusals, the tool and parameter descriptions an agent sees, the prompts it is given, the `armada` mod and the docs. What did not: code identifiers, wire field names, store columns, CSS classes, file names, route paths and error codes keep `session`, and `session_id` is still the parameter an agent passes. The four tools `start_another_session`, `fork_this_session`, `list_sessions` and `move_session` are listed as `start_another_job`, `fork_this_job`, `list_jobs` and `move_job`; the old names are still answered, so a running agent and a mod that name them keep working.

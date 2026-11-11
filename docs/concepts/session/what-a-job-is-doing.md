# What a Job is doing

**Part of:** [Job](../session.md).

**Nothing the person sees depends on the agent making a tool call.** Fleet reads what a Job is doing from what it observed, and `waiting_for` and `all_done` only add detail to it.
`SessionRecord.standing` says it, for a live Job; an ended one has none.

| `standing` | Read from | Item in `waiting_for` |
|---|---|---|
| `working` | a turn is running and something was heard from it within the bound, or, between turns, a `subagent` row of it stands: a parent that ended its turn to wait on its subagents read as idle while they ran (14 Oct 2026). So does a `background` row or a running Check of its own, where it would otherwise rest: he asked to see what each Job is doing, and an ask in its reply is still an item | none |
| `stuck` | a turn is running, nothing has been heard from it for **ten minutes**, and it is not held on a question or a permission | none |
| `done` | `done_since` stands (the agent said nothing is left) | `done:<done_since>`, text `Done` |
| `replied` | the last turn ended on a message to the person that only reports, and he has not written since | none: the unread mark |
| `stopped` | it stopped again after Fleet told it of a stop, or the word reached nobody. `SessionRecord.stopped` says why and what Fleet is doing about it | none: the mark and the call reach him |
| `idle` | none of the above, which includes a reply that asks (it is an item) | none |

A reply whose close asks him something is an item whatever the standing: `replied:<turn ended at>`, source `replied`, text the sentence that asks.
Each ask is its own item ([A reply that asks](waiting-on-the-person.md)).

**The reply is read off the turn.** For a hosted Job the turn is on its stream: the last thing the agent said before the turn ended.
For a terminal Job the mod reports `turn_started` when a prompt goes in and `turn_completed` with `said`, the close of the last assistant message (its last 1,500 characters, from a line start), when the turn ends; the agent is never asked to.
Fleet keeps the turn's end and the end of what was said (the last 1,500 characters, from a line start) in `session_turn_end` (one row a Job), so a reply survives a restart.
A terminal Job's mod sends that same close, so an ask is read from a terminal reply as from a hosted one; a mod older than 0.3.39 sent only the first line, and its asks are not read.
The item's text is the sentence that asks.
It stops standing when the person writes to the Job, when another turn begins by any route and when the Job ends.
A turn that ended on tools alone, with nothing said, leaves no reply, and a Job whose turn ended before Fleet observed this has none: nothing is read back from old turns.
A `done` outranks a reply.

**A reply that only tells is no item; a reply that asks is one.** The owner, 14 Oct 2026: "When a session replies to me, it shouldn't show up in needs me.
Needs me is when it needs something from me.
Replied should be idle or unread." And 17 Oct: "Part of me wonders if we could use a cheap rust natural language processor to detect if the latest message from an agent in a thread is something that needs me.
Then we can classify replies that dont need me as unread and give them more weight.
Then replies that need me to respond can show as something that needs me." A reply that only reports is `replied`, an idle Job with an unread mark, which Bridge draws and clears once he has been in the Job (`cockpit/viewed.ts`).
Fleet keeps the same visits (`view_session`, read onto `viewed_at`), and the row mark it serves reads `replied` only until he has been in the Job since its turn ended (`docs/practices/protocol/a-jobs-row-mark.md`).

**Whether the close asks is read by rules and by Fleet's local language model**, and the close asks if either says so (`session_host/asks.rs`, `session_host/asks_model.rs`).
The owner chose that on 20 Oct 2026: on 300 fresh replies it found 54 of 81 asks where the rules alone found 21, for 20 wrong calls where they made 8 (`docs/spikes/reply-asks-classifier.md`).
Until the model is loaded, the rules read alone.
The reading leans toward yes: a wrong call costs a dismiss, and a missed ask leaves a Job waiting unseen.
The item is not raised where the agent already asked through a tool (a turn held on a card or permission, or an agent `waiting_for` item, question card, permission or window question open), where a `done` stands, or while a subagent works.
A dismissed id never returns, and dismissing it sends the agent nothing; answering it with text is a message to the Job.
A `done` is an item too: it asks whether to end the Job.

**A reply that turns a Fleet tell down is read by the model too, and is built but not in force.** Fleet keeps the last tell it sent a Job of three kinds (a stranded slot, the Context rule, the Subagents rule) in `session_tells`, and marks the first reply kept after it.
Only that reply is read, with the `refused` head (`session_host/tell_refusal.rs`); on a refusal it raises `refused:<tell sent at>`, source `replied`, text the closing sentence the head scores highest.
It is not raised where the reply asks (that is the `replied` item), where the agent asked through a tool, where a `done` stands or while a subagent works, and a later reply that does not refuse takes it away.
One tell is one id, so a dismissed one stays dismissed.
With the model not loaded nothing is raised, since no rule stands in for it.
`tell_refusal::IN_FORCE` is false: `docs/spikes/tell-refusal-reader.md` shows the head caught none of the five refusals of these tells out of fold.

**Every stop answers for itself** (the owner, 3 Nov 2026: "Armada is about me not babysitting jobs").
A stop is named by when its turn ended (`last_turn_at`), so a turn that ended on tools alone counts, and whether it stands is read where the row is built (`session_host/stopping.rs`).
A stop is excused by facts, never by the agent saying so: a turn, subagent, background command or Check running, a live watch, its own open pull request (except one Fleet told it failed or conflicts, with nothing pushed since), a lifecycle holding it other than a ticket, a Job it started that is still running, any item in `waiting_for`, a question that lapsed (it is no item but still waits on him), a shown walk, a plan, draft, sketch or page awaiting him, a `done`, a pause, its start card, a limit, a wait on another Job, or a terminal that is not listening.
**Never while anything waits on him** (the owner, 5 Nov 2026: "if there is a needs me item, thats not silence. Thats waiting on me to act").
A minute after a stop nothing excuses (`STOP_AFTER`, unmeasured), Fleet sends one word, `stop_nudge`: to carry on, to name what it waits on with `waiting_for`, or to finish.
A Job stopped on a step only it can end hears that step's own words instead ([The rail](the-rail.md)).
**One nudge per stop.** The stop after it reads `stopped`, so a Job can never loop with Fleet.
A word the person writes, or another Fleet word that wakes it, gives the next stop a nudge of its own; a word that arrives mid-turn does not.
A nudge that reached nobody (muted, withheld or refused) reads `stopped` at once, and one that started no turn does a minute on.
His own Stop press passes over the stop it leaves, as the migration did for every live Job's stop when this landed.
Some stops are fine and get nudged too, which is what reading a week of them after landing is for.
Kept in `stop_nudges` (one row a Job).

**A stopped Job asks him to press nothing** (the owner, 3 Nov 2026: "If the app detects a stopped job, it should restart it. If it fails to restart it should dispatch a repair job unless fleet is down and the app is in a unhealthy state").
It raises no item.
A process that dies in a turn is started again once ([What a restart does to a job](a-job-fleet-hosts.md)).
One that dies again, with `session_open_turn` showing the restart spent, ends no turn, so no turn's end says it: the ledger reads it `stopped`, `process_gone`, or `fleet_down` where its open turn overlapped a stretch in `fleet_outages`.
Fleet then starts a hidden repair beside it (`repairs_started`, `repair_started`).
A repair is an agent process Fleet hosts, with no session row, no place in the Jobs list and no start card.
It runs one turn in the Job's own checkout, or in a slot of its own where the Job holds none, with the edit and run tools a hosted Job has, and ends by itself.
Fleet starts one dead Job's repair at a time, and never two for one Job.
If a probe of `get_health` fails, a new agent would fail the same way, so Fleet starts none and the row says so.
Fleet's note in the thread says what broke when it starts, such as `Repair: its process died twice`, and the Now panel reads `Repairing: its process died twice` until a `Repaired: ` or `Not repaired: ` note follows.
The Job's card says it too, on any state: `Stuck · Repairing: a command hung`, or `Stopped: process ended · Repairing: its process died twice`, where the repairing clause stands in for the restart's words (`SessionRecord.repairing`, [the repair under way](../../practices/protocol/a-jobs-repairing-line.md)).
A repair that ended without putting things right reads `Not repaired` beside the stop, and an unhealthy Fleet reads `No repair: Fleet is unhealthy`.
What is kept is in `job_repairs` (id, the Job, the kind of break, when it started and ended, the outcome and the sentence it said), and `stop_nudges.repair_job` names the one the row reads.

**Fleet reads how a repair ended, and the agent does not announce it.**
Its brief ends with one rule: the first line of its final message starts `REPAIRED: ` or `NOT REPAIRED: `, then one sentence.
`REPAIRED` writes `Repaired: <sentence>` in the thread.
For a quiet stop, a hung command, a dead process or a broken checkout it then wakes the Job once, `Fleet: a repair <did what>. Carry on.`
`NOT REPAIRED`, a final message with neither marker, a process that would not finish its turn, or twenty minutes (`REPAIR_LIMIT`, unmeasured; an interrupt, then its process group) write `Not repaired: <sentence or why>` and wake nobody.
The row reads amber with `gave_up` only once Fleet has run out: its repair ended without `REPAIRED` with the Job still dead, or Fleet is unhealthy.
A Job's next turn beginning, or the Job ending, stops a repair still running, and the note says the Job moved on first.
A Fleet that starts finds a repair left open, gives its slot back and ends it as abandoned.
A live agent that stopped again gets a Quiet repair, from the facts that it read `stopped`, nothing waits on him and no repair has run since he last wrote.
Its row reads `Repairing` while that runs, and amber with `gave_up` only once the repair ended without `REPAIRED`, Fleet is unhealthy, or its agent is gone.
A Quiet repair that put things right wakes the Job, and a Job that stops silently again before he writes reads amber, so Fleet and a Job never loop.
**A pull request Fleet told a Job failed is no wait once the Job pushes nothing.**
The notice, its pull request and the commit it was sent at are kept in `pull_notices`, and `pull_heads` keeps the commit Fleet last read the pull request at.
A notice at that same commit means the Job was told and has pushed nothing since, so the pull request no longer excuses the stop or holds the Job open, and the Job is nudged once like any other.
Its next stop draws a RedPull repair, under the same bound as Quiet: one repair of either kind between two things he says.
A push moves the commit, a passing rerun or a cleared conflict takes the notice away, and either puts the excuse back.
A turn beginning clears the repair, so a later death gets its own.

**A repair is suited to what broke.**
Each kind has its own brief, an overridable prompt under Repairs in Settings (`prompts.repairQuiet`, `repairHung`, `repairDied`, `repairCheckout`, `repairRedPull`, `repairStranded`), whose wording is [contract 5i](../../contracts/agent-prompt/5i-the-repair-briefs.md).
The Job's id and title, its branch, its checkout and what Fleet saw go last, after the rules.

| Break | What it does | Triggered |
|---|---|---|
| Quiet: it stopped twice with nothing asked, its agent alive | Reads the Job's transcript and plan, and reports the next step, what it waits on, or that its work has landed | Yes, `repairs_started`, once between two things he says |
| Hung: a turn running with nothing heard for `STUCK_AFTER` | Finds the command that hung in the Job's process tree and stops it | Yes, `standing_checked`, once a turn |
| Died: its process died and the restart is spent | Finds why from its log and what its process last said, and fixes what stopped it | Yes, `repairs_started` |
| Checkout: conflict markers, a half finished merge, missing dependencies or a binary | Finishes the merge, resolves the markers, reinstalls what is missing, on the Job's own branch | Not yet |
| RedPull: a pull request's checks failed or it conflicts, and its Job ended or stopped | On the pull request's branch, fixes the check or merges the base in, and pushes it | Yes, `repairs_started`, for a stopped Job once between two things he says |
| Stranded: a slot whose holder is gone, left with changes | Pushes the leftovers to a branch and scraps nothing | No, its last holder is told and a rescue Scout reads it where none can be (`stranding.rs`) |

`repair_started` is the one call a trigger makes.
It checks health and takes the Job's slot as the checkout.
A repair never runs in the repository's own checkout, which is the main checkout and is never written.
A Job that holds no slot gets one leased for the repair on a branch cut for it, and a pull request whose Job ended gets one leased on the pull request's branch.
The slot is given back on every ending, repaired or not, the twenty minutes, an early stop and a Fleet restart, before the repair leaves the list.
While a repair runs its slot reads as an open holder's, so it is never told and never read by a rescue Scout.
A repair never starts while anything waits on him: a Quiet repair is read off a stop that nothing excuses, and a Hung one off a stuck turn whose row lists no item.
Checkout and Stranded have no trigger: Fleet reads no checkout for conflict markers, and a stranded slot is the Scout's.

**Fleet's own acts stand in Now, Open and Done**, read from what is kept: each word and note of Fleet's in the thread is over, the nudge coming is read off the row, and the repair running is read off Fleet's notes, a start note with no end note after it.
Background model reads write no row, so they are none of these.

**Stuck is its own bound**, `STUCK_AFTER` in `session_host/standing.rs`, ten minutes of nothing heard from a running turn.
It is not `settings.session-quiet-timeout`, which is how long an idle process is kept and says nothing about a turn that went quiet.
A Job goes stuck by time passing, which no event says, so the notice loop's tick looks at every running turn and publishes the Jobs whose stuck state moved, in both directions (`standing_checked`).
A terminal turn's running state is kept in memory only: a Fleet restart forgets it, and the Job reads as idle until its next turn starts.
A mod from before `0.3.20` reports neither `turn_started` nor `said`, so its Job is never working or stuck by this reading and has no reply; a `done` still stands.

**A terminal's permission prompt.** Claude Code in a terminal asks its own permission for a tool call, and nobody may be at the terminal.
The mod forwards what the harness announced: its `PermissionRequest` hook reports `permission_asked` with the tool and its command or path, where no hook beneath decided it, and the tool call ending reports `permission_settled`.
Where Claude Code passes that hook over, the engine's own verdict of `ask` on the call says it instead, and a prompt both say is told once (*Where Claude Code skips the mod's hooks*).
The agent is never asked.
The mod also puts the prompt to Fleet as it puts a question, and races the terminal against the answer: **the call is answered from Bridge**, Allow once or Refuse, as a hosted Job's permission is, or at the terminal, whichever comes first.
An allow from Bridge runs the call again past its own check, and the terminal's dialog closes (`docs/spikes/031-can-bridge-answer-a-terminal-permission-prompt.md`).
In the Job the command is a block to run, and its ▶ is Allow once.
Allow and remember is not offered, because remembering is the harness's own.
Until the ask arrives Fleet raises the call from the fact alone, with no options and the `message` act naming the Job.
While it is open the turn is waiting on the person, never stuck.
A turn starting or ending, and the Job ending, closes it too.
A mod from before `0.3.22` reports neither, so its prompt raises no call and the turn reads stuck after ten minutes.

**Bridge reads the state from `standing` and from nothing else**: not from an event's `live` flag or its age, and not from whether a tool was called.
`Timeline::session_turn` marks no entry live, because a turn that finished is not moving.
An answer sent from a card counts as him being in the Job, so the items he has answered stop being calls (*Waiting on the person*).

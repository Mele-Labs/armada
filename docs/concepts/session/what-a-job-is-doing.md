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
A stop is excused by facts, never by the agent saying so: a turn, subagent, background command or Check running, a live watch, its own open pull request, a lifecycle holding it other than a ticket, a Job it started that is still running, any item in `waiting_for`, a question that lapsed (it is no item but still waits on him), a shown walk, a plan, draft, sketch or page awaiting him, a `done`, a pause, its start card, a limit, a wait on another Job, or a terminal that is not listening.
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
Fleet then starts a repair Job in the same repository, nested under it, with no start card, one at a time and never for a repair Job (`repairs_started`).
If a probe of `get_health` fails, a new Job would fail the same way, so it starts none and the row says so.
The row reads amber with `gave_up` only once Fleet has run out: its repair Job ended or said it is done with the Job still dead, Fleet is unhealthy, or the dead Job is itself a repair Job.
A live agent that stopped again reads amber too: its process is fine, so a repair Job would have nothing to fix.
A turn beginning clears the repair, so a later death gets its own.

**Fleet's own acts stand in Now, Open and Done**, read from what is kept: each word and note of Fleet's in the thread is over, the nudge coming and the repair running are read off the row.
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

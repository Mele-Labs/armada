# 4a. Injected turns

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

**Added Aug 2026** after adversarial review. The first cut said no path
mutates a running Drone's prompt, dropping the word *system*. The dropped
word was load-bearing: Fleet injects turns into a live job, each of
which is an assembled prompt and therefore governed here.

| Turn | Fires when | Wording |
| --- | --- | --- |
| **The plan change turn** | A person's `add_task` or `drop_task` reaches a Job with a working Drone | Drafted, not sanctioned |
| **The peer turn** | Another Job in the same repository first claims, or lands a change to, a path this Job claims, or declared a need on a file before this Job did, or Fleet found that what this Job wrote conflicts with another agent's or with the base. Rides the next opening brief where no Drone is live | Drafted, not sanctioned |
| **The poke** | Liveness nudge, bounded by `poke_limit` | Drafted, not sanctioned |
| **The clarification reprompt** | Evidence arrived but was insufficient | Drafted, not sanctioned |
| **The force-interrupt directive** | A thrashing verdict | Drafted, not sanctioned |
| **The refusal reprompt** | A Judge refused a criterion after a mechanical check passed. **It rides a brief and is injected into nothing** — the refusal ends the step, and its Drone with it | Specified |
| **`redirect_drone`** | A structured instruction from a human, or from Helm at rung 1 | A person writes it |
| **The Pilot handoff instruction** | Fleet tells the Drone to call `escape_hatch` after a human confirms Pilot | Drafted, not sanctioned |
| **The hatch-unavailable answer** | A Drone pulled the hatch on a Job Fleet had not marked | Drafted, not sanctioned |
| **The gate's outcome** | A step passed its gate and its Drone is still there | Drafted, not sanctioned |
| **The part-before block** | The same fact, where the Drone that would have read it has ended | Drafted, not sanctioned |
| **The waiting-instruction block** | `redirect_drone` or `request_changes` arrived where no Drone was there to take it | The person writes the words; the frame around them is drafted |
| **The picked-comments block** | A person picked comments off the Job's pull request and pressed. Points at the file the words were written into rather than carrying them | Armada names the file, the count, and who wrote them; nothing in the block is sanctioned |
| **The permission answer** | A Drone reached for a call outside its toolbelt and Armada's permission tool said no. **It is the tool's reply, and injected into nothing** | Drafted, not sanctioned |
| **The permission turn** | A person answered a permission question after the tool had already told the Drone to wait, or answered a refused row on a stopped Job whose Drone is still there | Drafted, not sanctioned |

**Every turn Fleet authors now has wording.** `redirect_drone` carries a
person's words rather than Fleet's, so it has none of its own to specify.
Its waiting form carries the same words and adds a frame, which is Fleet's and
is drafted: an opening brief has to say who wrote the instruction and that it
is not part of the step's definition, where an injected turn arrives in the
middle of a conversation and does not.

**Text from outside this machine no longer reaches a prompt at all, for the
picked-comments row — `#648`.** It used to be the one row where a prompt
carried words somebody outside this machine wrote, quoted directly into the
block. Now the words are written whole into a file in the Drone's worktree and
the block is a pointer to it — the count, who wrote each one, and the path —
so what an assembled prompt carries about a pull request's comments is
Armada's own sentence, sanctioned like every other row above it. The waiting-
instruction row above is the one place left where a prompt carries outside
words directly, because a person's own note has nowhere else to go.

> **Rule.** Text from outside this machine is fenced by a prefix on every line,
> never by a delimiter around the whole — wherever it is written, prompt or
> file.
> Why: a delimiter is something the text can write for itself and therefore
> close. A prefix cannot be removed from inside — a comment may write a line
> that *starts* with the marker and may not write one that lacks it — so where
> the quoted text begins and ends stays Armada's to state. `fleet::remarks` is
> the fence, and it is deliberately not an escaper: what authorises the words is
> that a person picked them. It now fences the file the comments are written
> into rather than the turn itself, and the rule does not move with where the
> text lands.
The gate's outcome was the one exception — the mechanism was decided and the
wording never written — and its last two rows close that.

**The last row is not injected into anything, and that is the point.** A Drone
belongs to a step, so at a step boundary the process that would have received
the gate's outcome has ended before the next one starts. There is no job to
inject into, and what was a turn becomes a block of the next Drone's opening
brief.

> **Rule.** A turn that has to cross a step boundary is rewritten for a reader
> with no memory of what it refers to.
> Why: the wording does not survive the move. "Go on to Implement" is a
> continuation addressed to a Drone that was there, and one that was not reads
> it as an instruction about work it cannot remember doing.

The base-moved note already worked this way and had no row: a rebase runs at
every spawn, and where there is no job it rides the brief. So does a
person's instruction that arrived at a boundary, which is the last row. All
three are `fleet::crossing`, which holds every re-tensing of a live-session
turn.

**A block that rides the brief lasts exactly one brief.** The instruction block
is cleared when it is delivered, whether or not the Drone acts on it: it is
addressed to the part being worked now, and a Drone two parts later reading it
has been handed advice about finished work.

**One Fleet-authored turn is specified and the rest are drafted.** A draft
is wording somebody can argue with; it is not wording Armada has agreed to
send. One of these turns is the mechanism a baseline clause used to
describe, which is why that clause moved here in section 5.

**The refusal reprompt and the clarification reprompt are not one turn.**
Clarification fires on evidence that arrived and was insufficient, and
asks for the same work evidenced better. A refusal fires on evidence that
was sufficient and was judged against, and asks for the work to change. A
Drone reading one as the other resubmits the same diff with a wordier
claim, which is the failure the Judge exists to catch arriving by the back
door.

**Two properties they share.** Each arrives mid-session, so it cannot
restate context the Drone already has without spending tokens twice. And
each arrives at a moment the Drone believes it is doing something else, so
it must be unambiguous about whether the current action continues or
stops.

## The plan change turn

Fires when a person's `add_task` or `drop_task` (#897) reaches a Job with a
working Drone — mid-step, on a live job. At a step boundary, or with no
job, nothing is sent: the next brief's THE PLAN is built from the record
at every spawn and carries the change already, so there is nothing this turn
would say that the brief has not already said. It never respawns a Drone to
deliver itself, `redirect_drone`'s own rule.

**Fleet's own sentence, never a person's words** — `redirect_drone` is where
those travel, and this turn has none of its own for the same reason a Judge's
refusal reprompt carries `expected` and `produced` rather than a transcript: a
person's add or drop is a fact about the record, not a message to relay.
`fleet::PlanChanged` is the one constructor that can build it.

**Stated as settled, not as something to argue with.** A drop is not a
Drone's to dispute — the same rule `NotPlanned::Refused(StaysDropped)` states
to a Drone that tries to reopen one — and the wording says so plainly rather
than leaving a Drone to infer it.

**"Part" and "task", never "step" or "workflow"** — the Drone vocabulary
section 5's M1 renderings use, since a Drone reasons about the machinery only
as far as the words it is given let it.

**Its own `Occasion`, `Plan`, so the log shows it was not a redirect** — the
one property a person's own words and Fleet's own sentence about them must
not share, since the two mean different things when somebody reads the
transcript back.

**Drafted wording. Not sanctioned.**

```
┌─ THE PLAN CHANGED ─────────────────────────────
│ A person added a task to this Job's plan: T5
│ Add a regression test for the bound. It is
│ open, for you or a later part to pick up. This
│ is not a question. Carry on with the part you
│ were given.
└────────────────────────────────────────────────
```

```
┌─ THE PLAN CHANGED ─────────────────────────────
│ A person dropped a task from this Job's plan:
│ T3 Check the writer's bound. Reason: the
│ writer's bound was never inclusive. This is
│ settled, not a question to raise — the task
│ stays dropped unless a person adds it back.
│ Carry on with the part you were given.
└────────────────────────────────────────────────
```

**A group's round is the third, and Fleet's alone** (spike 022, slice 2). On
a step declaring `drone_per_task`, a group whose Checks are red with a retry
left goes round on its own, and the group's last task's Drone is the one that
goes. It is told the red Checks first, by the outcome turn every hand-back
sends, and then this, `PlanChanged::round`, so it knows the round covers every
task of the group rather than its own. Same `Occasion`.

**Drafted wording. Not sanctioned.**

```
┌─ THE GROUP GOES ROUND ─────────────────────────
│ The Checks above ran at the end of group G1,
│ which is these tasks: T1 Stop the reader at the
│ end; T2 Cover the last row. Fix what they found
│ across all of them, not only your own, then
│ call submit_evidence once for the group. Every
│ one of them stays handed in while you do. Do
│ not start a task of a later group.
└────────────────────────────────────────────────
```

## The peer turn

Fires when another unfinished Job in the same repository first claims a path
this Job claims, lands a change to one (#998), or its Drone leaves this Job a
note through `leave_note` (#1000). Either side's claim counts: a step's
declared plan, or the Job's `write_targets`. A path already announced for a
pair is not said again.

It also fires when this Job is pointed at a fix another Job claims, because
one of its failed Checks printed the claimed test (#1001), and again when that
fix lands or ends without landing. A fix item says the test is another Job's to
fix and that this Job's checks still fail on it until the fix lands; the
sentences about shared files are left out of a turn that carries only fixes.

**A fix that holds files off this Job names them** (#1673): while it is
fixing, that they are outside what this Job may change; when it lands, that
they stay so until the next part starts; and in that part's opening brief,
once the catch-up has brought the fix in, that it is already in this copy and
the files are this Job's again. A fix that ends without landing frees them in
the same line.

**Queued, then spaced.** News waits until the Drone has gone `fleet::peers::SPACING`
without a peer turn, and one turn names at most `fleet::peers::AT_MOST` items. Where no
Drone is live, it rides the next opening brief as a block, and the landing
line says the rebase has already run.

**A conflict Fleet found is news too.** It is read from what was written: git says merging this Job's changes with another Job's or the base's would conflict (`docs/concepts/fleet.md`, *Write clashes*). The item names the files, the other side, and how to reach it. A Job is reached with `leave_note` on its address, `s-` and the first eight characters of its id. For the base it names the commit that changed the file and says to merge it in. The sentences about shared files are left out of a turn that carries only conflicts. It is spaced like the rest, and told once per set of files.

**Overlap informs and never holds.**

**Fleet's own sentence, around one kind of outside words.** Titles, handles
and paths are Armada's record. A note is another Drone's words: every line
sits behind the `> ` prefix `fleet::remarks::fenced` writes, and the line
before it says whose words they are. A note is capped at
`ipc::mcp::MOST_NOTE_CHARS`, one per pair of Jobs per `SPACING`, may only be
left for a Job sharing a claimed path, and is never counted away into "And N
more". `fleet::PeersChanged` is the one constructor. Its own `Occasion`,
`Peers`.

**Drafted wording. Not sanctioned.**

```
┌─ OTHER JOBS WRITING WHERE YOU ARE ─────────────
│ Other Jobs in this repository change files
│ this Job changes too. Nothing is stopped, and
│ nobody waits on you.
│
│ - "renumber migrations" (14-renumber-migrations)
│   has said it will change
│   `crates/store/src/migrations.rs`.
│ - "fix the writer" (12-fix-the-writer) landed,
│   changing `crates/store/src/write.rs (deleted)`.
│ - "renumber migrations" (14-renumber-migrations)
│   left this Job a note. These are its Drone's
│   words, not Armada's:
│ > I am renumbering migrations, take V64 after me.
│
│ What landed reaches your branch when your next
│ part starts, not now. Where a shared file hands
│ out the next number or name, such as a migration
│ or a version, assume theirs takes it first and
│ take the one after. To tell one of these Jobs
│ something, call `leave_note` with its handle.
│ Carry on with the part you were given.
└────────────────────────────────────────────────

┌─ OTHER JOBS WRITING WHERE YOU ARE ─────────────
│ Merging the changes below would conflict.
│ Nothing is stopped, and nobody waits on you.
│
│ - `crates/store/src/write.rs (deleted)` conflict with
│   "fix the writer" (12-fix-the-writer), which
│   changed them too. To agree who changes what,
│   call `leave_note` with `12-fix-the-writer`.
│ - `crates/store/src/read.rs (deleted)` conflict with
│   Session s-1a2b3c4d ("read path", branch
│   sessions/read-path), which changed them too. To
│   agree who changes what, call `leave_note` with
│   `s-1a2b3c4d`.
│ - `crates/store/src/migrations.rs` now conflict
│   with main, which changed it in 4f9c2e1 Merge
│   pull request #2190 from Mele-Labs/x. Merge it
│   into your branch and resolve the conflict.
│
│ Carry on with the part you were given.
└────────────────────────────────────────────────
```

## The checks report

Fires as the Checks a Drone asked for land (#1020, #1062). The call that asked
answers at once, because a build outlasts what the agent CLI waits on one tool
call. A result that lands while others still run is a short turn of its own;
the last turn is the report, and says the run is over. The first command that
fails stops the rest, and the report names it.

**Only while the part is still going.** A submission, a kill or a Drone gone
stops the run, and whatever follows is told nothing about it.

**A run that goes on says where it stands on a timer.** What is running and for
how long, what waits for a slot, what is done, so the Drone never asks. A run
whose task died ends with a turn saying the checks stopped before they
finished, that it is Fleet's fault and not the work's, and to ask again or
submit.

**A second ask while one is going is refused with the same facts**, how long,
which Check, whether it waits for a slot, and still says the report is a later
turn. It never tells the Drone to look again.

**Fleet's own report.** `fleet::ChecksReported` is the one constructor for the
turns, built from the run and nothing else. Its own `Occasion`, `Checks`.

**Drafted wording. Not sanctioned.**

```
┌─ THE CHECKS YOU ASKED FOR ─────────────────────
│ `build` passed in 12.3s. Still going: `test`,
│ `storybook`. Each result arrives as its own
│ turn, and the last one says the run is over.
└────────────────────────────────────────────────

┌─ THE CHECKS YOU ASKED FOR ─────────────────────
│ The run is over. This is what each one did:
│
│ (one row per Check: what it did, how long it
│ took, and where its Check log is — then the
│ sentence saying this is not a verdict)
└────────────────────────────────────────────────
```

**Stopped at a failure**, the report opens by naming the Check that did not
pass and saying the ones still going were stopped rather than finished, and
each stopped row says which Check stopped it.

## The fix report

Fires when the test a Drone said is broken on main has run there (#999). The
`draft_fix` call answers once that run has started, for the checks report's
reason, and what main showed arrives as this turn.

**Only while the part is still going**, the checks report's rule: a submission,
a kill or a Drone gone stops the run, and nothing is drafted from it.

**Fleet's own report.** `fleet::FixReported` is the one constructor, built from
the run and nothing else. Its own `Occasion`, `Fix`.

**Drafted wording. Not sanctioned.**

```
┌─ THE TEST YOU SAID IS BROKEN ON MAIN ──────────
│ `<test>` fails on main too, so the failure is
│ not your change. A fix is drafted as Job <id>
│ and waits for a person's approval. Until it
│ lands and reaches your copy, `<file>` are that
│ fix's and outside what this Job may change: a
│ declaration naming one is refused. Carry on with
│ the rest of your part: your own checks still
│ fail on that test until the fix lands, so say so
│ in your evidence.
│
│ (or, where it passes on main or could not run
│ there, the sentence saying why nothing was
│ drafted)
└────────────────────────────────────────────────
```

## The poke

Fires when nothing structured has arrived at all. `poke_limit` is a
liveness counter, distinct from the evidence clarification-round cap,
which checks content sufficiency. Both can end in `stalled`.

The turn cannot tell whether the Drone is working, finished, or stuck, so
it has to address all three without pressing any of them.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ Nothing has arrived from you for 12 minutes.
│
│ If you are working, keep going. If you are
│ finished, submit — work you do not submit is work
│ no one sees. If you are stuck, use the escape
│ hatch. It is a sanctioned outcome and not a
│ failure.
└────────────────────────────────────────────────
```

Three branches, because the poke cannot tell which one it is in.

The escape-hatch line repeats a baseline clause on purpose: the moment a
Drone most needs to know the hatch is sanctioned is the moment it has gone
quiet.

Elapsed time survives where a count would not. Twelve minutes names no
threshold and hands over no budget. This turn must never become "second of
two pokes", which would tell a Drone precisely how long it has left to
look busy.

**This draft decides something section 5 has not.** It uses the word
*sanctioned*, and framing on the `escape_hatch` clause is deliberately
unspecified pending measurement, because opt-out rates move with the
authority the option is framed through. Sanctioned is the maximal framing
available, and this draft reached for it.

## The clarification reprompt

Asks for the same work, evidenced better. A free round that does not
consume the retry budget.

The cap has no field name and no counting scope — per Job, per workflow
step or per loop iteration is undefined. Tracked as
`[drone-evidence-clarification-cap-scope]` in
[Drone](../../concepts/drone.md), and its value as
`[evidence-clarification-round-cap]` in
[Configuration](../configuration.md).

The baseline clause that used to describe this mechanism lives here, since
stating it in advance spends a constant on a case most Jobs never hit.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ Your submission arrived and there is not enough
│ in it to check.
│
│   Shown by names no artifact. "Tests pass" is not
│   one. Name the test and the command you ran.
│
│ Submit again.
└────────────────────────────────────────────────
```

**The turn says nothing about budgets.** Saying a round is free only means
something to a Drone that knows rounds are counted, and a Drone told
rounds are counted has a bar to optimise against.

Naming the report format is not naming the bar. Telling a Drone to name
its test and its command shapes the record. Telling it what the test has
to prove would shape the work, and that is the line.

## The force-interrupt directive

Stop and report current state now. Failure of that escalates as
`no_report`, not as `thrashing` — `thrashing` is the verdict the directive
is injected on, and a Drone that answers it is not stopped at all.

The mid-step Judge pass that produces the verdict does not read the Drone
transcript. It judges the work product as the branch stands against the
declared scope and the step's intent, so what the turn can carry is an
observable that has not moved rather than a turn count.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ Stop and report your current state now.
│
│   Expected   A second Job starts on its own path
│   Produced   worktree path already registered,
│              unchanged across every approach you
│              have tried
│
│ Submit what you have. Partial work with an
│ accurate Not claimed is worth more than carrying
│ on.
└────────────────────────────────────────────────
```

The mid-step record is what makes this actionable rather than a scolding.
"You are thrashing" tells a Drone nothing it can act on. An observable
that has not moved across every approach tells it which approach to stop
repeating.

Thrashing is the absence of change in `produced`. The turn count is not
the finding — it is what makes an unchanged observable mean something —
and it stays out of the Drone-facing rendering, for the same reason every
other counter does.

## The refusal reprompt

Carries the Judge record's `expected` and `produced` back to a Drone.
Never `consequence`, which is written for a person deciding whether to
care, and never a counter.

The closing line is where `what_changed` on the next work submission comes
from — [Agent Copy Contract](../agent-copy.md).

**It reaches no Drone in the middle of a job, and none of the fields
change for that.** A refusal stops the step, so the Drone that submitted the
work has ended by the time the verdict exists, and no path injects this.
Where it does arrive is the opening brief of a Drone a person restarts the
step with, under the rule above: the frame is re-tensed for a reader with no
memory of submitting anything, and the two rows are unchanged.

**So the baseline promises a later turn only where the part is coming back**
— section 5. A refusal is not that, and a Drone told to wait for a turn no
path sends waits, holding a job, until a person notices.

```
┌─ WHY THIS PART IS BEING DONE AGAIN ────────────
│ An earlier attempt at this part was checked and
│ did not pass. Its work is on the branch you are
│ in.
│
│   Expected   Suite red when a work machine's row
│              shows 68% quota left
│   Produced   Suite green on a row rendering no
│              spend at all
│
│ Address this and submit again. Say what changed
│ since the last submission.
└────────────────────────────────────────────────
```

## The permission answer

What Armada's permission tool says when it refuses a call. It is the tool's
reply rather than a turn, so the Drone reads it inside the call it made. Which
variant goes is Fleet's to decide, from the Job's setting, from whether a person
answered, and from whether the command can be allowed at all.

| Variant | Sent when |
|---|---|
| Not granted | The Job refuses and holds, or the call is a tool rather than a command |
| Asked | The Job asks, and nobody answered inside the hold |
| Already asking | The Job asks, and a person is already being asked about another call |
| Rejected | A person said no while the call was held |
| Withheld | The command is declared destructive, runs what one of the Manifest's Checks runs, or the harness cannot grant it |

**A command that runs a Check is withheld under every `when_blocked`**, as a
destructive one is: no person's setting widens what the repository already
declared.

**And the question it asked is answered by running what the repository
declared for it.** Fleet does not grant the command as typed; it answers the
permission call with the Check's own command, narrowed to what the Drone has
changed, and the harness runs that. A Drone typing a test command wants to know
whether its work holds up; being told to call a tool instead leaves it to go and
call that tool, and across six Drones none ever did — thirty-two commands by
hand against one call.

**What the transcript records is the command that ran.** The answer is a yes
with a different input, never a no carrying a result it did not earn, so a
reader of the record is never shown a refusal whose body is a passing test.

**The harness runs the substitute, and the Drone sees that it did.** Measured in
[spike 15](../../spikes/015-can-a-person-answer-a-blocked-command.md) — the asked
command never runs, and an agent asked to report what its command printed says
unprompted that a different one ran. Nothing has to hide the substitution for
the answer to be honest, which is why it is a yes.

**Handed over only where a command is the whole of what is needed.** The harness
runs one string and knows nothing about what Fleet would have put around it, so
a Check with a prerequisite, a command still naming a placeholder only Fleet's
environment fills, and a narrowing that comes to nothing are all refused as
before, naming the Check and pointing at `run_checks`.

**It does not carry the place in the machine's queue.** `${width}` is resolved
before the command is handed over, so a suite is bounded in workers; a command
the harness runs holds no place, so two Jobs could each have one going. That is
the second half of [#1444](https://github.com/NickMele/armada/issues/1444), and
the trade this shape makes for being honest about what ran.

The already-asking and ungrantable forms follow the same two sentences: what
is happening, then what not to do. Neither is a final refusal of the first
kind — the already-asking call is not recorded as one.

**Drafted wording. Not sanctioned.**

```
┌─ NOT GRANTED ──────────────────────────────────
│ This Job is not granted `npm publish --access
│ public`. A person decides whether to allow it.
│ Do not try to get the same result another way.
└────────────────────────────────────────────────
```

```
┌─ ASKED ────────────────────────────────────────
│ A person has been asked whether you may run
│ `npm publish --access public`, and has not
│ answered yet. Stop and wait: the answer arrives
│ as your next turn. Do not try another way to do
│ the same thing meanwhile.
└────────────────────────────────────────────────
```

```
┌─ REJECTED ─────────────────────────────────────
│ A person said no to `npm publish --access
│ public`. Do not run it, or anything that does
│ the same thing. Carry on without it if the work
│ allows, or ask a question if it cannot be done
│ without it.
└────────────────────────────────────────────────
```

```
┌─ WITHHELD ─────────────────────────────────────
│ `rm -rf .armada/store.db` is declared
│ destructive in this repository, and an
│ unattended Job never runs it. Do not try to
│ get the same result another way.
└────────────────────────────────────────────────
```

**The asked variant is why the permission turn exists.** Over HTTP the agent
CLI stops waiting on a tool within minutes, well before a person can be counted
on to answer — [spike 15](../../spikes/015-can-a-person-answer-a-blocked-command.md).
So the call is answered with a wait, and the person's answer follows as a turn.

## The permission turn

A person's answer to a permission question, arriving after the call it was
about has already returned. A Drone reading it is waiting on exactly this, so it
names the command and says what to do next and nothing else.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ A person allowed `npm publish --access public`
│ for this Job. Run it again now; it will not be
│ refused.
└────────────────────────────────────────────────
```

```
┌─ TURN ─────────────────────────────────────────
│ A person allowed `npm publish --access public`
│ in this repository. Run `npm publish --access
│ public` again now; it will not be refused.
└────────────────────────────────────────────────
```

```
┌─ TURN ─────────────────────────────────────────
│ A person allowed `npm publish --access public`
│ on this machine. Run `npm publish --access
│ public` again now; it will not be refused.
└────────────────────────────────────────────────
```

The third is an Always allow kept in Kit's allowlist, for every Job on this
machine (5 Oct 2026).

A person who says no sends the rejected answer's wording as the turn, unchanged.

**It never crosses a step boundary.** Where no Drone is left to tell, the step
restarts and the allow is in force for the next Drone, which is told nothing
about it.

## `redirect_drone`

Human-initiated, so its content comes from a person rather than from a
verdict. It carries no record.

## The Pilot handoff instruction

Human-initiated. The Drone's only contribution to the handoff is a
narrative of what it is stuck on, passed as the tool argument; Fleet
assembles everything else. It lands in the bundle a
[Pilot](../../concepts/pilot.md) job opens with.

Framing on the `escape_hatch` clause is deliberately unspecified in the
baseline, because opt-out rates move with the authority the option is
framed through. The same caution applies to this turn.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ A person is taking over this Job. Stop what
│ you are doing and call the escape hatch now.
│
│ It takes three things, and they are the only
│ part of the handoff that comes from you:
│
│   Trying to    what this part was meant to
│                produce
│   Blocked by   the specific thing in your way,
│                or nothing, if nothing is
│   Tried        what you attempted, and what
│                each attempt produced
│
│ Name the command, the path or the message. "I
│ am stuck" leaves nothing to act on, and a
│ blocker you did not hit leaves less.
│
│ Do not submit your work first. This is not a
│ submission and the handoff does not wait on
│ one.
└────────────────────────────────────────────────
```

**The turn offers nothing, so it frames nothing.** A Drone told to hand
over is not choosing the hatch, and framing moves an opt-out rate only
where there is an opt-out. This turn carries the caution section 5 asks
for by having no authority word in it at all, which is the one thing the
poke draft could not do.

**Nothing is a legal `blocked_by`.** Two of Pilot's three trigger paths are
human-initiated and the Drone may be fine. A Drone asked what is blocking
it when nothing is will name something, and a fabricated blocker is the
failure the hatch exists to prevent arriving through the hatch itself.

**The three fields are named and the bar for them is not.** `blocked_by`
is the field a person acts on, so the turn asks for the command, the path
or the message rather than a characterisation. What the blocker has to
amount to is left unsaid, the same line the clarification reprompt draws.

**The prohibition is specific.** *Stop* does not stop a Drone carrying a
baseline that says work it does not submit is work no one sees. *Do not
submit* does, and it is the move the M1 rendering makes on the parts that
are not the Drone's.

The tool is described rather than named, as it is in the poke and in the
baseline clause. And the turn carries no record: nothing observable fired
it, there is no verdict behind it, and the bundle already holds every fact
Fleet could put in one.

## The hatch-unavailable answer

Fires when a Drone calls `escape_hatch` on a Job Fleet has not marked for
the handoff. The pull does not go through and the Job escalates as
`hatch_unbidden` — [Pilot](../../concepts/pilot.md) owns the mechanism.

**Not the refusal reprompt.** That one carries a Judge verdict on evidence
that arrived. This one answers a tool call, and no verdict sits behind it.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ The escape hatch is unavailable.
│
│ Submit what you have through the evidence
│ submission tool you have been given, and say
│ what you did not reach. Name the command, the
│ path or the message that stopped you.
│
│ Partial work with an accurate Not claimed is
│ worth more than carrying on.
└────────────────────────────────────────────────
```

**The turn says the hatch is unavailable and nothing else.** A refusal
that explains itself teaches the Drone that a mark exists, and a Drone
that knows a mark exists has a gate to reason about rather than work to
do.

**It carries no record and no counter.** Nothing observable fired it — the
Drone's own tool call did, and the Drone already knows what it tried.

**It asks for the bar `blocked_by` asks for**: the command, the path or
the message rather than a characterisation. Submission is the action left
to the Drone, and the closing line is the force-interrupt directive's,
because what it asks the Drone to do is the same.

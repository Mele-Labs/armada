# Voice & Copy

**Part of:** [Design System — UI & Voice](../design-system.md).

## Typography of reference — applies to docs, not just UI

Unlike the rest of this section, these rules govern **internal
documentation and planning pages as well as product copy**. They exist
because the docs are read constantly and their conventions leak into the
product.

- **Never use `§`.** Write "M0 step 4," not "M0 §4." The section sign is
  legal-brief and academic-citation typography; it reads as affectation
  in a working document and nobody says it out loud. "Step" is one
  syllable longer and infinitely more readable.
- Same reasoning bans `¶`, `cf.`, `ibid.`, `op. cit.`, `viz.`, and
  `q.v.` Write "see," "compare," or "same source."
- `e.g.` and `i.e.` are fine — they are common enough to have stopped
  reading as Latin.

**Citing v1 code.** A bare file path on any Armada page refers to
**v1**. The convention is that such paths resolve against branch
`v1-archive` / tag `v1-final` and never `main`, because Ground Zero step
1 orphans `main` — but **that step has not run**, so neither the branch
nor the tag exists in the clone. Verified when the v1 port-allocation
extraction found the files present on `main` and cited a commit hash
instead.

So, until it runs: v1 paths resolve against `main`, and **a citation
should carry a commit hash**, because line numbers on a live branch are
not stable. After it runs: the convention applies as written and
existing citations need re-anchoring — a commit hash cited today still
resolves from `v1-archive`, so nothing is lost by citing one now. Line
numbers stay accurate against `v1-final` once it exists, because a tag
is frozen.

Approval prompts, status reasons and escalation messages are what you
read at 11pm deciding whether to kill a drone. These rules apply to
product copy, not to internal documentation.

**Scope split.** This contract governs static UI chrome, which is not
configurable. The Machine-level **Voice** setting tunes runtime-generated
prose (Judge summaries, Helm replies, job summaries) within this
contract. It may adjust length and formality. It may not override the
principles, the lexicon or the status grammar. "Terse" and "explanatory"
are legal Voice values. "Playful" is not.

## Principles

**P1. Metaphor lives in proper nouns only.** Nautical vocabulary is
confined to names: Armada, Fleet, Bridge, Helm, Drone, Manifest,
Job Board. Kit and Machine are lexicon terms but carry no metaphor —
they say what they are. Every verb, state, error and instruction is
plain English. Write "Drone 4 stopped reporting 12 minutes ago", not
"Drone 4 has gone dark". Pilot is a proper noun; "take the wheel" is not
a verb Armada uses.

**P2. Briefing register.** A message carries the facts needed to decide,
on screen, without a click. Weak: "Drone 4 stopped reporting. Poke limit
reached." Correct: "Drone 4 stopped reporting 12 minutes ago after 3
pokes. Step 2 of 5, last wrote `auth/session.rs`."

**P3. First person is Helm's alone.** Bridge and Fleet never say "I".
Helm says "I" only for what Helm itself did. Reporting a Fleet event,
Helm uses the same impersonal phrasing Bridge does.

**P4. Hedge by source.** Three source classes, three registers.

- **Measured** speaks flatly. "Tests passed." "`pnpm test` exited 1 on 4
  assertions."
- **Estimated** is marked as approximate. `~$2.40`, never `$2.40`. A
  derived figure is not a measured one, and rendering it with the
  authority of an exit code trains you to act on a number that may be
  wrong.
- **Judged** is visibly a judgment and names its source. "Judge read the
  evidence as not covering the error path."

Render any two of these identically and one bad value teaches distrust
of the other two.

**P5. Event-first, with cause.** The subject of a failure sentence is
the job or step, never the drone. Write "Step 3 did not advance. No
evidence after 3 clarification rounds", not "Drone 4 failed to submit
evidence". Known causes state flatly. Hypothesised causes hedge and name
their source, and only Judge and Helm may produce them.

**P6. Fixed copy is a template; generated copy is a substance
requirement.** Fleet's own strings should be identical every time,
because uniformity is scannability. Generated text is specified by what
it must contain, never by what shape it takes, because a structural
rule produces twenty interchangeable paragraphs. A summary that would
read plausibly under a different job has failed.

## Prose rules

- **Sentence case everywhere.** No title case, and ALL CAPS in three places
  only: table headers, an eyebrow — the small label that heads a
  section or card, like a transcript card's DRONE — and a worktree slot
  bay's state word (HELD, BUSY, STRANDED, FREE), all at `--text-2xs`
  with `0.04em` tracking. The owner allowed the eyebrow on 29 Sep 2026,
  from a sketch that drew one, and the bay's word on 4 Oct 2026, from a
  preview he picked. Lexicon proper nouns keep their capitals
  inside sentence case.
- **One column, one case — and the case is the one that can hold for
  every line in it.** Where two producers write lines into the same
  column, the lines read alike or a reader sees a seam where there is no
  difference in kind. Helm's thread and its session record are the case:
  Fleet writes the *no reply came* sentences and Bridge words the row about
  a session it could not resume. Fleet's open lower case and cannot do
  otherwise — some of them lead with the program that would not start,
  `claude` or `codex`, and the lexicon refuses a model or program name
  promoted to an actor — so on **2026-09-18 Bridge's own thread lines
  lowered to meet Fleet's**, rather than capitalising a program into an
  actor to meet Bridge's. A proper noun still keeps its capital wherever
  it falls, so a line opening on Fleet or Helm reads as it always did.
  **The exception is the column and nothing around it**: a button, a
  heading, a dialog's title and a table's own headings on the same
  surface stay sentence case, because no other producer's prose sits
  beside them.
- **Name things by what the person controls.** "Approve dispatch", not
  "Submit job payload".
- **No mid-sentence asides.** The rule targets the reflex rather than
  the character, because banning the em dash breeds a colon and banning
  the colon breeds a trailing negation. A colon separating a field from
  its value stays legal: "Step 3 stalled: no evidence after 3 rounds."
- **No adverbs by default.** "Successfully completed" is "Completed".
  "Currently running" is "Running".
- **No Wh- sentence openers.** They survive as panel headings: "Why
  this stalled", "What ran", "What changed".
- **No sentence that survives deletion without loss.** Remove it and
  see whether anything was lost.
- **Errors say what happened and what to do.** Never apologise, never
  be vague.
- **An action keeps its name through the flow.** A button that says
  Kill produces "Killed". The verb table below enforces this.

  An act that moves no Job status is not in that table, because there is
  no status for it to render:

  | Button | Produces |
  | --- | --- |
  | Add task | Added |
  | Drop | Dropped |

  `copy.ts`'s own labels are what enforce these.

## Lexicon

- **Armada** the app. Never the tool, the system.
- **Fleet** the daemon. Never the backend, the server, the sidecar.
- **Bridge** the operational surfaces. Never the dashboard, the UI.
- **Helm** the dock and its agent. Never the assistant, the chat, the
  surface.
- **Drone** one agent instance. Never the agent, the bot, the AI,
  Claude.
- **Job** one agent a person runs, in a terminal or in Bridge, and the work it does.
  Never run, ticket, session. The old Job, the unit a Drone worked on a plan, is
  deleted, and pages that still describe it are stale.
- **Task** one line of a Job's [plan](../../concepts/plan.md). Never step —
  a step is the workflow's own unit, and a Job's plan is its own account
  of the work rather than the workflow that gates it.
- **Job Board** the open queue. Never the queue, the backlog.
- **Job proposer** the model call that reads a request — a prompt, a
  ticket link — and proposes a Job: its workflow, what to call it, and
  where the work is several Jobs, the order between them. Scope is not
  among them, and [Job proposer](../../concepts/job-proposer.md) owns why.
  Lowercase, because it is a call rather than a component. Never the
  classifier, the Job-shape classifier, the shape classifier. What it
  produces is a **Job proposal**, and that is what the dispatch gate
  approves.
- **Kit** the tool set you bring — Skills, MCP, sub agents, Agent
  files, Plugins, Commands, the allowlist, the models list. Never
  global settings, preferences. Replaces Guild, retired Aug 2026.
- **Machine** how this installation behaves — resources, timing,
  budget, interface, notification routing. Never system settings,
  environment.
- **Manifest** per-project config. Never the config, the yaml.
- **Judge** the semantic verification layer. Never the auditor, the
  reviewer, AI review.
- **Evidence** the structured completion report. Never the report,
  output, proof.
- **Doctor** the health check. Never diagnostics, system status.
- **Workspace** one unit inside a repo. Never package, module,
  sub-repo.
- **Studio** the typed graph of one stretch of work, kept per repository.
  Never canvas, playground.
- **Scout** the read-only agent a Studio sends to read and report back as a
  Finding. Never researcher, explorer.
- **Drone transcript** the record of a Drone's turns. Never the Drone
  log, the Drone output.
- **Judge record** one Judge call and every judge's verdict inside it.
  Never a transcript — a Judge call is one-shot, and the Judge never
  reads the Drone's transcript, which is the isolation that makes its
  verdict worth anything.
- **Check log** what a Check wrote to stdout and stderr. Never the
  Check output, the Check results.
- **Retro** what got in the way while one Job ran, written once it ends.
  Never the post-mortem, the review — a review is of the change. What the
  retros' items make together is read on **Retros**, the page that lists them.
- **Lands in** where a retro item's fix lands: Armada, Kit or Manifest,
  exactly one. Never the owner, the area, the component — whose way it got
  in is a separate question.

**Claude is a model name, never an actor.** Write "Drone 4 stalled", not
"Claude stalled". The word appears only where a model is selected or
reported.

## Retired terms

These were in use and are not any more. A page still carrying one is
stale, not merely old-fashioned. Search for them when cleaning up a
page.

| Retired | Now | Note |
| --- | --- | --- |
| Guild | **Kit** and **Machine** | A split, not a rename. Tools and the allowlist became Kit; resources, timing, budget, interface and notification routing became Machine. Each site needs judgment about which one it was |
| Armada Server | **Armada API** | Named for the `api` crate |
| Job-shape classifier | **Job proposer** | It stopped classifying a shape when shape became derived, and stopped naming scope at all when scope became the workflow's first step. Neither half of the old name survived, and both halves misled — the second reading hid the fact that nothing chose a Job's workflow at all. A page still calling it a classifier, or saying it proposes scope, is describing a different call from the one that exists |
| Daemon | **Fleet** | Dropped as a redundant second name for the same process |
| Ground Zero | **M0 — Foundations** | Archived with the phase plan |
| Phase 0 through Phase 6, and numbered implementation steps | **Milestones** and their **Steps** | The nine-phase plan and its ~110 Steps live under "Archive — v2 phase plan" and are reference only. Milestone Steps are disposable and discarded when the milestone is met |
| Session | **Job** | Renamed on 17 Oct 2026, by the owner: *"I would love to actually rename Sessions to Jobs everywhere."* The old Job went in the 15 Oct 2026 decision, so the word was free. Prose, tool descriptions and prompts say Job. Code identifiers, wire field names, store columns, route paths, error codes and file names keep `session`, and `session_id` is still the parameter an agent passes |
| Convoy, Train, Atomic | *Nothing.* Say what the Job does | Names for kinds of Job. A Job is a Job: a screen says "three pull requests, landing in order" and never names a shape. Where a page meant the rules for a Job gated by several Manifests, they are on [Manifest](../../concepts/manifest.md); where it meant members in order, [Landing](../../concepts/landing.md) |

**Casing.** Docs capitalise throughout. UI capitalises the singular
named things (Armada, Fleet, Bridge, Helm, Doctor, Judge, Kit, Machine,
Job Board) and lowercases anything countable (job, drone, manifest,
workspace, evidence, workflow). So: "No active jobs. 3 waiting
on the Job Board."

## Status grammar

**Shape: headline plus fields.** A headline sentence, facts as labelled
fields beneath. Machine-derived fields render in mono, per the
typography rule above.

> **Job 12 stalled at step 3**
> Workspace `api` · 3 pokes · `auth/session.rs` · 12m · ~$1.80

**Verbs are generated from the enum, never written.** `stalled` always
renders "stalled", never "went quiet". For `escalated` and `queued`, the
headline verb is the reason rather than the state, because nobody says
"Job 12 escalated at step 3". This supplies the labels the token
section relies on to differentiate escalation reasons and `queued`'s
reasons by label rather than hue.

**`queued` takes its reason's verb where one is set.** With no reason it
reads queued; with one set, the reason supplies the headline and the
glyph. A Job out of headroom therefore reads "waiting on resources"
rather than falling through unrendered, which is what the old two-axis
field did.

**The map is a database, not a table on this page.** Every vocabulary
the UI renders is one row per variant, grouped by axis, carrying the
verb alongside the glyph and the hue that variant owes. A row with an
empty verb is a variant with no sanctioned copy, which is what the test
fails on. Pages needing the labels embed a filtered view; none of them
restates a verb. The map itself lives in the Armada Enum Verbs database.

**The plain label is what a queue row shows. The raw enum is
recoverable, never primary.** A row in Alerts carries the verb and
nothing else, matching the voice contract. The enum sits set back in
the **detail view header**, so an engineer can grep Fleet's logs with
the exact string without the queue reading like a stack trace. This
matters most for `fan_out` and `evidence_suspect`, whose plain forms are
not guessable back to the enum; the triggers named for their condition
are near-identity and lose little either way.

A Trigger's firing has its own states, `skipped`, `running`, `passed`, `failed` and `awaiting_owner`.
`awaiting_owner` renders as **waiting on you**: a Trigger on a destructive Command is not run until he says so.
It is not **needs you** in the Job's own sense, because the Job carries on.

`silent` takes no verb of its own. It is a sub-kind of `stalled` and
renders as **stalled** — the difference is entirely in the suggested
action on the payload, which is rephrase and redispatch rather than
plain redispatch. A badge that distinguished them would imply the Job
behaves differently, and it does not.

`thrashing` renders as **churning**. The enum name is OS jargon, and
the distinction that has to survive is busy-but-going-nowhere against
silent, since `stalled` owns silent.

`no_report` renders as **went quiet**, and it means the Drone said
nothing back — never that the Drone did nothing. That word belongs to
`stalled`, which is a Drone producing nothing at all; this one was
producing plenty and ignored an instruction to stop and report. The two
badge apart because a person answers them differently, and a reader who
collapses them has erased the trigger. It is not **churning** either:
that is the finding this followed, and the Drone may have been writing
the whole time it was said of.

`drone_killed` renders as **the Drone was ended by hand**, and the last
three words are the verb rather than padding. `interrupted` renders as
**interrupted** and draws a severed plug, a connection lost; this is a
person pulling it out on purpose, from a step they mean to run again.
Drop "by hand" and the two badges say the same thing about opposite
events, and a reader goes looking for a failure somebody caused
deliberately. It is not **stalled** either — that is usually what the
person killed the Drone *for*, and the Job's own escalation reason
keeps it. This one is the step's.

`run_ended` renders as **the Drone's run ended**, and it is
`drone_killed`'s sibling one actor over: there a person took the process
away, here the Drone said its own run was over and Fleet took it at that
word. Both are passive and neither names who acted, because what a step
badge answers is what became of the step. Not **stalled** for the same
reason as above — that is what the *Job* is held for, and one Job carries
both: the step says what happened to it, the Job says what it happened
for.

`scope_refused` renders as **the scope request was refused**, and the
Judge is deliberately not in it — attribution goes in the source field,
which is `evidence_suspect`'s rule below. It is not **blocked by
policy**, which is the badge it is being split away from: that one names
a configuration to go and edit, and a reader who cannot tell them apart
on the word edits a Manifest when what they should do is read what the
Drone was trying to write. It is not **stopped at the gate** either —
that says work was weighed, and here nothing was submitted.

`unheard` renders as **nothing is reading this Drone**, and it is the
one reason in this vocabulary written in the present tense. Every other
one names something that is over; this names a condition still true
while a person reads it — the Drone is working at that moment, and
nobody is on the other end of it. It is not **stalled**, which is the
badge it is split away from: that word is a Drone that stopped
producing, and a person who reads it here redispatches and ends a Drone
that was finishing its step. It is not **went quiet** either, for the
same reason from the other side — `no_report` is a Drone that could
hear an instruction and did not answer, and nothing was ever sent into
this one. The verb is about the reading rather than about the Drone,
which is what puts the fault where it is; "Fleet cannot hear it" was
refused because a failure headline names what happened and not who,
which is `no_worktree`'s rule above.

`evidence_suspect` renders as
**evidence disputed**, with Judge in the source field, which keeps
attribution out of the headline so P5 holds. Avoid "Judge rejected the
evidence", since `rejected` is already a job state.

The enum-to-verb map is one artifact with a test asserting every
variant has an entry, so a new reason cannot ship with no copy. Same
codegen intent already noted for the status tokens.

**The verdict vocabularies are not Job states and sit on their own
axes**, which is why they group separately above rather than joining
the status list. Step verdict is `workflow_status.last_step_verdict`; a
criterion verdict lives per criterion inside the Judge record and reads
differently by verification source, which is the P4 hedging device
working at the smallest scale it has.

**"No objection" rather than "accepted", and the step rather than the
Judge.** The Judge declines to refuse; it never grants. A pass headline
names the step — "Step 3 of 5 verified" — so attribution stays out of
the headline and the reader knows where they are in the workflow. Avoid
"Judge passed", which breaks both rules at once.

**A criterion attested by a person takes neither vocabulary above.**
Source Attestation reads **confirmed** · **withheld**. Affirmative where
the Judge's vocabulary is not, because a person may grant and the Judge
may only decline to refuse — the three registers are measured, hedged
and vouched, and the words have to carry that before the source field
is read. **Withheld**, not failed or refused: a person who looked and
would not put their name to it has done something neither other source
can do.

All of these belong in the same one-artifact-one-test map as the Job
states.

**Icons.** The token section differentiates escalation and `not_started`
values by label and icon. The verb table supplies the labels. The
icons, across every badge state rather than only these, are specified
on [Iconography](../iconography.md) along with navigation, actions, and
the rule for anything unlisted. lucide-react only, per hard rule 5.

That document supersedes the ten-row table this section used to carry.
Four entries changed: `stalled` moved off `hourglass`, which read as be
patient for the state that most needs to read as wrong and is now
banned outright; `blocked_by_dependency` moved off `lock`, which claims
a permissions problem that does not exist; `fan_out` moved off
`git-fork`, which loses its nodes at 12px; and the six base states
gained icons they previously lacked.

**Fields: universal in lists, per-state in detail.** The universal row
carries job identity, state, step N of M, elapsed, spend so far,
verification source, actor. The detail view expands per state and shows
only what applies.

**A fact may point somewhere, and it stays in the run.** Job detail's
header names the pull request Fleet opened — `Pull request #4711`, and
`Pull request #4711, merged` once somebody has taken it — beside the
branch it was opened from, not among the acts at the trailing edge.
Going to read something is not one of the things that end a Job. The
number is what is drawn and never the address: a forge address is sixty
characters of which a person reads four, and the whole of it stays on
the link's `title`. A Job with no pull request draws nothing here rather
than an empty field.

**Workspace is not in the row.** It was, and it came out: a row is
scanned, and the workspace is the field a reader already knows — they
opened this board, and every Job on it is theirs. Spending a track on it
costs the one that answers "is this stuck", which is elapsed. It stays on
the detail view, where a reader is asking about one Job rather than
comparing several.

**On All repositories, with more than one served, the row names its
repository.** There the repository is not implicit the way workspace is,
so a reader needs it to tell one Job's row from another's — see
[Job Board](../../concepts/job-board.md).

**Spend follows the active billing mode.** Personal-machine mode gates
on the quota % floor, so the row shows quota % remaining, which is
provider-reported and therefore measured. Work-machine mode gates on
the $ cost cap, so the row shows dollars, marked approximate (`~$2.40`)
until v1's figures are validated against actuals. The visible number is
always the number that gates dispatch. A permanently visible figure
that is not the gating figure is its own failure.

**Verification spend takes its own line, in the active mode.** A Judge
call is Job spend and renders like every other spend figure — quota on
a personal machine, dollars on a work machine. A Judge call priced in
dollars on a machine that gates on quota is a permanently visible
non-gating number, which the rule above forbids.

Spend stays in the row rather than moving to the detail view. It lost
the headline at the approval gate on the promise that it is always
visible, and removing it from the row breaks that trade.

**Layout consequence — the spend column is sized for both modes, not
one.** The two billing modes produce strings of very different width
and shape: `68% quota` against `~$2.40 of $20`. The column is sized for
the wider of the two and the row survives both, rather than fitting
whichever example appears first in this document. Neither mode is the
default — which one renders depends on the machine, and both are
first-class.

**Spend and quota render on the row and on `JobDetail` only.** Bridge/1088
removed both from the shell chrome along with the status bar — Fleet's
panel carries neither, and nothing in the shell
states a running total any more.
**The one exception, the owner's, 12 Oct 2026**: plan usage, the Claude plan's 5-hour and weekly limits, is one ring in the title bar, the 5-hour limit outside and the week inside (the owner, 15 Oct 2026), opening one inspect card of both, each limit's name and percent on one line over when it resets.

**Repetition.** A second stall at step 3 reads "stalled at step 3, 2nd
time", and the detail view surfaces the prior attempt. Presentation
only for now. Recurrence changing behaviour is a separate decision.

## Register by surface

**Approval gates** stay descriptive: what the job is, which workspaces,
which workflow. They go consequence-forward on blast radius alone,
meaning a job writing in several workspaces, auto-merge on, a job
touching root `armada.yml`, or a pre-approved batch. Cost never triggers it.

**Push alerts** carry facts rather than a ping. "Drone 4 stalled on
step 2 of 5, `auth/session.rs`, 12 min." Kill and Redirect are not
notification actions. When the line is cut, identity and verb survive,
then location, then elapsed.

**Empty states** draw nothing. An empty slot stays empty: a sentence or a
placeholder that only fills the space, or says that something is absent, is
filler — "No Drone under this filter", "Nothing recorded yet", "Ask Helm
about this repository." Two things still speak: a fact a person would act on,
where the empty result is the result ("This run changed nothing.", "Printed
nothing." for a command that finished), and a failure ("Fleet did not
answer"). A label left over nothing goes with the sentence. The owner, 29 Sep
2026, swept on 2 Oct 2026: *"We dont need to say anything."* The Overview
empty state below predates the sweep and is still drawn.

**Helm** answers, then may add a single observation, only after
actually looking, always flagged as its own inference. No
throat-clearing openers.

**Confirmations** appear on everything destructive and state what
happens and what survives. "Kill the drone on job 12? Step 3 of 5, 14
minutes in. Evidence carries forward if you redispatch." Action buttons
name the action. Pilot has no confirmations, since your hands are
already on the terminal.

**Not configurable.** Bridge confirmations always appear. The Kit →
Manifest "destructive-op list" setting governs Drone-initiated
operations only, not your own clicks.

## Behaviour rules that shape copy

**A screen reports. An explanation is something a person chooses to see.** The
owner, 23 September 2026: *"I hate it, I just want the app to show facts. Any
hints or guides should be something I choose to see."* A sentence that would
still be true of a Job that had never run is an explanation, and it belongs in
a guide rather than on the screen. #1602.

**A guide is data, numbered, in `packages/components/src/guides/`.** One file
each, one catalogue listing them in order, and a `?` beside the piece a guide
explains. The mark goes where the vocabulary is Armada's own — a group
boundary, a landing rule, how a graph is read — and never on every noun. The
mark is the character `?` rather than a glyph: `circle-*` is reserved to Judge
criterion verdicts, and the owner's own word for the control is a question
mark. An empty slot draws nothing at all — *Empty states*, above.

**A guide's piece is drawn somewhere, or the guide is retired.** Guide 8
explained a bar the Overview reframe of 29 September 2026 took off every
screen, and it stayed in the catalogue with nothing on a Job to open it. #1718.
`cargo xtask verify-foundations` now fails a guide whose const no screen names
outside a story, a test or the guides directory. The five pieces the catalogue
alone reaches are listed in the rule. A retired guide's number goes into
`RETIRED_GUIDE_NUMBERS` and is never reissued; 8, 11 and 20 are retired.

**A healthy state is stated, never implied.** Running renders in the Fleet
panel's head even when nothing is wrong, because a panel that said nothing
would read the same whether Fleet is healthy, loading or dead, and Fleet
outlives Bridge. The head's dot is where it is said, since 28 Sep 2026 — which
is what lets the body be empty in the one state that has nothing for it. An
unhealthy state adds a sentence naming what to do about it; a healthy one does
not, because there is nothing to do.

**Escalations interrupt, approvals queue.** Escalations cost money in
real time. Approvals cost latency. Push inherits this, so escalations
reach your phone and approvals never do.

**Routing config is bounded by this rule.** The loudness order is
silent < in-app < OS notification < push. Routing may move an event
type *down* that order, never up, and **an approval may never be
promoted to push.**

**This one is a contract rather than a preference, and it is the only
"you may not" left in configuration.** It holds because the two event
classes mean different things. An escalation means work has stopped and
nothing progresses until a person looks. An approval means work is
waiting to start and will keep. If approvals could reach push, the
distinction collapses and the escalation signal stops being trusted —
which is what Overview's counts and the push-alert design both
rest on. It is a product rule about what these events mean, not a
config-tier rule: notification routing is a Machine setting with one
value and no merge, so no Manifest is party to it.

**Fleet**, one dot in the title row on every surface and its panel under the dot, reads "Fleet running"
when idle, and pid, port, protocol and up as rows under it; it carries neither
spend nor quota, which left the shell chrome entirely with the status bar
Bridge/1088 replaced. The title bar draws the plan's limits as one ring in the title row and not in Fleet.

**Two separate fields, not one.** These do different jobs and a reader
should not have to guess which.

- **Verification source** is the P4 hedging device and nothing else.
  Closed vocabulary of three: **Check**, **Judge**, **Attestation**.
  Only a human may set Attestation — never a Drone, never Helm, never a
  Judge — and a Job carrying an attested criterion must not render
  identically to one where everything was mechanically verified. It
  answers how far to trust a result. **Attestation names the record
  rather than a verifier**, because there is no third verifier: Check
  and Judge are things Armada runs, and this is what a person leaves
  behind.
- **Actor** is audit attribution, and the field the three-way
  separation depends on. Vocabulary: **human**, **Helm**, **Drone**,
  **Fleet**, and **commander** for a Job's Commander. It answers who did this.

They are orthogonal, and events may carry one, both or neither. A
manual change during Pilot is actor=human with no verification source.
An allowlist denial is actor=Fleet with no verification source, since
Fleet blocking an operation is not a verification result. A failed gate
is verification source=Check with actor=Drone.

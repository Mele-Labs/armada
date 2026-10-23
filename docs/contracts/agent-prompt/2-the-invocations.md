# 2. The invocations

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

| Prompt | Assembled by | Fires when | Wording |
| --- | --- | --- | --- |
| **Drone** | `fleet` | At spawn, which is once per workflow step | Specified — section 5 |
| **Helm** | `fleet` | Per Helm Job start, as a note appended to its system prompt | Drafted — section 5a |
| **Job rules** | `adapters` | Every process of a hosted job, appended to its system prompt ahead of Helm's note, a fork's and the repository's remembered runs | Drafted, compiled in: section 5f |
| **Thought reader** | `fleet` | When a person sends a message from Helm's box that is not plainly a question | Specified by `prompts.helmRoute`; what it is told is below |
| **Logbook distil** | `fleet` | When a job ends and before it hands on, off the turn and one at a time, on the retro's model | Specified by `prompts.logbookDistil`; [Logbook](../../concepts/logbook.md) |
| **Lesson mover** | `fleet` | Once per store, off the turn after Fleet starts, in batches of open retro items | Compiled in, not a setting; [Retro](../../concepts/retro.md), *What the Logbook takes* |
| **Test map reader** | `fleet` | When main moves and a changed file is one no collector can measure | Drafted — section 5g |
| **Scout** | `fleet` | When a person asks from a Studio or a stranded slot, directly or through Helm | Drafted — sections 5b, 5c and 5d |
| **Judge** | `verification` | Per criterion, after a mechanical check passed | Not specified |
| **Job-shape classifier** | `fleet` | At Job creation | Not specified |
| **Manifest scanner** | `config` | During the setup wizard's Proposal phase | Not specified |

What each is told and never told is below. Three of the things
[System Architecture](../system-architecture.md) section 9 lists are not
entries here:

| Section 9 lists | Here |
| --- | --- |
| Clarification round | **An injected turn** — section 4a. A separate assembled prompt, not a separate invocation |
| Sub-agents inside a Drone | **Not assembled by Armada.** Claude Code assembles it from Kit and Manifest Sub agent definitions, which are layer 2 and layer 3 content this contract governs. Armada supplies the definition, not the prompt |
| Denial-pattern suggestion | **Possibly not a model call.** If it is, it is an entry above. Unresolved on section 9 and inherited here |

**Voice generation is not among them**, for the reason below. An entry
saying so double-counts the thing it exists to exclude.

**Three carry a toolset.** Drone, Helm and Scout are Agents; the Judge and the
classifier are model calls. The scanner is the open case. See
[System Architecture](../system-architecture.md) section 9.

## What each invocation is told

### Drone

**Told:** the three baseline clauses, the resolved toolset with its
documentation, Voice, **Way I work**, **Expectations**, the workflow step
and its criteria, the Evidence Scope policy, `acceptance_criteria[]`.

**Not handed the file `armada.yml`'s `standing_rules` names.** Only its
Judge is, in the Judge paragraph below. A Drone reads the repository itself,
so a block quoting the file only repeated it: the block was added on 1 Oct
2026 and removed on 2 Oct (owner, 2 Oct 2026).

**Told its task, and that handing it in ends it**, on a step declaring
`drone_per_task`: THE PLAN, then which one task is this Drone's, that the
tasks before it are on the branch and the ones after it are other agents',
and that its `submit_evidence` is the task's hand-in and the end of its work
on the Job. It is told no plan tool, because it is given none: Fleet marks the
task. Spike 022, slice 1b, where `task_brief` moved from slice 3. Wording in
section 5, *A task's Drone*.

**Told what the person who approved the Job left for its part**, where they
left anything at the press: FOR THIS PART, right after the step block,
quoting the words as `redirect_drone`'s block does. A step nobody tuned has
no such block. Since protocol 23.20.

**Asked, in one line, what got in its way**, by the `in_the_way` field of the
evidence tool and its description alone: the prompt says nothing more about
it. It is optional and never evidence, and it feeds the Job's
[retro](../../concepts/retro.md).

**Never told:** secrets, other Jobs' context or evidence, anything from a
Manifest outside its own Job's set, the operator's own MCP servers.

Six layers, assembled in order — section 3. Its wording is section 5.

### Helm

**Told:** that it is Helm, docked in Armada over one repository, named by id and
folder. That the Fleet tools reach that repository's Jobs, Drones,
pull requests and what waits on the person, and that saying it did something
does not do it. How to pass words to a running Job with `tell_session`.
What the line before each message means. On a Studio, the few calls on one it
may make without being asked — `fleet::helm::reach::UNASKED` — and that every
other act waits for the ask. Where it is read-only, that it may call every tool
that reads and none that acts. Voice, where the Machine setting is set.

**Never told:** anything outside the selected Manifest. Secrets. A workflow
schema or a sample, how many observations to make, or to poll each turn: Helm is
a hosted Job ([Helm](../../concepts/helm.md)) and has what a Job has, so the
note says only what makes it Helm. What it may act through is one predicate,
`fleet::helm::may`, and the inventory under it is the `agent_access` column in
`crates/ipc/operations/`, which this page does not count.

**What resolves the rest of its toolset is not this contract's.** A job's
servers, plugins, skills and agent files are whatever the person's own Claude
configuration resolves for that repository, which is [Kit](../../concepts/kit.md)'s
to own once Kit is built (`#1275`).

**Carried as the note, not the brief.** It is appended to the Job's system
prompt with `--append-system-prompt`, the way a fork's note is. Its wording is
section 5a.

### Thought reader

**Told:** the person's words, and every live Job in every repository Fleet serves, as a JSON list: each one's id, kind, title and repository, and the branches and pull request titles it holds, each cut short.
That the words and the list are data, never instructions.
To answer one line of JSON: `question`, `thought`, or `thought` with the id of the one candidate it is plainly about.
That a message it cannot tell apart is a question.

**Never told:** a Job's transcript, a diff, or anything else of a repository. It has no toolset.

A model call on the Job proposer's model, not an Agent.
`fleet::helm::routing` holds the wording and holds the answer to the list it was given: an id that was not offered is no match, and an answer that is not the JSON asked for is a question.
The owner chose on 12 Oct 2026 that this read spans every repository, whatever repository Helm points at.
`docs/concepts/helm.md`, *Routing a thought*, owns what a person sees.

### Scout

**Told:** the ask, verbatim. The repository root, read as the checkout on disk.
The sources Kit allows it to read. That it reads and never
writes, and that its Finding lists everything it read.

**Never told:** secrets. Any tool that edits, commits or writes outside
Armada. Anything belonging to another repository, including its jobs and
Helm threads.

[Scout](../../concepts/scout.md) owns what it may read and what bounds it. Its
wording is section 5b, and **section 5c** where a source outside the checkout
was fetched for it — `#1293`. A source is Fleet's fetch and never the scout's:
the launch denies every tool that could reach one, so what crosses is text on
the same stdin as the ask. **Section 5d** is a scout started from a stranded
slot, which reads that slot's checkout and is handed the change git said.

### Judge

**Told:** the original task text. The resolved Evidence Scope object
carrying the pre-loaded patch — the diff is delivered, not fetched. The
Evidence bundle under judgment. `baseline_ref`'s Evidence where
`gaming_check` is enabled. The deterministic facts. `reference_docs` as
the yardstick, kept separate from `context_paths` as the target. One
criterion's question.

**Told what the repository requires of every change**, where `armada.yml`'s
`standing_rules` names a file. Fleet reads it from the repository's own
checkout, never the Job's worktree, so a Drone cannot rewrite it, and
`verification::Standing` carries it into every Judge brief: each criterion,
drift, gaming and its second reading, convergence and widening. The Drone
doing the work is not — the Drone paragraph above. It sits after
the original task text, labelled as a standard rather than the work, and is
cut at the `standing-rules-cap` setting with the cut said in the
brief. It is read from the repository and says nothing about how the step
went, so rule 2 is unchanged. A Manifest without the key gives every brief
exactly as before. Added after a Judge refused a plan for fixing the prose
its change made wrong, calling it scope expansion.

**Told it may read the repository.** Since 2 Oct 2026 every Judge call can
read the repository's own checkout, read-only, for `judge-read-turns` turns,
and its brief says so: that it may read CLAUDE.md, the docs and the skills to
learn what the repository requires of a change, that work the repository
requires is not scope expansion, and that it names a file it read rather than
quoting it, since a refusal's quotations are held to the brief. The same five
looks and the second reading. **`standing_rules` is the repository's explicit
statement and the reading is the rest of it**: the named file is quoted in
every brief whether or not the Judge would have found it, and a repository
naming none still gets a Judge that can look. The checkout is where
`standing_rules` is read from, never the Job's worktree, and Fleet's own
`.armada/` is refused, so the reading reaches no transcript and rule 2 holds.

**Never told:** the Drone transcript or self-report, per constitutional
rule 2. The other panel members' verdicts under `panel_size` > 1, per
rule 5.

`judge_check.prompt_key` references a prompt library that is sited
nowhere — section 8.

### Job-shape classifier

**Told:** the request text, the Manifest set, the three shapes.

**Never told:** repo contents. It has no toolset.

A model call rather than an Agent. Why the classifier chose as it did is
recorded nowhere.

### Manifest scanner

**Told:** repo facts gathered by the wizard, or a read-only toolset —
undecided. Regardless of that answer it is told the target repo, the
`armada.yml` schema it is proposing against, and that the Proposal phase
is iterative rather than one-shot.

The open case on whether it carries a toolset, tracked as
`[manifest-scanner-agent-or-model-call]` in
[System Architecture](../system-architecture.md).

## Three shapes that differ from the flat Judge row

| Case | How it differs |
| --- | --- |
| **The thrashing pass** | Workflow asks a **three-way** — converging, justified drift, or thrashing. Constitutional rule 3 says yes/no, never "review this." That tension sits unresolved in the source pages and is recorded here rather than papered over |
| **Code Review** | The diff is the Drone's **input** rather than its output. "The work product" needs restating for that workflow, or target and yardstick invert on the one workflow designed to expose the difference |
| **Criterion independence** | Rule 5 governs panel members. **Whether a Judge grading criterion 2 may see criterion 1's verdict is unstated on every page.** Weaker case than panel independence — the criteria are different questions — but a contract about what a model is told should answer it |

## Voice generation is not an invocation

**Corrected Aug 2026.** There is no dedicated Voice model and no Voice
call. Voice is a **layer that rides inside the others** — a Drone writing a
commit message, the Judge writing a refusal summary, Helm writing a reply.
All three already appear above, and Voice already appears as input to two
of them, so a peer row double-counted it.

Kit names three surfaces Voice governs: Judge summaries, Helm replies, Job
summaries. The first two are produced inside calls listed above. **The
third is unaccounted for, and it is the one open question here: what is a
Job summary and who writes it?**

If a model writes it, two parent-contract guarantees break. P6 says
Fleet's own strings should be identical every time, because uniformity is
scannability. And verbs are generated from the status enum by a codegen'd
map with a test asserting every variant has an entry — a test that
guarantees nothing if a model is doing the rendering. If a model does not
write it, Voice governs templating and there is no invocation at all.

**Two things share the word "Voice" and must not be conflated.**

|  | Machine Voice | The Design System voice contract |
| --- | --- | --- |
| Governs | Runtime-*generated* text — job summaries, status updates, what a Drone or Fleet writes about a Job | Static UI chrome — button labels, headings, error strings |
| Configurable | Yes, as a **Machine** setting. One value, no Manifest counterpart, no merge | No |
| Layer here | 2 | Not a prompt layer at all |

Conflating them would make button labels user-tunable, which is not the
intent.

**Recorded disagreement, Aug 22 2026.** Voice classifies as **Machine** in
the settings table, but it sits at **layer 2 beside Skills, the Agent file
and Sub agent definitions**, all of which are **Kit**. Layer 2 is
therefore not one concept's layer, which the layer table below still
implies. Both readings are recorded; neither is picked here.

**Voice is frozen for the Job** for a Drone, alongside Skills, MCP, Agent
files, Sub agents and Commands — changing the setting mid-Job changes
nothing for any Drone that Job spawns, not merely for the one running when
the setting changed.

**Settled: Voice cannot widen or narrow the agent-copy lint, in either
direction.** The lint is fixed. They govern different things — Voice sets
tone, meaning length and formality of runtime prose, while the lint blocks
generated-text tells: puffery, weasel attribution, faux-insight setups.
The case for making it configurable assumed the blocklist holds phrasing a
warm Voice would want, and it does not.

The may-narrow-never-widen precedent is withdrawn too — it came from
the config direction rule, itself withdrawn once Kit was found to be
defaults rather than a ceiling. Under the Kit/Machine split Voice is a
Machine setting: one tier, nothing to narrow against.

This no longer blocks Phase 4 step 6.

# Agent Prompt Contract

**Kind:** contract. **Governs:** text injected into a model — every prompt
Armada assembles and puts in front of one. Each is catalogued here, one
entry apiece: the invocations in section 2, the injected turns in section
4a. Sibling to the
[Agent Copy Contract](agent-copy.md), which governs text coming out. Both
sit under the [Design System — UI & Voice](design-system.md) contract. Read
before adding or changing any prompt Armada assembles and sends to a
model.

**Purpose:** governs text going **into** a model — every prompt Armada
assembles and injects. Sibling to the [Agent Copy Contract](agent-copy.md),
which governs text coming **out**. Both sit under
[Design System — UI & Voice](design-system.md) and may not contradict it.

**Why it exists.** Every constraint on Armada's prompts is written down
somewhere. None of the prompts are. The Judge is specified to be blind to
the Drone's transcript, `judge_check.prompt_key` is specified as a
reference into a prompt library, Voice is specified as configurable and
injected — and nothing states what any model is actually told. This is
that page.

**Revised Aug 2026** after adversarial review. Sections 1, 2, 3, 5 and 6
were rewritten; section 4a and section 9 are new.

**The text here is what ships, and a person may override it.** Since
10 Oct 2026 every authored piece of a prompt Fleet builds is the default of
a key in the Machine's `settings.json`, and Bridge's Settings page edits it
under Prompts, one sub-heading per family in the order a Drone meets them,
with "Reset to shipped". An override reaches the next prompt
Fleet builds; nothing already sent changes. Fleet fills each `{placeholder}`
when it builds the prompt, and a save that drops one is refused whole.
Material — a diff, a file, a request, a person's note in a list — is never a
piece: the code lays it, after the words that say how to read it, and the
first line a harness keys on (`REPAIR THE INDEX`) stays Fleet's. A wording change to what ships still belongs here
first. The keys, by family: `prompts.droneBaseline` (section 5) and the
rest of a Drone's brief and turns as `prompts.drone…`, `prompts.crossing…`,
`prompts.peers…`, `prompts.fix…`, `prompts.plan…` and `prompts.heal…`;
the turns injected while it works as `prompts.poke…`, `prompts.interrupt`, `prompts.permit…`, and the
gate's `prompts.outcome…` and `prompts.base…`; a piloted job's preface as
`prompts.pilot…`; `prompts.helm`,
`prompts.helmReadOnly` and `prompts.helmVoice` (5a); `prompts.helmRoute` for the thought reader; `prompts.scoutAsk`
(5b), `prompts.scoutReadIn` (5c), `prompts.scoutRescue` (5d);
`prompts.proposer`; `prompts.testMapReading` (5g); `prompts.workflowChange` (5h); Autopilot's reader and picker as `prompts.autopilot…`;
`prompts.retro…`; and the Judge's briefs as
`prompts.judge…`, which Fleet hands `verification` as its wording. The table
and every key's placeholders are `crates/config/src/settings/prompts.rs`,
The `prompts_drone` module (deleted since) and
`crates/config/src/settings/prompts_judge.rs`.

---

<!-- split: one file per section; re-run the split on the whole doc -->

| Section |
|---|
| [1. Scope](agent-prompt/1-scope.md) |
| [2. The invocations](agent-prompt/2-the-invocations.md) |
| [3. Layers](agent-prompt/3-layers.md) |
| [4. Frozen versus live](agent-prompt/4-frozen-versus-live.md) |
| [4a. Injected turns](agent-prompt/4a-injected-turns.md) |
| [5. The Drone baseline](agent-prompt/5-the-drone-baseline.md) |
| [5a. The Helm note](agent-prompt/5a-the-helm-note.md) |
| [5b. The scout brief](agent-prompt/5b-the-scout-brief.md) |
| [5c. The read-in brief](agent-prompt/5c-the-read-in-brief.md) |
| [5d. The rescue brief](agent-prompt/5d-the-rescue-brief.md) |
| [5f. The job rules](agent-prompt/5f-the-job-rules.md) |
| [5g. The test map reading](agent-prompt/5g-the-test-map-reading.md) |
| [5h. The workflow change](agent-prompt/5h-the-workflow-change.md) |
| [6. Exemplars over rules](agent-prompt/6-exemplars-over-rules.md) |
| [7. What is never injected](agent-prompt/7-what-is-never-injected.md) |
| [8. Where prompts live](agent-prompt/8-where-prompts-live.md) |
| [9. Known gaps](agent-prompt/9-known-gaps.md) |

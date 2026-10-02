---
name: annotations
description: Work through the notes the owner left on Bridge with the annotation layer — read each open one, find the code it points at, ask what is his to decide, dispatch a subagent to make each change, verify it, and mark it done. Load when the owner says /annotations, or asks you to read or act on his annotations.
---

# Working through annotations

**A note is a file.** The annotation layer (⌥⌘A in Bridge under `pnpm dev`, or in
`pnpm -C apps/desktop mock`) writes one JSON file per note under
`.armada/annotations/` at the repository root, gitignored. In a worktree it is
that worktree's root, so read the checkout the owner ran Bridge from — usually
the main one. The shape is `Annotation` in `apps/desktop/src/shared/annotations.ts`.

**Two ways a note reaches work.** In the layer, *Send to Fleet* proposes it as a
Job at the approval gate, and the note carries `sent` with that Job's handle.
This skill is the other way: a session reads the notes itself. Never do both to
one note — a note with `sent` belongs to its Job.

## The loop

### 1. Read the open notes

```
setopt nullglob; ls .armada/annotations/ .claude/worktrees/*/.armada/annotations/
```

**The shell is zsh, and a glob that matches nothing aborts the whole line.**
Without `nullglob`, no worktree holding notes means `no matches found` and not
even the main folder is listed. Confirmed 1 Oct 2026: it cost a second call.

**Every worktree's folder, not only the main checkout's.** A mock started from
a worktree writes its notes into that worktree, so a note pinned on a session's
review mock is invisible from the main checkout. Confirmed 29 Sep 2026: the
owner pinned a note on the Workflow mock (`:41060`, run from
`.claude/worktrees/workflow-board-pass`), the next `/annotations` read only the
main folder, and he had to ask where his note had gone. A note in a worktree's
folder is deleted with the worktree, so when it is done, copy it into the main
checkout's folder, marked `done`, before the worktree is given back.

Read every file. Skip `status: "done"`, and skip any with `sent` — say which Job
each went to, so the owner knows it is not lost. Group what is left by
`component`: three notes on one component are one change.

### 2. Find the code from the note, not from a guess

| Field | What it tells you |
|---|---|
| `text` | What the owner wants. The only field he wrote |
| `source` | The file and line of the JSX that drew the element, from the repository root. **Open this first** — it is the exact line, where the layer had it. A note left in Bridge under `pnpm dev` has no `source`: only the mock's build stamps it |
| `component`, `owners` | Search for these names where there is no `source`. `ownersFrom: "parent"` (Bridge under `pnpm dev`) is the tree it sits in, not who rendered it — walk it outward until a name is a component this repository defines |
| `element.text`, `selector` | Confirms you found the right instance, where a component draws in several places |
| `screen`, `layer`, `scenario` | Where to look at it: `pnpm -C apps/desktop mock` with `?scenario=` reproduces a mock note exactly |

**Look before changing anything.** A note is a reaction to what was on screen,
and the screen may have moved since. If the element is gone or already reads the
way the note asks, say so rather than inventing a change.

### 3. Decide, one note at a time

| The note is | Do |
|---|---|
| Small and clear — a word, a spacing, a missing state | Dispatch it, step 4 |
| A design decision — a new illustration, a changed layout, a contract rule | Ask with `asking-a-person`, citing the contract it touches (`docs/contracts/design-system.md`, `iconography.md`). Dispatch it once he has answered. Do not decide it |
| A defect | `armada-bug` |
| Bigger than a session, or better run as a Job | Tell the owner to press *Send to Fleet* on it, or file it as an issue if he prefers |

**Ask before dispatching, never after.** A subagent cannot put a question to the
owner. A note that touches a decision he made (a contract line, say) is settled
in this session first, and the brief carries his answer as a given.

### 4. Dispatch each change to a subagent

**The main session does not make the change.** The owner asked for this on
17 Sep 2026, after three notes in a row were each built inline. The main session
reads, asks, briefs, verifies and marks done. The agent builds.

**One agent per change**: a note, or a group of notes on one component.
`bridge-engineer` for anything under `apps/` or `packages/`, with
`isolation: "worktree"`. `work-issue`'s *Dispatching several agents at once*
section applies in full. Scopes must be disjoint to run in parallel, and two
changes that touch one file run one after the other.

The brief is the only context the agent has, so it carries:

| | |
|---|---|
| The note | Its file path under `.armada/annotations/` in the main checkout, and the owner's `text` verbatim |
| Where it is | The component and file already found in step 2, and the mock `?scenario=` that reproduces it |
| The decision | The option the owner picked, and its stated cost, word for word |
| The proof | `armada-components`: a story `play` or a mock test through `App`, run once against the change broken on purpose. A screenshot is how it is looked at, not the proof |
| The evidence | For a visual change, a walk under `mock/walks/` covering every surface it changed, on fixture data that shows the change, in a file named for this change alone. The report names the walk and the worktree's absolute path, so the mock can be served from it |
| The landing | Commit and push after each piece that passes, open a PR, **do not merge**. Run heavy commands in the foreground and wait. A decision it runs into goes in a single `**QUESTION:**` line at the end, and nothing that depends on the answer gets built |

**Verify what comes back yourself.** Read the diff, rerun the tests it added,
and look at the screen in the mock. **The whole suite the change reaches is run
twice, by the agent and by the merge line, and not a third time here.** Check
that the report names a whole-suite run with its counts, and send it back if it
does not: on 1 Oct 2026 the rail change in #1721 broke an existing Link test, it
was reported to the owner as verified on its three new tests, and the next agent
found the break. `scripts/land` reruns every Check the change hits before it
pushes `main`, so a repeat here buys nothing. On 2 Oct 2026 it was a repeat on
every change, and the owner asked for it to stop.

**A brief forbids the owner's clipboard, screen and apps.** On 1 Oct 2026 an
agent measuring paste shapes overwrote his clipboard and opened Finder, Safari
and Terminal on his screen. Playwright's own clipboard answers the same question. An agent's report of green has been wrong here.

**A visual change lands only after the owner has looked at it.** It ships with a
walk committed under `apps/desktop/src/renderer/src/mock/walks/` covering every
surface it changed, on fixture data that shows the change
(`docs/practices/running-locally.md`, *Walks*). Serve the mock from the agent's
worktree (`pnpm -C <worktree>/apps/desktop mock`), send him `?walk=<name>` on
it, and wait for his OK before `scripts/land`. A renderer change with no screen
to reach yet sends a Storybook story link instead. Confirmed 1 Oct 2026: five
agents were dispatched on the markdown change, and the plan landed them with no
step where he saw the app.

**Open the walk for him; a link in a message is not a look.** Run `open
<url>` on the served mock once it answers, then ask. Confirmed 1 Oct 2026: a
walk link went out under a "should this land?" prompt, and his answer was "I
haven't walked anything. Please show me."

**A walk shows the primitive its branch was cut from.** When a shared component
and its callers are dispatched in parallel, land the component first and merge
it into each caller before sending their walks. Confirmed 1 Oct 2026: four
call-site walks still drew the old `Prose`, so his only look at the new styles
was in Storybook, and he had to ask what was left to walk after all four landed.
Two of those agents also named their walk file `markdown-from-agents.ts`, and the
second was refused by the merge line on the clash.

**Any other green change lands with `scripts/land` without asking**; report the
merge commit it landed as. Whoever merges gives the worktree back, as
`work-issue` says.

**A component change is still a component change.** `armada-components` applies:
the contract wins, and the proof is a story or a mock test, not a screenshot.

### 5. Mark it done

Set `status` to `"done"` and `updatedAt` to now, in the file, once its change is
merged or its question answered. The layer re-reads the files each time it is
turned on, so the pin is gone there and the numbering closes over it — the bar
says how many are done and shows them again on request. **Never delete a note**,
unless the owner says to. He deletes his own.

**A note left on a worktree's mock lives in that worktree, and removing the
worktree deletes it.** `.armada/` is gitignored, so nothing carries it over.
Before giving a worktree back, move its notes into the main checkout's
`.armada/annotations/`, with the done ones marked. Confirmed 29 Sep 2026: the
Record's review ran on a mock served from its own worktree, and 24 of the
owner's notes were in that worktree when the cleanup commands were printed.

### 6. Report

**Lead with the walk link for each visual change, and say which changes are
waiting on his look** — they are not landed until he says so. Then one line per
note: its text, what you did, and the PR, issue or question it became. Notes you
left open, and why, come next. Name any agent still running.

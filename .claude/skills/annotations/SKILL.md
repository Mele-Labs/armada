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
A note with `scope: "window"` is on the whole window rather than one element:
its `<id>.png` is the window, and its `context` names the open Job and
Doctor's read — start from those, not from `selector`, which is just `body`.

**Three ways a note reaches work.** In the layer, *Start job* starts a Job
for it, and sending it to a Job messages that Job;
either way the note carries `sent` with where it went. This skill is the third:
a job reads the notes itself. Never two to one note — a note with `sent`
belongs to the Job it went to.

## The loop

### 1. Read the open notes

```
setopt nullglob; ls .armada/annotations/ .armada/preview/.armada/annotations/ .armada/slots/*/.armada/annotations/ .claude/worktrees/*/.armada/annotations/
```

**Bridge run from the preview writes its notes under
`.armada/preview/.armada/annotations/`.**

**The shell is zsh, and a glob that matches nothing aborts the whole line.**
Without `nullglob`, no worktree holding notes means `no matches found` and not
even the main folder is listed. Confirmed 1 Oct 2026: it cost a second call.

**Every worktree's folder, not only the main checkout's.** A mock started from
a worktree writes its notes into that worktree, so a note pinned on a job's
review mock is invisible from the main checkout. Confirmed 29 Sep 2026: the
owner pinned a note on the Workflow mock (`:41060`, run from
`.claude/worktrees/workflow-board-pass`), the next `/annotations` read only the
main folder, and he had to ask where his note had gone. A note in a worktree's
folder is deleted with the worktree, so when it is done, copy it into the main
checkout's folder, marked `done`, before the worktree is given back.

Read every file. Skip `status: "done"`, and skip any with `sent` — say which
Job each went to, so the owner knows it is not lost. Group what is left by
`component`: three notes on one component are one change.

### 2. Find the code from the note, not from a guess

**A note delivered to you starts from its Candidates.** Fleet searched the
checkout that served the page for the words on screen and put up to five
`file:line` lines under `Candidates:`, best first, each with why it matched.
Open the first. Confirm it by the owner's words and by the crop, the picture
attached to the turn, then take the next if it is the wrong instance. A line
`Stamp:` says a `data-armada-source` stamp was dropped because its line held none
of the words. `No line on screen matched the code in <checkout>` means the
search found nothing: go on to the fields below, and what the page cannot give
is in `docs/practices/capture-window.md`, *What a Job's note carries*.

| Line in the delivered note | What it tells you |
|---|---|
| `Pointed at:` | The element, by its label or its text. A note from the ⌥⌘A layer ends it with `(note <id>)`, the file in `.armada/annotations/` that step 5 marks done |
| `Candidates:` | Where to look, in order |
| `State:` | The tab or option selected, the walk step, the take and the scenario on screen. Reproduce these before judging the note |
| `Page:`, `Inside:` | The address as loaded, and the lists, sections and headings around the element, then the component chain |

A note read from a file, with no Candidates, uses these fields:

| Field | What it tells you |
|---|---|
| `text` | What the owner wants. The only field he wrote |
| `source` | The file and line of the JSX that drew the element, from the repository root. **Open this first** — it is the exact line, where the layer had it. A note left in Bridge under `pnpm dev` has no `source`: only the mock's build stamps it |
| `component`, `owners` | Search for these names where there is no `source`. `ownersFrom: "parent"` (Bridge under `pnpm dev`) is the tree it sits in, not who rendered it — walk it outward until a name is a component this repository defines |
| `element.text`, `selector` | Confirms you found the right instance, where a component draws in several places |
| `screenshot` | `<id>.png` beside the note: the element as the owner saw it when he saved the note. **Look at it before the live screen**, which may have moved since. Absent on a note from before screenshots were kept, and on one left in a browser |
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
| Bigger than a job | Tell the owner to press *Start job* on it, or file it as an issue if he prefers |

**Ask before dispatching, never after.** A subagent cannot put a question to the
owner. A note that touches a decision he made (a contract line, say) is settled
in this job first, and the brief carries his answer as a given.

### 4. Dispatch each change to a subagent

**The main job does not make the change.** The owner asked for this on
17 Sep 2026, after three notes in a row were each built inline. The main job
reads, asks, briefs, verifies and marks done. The agent builds.

**One agent per change**: a note, or a group of notes on one component.
`bridge-engineer` for anything under `apps/` or `packages/`, in a leased slot
(`agent-worktrees`: the preview sees only slots).
Briefing is `orchestrate-milestone`, *The brief*, and `session-habits`,
*Briefing agents*; `work-issue`, *Dispatching several agents at once*, has the
scopes. Two changes that touch one file run one after the other.

The brief is the only context the agent has, so it carries:

| | |
|---|---|
| The note | Its file path under `.armada/annotations/` in the main checkout, and the owner's `text` verbatim |
| Where it is | The component and file already found in step 2, and the mock `?scenario=` that reproduces it |
| The decision | The option the owner picked, and its stated cost, word for word |
| The proof | `armada-components`: a story `play` or a mock test through `App`, run once against the change broken on purpose. A screenshot is how it is looked at, not the proof |
| The evidence | For a visual change, a walk under `mock/walks/` covering every surface it changed, on fixture data that shows the change, in a file named for this change alone. The report names the walk and the slot's absolute path, so the mock can be served from it |
| The landing | Open a PR and **do not merge**; the rest of the landing is in the brief's own list (push as it goes, foreground runs, one `**QUESTION:**` line) |

**Verify what comes back yourself.** Read the diff, rerun the tests it added,
and look at the screen in the mock. **The whole suite the change reaches is run
by `ci` on the pull request, and nowhere on the owner's Mac.** The agent runs
the tests it added or touched, by filter, with `CARGO_BUILD_JOBS=2`, and its one
walk; the brief says so (8 Oct 2026: agents briefed to run whole crates and all
four walk shards were killed when the owner said the tests were "murdering my
machine"). A break outside the new tests is `ci`'s to catch, so never report a
change as verified before `ci` has passed, and never repeat what `ci` reruns
(2 Oct 2026: the owner asked for the repeat to stop).

**A brief forbids the owner's clipboard, screen and apps.** On 1 Oct 2026 an
agent measuring paste shapes overwrote his clipboard and opened Finder, Safari
and Terminal on his screen. Playwright's own clipboard answers the same question. An agent's report of green has been wrong here.

**A visual change gets its pull request only after the owner has looked at it.**
It ships with a walk committed under
`apps/desktop/src/renderer/src/mock/walks/` covering every surface it changed, on
fixture data that shows the change (`docs/practices/running-locally.md`,
*Walks*). Serve the mock from the agent's slot (`pnpm -C <slot>/apps/desktop
mock`), send him `?walk=<name>` on it, and wait for his OK before you open the
pull request. The rules for showing it are `session-habits`, *Walks*. A renderer
change with no screen to reach yet sends a Storybook story link instead
(1 Oct 2026: five agents were dispatched on the markdown change and the plan had
no step where he saw the app).

**A walk shows the primitive its branch was cut from.** When a shared component
and its callers are dispatched in parallel, land the component first and merge
it into each caller before sending their walks. Confirmed 1 Oct 2026: four
call-site walks still drew the old `Prose`, so his only look at the new styles
was in Storybook, and he had to ask what was left to walk after all four landed.
Two of those agents also named their walk file alike, and the second was refused
by the merge line on the clash.

**Any other green change goes into the preview, is adopted, and goes up as a
pull request, with no question in between** (`work-issue`, step 6); report the
pull request. It merges once `ci` passes, and the worktree goes back after, as
`work-issue` and `agent-worktrees` say.

**A component change is still a component change.** `armada-components` applies:
the contract wins, and the proof is a story or a mock test, not a screenshot.

### 5. Mark it done

Once its change is merged or its question answered: **a note sent to your
job**, call `mark_annotation_done` with its id (the `note` in its `Pointed
at:` line); it refuses a note sent to any other. A note pinned in the capture
window has no id and no file, so there is nothing to mark. A note not sent to a
job, set `status` to `"done"` and `updatedAt` to now, in the file. The layer re-reads the files each time it is
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

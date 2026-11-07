# The complex Job with subagents: armada-vs8t and armada-20v4

One bundle of two real open issues, TypeScript only, run once on opus through the fixed prototype of the recommendation. The text under "Brief" is sent as the first message of the Job, unchanged. Everything else on this page is for the person running the spike.

## Brief

Bridge keeps state for each Job in two places that nothing ever cleans up. Two open issues name them, and I want both fixed in one Job.

Part A is armada-vs8t, filed on 1 Nov:

> Job panes (#3132) keeps each Job's arrangement under a job-panes:job:<id> kept key. Nothing removes it when the Job ends, so the keys grow without bound. Drop a Job's key when the Job is gone.

It lives in `apps/desktop/src/renderer/src/job-panes/store.ts`. `createPaneFamily` builds three families of owners (`jobPanes`, `prPanes`, `operatorPanes`), each with its own kept key prefix, an in-memory state, ⌘Z histories and areas. `forget()` drops a whole family and is meant for tests and mocks. Nothing drops one owner. Bridge learns the state of its Jobs from `inputs.sessions` in `apps/desktop/src/renderer/src/sessions-wired.tsx` (a `read` state, and records with `state` and `dead`).

What I want from part A:

- A way to drop one owner of a family: its kept key and everything the family holds in memory for it, so it can be opened again later as a new owner.
- A rule for when a Job counts as gone, and Bridge applying it while it runs.
- A sweep when Bridge starts, for keys of Jobs that went while it was closed. It must never drop the key of a Job that is open, and never act before Bridge has read the list of Jobs.
- Your call, in the design note, on the pull request and operator families.

Part B is armada-20v4:

> job-panes/custom.ts keys Custom tab state by tab id alone, and tab ids are minted per window, not per Job. Two Jobs that mint the same id share one Custom tab's stage, mod and draft. Key the state by Job and tab. Found while building the tab needs-you card (#3146); its mock mints the seeded tab at id 900 to avoid it.

What I want from part B: Custom tab state keyed by Job and tab, so two Jobs with the same tab id no longer share it, every caller of `job-panes/custom.ts` updated, and a Job's Custom tab state dropped when that Job is dropped by part A.

Tests for each of these.

Do not change anything outside `apps/desktop/src/renderer/src`, and do not change how a layout or a Custom tab looks or behaves while its Job exists.

Load limits. This machine is shared. Run vitest only on the test files you add or change, from the app folder, for example:

```bash
cd apps/desktop && node_modules/.bin/vitest run src/renderer/src/job-panes/store.test.ts
```

A fresh slot has no dependencies, so run `armada run bootstrap` once in your slot (a few seconds, it copies them). Do not run cargo, do not run `pnpm install`, do not run a whole suite or a whole folder, do not start Bridge or a browser.

How to work. First record your plan with `record_plan` as groups, in this order.

- The first group is titled "Plan the change"; its `gives` text ends with `[gate: plan_approved]`.
- The second is titled "Write the design note"; its `gives` ends with `[gate: document_approved]`. The design note is an HTML page written with the `armada:writing-a-document` skill, saved under `.armada/documents/` in your worktree. It names the decisions this change turns on (at least: when a Job counts as gone, and how Custom tab state is keyed and dropped), the options, what each costs and your pick. I read it in Bridge and approve it before anything is built. Markdown, a file shown in the side panel, or an answer in this thread is not the design note and does not count.
- Then your own build groups. Part A and part B touch different files, so make them separate build groups, each with a title, its `paths` (the files it edits) and tasks. Independent build groups go to subagents working side by side, one subagent per group, as my Jobs do. Each subagent edits only its group's paths and runs vitest only on its own test files. If a group needs the other's work, split the join into its own later group. The `gives` of each build group ends with `[gate: none]`, except the last of them, which ends with `[gate: checks_green]`.
- Last is a group titled "Deliver"; its `gives` ends with `[gate: merged]`. When the branch is pushed and the work is ready, put the merge in `waiting_for` as one item that asks me to merge the branch; the merge is mine, and it goes past "no pull request" below.

A gate is closed by Fleet when it sees the fact, not by you, and Fleet tells you in one line when one closes. If your plan changes, record it again. Push your branch to `origin` after each build group is in. The `checks_green` gate is Fleet running vitest on the test files your branch adds or changes, at the commit you pushed, once every build group's files are in it.

There is no forge here, so no pull request. The branch on `origin` is what is delivered.

## What the right answer looks like

Not one right answer; the Job is scored on how the progress line followed it, not on the code. What would be sound:

| Piece | A sound shape |
|---|---|
| Drop one owner | A `drop(id)` on the family that removes the kept key, the in-memory entry, its history and its area. Opening the same id afterwards makes a fresh entry from the layout for all |
| Gone | The decision the design note is for. Ended (`record.state === "ended"` or `dead`), or no longer listed. An ended Job can still be opened and forked in Bridge, so dropping on end loses a layout a person might come back to. Dropping only when the list no longer has it is safer and slower |
| The sweep | Reads the kept keys of the family, compares with the Jobs list once `read.state === "read"`, drops the rest except the open Job |
| Other families | A pull request or the operator is not a Job: left alone, or given their own rule, and said so |
| Custom state | A map keyed by `job + tab`, with `dropCustomOf(job)` for the join. Every caller passes the Job. Two Jobs minting the same tab id get two entries |
| Tests | `store.test.ts` for drop and sweep; a wiring test for the live rule; a test in `job-panes` for two Jobs with one tab id |

## Why this one

| Pick | Why |
|---|---|
| armada-vs8t with armada-20v4 (this) | Both are TypeScript in Bridge's `job-panes`, vitest in tens of milliseconds, nothing to look at. Part A is `store.ts`, a sweep and `sessions-wired.tsx`. Part B is `custom.ts` and its seven callers (`PaneView`, `custom-body`, `tab-host`, `mod-tabs`, a mock scenario and two tests). Two files with their own tests, which a capable agent hands to two subagents at once. The join is small (dropping a Job drops its Custom state). The design decision is real and has a cost either way: when a Job counts as gone, and what a Custom tab's key is |
| armada-vs8t alone | Run in spike 48: the agent folded it into two groups and no subagent had anything to take |
| armada-2805 (watch state words into the shared verbs list) | The shared list is `crates/.../enum-verbs.toml`, which needs the generator and cargo |
| armada-gqq3 (Enter in a question card's Other field) | One file, a behaviour a person judges by pressing a key; no second part |
| armada-2586 and armada-2836 | Tests only, no design decision |
| armada-1707 (a notifications area for dismissed failures) | Needs a walk to judge |

## The scripted change request

Written before the run. After the first push of the Job's own branch once the design note is closed, the person sends:

> One more thing: armada-mttk. A tab mod's card in a Job's thread titles it with `modTitle` in `mods-made.ts`, so `review-queue` reads Review-queue, while its tab reads Review queue through `tabTitle` in `job-panes/custom.ts`. Make the card read the tab's name.

It is small and real, it adds a build group on a different file (`mods-made.ts`, which neither part touches), and a capable agent has to re-record its plan. If the agent ran both parts with subagents it has already pushed by then, so the change request lands on finished work.

## What the clone gives the agent

Base commit 734ba2c5c6 (current `main` of this tree) plus one rig commit: a one-check Manifest, no `.mcp.json`, no tracker. The clone is made with a smart transfer of `main` alone, so no spike branch or object of this spike is in it. `origin` is a bare repository in the scratch directory, made for this run. `armada run bootstrap` copies `node_modules` from this tree so vitest runs without the network.

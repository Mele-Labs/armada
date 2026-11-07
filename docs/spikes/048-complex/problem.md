# The complex Job: armada-vs8t

One real open issue, TypeScript only, run once on opus through the prototype of the recommendation. The text under "Brief" is sent as the first message of the Job, unchanged. Everything else on this page is for the person running the spike.

## Brief

Bridge keeps every Job's pane arrangement in the browser's kept storage and never removes it. The issue is armada-vs8t, filed on 1 Nov, and it is one sentence:

> Job panes (#3132) keeps each Job's arrangement under a job-panes:job:<id> kept key. Nothing removes it when the Job ends, so the keys grow without bound. Drop a Job's key when the Job is gone.

Where it lives: `apps/desktop/src/renderer/src/job-panes/store.ts`. `createPaneFamily` builds three families of owners (`jobPanes`, `prPanes`, `operatorPanes`), each with its own kept key prefix, an in-memory state, ⌘Z histories and areas. `forget()` drops a whole family and is meant for tests and mocks. Nothing drops one owner. Bridge learns the state of its Jobs from `inputs.sessions` in `apps/desktop/src/renderer/src/sessions-wired.tsx` (a `read` state, and records with `state` and `dead`).

What I want:

- A way to drop one owner of a family: its kept key and everything the family holds in memory for it, so it can be opened again later as a new owner.
- A rule for when a Job counts as gone, and Bridge applying it while it runs.
- A sweep when Bridge starts, for keys of Jobs that went while it was closed. It must never drop the key of a Job that is open, and never act before Bridge has read the list of Jobs.
- Your call, in the design note, on the pull request and operator families.
- Tests for each of these.

Do not change anything outside `apps/desktop/src/renderer/src`, and do not change how a layout looks or behaves while its Job exists.

Load limits. This machine is shared. Run vitest only on the test files you add or change, from the app folder, for example:

```bash
cd apps/desktop && node_modules/.bin/vitest run src/renderer/src/job-panes/store.test.ts
```

A fresh slot has no dependencies, so run `armada run bootstrap` once in your slot (a few seconds, it copies them). Do not run cargo, do not run `pnpm install`, do not run a whole suite or a whole folder, do not start Bridge or a browser. Use at most two subagents at a time.

How to plan. Record your plan with `record_plan` as groups, in this order. The first group is titled "Plan the change"; its `gives` text ends with `[gate: plan_approved]`. The second is titled "Write the design note"; its `gives` ends with `[gate: document_approved]`. It is a short page under `.armada/documents/` that names the one decision this change turns on, the options, what each costs and your pick, and the person reads it before anything is built. Then your own build groups, as many as the work needs, each with a title, paths and tasks; the `gives` of each ends with `[gate: none]`, except the last of them, which ends with `[gate: checks_green]`. Last is a group titled "Deliver"; its `gives` ends with `[gate: merged]`. For it, ask the person to merge the branch with `ask_the_person` once it is pushed; the merge is theirs, and it goes past "no pull request" below. A gate is closed by Fleet when it sees the fact, not by you, and Fleet tells you in one line when one closes. If your plan changes, record it again. Push your branch to `origin` after each build group. The `checks_green` gate is Fleet running vitest on the test files your branch adds or changes, at the commit you pushed.

There is no forge here, so no pull request. The branch on `origin` is what is delivered.

## What the right answer looks like

Not one right answer; the Job is scored on how the progress line followed it, not on the code. What would be sound:

| Piece | A sound shape |
|---|---|
| Drop one owner | A `drop(id)` on the family that removes the kept key, the in-memory entry, its history and its area. Opening the same id afterwards makes a fresh entry from the layout for all |
| Gone | The decision the design note is for. Ended (`record.state === "ended"` or `dead`), or no longer listed. An ended Job can still be opened and forked in Bridge, so dropping on end loses a layout a person might come back to. Dropping only when the list no longer has it is safer and slower |
| The sweep | Reads the kept keys of the family, compares with the Jobs list once `read.state === "read"`, drops the rest except the open Job |
| Other families | A pull request or the operator is not a Job: left alone, or given their own rule, and said so |
| Tests | `store.test.ts` for drop and sweep; a wiring test for the live rule |

## Why this one

| Pick | Why |
|---|---|
| armada-vs8t (this) | TypeScript in Bridge only, vitest in tens of milliseconds, nothing to look at. A real decision (what "gone" means, with a cost either way). Splits into a store change, a rule, a startup sweep and wiring, and the store change can be worked beside the wiring by a subagent. The issue is one sentence, so the scope is the agent's to find |
| armada-1707, nobody can find a failure again once its toast is dismissed | A notifications area in the title row with an unread mark. The owner would judge it by looking, which is outside this spike |
| armada-1356, a Tailwind class in packages/screens draws nothing | A search for utility classes in `packages/screens/src` found none, so the sweep it asks for seems done. What is left is a gate, and the rule is a contract decision, not a build |

Left out on purpose: armada-1407 and armada-z9s need `crates/core-model/domain/actions.toml` and its generator, which needs cargo. armada-nb3l (a finished subagent drawn as running) comes from the ledger's `subagent` attachments, which Fleet writes, so the fix is in Rust. armada-9ihy (a mock scenario) only exists to be walked. armada-2542 and armada-463 are Rust.

## The scripted re-plan

Written before the run. After the first build group's work is pushed, the person sends:

> One more thing: the Custom tab's state in job-panes/custom.ts is held by tab id alone and never dropped either. Drop it when its Job is dropped, and key it by Job and tab, as armada-20v4 asks.

It is small and real (armada-20v4 is open and names the same file), it adds a build group and changes the files the plan covers, and a capable agent has to re-record its plan.

## What the clone gives the agent

Base commit a2d2f0f8bf (current `main` of this tree) plus one rig commit: a one-check Manifest, no `.mcp.json`, no tracker. `origin` is a bare repository in the scratch directory, made for this run. `armada run bootstrap` copies `node_modules` from this tree so vitest runs without the network.

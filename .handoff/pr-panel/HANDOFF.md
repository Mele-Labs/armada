# Handoff: pull request panel redesign

Proposal only. Nothing in Bridge has changed and no tests are written. The
owner asked to see screenshots before any code; this folder is that proposal,
carried to a local session so it can be walked against the real app.

**Delete `.handoff/` before this branch becomes a pull request.** It is not a
document and is not in `docs/INDEX.md`.

## What the owner asked for

1. **One behaviour for every pull request link.** The Session reading sheet
   opens a panel, but other surfaces open the browser.
2. **A setting** with two options: open pull requests in Armada (the panel), or
   in the browser.
3. **Rework the panel.** Today it is only `PullRequestCard`. He wants something
   like GitHub, but better:
   - merge, comment and **Armada review** (a dispatched review Job that posts
     nothing on the pull request) up front;
   - status and progress readable at a glance, including **position in the
     merge queue**;
   - **auxiliary cards**: separate cards docked to the panel, for extra
     status such as the queue line.
4. The Cockpit's new **call cards** already have docked auxiliary cards. That
   work may exist **only on the owner's machine**. Find it and build the PR
   panel's auxiliary cards on the same primitive, not on a second one.

## The mockup

Open `mock.html` in a browser from this folder. It loads the repository's real
`packages/tokens/tokens.css`. Switch states with the hash: `#queue`, `#ready`,
`#failed`, `#settings`. Screenshots of each are next to it.

| State | Status block | Actions | Auxiliary cards |
|---|---|---|---|
| `#queue` | 3rd of 7, progress bar, ETA | Leave queue · Armada review · Comment | Merge queue list with this PR highlighted; latest Armada review |
| `#ready` | Ready to merge | Squash and merge ▾ · Armada review · Comment | Checks with durations; where merging would put it in the queue |
| `#failed` | 2 checks failed, 19/21 | Fix with Armada · Armada review · Comment | Failing checks with log excerpts, Open logs, Re-run failed |
| `#settings` | — | — | Links → "Open pull requests in": Armada / Browser |

Panel layout, top to bottom: eyebrow (repository, pin, open in browser,
close) → number and title → state, branches, author, diff size → status block
with four facts (checks, approvals, conflicts, Armada review) → actions →
tabs (Conversation, Checks, Files, Commits) → comment box pinned at the bottom
(Comment / Approve / Request changes).

Known flaw: the `#failed` timeline reuses the other states' filler and wrongly
says "Armada review: Passed" and "Added to the merge queue".

## Open decisions (put these to the owner)

| # | Question | Proposed default |
|---|---|---|
| 1 | Does ⌘-click open the PR the other way from the setting? | Yes |
| 2 | Keep **Fix with Armada** (dispatch a repair) when checks fail? It was not in the ask. | Keep |
| 3 | Approve and Request changes in the comment box, or Comment only? | Include them |
| 4 | Where does the setting live? | A new **Links** section in Settings |

## What exists today (starting points)

| Thing | Where |
|---|---|
| The panel today: `ReadingSheet`, a floating `Sheet` with `PullRequestCard`, `MergeLinePlace` and `PullRequestActs` | `apps/desktop/src/renderer/src/sessions.tsx` (~L447–540) |
| The card it renders | `packages/components/src/compositions/PullRequestCard/` |
| Merge / auto-merge / ready / review acts | `packages/components/src/compositions/PullRequestActs/` |
| Merge-queue place rows | `packages/components/src/compositions/MergeLine/`, `apps/desktop/src/renderer/src/merge-line.tsx` |
| Link opening in main, and its tests | `apps/desktop/src/main/links.ts`, `apps/desktop/src/main/forge.ts` |
| Surfaces that open a PR in the browser today | `packages/surfaces/jobs/src/` (`tab-overview`, `verdict`, `verdict-answered`, `LandBoard`, `heading`, `JobDetail`), `packages/surfaces/manifest/src/Kit.tsx`, `apps/desktop/src/renderer/src/walk-panel/WalkJob.tsx`, `App.tsx` |
| Settings values | `crates/config/settings.toml` |

## How to work it

1. Walk the mockup and the real app side by side (`preview-app` or
   `restart-app` skills), and settle the four decisions with the owner.
2. Find the Cockpit call-card auxiliary card primitive locally, and match it.
3. Then follow `work-issue`: worktree, an acceptance test written first
   (`docs/practices/acceptance-tests.md`), `armada-components` before touching
   `packages/components`, `armada-voice` for every string, and the code graph
   before and after editing (`docs/practices/code-graph.md`).
4. Seam work: the panel needs facts Fleet may not serve yet (queue position and
   length, ETA, comments, reviews, per-check durations, posting a comment).
   Check what Fleet serves before drawing a line for it. Each line is drawn
   only where its fact was served, as `PullRequestCard` already does.

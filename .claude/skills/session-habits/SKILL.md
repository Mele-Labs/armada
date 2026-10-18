---
name: session-habits
description: How the owner wants a job to work in this repository — walks, briefs to agents, load, scope. Load before showing him a walk, before briefing or dispatching an agent, and before offering follow-up work.
---

# Job habits

Each rule is something the owner corrected in a job. They lived in one
machine's memory until a second machine behaved differently; they are here so
every job on every machine reads the same ones. Design rules for screens
are not repeated: see `how-the-owner-decides`, `armada-voice` and
`docs/contracts/design-system.md`.

## Walks

- **A change he would see does not land until he has walked it** (1 Oct). Serve
  the mock, call `show_window` with the `?walk=` url. Never send screenshots
  (2 Oct: "Dont send me pictures. I prefer walks.").
- **Play the walk first**, with its own engine, before showing it: in
  `apps/desktop`, `npx vitest run src/renderer/src/mock/walks-{1..4}of4.test.tsx
  -t "<walk>"` (13 Oct: two broken walks sent in a row).
- **A walk plays inside the real Bridge shell**, against a mock Fleet scenario,
  never a stand-in window (6 Oct).
- **One mock per walk.** Stop the old port before serving a new one, and tell him
  to reload; a loaded tab keeps its old code (8 Oct).
- **Never switch a slot's branch, or stash its tree, while a walk is served from
  it** (11 Oct). Build a branch without checking it out instead.
- **While any walk on a branch is unapproved, no agent on that branch runs
  tests.** Writing them is fine, and so are `tsc` and `cargo check` (8 and 12 Oct).
- **A phone-sized UI is the real app on fixtures at a plain url** he narrows. No
  frame, captions or scripted walk (8 Oct).

## Asking and handing over

- **A decision is asked one way**: in the prompt or in `waiting_for`, never
  both (13 Oct).
- **A restart he has to run goes in `waiting_for` as a `run` item**,
  `scripts/preview --restart`. Bridge presses Refresh preview for it. A plain
  restart onto `main` is never an option (7 and 13 Oct).
- **Documentation a change made wrong is fixed with the change.** Do not ask
  (18 Sep).
- **Never ask him to delete a merged branch.** A hosted job calls
  `finish_up`; `agent-worktrees` has why (15 Oct).
- **A question he sends mid-turn is answered before the next tool call**, and
  a check or build runs with `run_in_background` so the turn ends and he can
  reach the job (15 Oct: asked twice during a five-minute foreground check).
- **Plumbing is yours to decide** (CI, manifests, layers, test layout). State
  the choice in a line. Ask about what he would see (6 Oct).

## Briefing agents

- **At most two children at once.** Each pushes its branch and stops; the parent
  lands it (2 Oct).
- **Lease a slot** for a child (7 Oct).
- **Never run or brief `rustfmt` or `cargo fmt`** (8 Oct).
- **A brief that adds user-facing strings** carries the voice check: facts only,
  no narration, no dashes, read against `armada-voice` and the `humanizer` rules
  before he walks it (2 and 4 Oct).
- **A brief for a screen** says: no aggregate counts, a state is a registry
  glyph with a tooltip, never an emoji, textareas pass a `keep` key (29 Sep,
  2 and 8 Oct). A styling pass is bold in weight, not volume: `PoolSlots` is
  the bar (4 Oct).
- **When an agent's report says it skipped a step a doc requires**, send it
  back before relaying the report (7 Oct).

## Load

**One machine carries every Check, agent, Fleet and Bridge** (2 and 8 Oct, twice
frozen).

- Never create load to reproduce a flaky test: no burners, no `dd` loops. One
  `--stress-count` run of that one test at `taskpolicy -c utility`.
- A brief names the tests the change adds or touches. Whole suites are CI's.
- Rust: `armada check test a b c` with every name in one call, or `--changed`
  with the paths on stdin. One call per name is a build and a slot wait each
  (17 Oct, load 20). Both are light only with an `armada` built after #2200. TypeScript: `pnpm exec
  vitest run <touched files>`; `--changed` is for Rust and hangs without stdin.

## Scope

- **An idea he floats is built by an agent, not filed** (1 Oct). Gaps found
  mid-work that nobody builds now are filed.
- **An agent's leftovers are offered once, as a list, defaulting to "file it".**
  Do not chain from a "found, not touched" list into another dispatch (2 Oct:
  a markdown note became 18,000 lines of cleanup).
- **"Why is X slow" gets the one structural cause first**, with evidence, after
  searching `docs/concepts/` and `docs/OPEN.md` for a decision on it (6 Oct).
- **"The Drone could not, so a person must" is a gap to build a heal for**, not
  a workaround to hand him (7 Oct).

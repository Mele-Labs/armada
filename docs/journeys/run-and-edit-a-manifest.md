# Journey 9 — Run and edit a Manifest

**What it is:** View a project's Manifest, run any single Check or Command against it on demand, and edit it.

Design fidelity: not set. Analysis: Complete. UI/UX design: In progress.

---

**Trigger:** "I want to run the lint here without starting a Job," or "this project's checks are wrong and I want to fix them."

**Concepts touched:** Bridge (Manifest — a new surface), Manifest, Fleet.

**Milestone:** Reach.

**Drawn against** `docs/contracts/configuration.md`.

## Why this journey did not exist

Most of this capability was already sanctioned and had no surface. The Manifest concept page states that Commands are invoked by both an agent and by you directly via Bridge, off the same named registry, and that your own manual invocation needs no second approval because you are the one triggering it. It also specifies **Update** as manual editing and **Verify** as drift detection plus a live dry-run.

**Running a single Check on demand is the one new capability.** Checks are specified as run by an agent, and the only ad-hoc path was Verify's live dry-run, which runs all of them. Running one is new capability, not a missing screen.

Manifest carries the `file-cog` glyph already assigned to it under Git and config. **It is reached by name from the palette, and from the title menu's repository entries where the row is narrow**, rather than from a tab or a strip page: the owner folded it into the picker on 28 Sep 2026, so the surface opens against whatever the picker has picked.

## Flow

Open Bridge → Manifest → read the current project's Checks and Commands → run one, or edit.

> **Rule.** One Manifest is open at a time, chosen by the repository picker at the top of the deck's left column.
> Why: the switcher is how the rest of Bridge scopes itself, and a second selector inside the surface would answer the same question twice.

A monorepo's root `armada.yml` appears as its own entry in the switcher, since its Checks run only when the diff touches root paths and it is not just another workspace.

> **Rule.** Checks and Commands render as two separate groups, never one list with a badge.
> Why: they are separate registries that differ in what they mean — a Check gates code, a Command does not — and one list would ask a person to learn a distinction the layout could just state.

## Running one

> **Rule.** A run from this surface executes in the working tree as it is on disk, and there is no Where control.
> Why: a throwaway copy is a worktree by another name, and the tree on disk is the one you are working in.

> **Rule.** A destructive Command confirms once, naming the command, before it runs.
> Why: the flag says nothing in a worktree that is about to be thrown away and everything in the tree you are working in. It is a confirmation and not an approval — you are the one who pressed it, which is what the Manifest concept page settles.

> **Rule.** After a run that changed the checkout, the panel lists the changed files and offers Open the diff and Undo this run. Undo confirms, naming what it would discard.
> Why: the snapshot restores the tree as it was just before the run, and this tree holds work of your own.

> **Rule.** Output goes to a panel on this surface — the result in status colours with its exit code, duration, and the output — and stays until dismissed.
> Why: a toast would throw away the thing you ran it to read.

### A server

A Command with `serve` stays running, in the main checkout and on ports from that checkout's span; it stops on Stop, exit, or Fleet stopping. [Manifest](../concepts/manifest.md) defines its keys and [Fleet](../concepts/fleet.md) holds it, one instance per checkout. A [Studio](../concepts/studio.md) starts one in the same checkout, on the same span and the same one instance, and holds it on a Run node.

| Moment | What the panel shows |
|---|---|
| Before `ready` passes | The row reads *starting*, with the log streaming |
| Serving | *serving*, how long it has been up, a button per link, and Stop |
| The panel closed | Nothing stops |
| It exits on its own | Its exit code and log, as a failure |

> **Rule.** A link hands its address to the system browser.
> Why: no surface in Bridge navigates, per the design system's hard rules.

### A passing Check leaves no verdict behind — but the output is a log

The output and the judgement are two different things.

Each run writes its output to `./.armada`, one run directory per run, so the panel is a view onto a file rather than a buffer that evaporates — Dismiss closes the panel, not the log.

> **Rule.** Ad-hoc run log retention is a Machine setting, default 30 days.
> Why: a run is only worth keeping as long as you might still be debugging what you just ran.

> **Rule.** A run a [Studio](../concepts/studio.md) holds keeps its result and its log's last lines on the Studio's own node before the sweep takes the run, and nothing else survives it.
> Why: a Studio is kept until a person deletes it, and this window is the only thing that expires a run. #1289.

> **Rule.** No Check gets a stored pass or fail from a run on this surface.
> Why: a remembered per-Check verdict would read as evidence the work was checked, and the tree can change between the run and any later work that leans on it.

The transcript is keepable, the judgement is not.

## Editing

> **Rule.** Editing shows forms by default — one section each for Checks and Commands, with the raw file behind a toggle. Always-allows are Fleet's own table, listed on the run view, and not a section of the file.
> Why: those are the parts of a Manifest with known shapes and enumerated values.

> **Rule.** The form expresses everything the schema has, so an edit made in the file always round-trips back into the form and nothing can be dropped.
> Why: the file view is there for people who would rather read the file than the form, not for what the form cannot reach.

The schema and the form move together — a field added to one is a field added to the other. The file stays reachable because hiding that a committed file is a file would hide what lands in the repo, not because the form falls short.

> **Rule.** A form changes only the lines it touches. Removing a Check, a Command or a port takes that entry's lines and the comment block directly above it — the section's own too, where it was the last — and every other byte of the file stays as it was.
> Why: a comment written directly against an entry describes it, and left behind it would describe something the file no longer holds; a comment set off by a blank line may be about the section or a neighbour, so it stays.

> **Rule.** The toggle is never labelled with the file's format; it is named by its path.
> Why: the lexicon bans naming a Manifest by its format — it is a Manifest, never "the yaml."

> **Rule.** A form's save goes out as a pull request and applies at once; the checkout is not written. Only where the file is tracked and not ignored, the repository has an `origin` and the Manifest names a base: anywhere else the form writes the file to disk, and the file view's Save always does, with no staging and no commit.
> Why: an edit left uncommitted in the checkout Fleet serves blocks every catch-up, because Fleet only fast-forwards a clean one (14 Oct 2026). Decided 15 Oct 2026, the owner's words: "PR, applies now, only in repos where the file is not gitignored."

A second form save while that pull request is open commits on top of its branch, so it stays one pull request. Until the checkout has its commit, or the forge reports it closed unmerged, the forms and the live keys read the pull request's text. Checks and Commands wait for a restart, as they do for any edit.

> **Rule.** Write always commits whatever the proposal has iterated to — there is no separate Write-path mode for a single workspace versus a monorepo batch.
> Why: iteration happens per workspace, at whatever pace suits its own evidence, so there is nothing left for a Write-path mode to choose between.

## Verify

Both halves belong on this surface: **drift detection** (re-scan and flag) and the **live dry-run** (run everything once).

> **Rule.** This is the surface where you look at what drifted and act on it, not Doctor.
> Why: Doctor's Manifest module only reports *that* a manifest has drifted, and Doctor is a scan that changes nothing, so the re-scan and acting on it cannot live there.

> **Rule.** Verify runs setup and every Check once, and reports what happened — nothing more.
> Why: a manifest that parses cleanly can still be wrong in every command it names, which is most likely right after it is written or edited.

> **Rule.** Verify and drift are two separate things, not two halves of one panel.
> Why: one panel with two verdict groups reads as an audit of the file rather than a rehearsal of it.

> **Rule.** Drift is a read — whether everything the file names still exists — and runs on opening, at no cost.

> **Rule.** Verify is an act — it runs the file, behind its own button, in the working tree.

> **Rule.** Each is reached from its own control on the surface rather than drawn on it, and the control carries that reading's headline — how many lines went, or how the last Verify ended.
> Why: drawn, the two took 45% of an 817px window to report that nothing had changed and that nobody had pressed Verify, and the Checks this surface exists to run started below them with room for four of nine. The headline on the control is what keeps tucked away different from dismissed. #1383.

Running Verify straight after Write, as [Set Up a Project (Manifest)](set-up-a-project-manifest.md) does, is correct: a brand-new file is exactly the case Verify exists for.

> **Rule.** Drift never reports a script the repo picked up that the file does not yet name.
> Why: the file carries no record of what it was written against, so nothing can call a script new or changed without storing a scan — existence alone needs no history.

> **Rule.** Drift says nothing about whether a listed command still does the right thing — a `test` script narrowed to one directory still reads as existing.
> Why: keeping a script and its Check in step is an engineering judgement, not something Armada checks for.

> **Rule.** Verify and drift say nothing about policy or permissions, because neither names anything runnable in the repo; the panel states that explicitly.
> Why: about a third of a mature Manifest names nothing runnable, and a clean result must not read as "all of this is still right" when that much carries no verdict.

> **Rule.** Drift's two verdicts are `gone` and `current`, answering *is this file still true* — a different question from the dry-run's *does this file still work*.

> **Rule.** The dry-run never runs automatically — not on opening the surface, and not as part of drift.
> Why: it runs real Checks and a real test suite, which is expensive, while drift is a free read that can run on opening.

> **Rule.** Verify reports and never fixes — no Apply, no Accept all — and acting on a row means going to Edit, where the consequence is stated.
> Why: the Manifest concept page specifies Verify as flagging without changing the file, with Update as the only manual edit path.

> **Rule.** A drifted `id` row offers no fix action at all.
> Why: `id` reads as drifted whenever a package is renamed, and changing it orphans everything that references this Manifest.

> **Rule.** Drift renders amber throughout, never red.
> Why: a drifted file is behind, not broken — the dry-run beside it is what would prove broken.

Verdicts render as words in the status colour, matching Doctor: `gone` and `current`.

> **Rule.** A dry-run failure never becomes a Doctor fail.
> Why: Doctor reports live, stateless service health, while a dry-run is a rehearsal chosen on one project — routing it into Doctor would store a verdict there and read one project's broken Check as a machine-level failure.

## Open questions

- **[runedit-no-manifest-state]** What does this surface show when no Manifest exists for the selected project?
  Setup's territory, and this journey meets Set Up a Project (Manifest) here without either page claiming ownership of the empty state.

## Related

- [Set Up a Project (Manifest)](set-up-a-project-manifest.md) — where a Manifest first comes to exist, and the surface this journey's empty state hands off to.

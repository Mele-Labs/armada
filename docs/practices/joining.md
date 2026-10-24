# Joining Armada's work

**Kind:** practice. **Governs:** getting access to Armada's backlog, putting it on a machine, and claiming an issue without two machines doing the same work.

The backlog is beads (`bd`), an issue tracker on Dolt.
Its data lives on Armada's own repository, `Mele-Labs/armada`, under the ref `refs/dolt/data`.
Each machine holds one copy, in the main checkout, and every git worktree of that checkout reaches it.
Issues filed before the move stay on GitHub, and `#123` still means the GitHub issue.

---

## Access

| To | You need |
|---|---|
| Read the backlog | Nothing. The repository is public |
| File, claim or close an issue | Push rights to `Mele-Labs/armada` |

**A write is a push to the repository.** A person without push rights can read and cannot change anything.

## Setup

```bash
scripts/beads-setup
```

It installs beads with Homebrew if `bd` is missing, turns beads' usage metrics off, and pulls the backlog from `origin` into the main checkout.
It sets `git config beads.role maintainer` and ends with `beads-setup: N issues on this machine`.

| | |
|---|---|
| Run it | Once on a new machine, and again whenever you doubt the setup. It can be run from any worktree |
| Needs | `git`, a network, and Homebrew only when `bd` is missing. No push rights |
| Changes | `.beads/` in the main checkout, and `.git/info/exclude` so git ignores it. Nothing in the tracked tree |
| Safe to repeat | A machine that already has a database prints `already has a beads database, leaving it as it is` and changes nothing |

**It never runs `bd init`.** Re-initialising can replace a database or the remote's history, and a second backlog started beside the first never joins it.
It stops with `holds no beads data` when `origin` has nothing under `refs/dolt/data`, and with `Homebrew is not available` when `bd` is missing and cannot be installed.

## Claiming

Two machines that claim the same issue at once are both told yes, and the sync that follows keeps the later claim on both.
**A claim is known only after claim, sync, read back.**

1. `bd update <id> --claim` takes the issue on this machine.
2. `bd sync` pushes the claim and pulls everyone else's.
3. `bd show <id>` names the holder. If it names you, start. If it names someone else, the issue is theirs.

Fleet runs the sync after a Job writes and every 30 seconds, and does the three steps for a Job that claims an issue.
**Another machine sees a change about a minute later, never in real time.** Each machine reads its own copy.

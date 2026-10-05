# Does freeing a worktree slot need the work pushed?

**Decided 2026-10-05.**

A release refused any commit on neither the remote nor the base, though `git switch --detach` leaves the branch where it is. He said: *"Freeing a worktree wont lose the work if its committed on a branch. We can choose to push that branch to the remote if we want but no one said dont commit the work. Just commit the work and free that worktree up for someone else to check out a branch in."*

**Chosen: a commit on the lease's branch is safe, and a push is optional.**

- **Uncommitted files still refuse a release.** Untracked and modified files do not survive the detach.
- **A park commits and frees.** It stages everything but ignored files onto the slot's branch under a `WIP:` message, releases, and never pushes.
- **A lease can take an existing branch at its tip**, so work parked on one continues where it stopped.
- **A dead holder's clean slot is taken back** when its commits are on its branch, since the next lease leaves that branch alone.

**Not changed:** `armada clean --force` deletes the branch after releasing, so it still refuses commits on neither the remote nor the base. A fresh lease still refuses to reset an existing branch holding such commits.

**Where it landed:** `fleet/pool-parks-on-a-branch`, phase 1 of pausing and resuming a Job. Nothing in Job state, the wire or Bridge changes in this phase.

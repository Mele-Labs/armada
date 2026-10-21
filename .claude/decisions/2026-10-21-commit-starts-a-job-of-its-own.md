# Where does a piece committed to the repository land?

**Decided 2026-10-21.** Walking Commit to repository, he found it added one commit on the branch of the Job named by the request, so a tile, workflow, review tool or `armada.yml` rode in that Job's pull request with unrelated work, and with no Job open it was refused.

**Chosen: Commit to repository starts a Job of its own.** Its own worktree and branch, one commit adding the piece, and a pull request for that alone, landed like any other. It works with no Job open.

| Asked | He chose | The cost he took |
|---|---|---|
| Which branch takes the commit | A new Job's own, cut from the base | One more Job for each piece committed |
| Which pull request carries it | One opened for that piece alone | One more pull request for each piece committed |
| A commit with no Job open | Works | None |
| How the pull request lands | As every Job's does: watched, merged and reported by Fleet | None |

**What it overrode.** The 19 Oct build committed on the open Job's branch and refused without one (`fleet.commit_no_job`), which is gone. Written into `docs/design/who-sees-each-piece-of-configuration.md`, *Commit to the repository is an option on a piece*.

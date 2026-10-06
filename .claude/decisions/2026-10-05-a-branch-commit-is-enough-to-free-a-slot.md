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

## Clear commits instead of refusing

**Decided 2026-10-05**, the same day, after two Clears on a killed Job's dirty slot failed with `fleet.not_reclaimable` and told him to run git, which Bridge cannot do (slot-2 with 5 files, slot-3 with 13). Both times the files were committed to the Job's branch by hand and the slot released, which worked.

**Chosen: Clear parks.** Where the pool refuses a Job's slot for uncommitted files, Clear commits them to the Job's branch as a WIP commit naming the Job, then releases the slot. A Job's own worktree outside the pool is committed the same way before `git worktree remove`, which would otherwise take the files with it. Nothing is pushed. The confirm says so before it is sent and the receipt after.

- **Refusals that stay:** a Job that has not ended, a locked worktree, a Job another is waiting on, and a park git cannot make (detached HEAD, the base branch, a branch other than the lease names, a busy slot), the last said in git words.
- **The sweep never parks.** It takes only what is provably safe, and a tree with uncommitted files is not.
- **Release on a session's slot** is the same act for a slot an agent session holds, sent for the holder shown, so a slot re-leased since is refused.

**Decided afterwards:** see the next section.

## A killed or failed Job is parked the moment it ends

**Decided 2026-10-05**, asked whether a killed or failed Job's dirty slot should be saved and released as it ends so it never sits `kept`: **yes, on a killed or failed Job.** Job 3 sat on slot-2 for 68 hours and Job 4 on slot-3 for the same reason.

**Chosen:** at `killed` and `completed_failed`, where the pool refuses the release for uncommitted files, Fleet makes the same park a Clear does, a WIP commit on the Job's branch and the slot freed, and writes one line in the Job's log naming the commit, the branch and the files. It is called from the end of the Job (`slot_at_the_end`) and from forgetting a Job's record.

- **Refusals that stay:** the same park refusals as a Clear. The slot stays `kept` and the log says why.
- **Not changed:** a `completed_success` Job holds its slot until a person clears it; `rejected` and `superseded` keep the pool's plain release; a Job that has not ended, and a paused one, are never parked. The sweep still never parks.
- **Startup recovery needs nothing:** `reconcile` never moves a Job to `killed` or `completed_failed`, it escalates, so no Job reaches either end without passing the call above.

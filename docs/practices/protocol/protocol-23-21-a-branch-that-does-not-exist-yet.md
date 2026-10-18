# Protocol 23.21: a branch that does not exist yet

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: a Job may start from a branch a person names on the canvas before it exists.

**One optional field, additive.** `LandingChoice` gains `start_point`, a branch the repository
holds. Where `from_ref` names a branch the repository does not hold, Fleet makes it at
`start_point`'s commit at the press, a local branch, and the worktree is cut from it as from any
`from_ref`. `target` may name the same new branch. **Read only then**: beside a `from_ref` the
repository holds, or none, it is not used. A 23.20 Bridge sends none and is refused an unknown
`from_ref` as before.

**Refused before anything is kept**: a `from_ref` the repository lacks with no `start_point`, and
a `start_point` it lacks, are 422 `fleet.no_such_branch` naming the branch. git refusing to make
the branch is a `fleet.fault` naming both. The branch is made before the proposal is written, so
an approval refused after it, by the machine, leaves the branch at its start point.

**`Vcs` gains `create_branch`**, which refuses a name already a branch. No store change: the
landing row keeps `from_ref` as before, and `start_point` is not kept. The event stream is
untouched.

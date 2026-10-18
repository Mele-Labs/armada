# Protocol 21.8: a merge refused because its Checks went red on the moved base

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`merge_pull_request` gains one refusal, `fleet.merge_gate_failed`, a 409 carrying `refused:
gate_failed`. Under `merge_by: push`, Fleet now answers a moved base by merging it into the Job's
branch and running the Job's Checks again before pushing, and this is that run going red. A
conflict on the way is the `fleet.merge_conflicted` that already existed, and `fleet.merge_base_moved`
now means the base kept moving through every round. No shape moves.

**Minor because a refusal code added is additive**, for 14.14's reason.

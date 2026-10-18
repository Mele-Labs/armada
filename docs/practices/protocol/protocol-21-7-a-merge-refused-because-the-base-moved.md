# Protocol 21.7: a merge refused because the base moved

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`merge_pull_request` gains one refusal, `fleet.merge_base_moved`, a 409 carrying `refused:
base_moved`. Only a repository whose Manifest says `merge_by: push` meets it: Fleet makes the
merge commit itself and refuses a branch that does not hold the base it would land on, which the
forge would have merged. No shape moves.

**Minor because a refusal code added is additive**, for 14.14's reason.

# Protocol 10.12: keeping a pull request current, and resolving its conflicts

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#663`. Fleet stopped closing and reopening a Job's pull request when main
moved under it — closing and reopening re-pinned the forge's comparison but
never touched the branch, so a pull request behind its base with conflicts
stayed that way. Fleet now rebases the branch and pushes it: clean, in place;
conflicted, the branch is left exactly as it was and a person is told.

`ipc::Currency`, additive on `PullRequestDetail`, names the base a branch was
last brought up to (`rebased_onto`, `rebased_at`) and, where the attempt
conflicted, the files (`conflict_files`) — absent is a branch that has never
needed to move. `resolve_pull_request_conflict` is a new route and `Commands`
method at the review gate: a person sends the branch back for a Drone that
can edit files to bring it current, on the step before the one that
delivers, never the gate's own — #660 found a picked pull-request comment
landing on a summarising step's Drone, which has no git and no code to edit.

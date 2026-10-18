# Protocol 11.2: a delivery that skipped its push says so

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#691`. A delivering step's catch-up can conflict with its base after the
step has already made its commit, and `fleet::delivery::deliver` was right to
leave the branch unpushed there — a pull request opened over a conflict is a
review request nobody can act on. What it did not do was say so: the commit
still landed, the Job's log carried nothing about the push it skipped, and a
redelivery onto a pull request that already existed left that pull request
showing the commit before it, silently, all the way to a person approving
work it did not contain.

`ipc::JobDelivery.unpushed`, additive, names why the last commit here never
reached its remote — absent is a push that went out, a repository with no
remote, or a Job that has not reached a delivering step; present is the one
fact the field exists for. Fleet's own record no longer clears `pushed` and
`pull_request` on a skipped push either: nothing about the remote changed
that turn, so the store keeps naming the pull request `resolve_pull_request_conflict`
already knows how to send a Drone back to.

A delivering step whose own catch-up conflicted can no longer carry a Job to
`completed_success` silently: `Ruling::Finished` on such a step is held for a
person instead, and an approval or an override reaching the Job's own ending
while the last commit is unpushed is refused rather than completing over a
pull request that does not carry it.

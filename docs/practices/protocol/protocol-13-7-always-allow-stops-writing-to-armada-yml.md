# Protocol 13.7: Always allow stops writing to `armada.yml`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#836`. Fleet's own commit of the always-allowed line was itself a change on
the Job's branch, so the absolute boundary on `armada.yml`
(`crates/verification/src/forbidden.rs (deleted)`) refused every later step of the Job
the allow was pressed on. Always allow now commits nothing: the rule is kept
in a table of its own, per Manifest, and granted to every Job against it the
way a declared, non-destructive Command already is.

`get_repository_allowed_commands` and `remove_repository_allowed_command` are
new routes, so the minor moves. `ipc::JobDetail` gains
`repository_allowed_commands`, additive for the same reason — every rule a
person always-allowed for the Manifest, read-only there, beside the Job's own
`allowed_commands`. **`allowed_commands` itself changes what it means, not its
shape**: a `repository`-reach row there is now one an older Fleet wrote before
13.7, kept rather than migrated, and never one this Fleet writes going
forward — an old Bridge reading it as before still reads a real historical
row, so nothing about the field's shape or presence changed under it.

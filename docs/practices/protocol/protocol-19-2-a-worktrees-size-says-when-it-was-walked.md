# Protocol 19.2: a worktree's size says when it was walked

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

One optional field added to `WorktreeOnDisk` — `measured_at`, the instant the `du` that found
`bytes` ran. Additive: a Bridge built before this ignores it, and draws the size under the
reading's age as it always did.

**Fleet now keeps a worktree's size for 30 s** rather than walking it on every read, so the size
can be up to that much older than `read_at`. `read_at` used to say every figure was as of it; it
now says every figure but this one, and Pulse draws the size's own age under it.

**Present exactly where `bytes` is.** A walk that did not finish measured nothing, so there is no
instant to put on it; an absent `measured_at` beside a present `bytes` is a Fleet older than 19.2.

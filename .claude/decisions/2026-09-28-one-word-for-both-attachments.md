# A Job's attachment is frozen. A task's is not. One word or two?

**Decided 2026-09-28.**

A handoff decided that a plan, a group and a task can each carry an
**Attachment**, reasoning that "Artifact" was taken by a task's `expects`. The
word it picked is taken too, and built: `[fields.Attachments]` at
`crates/core-model/domain/job-fields.toml:106`, table `job_attachments` at
`crates/store/src/schema.rs:426`, and an `AttachmentChip` primitive. A Job's
attachment is frozen when the Job is created and copied into the worktree, and
there is deliberately no method that adds one afterwards.

Offered a second word, or unfreezing the Job's so one word means one behaviour:

**Chosen:** One word for both.

**Cost he took:** The word stops predicting whether the thing can still change.
A person reads *Attachment* on a dispatch form and on a running task and gets
two lifetimes under one label.

**Why it is in character:** the alternative was minting a word for a shape that
already has one, which he has refused twice —
[[2026-09-22-no-kind-names]] is the same instinct. The difference between the
two is *when it was added*, not what it is.

**Where it landed:** the workbook/region/attachments handoff, not yet filed.

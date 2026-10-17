//! Reading the machine for one Job: the processes it holds, what they are
//! burning, the disk its worktree has taken, and when anything last moved.
//!
//! **Read on demand and never on the turn loop.** Fleet turns every 250ms, and
//! a process table plus a directory walk per Job per turn is a cost paid
//! constantly to answer a question asked rarely. [`crate::footprint`] pays a
//! throttled version of that bill because a live file list is what a person
//! watches; nobody watches a memory figure change.
//!
//! **What counts as this Job's is written down here**, because otherwise the
//! figure has no referent. It is the process Fleet recorded at the spawn and
//! everything descended from it — so a Drone's own session counts, and so does
//! a build it started. A Check that Fleet ran, and a preparation command Fleet
//! ran, are Fleet's children rather than the Drone's and are **not** counted.
//! `Held::None` is the honest answer during those spans, and it is a stated
//! answer rather than an empty list.
//!
//! **The base checkout is not this Job's disk**, by that same rule. One
//! checkout serves every Job on a commit, so charging it to whichever Job was
//! asked about would report a figure four Jobs each claim in full — and killing
//! that Job would free none of it. `crate::basing` gives it back.
//!
//! **Both readings are bounded and both shell out**, which is
//! [`crate::headroom`]'s precedent and its argument: `ps` and `du` are one
//! spelling on darwin and Linux, need no `unsafe` and no platform crate.

/// A Job's log files, and which of them a writer holds.
pub(crate) mod logs;


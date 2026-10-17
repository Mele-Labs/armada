//! The review Fleet writes — one composition, two readers: the pull request's
//! Markdown body, and `get_job`'s `ipc::JobReview` for the review area, at a
//! gate that never opens one too. `#665`.
//!
//! # Nothing a Drone wrote reaches this
//!
//! A Drone's claim is a signal the gate ruled on; the record is what Fleet
//! verified. Every line below comes from a Job's own fields, its step rows and
//! its Check rows — no parameter here can carry a Drone's sentence, which is
//! `landing`'s commit message's shape and for its reason.
//!
//! # The four headings, and the one Fleet fills differently
//!
//! `docs/contracts/agent-copy.md` fixes the shape of every pull request body
//! that leaves Armada — why, outcome, risks, evidence — and it is a Drone's
//! contract. This is Fleet's body and it keeps the same four, so a reviewer
//! reading a queue finds the same section in the same place whoever wrote it.
//!
//! **Outcome is the one Fleet cannot write in a Drone's terms**: it says what
//! the record proves rather than what the change does, because admitting a
//! Drone's sentence is what this file exists to prevent.
//!
//! Risks come before evidence: a body ending on its passes reads as a claim
//! that everything is fine.
//!
//! # Counted, not listed
//!
//! **The review names no file and no Check that passed or was skipped**: lists
//! that long buried the one line that needed a reviewer (#2161). The forge has
//! the files, and Bridge draws every Check from the Job's steps.


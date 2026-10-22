//! The fake harness, and the fixtures that drive it.
//!
//! `testkit` exists because v1 shipped 586 commits, 2,181 passing tests and a
//! working screen, and the Jobs it completed did not do what they claimed. A
//! fake emitting only happy-path output reproduces exactly that: a fast green
//! suite proving nothing about the part that failed. So the fixtures are
//! weighted towards misbehaviour, and each is a specification of what a
//! misbehaving Drone's output stream really looks like, written before the
//! detector that catches it. They live in `crates/testkit/fixtures/ndjson/`,
//! one file per failure mode, beside the pinned real capture the format-drift
//! contract test compares against, and
//! `docs/contracts/testkit-fixtures.md` is the roster.
//!
//! **A fake stands in for a seam, never for the thing being asserted.**
//! [`FakeHarness`] reads scripted lines; whether a line of a vendor's stream
//! becomes the right event is `adapters`', against the real decoder. [`FakeJudge`] renders
//! a shell printing a scripted verdict, so a Judge call exercises Fleet's
//! runner and `verification`'s parser with no model, network or credential.
//! [`FakeVcs`] and [`FakeWorkProduct`] stand in for a repository, because a
//! suite needing a real checkout per case is one people stop running.
//!
//! [`resolved`] fakes nothing: the parsing is real, and it is here because a
//! workflow is what every gate test needs and not a thing a test can write out
//! in a line.

mod bucket;
mod harness;
mod judge;
mod link_lookup;
mod sign_in;
mod store;
mod test_map;
mod tracker;
mod trackers;
mod vcs;
mod work_product;
mod workflow;

pub use bucket::FakeBucket;
pub use harness::{FakeHarness, EDITS, READS, REACHES, REPLAYED, RUNS};
pub use judge::{refusal, FakeJudge};
pub use link_lookup::FakeLinkLookup;
pub use sign_in::{FakeSignIn, USER_CODE, VERIFICATION_URI};
pub use store::FakeStore;
pub use test_map::FakeCollector;
pub use tracker::FakeTracker;
pub use trackers::FakeTrackers;
pub use vcs::{CommitScope, Delivered, Delivering, FakeCommit, FakeVcs, FakeVcsError, Merging};
pub use work_product::{FakeDiffRefused, FakeWorkProduct, Holding, Written};
pub use workflow::{
    delivering, frozen, handing_off, modelled, narrowing, narrowing_and_testing_one, panelled,
    patient, requiring, resolved, retried, retried_and_testing_one, testing_one, Gate,
    Narrows, OneTest, Patience, Scoped, Sketch,
};

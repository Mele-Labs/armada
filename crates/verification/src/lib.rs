//! The two verification tiers, and the gate between them.
//!
//! Mechanical Checks are pure functions over facts that already exist — an exit
//! code, a file's presence, whether a diff is empty. **Armada does not parse:**
//! which lines of a test run were the failure is a Judge's question, answered by
//! reading the diff, never a runner's output. The Judge is a veto and not a
//! grant — it fires on mechanical triggers, is blind to how the step went, and
//! [`Verdict::but_for`] has no arm producing an advance.
//!
//! **A Drone claiming completion in prose advances nothing, and there is no
//! path from text to a transition.** Not a check anywhere here — the types:
//!
//! | To reach | You need | Which you can only get from |
//! |---|---|---|
//! | [`Verdict::Advance`] | [`decide`] | An [`Accepted`] and a [`Ran`] |
//! | [`Accepted`] | [`Accepted::of`] | A [`Submission`] |
//! | [`Submission`] | [`Submission::submitted`] | The fields of the tool call |
//! | [`Ran`] | [`Ran::of`] | One observation per check the step declared |
//!
//! Nothing in that chain accepts a message, a turn, a transcript or a claim.
//! [`submission`](mod@submission) holds the fields that do not exist, and
//! [`product`](mod@product) the one step type whose writing *is* the
//! deliverable. **The raw diff-computation adapter method is exposed only
//! here**, because two places deciding whether files changed outside their
//! declared scope is two answers — [`scope`](mod@scope) is the one place.

mod answered;
mod commented;
mod converging;
mod drift;
mod forbidden;
mod gaming;
mod gate;
mod judge;
mod located;
mod mechanical;
mod outcome;
mod product;
mod quoted;
mod request;
mod review;
mod scanned;
mod scope;
mod second_opinion;
mod shown;
mod standing;
mod submission;
mod widening;
mod wording;

#[cfg(test)]
mod tests;

pub use answered::{Answered, Printed};
pub use converging::{Convergence, ConvergenceBrief, NotConverging};
pub use core_model::{
    Area, Bucket, ChangedTest, Confidence, Finding, Proves, TestChange, TestsInChange, Untested,
};
pub use drift::{drift_criterion, DECLARED_PLAN_DRIFT};
pub use forbidden::{forbidden, forbidden_among, out_of_bounds, Forbidden};
pub use gaming::{judged_patterns, Baseline, Flagged, GamingBrief};
pub use gate::{decide, Accepted, NotWhatTheStepAsked, Verdict};
pub use judge::{field, Brief, Refusals, Unreadable};
pub use mechanical::{
    how, never_ran, Artifact, CheckFailed, ChecksOutstanding, Exit, NeverRan, Observed, Ran,
    EVIDENCE_SCOPE, HELD_OFF, OUT_OF_BOUNDS,
};
pub use outcome::{OutcomeTurn, TheBaseMoved, Verified};
pub use product::{
    Delivered, NothingToJudge, Product, Reference, TooBigToJudge, Written, A_DELIVERABLE,
};
pub use request::Request;
pub use review::{AcceptedReview, Review, ReviewRefused};
pub use scanned::in_the_diff;
pub use scope::{drifted, InScope, Lifted, OutsideScope};
pub use second_opinion::SecondOpinion;
pub use shown::{digest, digest_of};
pub use standing::{Standing, STANDING_RULES};
pub use submission::{Claimed, NotASubmission, NotClaimed, ShownBy, Submission};
pub use widening::{NotWidened, Widened, WideningBrief};
pub use wording::{fill, Piece, Wording};

/// Every authored piece of a Judge brief, by name, for a caller that hands in
/// its own words for one. What each says is beside the code that lays it.
pub mod pieces {
    pub use crate::answered::{JUDGE_CHECKS_RAN, JUDGE_PRINTED, JUDGE_PRINTED_TAIL};
    pub use crate::converging::{
        JUDGE_CONVERGE_ALONGSIDE, JUDGE_CONVERGE_ANSWER, JUDGE_CONVERGE_CITES_DIFF,
        JUDGE_CONVERGE_CITES_FILE, JUDGE_CONVERGE_DECLARED, JUDGE_CONVERGE_DIFF,
        JUDGE_CONVERGE_LAST_TIME, JUDGE_CONVERGE_LAST_TIME_RULE, JUDGE_CONVERGE_NOTHING_ELSE,
        JUDGE_CONVERGE_NOTHING_WRITTEN, JUDGE_CONVERGE_OFF_PLAN, JUDGE_CONVERGE_OPENING,
        JUDGE_CONVERGE_QUESTION, JUDGE_CONVERGE_TOO_BIG, JUDGE_CONVERGE_WRITTEN,
    };
    pub use crate::gaming::{
        JUDGE_GAMING_ANSWER, JUDGE_GAMING_BASELINE, JUDGE_GAMING_DIFF, JUDGE_GAMING_HOW_TO_READ,
        JUDGE_GAMING_NO_BASELINE, JUDGE_GAMING_OPENING, JUDGE_GAMING_QUESTION,
        JUDGE_PATTERN_ASSERTION_WEAKENED, JUDGE_PATTERN_FINDINGS_GENERIC,
        JUDGE_PATTERN_FINDINGS_NOT_TIED, JUDGE_PATTERN_NO_FINDINGS,
        JUDGE_PATTERN_TAUTOLOGICAL_TEST, JUDGE_PATTERN_TEST_SCOPE_NARROWED,
    };
    pub use crate::judge::{JUDGE_ANSWER, JUDGE_OPENING, JUDGE_QUESTION};
    pub use crate::outcome::{
        BASE_CONFLICTED, BASE_NOT_MOVED, BASE_UP_TO_DATE, OUTCOME_ACCEPTED, OUTCOME_GO_ON,
        OUTCOME_HANDED_BACK, OUTCOME_LAST, OUTCOME_VERIFIED_ALL, OUTCOME_VERIFIED_NONE_COVERED,
        OUTCOME_VERIFIED_SOME, OUTCOME_WORK_AGAIN,
    };
    pub use crate::product::{
        JUDGE_ALSO_CHANGED, JUDGE_DELIVERABLE, JUDGE_DIFF, JUDGE_PLAN, JUDGE_REFERENCES,
        JUDGE_SUMMARY, JUDGE_SUMMARY_AFTER,
    };
    pub use crate::request::{JUDGE_REQUEST, JUDGE_REQUEST_DONE_WHEN};
    pub use crate::second_opinion::{
        JUDGE_SECOND_ANSWER, JUDGE_SECOND_AT_LINE, JUDGE_SECOND_CITED, JUDGE_SECOND_HOW_TO_WEIGH,
        JUDGE_SECOND_OPENING, JUDGE_SECOND_QUESTION, JUDGE_SECOND_REMOVED_LINE,
    };
    pub use crate::standing::{
        JUDGE_READABLE, JUDGE_STANDING, JUDGE_STANDING_CUT, JUDGE_STANDING_UNREADABLE,
    };
    pub use crate::widening::{
        JUDGE_WIDEN_ANSWER, JUDGE_WIDEN_ASKED, JUDGE_WIDEN_DELIVERABLE, JUDGE_WIDEN_FENCED,
        JUDGE_WIDEN_HELD, JUDGE_WIDEN_OPENING, JUDGE_WIDEN_QUESTION, JUDGE_WIDEN_STEP,
        JUDGE_WIDEN_WHY,
    };
}

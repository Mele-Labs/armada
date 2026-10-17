//! Filing what a person knows went wrong, over the router that ships.
//!
//! **Every case starts from a real refusal**, for `overruling`'s reason: a Job
//! moved into the state by hand would hide whether a report can be filed from
//! where a wrong verdict actually leaves one. The Judge is asked, it refuses on
//! a criterion, the Job escalates, and the person disagrees — which is the case
//! this whole operation was written for, twice over, in one week.
//!
//! The five claims here are the five that decide whether this is worth having:
//! the record carries what the Job left behind, a report with no sentence is
//! not a report, a report outlives the Job it is about, nothing credential-
//! shaped and no home directory reaches the file, and **the count reads the
//! claim rather than the sentence** — which is what makes it survive a reason
//! that says `probe`.
//!
//! And three about the scope, which the first pass of this got wrong in both
//! directions: a step with no criterion is a scope rather than a refusal, a
//! criterion with no step is still a refusal, and each refusal names the field
//! it is about rather than sharing one sentence written for a different act.

use axum::http::StatusCode;
use ipc::RunId;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

/// What the button sends: the claim, the sentence, and the criterion whose
/// verdict is disputed.
fn a_filing(said: &str) -> String {
    format!(
        r#"{{"claim":"wrongly_refused","said":{said},"step_id":"implement","criterion_id":"c1"}}"#,
        said = quoted(said)
    )
}

/// A JSON string, without reaching for a serialiser this crate is not allowed
/// to have: reading and writing JSON is `store`'s and `ipc`'s, and a fixture
/// that needed one would be a test earning the crate a dependency.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A report about a Job Fleet does not hold is a 404, not a report about
/// nothing. The one thing filed on a Job has to be about that Job.
#[tokio::test]
async fn a_report_about_a_job_that_does_not_exist_is_refused() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(
        &app,
        "POST",
        "/jobs/01NOSUCHJOB/report",
        &a_filing("this job never existed"),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

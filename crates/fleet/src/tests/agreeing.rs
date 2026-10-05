//! A person's answer to a retro item, end to end over the router: agreeing
//! proposes a Job at the approval gate or keeps a Kit item, disagreeing keeps
//! the row, and the listing narrows by state. `docs/concepts/retro.md`.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::Router;
use ipc::{JobList, JobSummary, Lesson, Lessons};
use testkit::FakeJudge;

use super::retro::{killed, running, sent, From};
use crate::tests::tmp::TempDir;

/// A retro of three items, one for each place a fix lands, and the answer the
/// Job proposer gives to a request that carries one of them.
fn judge() -> Arc<FakeJudge> {
    Arc::new(FakeJudge::answering(&[
        (
            "-----BEGIN RECORD-----",
            "{\"items\":[\
             {\"who\":\"fleet\",\"lands_in\":\"manifest\",\"title\":\"Browser tests wait a fixed time\",\
              \"what\":\"desktop_test timed out after 15 seconds on three attempts.\",\
              \"fix\":\"Raise the timeout in the desktop test config.\",\"evidence\":[\"act:1\"]},\
             {\"who\":\"drone\",\"lands_in\":\"kit\",\"title\":\"The Drone was refused grep\",\
              \"what\":\"The plan Drone asked for grep and a person had to allow it.\",\
              \"fix\":\"Add grep to the default allowlist.\",\"evidence\":[\"act:1\"]},\
             {\"who\":\"fleet\",\"lands_in\":\"armada\",\"title\":\"Gate measured a stale main\",\
              \"what\":\"Two upstream commits counted as the Drone's work.\",\
              \"fix\":\"Fetch main before the gate measures.\",\"evidence\":[\"act:1\"]}]}",
        ),
        (
            "From the retro of Job",
            "workflow: bug\ntitle: Raise the desktop test timeout\n\
             because: a defect with a reproducible symptom\nwrites: apps/desktop/vitest.config.ts",
        ),
    ]))
}

/// A Fleet with an ended Job and its retro written, and the three items' ids in
/// the order they were written: manifest, kit, armada.
async fn retro_written(
    home: &TempDir,
) -> (
    Arc<super::retro::Fixture>,
    Router,
    Arc<FakeJudge>,
    Vec<String>,
) {
    let judge = judge();
    let (fleet, app, id) = running(home, Arc::clone(&judge)).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");
    let ids = (0..3).map(|place| format!("{id}-{place}")).collect();
    (fleet, app, judge, ids)
}

async fn answered(app: &Router, id: &str, act: &str) -> (StatusCode, Vec<u8>) {
    sent(
        app,
        "POST",
        &format!("/lessons/{id}/{act}"),
        "",
        From::Bridge,
    )
    .await
}

fn lesson(body: &[u8]) -> Lesson {
    ipc::decode("a lesson", body).expect("a Lesson")
}

async fn listed(app: &Router, query: &str) -> Vec<Lesson> {
    let (status, body) = sent(app, "GET", &format!("/lessons{query}"), "", From::Bridge).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    ipc::decode::<Lessons>("the lessons", &body)
        .expect("Lessons")
        .lessons
}

async fn jobs(app: &Router) -> Vec<JobSummary> {
    let (_, body) = sent(app, "GET", "/jobs", "", From::Bridge).await;
    ipc::decode::<JobList>("the jobs", &body)
        .expect("a JobList")
        .jobs
}

async fn job_count(app: &Router) -> usize {
    jobs(app).await.len()
}

/// **Agreeing with a manifest item proposes exactly one Job**, at the approval
/// gate, with the item as its request, and keeps which Job on the item.
#[tokio::test]
async fn agreeing_with_a_manifest_item_proposes_one_job_with_the_item_as_its_request() {
    let home = TempDir::new();
    let (_fleet, app, judge, ids) = retro_written(&home).await;
    let before = job_count(&app).await;

    let (status, body) = answered(&app, &ids[0], "agree").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let agreed = lesson(&body);
    assert_eq!(agreed.state.as_wire(), "agreed");
    let proposed = agreed.job_proposed.expect("the Job proposed for it");
    assert_eq!(job_count(&app).await, before + 1, "exactly one Job");
    let job = jobs(&app)
        .await
        .into_iter()
        .find(|job| job.id == proposed)
        .expect("the Job is on the Board");
    assert_eq!(
        job.status.as_wire(),
        "awaiting_approval",
        "a person approves it"
    );
    let requests: Vec<String> = judge
        .asked()
        .into_iter()
        .filter(|question| question.contains("From the retro of Job"))
        .collect();
    let [request] = requests.as_slice() else {
        panic!("one proposer call: {requests:?}");
    };
    for carried in [
        "Browser tests wait a fixed time",
        "desktop_test timed out after 15 seconds on three attempts.",
        "Fix: Raise the timeout in the desktop test config.",
        &format!("From the retro of Job {}", agreed.handle),
    ] {
        assert!(request.contains(carried), "{carried}: {request}");
    }
}

/// **Agreeing with a Kit item proposes no Job**: it is kept, `accepted`.
#[tokio::test]
async fn agreeing_with_a_kit_item_keeps_it_as_accepted_and_proposes_nothing() {
    let home = TempDir::new();
    let (_fleet, app, judge, ids) = retro_written(&home).await;
    let before = job_count(&app).await;

    let (status, body) = answered(&app, &ids[1], "agree").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let kept = lesson(&body);
    assert_eq!(kept.state.as_wire(), "accepted");
    assert_eq!(kept.job_proposed, None);
    assert_eq!(job_count(&app).await, before, "no Job");
    assert!(
        !judge
            .asked()
            .iter()
            .any(|question| question.contains("From the retro of Job")),
        "the proposer was never asked"
    );
}

/// **Agreeing twice makes one Job**: the second press answers with where the
/// item stands.
#[tokio::test]
async fn agreeing_twice_proposes_one_job() {
    let home = TempDir::new();
    let (_fleet, app, judge, ids) = retro_written(&home).await;
    let before = job_count(&app).await;

    let (_, first) = answered(&app, &ids[0], "agree").await;
    let (status, second) = answered(&app, &ids[0], "agree").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(lesson(&first), lesson(&second), "the standing state, again");
    assert_eq!(job_count(&app).await, before + 1);
    let asked = judge
        .asked()
        .iter()
        .filter(|question| question.contains("From the retro of Job"))
        .count();
    assert_eq!(asked, 1, "one proposer call");
}

/// **Disagreeing keeps the row as `discarded`**, and the default listing, which
/// is `open`, omits it.
#[tokio::test]
async fn disagreeing_discards_the_item_and_the_default_listing_omits_it() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    assert_eq!(listed(&app, "").await.len(), 3);

    let (status, body) = answered(&app, &ids[2], "disagree").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(lesson(&body).state.as_wire(), "discarded");
    let open: Vec<String> = listed(&app, "").await.into_iter().map(|l| l.id).collect();
    assert_eq!(open, vec![ids[0].clone(), ids[1].clone()]);
    let discarded: Vec<String> = listed(&app, "?state=discarded")
        .await
        .into_iter()
        .map(|l| l.id)
        .collect();
    assert_eq!(discarded, vec![ids[2].clone()], "the row stays");
}

/// **`?state=accepted` is a person's saved Kit items**, and only those: a
/// manifest item agreed and a Kit item left open are not in it.
#[tokio::test]
async fn the_accepted_listing_holds_only_the_kit_items_agreed() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    answered(&app, &ids[0], "agree").await;
    answered(&app, &ids[1], "agree").await;

    let accepted = listed(&app, "?state=accepted").await;

    let [kept] = accepted.as_slice() else {
        panic!("one item: {accepted:?}");
    };
    assert_eq!(kept.id, ids[1]);
    assert_eq!(kept.lands_in.map(|place| place.as_wire()), Some("kit"));
    let agreed = listed(&app, "?state=agreed").await;
    assert_eq!(agreed.len(), 1, "the manifest item is agreed, not accepted");
    let (status, _) = sent(&app, "GET", "/lessons?state=maybe", "", From::Bridge).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a state that is not one");
}

/// **A fix in Armada is refused where this Fleet does not serve Armada's own
/// repository**, with its own code, and the item stays open with nothing
/// proposed.
#[tokio::test]
async fn agreeing_with_an_armada_item_is_refused_where_armada_is_not_served() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    let before = job_count(&app).await;

    let (status, body) = answered(&app, &ids[2], "agree").await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        String::from_utf8_lossy(&body).contains("fleet.lesson_armada_not_served"),
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(job_count(&app).await, before);
    let open: Vec<String> = listed(&app, "").await.into_iter().map(|l| l.id).collect();
    assert!(open.contains(&ids[2]), "still open");
}

/// An id that names no item is a 404 with its own code, and so is a malformed
/// one.
#[tokio::test]
async fn an_id_that_names_no_item_is_not_found() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    let missing = format!("{}9", ids[0]);

    for act in ["agree", "disagree"] {
        for id in [missing.as_str(), "nothing"] {
            let (status, body) = answered(&app, id, act).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{act} {id}");
            assert!(String::from_utf8_lossy(&body).contains("fleet.no_such_lesson"));
        }
    }
}

/// **`GET /jobs/:id/retro` carries each item's answer**, as the Lessons list
/// does: the Job proposed on the one agreed, `discarded` on the one disagreed
/// with, and `open` on the one left alone.
#[tokio::test]
async fn the_job_retro_carries_each_items_answer() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    let (_, agreed) = answered(&app, &ids[0], "agree").await;
    let proposed = lesson(&agreed).job_proposed.expect("a Job proposed");
    answered(&app, &ids[2], "disagree").await;
    let job = ids[0].rsplit_once('-').expect("an id").0;

    let (status, body) = sent(&app, "GET", &format!("/jobs/{job}/retro"), "", From::Bridge).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let retro: ipc::JobRetro = ipc::decode("a retro", &body).expect("a JobRetro");
    let states: Vec<(&str, Option<ipc::JobId>)> = retro
        .items
        .iter()
        .map(|item| {
            (
                item.state.expect("an answer state").as_wire(),
                item.job_proposed.clone(),
            )
        })
        .collect();
    assert_eq!(
        states,
        vec![
            ("agreed", Some(proposed)),
            ("open", None),
            ("discarded", None)
        ]
    );
}

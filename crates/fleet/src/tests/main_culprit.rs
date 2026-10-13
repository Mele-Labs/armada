//! Who turned main red: the Session that held the merging pull request when no Job owns it, and
//! the first commit of a red that two runs of one commit named twice.

use adapter_traits::{CiState, FromOutside, MergedPull, RecentlyMerged};
use store::{AttachmentState, Holder, KeptAttachment, KeptSession, SessionFigures, SessionState};

use super::main_ci::{a_fleet_over, kept, run, the_hub, Fixture, MANIFEST, ONE, TWO};
use crate::tests::tmp::TempDir;

const SESSION: &str = "c279e883-d316-4202-8134-76ea8b531835";
const BRANCH: &str = "armada/session-c279e883";

fn pull(number: u64, branch: &str) -> MergedPull {
    MergedPull {
        number,
        url: None,
        branch: Some(FromOutside::verbatim(branch)),
    }
}

fn listed(number: u64, commit: &str, at: &str) -> RecentlyMerged {
    RecentlyMerged {
        number,
        title: FromOutside::verbatim(format!("Change {number}")),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        merged_at: FromOutside::verbatim(at),
        commit: Some(FromOutside::verbatim(commit)),
    }
}

fn manifest_of(fleet: &Fixture) -> String {
    fleet.repositories().served()[0]
        .manifest()
        .id()
        .as_str()
        .to_string()
}

async fn a_session(fleet: &Fixture, id: &str, title: Option<&str>) {
    let now = fleet.now().as_str().to_string();
    fleet
        .store()
        .lock()
        .await
        .keep_session(&KeptSession {
            id: id.to_string(),
            harness: "claude".to_string(),
            origin: "bridge".to_string(),
            manifest_id: Some(manifest_of(fleet)),
            cwd: "/slot".to_string(),
            title: title.map(str::to_string),
            state: SessionState::Ended,
            started_at: now.clone(),
            last_seen_at: now,
            last_turn_at: None,
            ended_at: None,
            end_reason: None,
            figures: SessionFigures::default(),
            mod_version: None,
        })
        .unwrap();
}

async fn holds(fleet: &Fixture, id: &str, kind: &str, target: &str, state: AttachmentState) {
    let at = fleet.now().as_str().to_string();
    fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id),
                kind: kind.to_string(),
                manifest_id: manifest_of(fleet),
                target: target.to_string(),
                state,
                detail: Default::default(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .unwrap();
}

#[tokio::test]
async fn a_pull_request_no_job_owns_is_traced_to_the_session_that_held_its_branch() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    a_session(&fleet, SESSION, Some("Session Messages Fail to Send")).await;
    // Given back, not standing: the Session finished, and it still made the pull request.
    holds(&fleet, SESSION, "branch", BRANCH, AttachmentState::GivenBack).await;
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.merged_by(ONE, pull(2175, BRANCH));

    fleet.turn().await.unwrap();
    let merge = kept(&fleet).await.unwrap().merge.unwrap();
    assert_eq!((merge.number, merge.job), (2175, None));
    assert_eq!(merge.session.as_deref(), Some(SESSION));
    let named = the_hub(&fleet)
        .await
        .main
        .unwrap()
        .merge
        .unwrap()
        .session
        .unwrap();
    assert_eq!(named.id, SESSION);
    assert_eq!(named.title.as_deref(), Some("Session Messages Fail to Send"));
}

#[tokio::test]
async fn a_session_that_held_the_pull_request_itself_is_named_before_one_that_held_the_branch() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    a_session(&fleet, SESSION, None).await;
    a_session(&fleet, "other", None).await;
    holds(&fleet, "other", "branch", BRANCH, AttachmentState::Standing).await;
    holds(&fleet, SESSION, "pr", "2175", AttachmentState::Spent).await;
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.merged_by(ONE, pull(2175, BRANCH));

    fleet.turn().await.unwrap();
    assert_eq!(
        kept(&fleet).await.unwrap().merge.unwrap().session.as_deref(),
        Some(SESSION)
    );
    let named = the_hub(&fleet)
        .await
        .main
        .unwrap()
        .merge
        .unwrap()
        .session
        .unwrap();
    assert_eq!(named.title, None, "a Session with no title is named by id alone");
}

#[tokio::test]
async fn a_pull_request_nobody_held_names_no_session() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    a_session(&fleet, SESSION, None).await;
    holds(&fleet, SESSION, "branch", "armada/another", AttachmentState::Standing).await;
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.merged_by(ONE, pull(2175, BRANCH));

    fleet.turn().await.unwrap();
    assert_eq!(kept(&fleet).await.unwrap().merge.unwrap().session, None);
}

/// ONE merged as #2173 and TWO as #2175, each failing the jobs named.
async fn two_merges(fleet: &Fixture, older: &[&str], newer: &[&str]) {
    let jobs = |names: &[&str], first: usize| {
        names
            .iter()
            .enumerate()
            .map(|(at, name)| run(name, &format!("{}", first + at), CiState::Failed))
            .collect::<Vec<_>>()
    };
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(TWO));
    forge.runs_on(ONE, jobs(older, 10));
    forge.runs_on(TWO, jobs(newer, 20));
    forge.recently_merged_are(Some(vec![
        listed(2175, TWO, "2026-10-12T14:32:00Z"),
        listed(2173, ONE, "2026-10-12T14:19:00Z"),
    ]));
    forge.merged_by(ONE, pull(2173, "armada/session-01b2b572"));
    forge.merged_by(TWO, pull(2175, BRANCH));
}

#[tokio::test]
async fn a_red_that_began_on_the_older_commit_names_that_commit_and_each_job_once() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    let failing = ["app_smoke_shard (1)", "desktop_test"];
    two_merges(&fleet, &failing, &failing).await;

    fleet.turn().await.unwrap();
    let main = kept(&fleet).await.unwrap();
    assert_eq!(
        main.red_commit.as_deref(),
        Some(TWO),
        "the red stands at the newest finished commit"
    );
    assert_eq!(
        main.merge.unwrap().number,
        2173,
        "and began at the older one"
    );
    let names: Vec<&str> = main.failed.iter().map(|job| job.name.as_str()).collect();
    assert_eq!(names, failing);
}

#[tokio::test]
async fn a_job_only_the_newer_commit_fails_makes_the_newer_commit_the_culprit() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    two_merges(
        &fleet,
        &["desktop_test"],
        &["desktop_test", "app_smoke_shard (4)"],
    )
    .await;

    fleet.turn().await.unwrap();
    assert_eq!(kept(&fleet).await.unwrap().merge.unwrap().number, 2175);
}

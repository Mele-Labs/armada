//! A pull request Fleet has not seen, in a repository whose project names review tools: one item in Your turn, `Run
//! <tools> on #N?`, whose Run starts a review run for each tool and whose Not now is for good. `docs/practices/protocol/projects.md`,
//! *Review on a new pull request*.

use std::path::Path;
use std::time::Duration;

use adapter_traits::{FromOutside, OpenPull, PullRequestFacts, PullRequestStanding};
use api::PullRequestPanel;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorTurnItem};

use super::operator_sources::on;
use super::pull_request_panel::a_page;
use super::review_runs::HEAD;
use super::session_host::{rig_quiet_after, Rig};
use crate::tests::tmp::TempDir;

fn opened(number: u64, draft_by: &str) -> OpenPull {
    OpenPull {
        number,
        title: FromOutside::verbatim(format!("Pull request {number}")),
        branch: FromOutside::verbatim(format!("feature/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: Some(FromOutside::verbatim(draft_by)),
        ci: None,
        head: None,
        failing: Vec::new(),
        auto_merge: false,
        facts: Default::default(),
    }
}

/// A rig whose project names `names` and whose repository defines `defined`, the Operator on.
async fn rig_naming(names: &str, defined: &[&str]) -> Rig {
    let home = TempDir::new();
    let kept = home.path().to_path_buf();
    let rig = rig_quiet_after(home, Duration::from_secs(600), |_| {});
    for id in defined {
        let tools = Path::new(&rig.root).join(".armada/review-tools");
        std::fs::create_dir_all(&tools).expect("a folder");
        std::fs::write(tools.join(format!("{id}.md")), format!("---\nkind: tool\nname: {id}\n---\nRead it for {id} problems.\n")).expect("a tool");
    }
    if !names.is_empty() {
        let at = kept.join("kit/projects/mine.yml");
        std::fs::create_dir_all(at.parent().expect("a folder")).expect("its folder");
        let values = format!("values:\n  review_on_new_pr: [{names}]\n");
        std::fs::write(at, format!("version: 2\nproject_id: mine\nname: Mine\nmembers:\n  - repository: {}\n{values}", rig.manifest.as_str())).expect("a project");
        rig.fleet.projects_reload().await;
    }
    on(&rig).await;
    rig
}

async fn look(rig: &Rig, numbers: &[u64]) {
    rig.fleet.vcs().main_ci.pulls_are(Some(numbers.iter().map(|n| opened(*n, "teammate")).collect()));
    let served = rig.fleet.repositories().served().remove(0);
    assert!(rig.fleet.notice_pulls(&served).await, "the listing answered");
}

async fn items(rig: &Rig) -> Vec<OperatorTurnItem> {
    rig.fleet.operator_page_wire().await.expect("read").turn.into_iter().filter(|one| one.item.id.starts_with("review:")).collect()
}

fn press(item: &OperatorTurnItem, option: &str) -> OperatorAnswerPress {
    OperatorAnswerPress { id: item.id.clone(), answer: OperatorAnswer::Yes, then: None, option: Some(option.to_string()), project: None }
}

#[tokio::test]
async fn a_pull_request_not_seen_before_is_offered_once_with_run_and_not_now() {
    let rig = rig_naming("security, style", &["security", "style"]).await;
    look(&rig, &[7]).await;
    assert!(items(&rig).await.is_empty(), "what was open at the first look is not new");

    look(&rig, &[7, 8]).await;
    look(&rig, &[7, 8]).await;

    let offered = items(&rig).await;
    let [one] = offered.as_slice() else { panic!("one item: {offered:#?}") };
    assert_eq!(one.item.text, "Run security, style on #8?");
    assert_eq!(one.item.options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), ["Run", "Not now"]);
}

#[tokio::test]
async fn run_starts_one_review_run_for_each_tool() {
    let rig = rig_naming("security, style", &["security", "style"]).await;
    look(&rig, &[7]).await;
    look(&rig, &[7, 8]).await;
    let one = items(&rig).await.remove(0);
    // What the forge says of pull request 8, so a run can start on it.
    rig.fleet.vcs().now_pull_request(Some(PullRequestFacts {
        merge_state: adapter_traits::MergeState::Unknown,
        standing: PullRequestStanding::Open,
        branch: "feature/8".into(),
        base: "main".into(),
        auto_merge: false,
        title: "Pull request 8".into(),
        url: "https://forge.invalid/armada/pull/8".into(),
        body: None,
        closes: Vec::new(),
    }));
    rig.fleet.vcs().now_pull_request_page(Some(a_page()));
    rig.fleet.vcs().reading_pull_requests_at(HEAD);

    rig.fleet.operator_answered(press(&one, "Run")).await.expect("answered");

    let runs = rig.fleet.get_review_runs(rig.manifest.clone(), 8).await.expect("read").runs;
    let mut names: Vec<_> = runs.iter().map(|run| run.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["security", "style"]);
    assert!(items(&rig).await.is_empty(), "the item is settled");
}

#[tokio::test]
async fn not_now_starts_nothing_and_the_pull_request_is_never_offered_again() {
    let rig = rig_naming("security", &["security"]).await;
    look(&rig, &[7]).await;
    look(&rig, &[7, 8]).await;
    let one = items(&rig).await.remove(0);

    rig.fleet.operator_answered(press(&one, "Not now")).await.expect("answered");
    look(&rig, &[7, 8]).await;
    look(&rig, &[8]).await;
    look(&rig, &[7, 8]).await;

    assert!(items(&rig).await.is_empty(), "dismissed for good");
    assert!(rig.fleet.get_review_runs(rig.manifest.clone(), 8).await.expect("read").runs.is_empty());
}

#[tokio::test]
async fn no_setting_offers_nothing() {
    let rig = rig_naming("", &["security"]).await;
    look(&rig, &[7]).await;
    look(&rig, &[7, 8]).await;

    assert!(items(&rig).await.is_empty());
}

#[tokio::test]
async fn only_tools_the_repository_defines_are_offered() {
    let rig = rig_naming("security, ghost", &["security"]).await;
    look(&rig, &[7]).await;
    look(&rig, &[7, 8]).await;

    let offered = items(&rig).await;
    assert_eq!(offered.iter().map(|one| one.item.text.as_str()).collect::<Vec<_>>(), ["Run security on #8?"]);

    let none = rig_naming("ghost", &["security"]).await;
    look(&none, &[7]).await;
    look(&none, &[7, 8]).await;
    assert!(items(&none).await.is_empty(), "a tool nobody defined is not offered");
}

#[tokio::test]
async fn an_item_for_a_pull_request_that_closed_is_withdrawn() {
    let rig = rig_naming("security", &["security"]).await;
    look(&rig, &[7]).await;
    look(&rig, &[7, 8]).await;
    assert_eq!(items(&rig).await.len(), 1);

    look(&rig, &[7]).await;

    assert!(items(&rig).await.is_empty());
}

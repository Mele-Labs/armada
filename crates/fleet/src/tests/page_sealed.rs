//! The claim, written before the code: **any kept revision of a Job's page is served as one sealed HTML answer: its
//! policy is the first child of its `<head>`, only the kit's script carries the nonce the policy names, the kit's
//! stylesheet is inlined and no relative address is left to fetch, and the presses and votes made on that revision,
//! and no other, ride with it. A revision never kept is refused, and an archived page still serves.**
//! `crates/ipc/operations/get_artifact_page.toml`.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, SealedPage};
use ipc::{ArtifactBy, ArtifactRef, CastPageVote, PagePress, PressPageAction, SessionId, VoteCast, VoteResult};
use serde::Deserialize;

use super::page_actions::{a_job, page_holding};
use super::session_host::{eventually, rig_placing, Rig};

/// A page that links the kit, carries a script of its own and addresses of every sort, with one action and one vote.
const PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Flaky tests</title>\
    <link rel=\"stylesheet\" href=\"armada-kit.css\"><link rel=\"icon\" href=\"favicon.ico\"><base href=\"/elsewhere/\">\
    <meta http-equiv=\"refresh\" content=\"0; url=next.html\"><script>window.ran = true;</script>\
    <script nonce=\"guessed\" src=\"own.js\"></script></head><body><h1>Flaky tests</h1>\
    <img src=\"shot.png\" alt=\"Shot\"><img src=\"data:image/png;base64,AAAA\" alt=\"Inline\">\
    <a href=\"other.html\">Other</a> <a href='#top'>Top</a> <a href=\"https://example.com/x\">Out</a>\
    <table><tr><td>lease_clock drifts</td><td><armada-action id=\"discuss-1\" act=\"message\" label=\"Discuss\" item=\"row-1\"></armada-action></td></tr></table>\
    <armada-vote id=\"layout\" form=\"pick\" question=\"Which layout?\"><armada-take id=\"dense\" title=\"Dense\"><p>Tight</p></armada-take>\
    <armada-take id=\"roomy\" title=\"Roomy\"><p>Wide</p></armada-take></armada-vote></body></html>";

/// What the kit's script is handed.
#[derive(Deserialize)]
struct Record {
    artifact: String,
    revision: u32,
    presses: Vec<PagePress>,
    votes: Vec<VoteCast>,
}

async fn page(rig: &Rig, id: &SessionId, path: &str, revision: Option<u32>) -> Result<SealedPage, Refusal> {
    rig.fleet.get_artifact_page(id.clone(), path.to_string(), revision).await
}

/// The nonce the policy names.
fn nonce_of(policy: &str) -> &str {
    let from = policy.find("'nonce-").expect("a nonce") + "'nonce-".len();
    &policy[from..from + policy[from..].find('\'').unwrap()]
}

/// Every `<script ...>` opening tag of the page, as written.
fn script_tags(html: &str) -> Vec<&str> {
    html.match_indices("<script").map(|(at, _)| &html[at..at + html[at..].find('>').unwrap() + 1]).collect()
}

fn record_of(html: &str) -> Record {
    let open = html.find("id=\"armada-page-record\"").expect("the record");
    let from = open + html[open..].find('>').unwrap() + 1;
    let to = from + html[from..].find("</script>").unwrap();
    ipc::decode("the page's record", html[from..to].as_bytes()).expect("the record reads")
}

/// The Job writes the page again, and Fleet keeps it as the next revision.
pub(super) async fn rewritten(rig: &Rig, id: &SessionId, at: usize, path: &str, html: &str, revisions: usize) {
    std::fs::write(path, html).unwrap();
    rig.stand_in.says(
        at,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: "toolu_2".into(), detail: CallDetail::of(&format!("{path} +1")) },
            DroneEvent::Answered { call: "toolu_2".into(), failed: false },
        ],
    );
    eventually(|| async {
        rig.fleet.get_job_artifacts(id.clone()).await.unwrap().iter().any(|one| one.id == path && one.revisions.len() == revisions)
    })
    .await;
}

#[tokio::test]
async fn the_policy_is_the_first_child_of_head_and_sent_again_whole() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;

    let served = page(&rig, &id, &path, None).await.expect("served");

    let nonce = nonce_of(&served.policy);
    assert!(nonce.len() >= 16, "{nonce}");
    let policy = format!(
        "default-src 'none'; style-src 'unsafe-inline'; img-src data:; script-src 'nonce-{nonce}'; connect-src 'none'; form-action 'none'; base-uri 'none'"
    );
    assert_eq!(served.policy, policy);
    let head = served.html.find("<head>").expect("the head") + "<head>".len();
    let first = format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\">");
    assert!(served.html[head..].starts_with(&first), "{}", &served.html[head..head + 200]);
    assert_eq!(served.html.matches("http-equiv").count(), 1, "the page's own refresh is dropped");
    assert_eq!(served.revision, 1);
}

#[tokio::test]
async fn the_pages_own_scripts_have_no_nonce_and_only_the_kits_carry_it() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;

    let served = page(&rig, &id, &path, None).await.expect("served");

    let nonce = nonce_of(&served.policy).to_string();
    let tags = script_tags(&served.html);
    let carrying: Vec<&str> = tags.iter().copied().filter(|tag| tag.contains("nonce=")).collect();
    assert_eq!(
        carrying,
        [format!("<script type=\"application/json\" id=\"armada-page-record\" nonce=\"{nonce}\">"), format!("<script nonce=\"{nonce}\">")],
        "the record and the kit's script, and nothing of the page's"
    );
    assert!(tags.contains(&"<script>"), "the page's own script stays, inert: {tags:?}");
    assert!(served.html.contains("<script>window.ran = true;</script>"));
    assert!(served.html.contains("document.querySelectorAll(\"armada-action\")"), "the kit's script is inline");
    let again = page(&rig, &id, &path, None).await.expect("served again");
    assert_ne!(nonce_of(&again.policy), nonce, "a nonce per answer");
}

#[tokio::test]
async fn the_kit_css_is_inlined_and_no_relative_address_is_left() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;

    let served = page(&rig, &id, &path, None).await.expect("served");
    let html = &served.html;

    assert!(!html.contains("armada-kit.css"), "the link is gone");
    assert!(html.contains("li::marker { color: var(--fg-muted); }"), "the kit's rules are inline");
    assert!(html.contains("--status-completed-success:"), "the tokens the rules read are declared");
    assert!(!html.contains("<link") && !html.contains("<base"), "no other link and no base");
    for gone in ["favicon.ico", "own.js", "shot.png", "other.html", "next.html"] {
        assert!(!html.contains(gone), "{gone} is not addressed");
    }
    assert!(html.contains("<img alt=\"Shot\">"), "the image stays, with nothing to fetch");
    assert!(html.contains("<a>Other</a>"), "the link's words stay");
    for kept in ["src=\"data:image/png;base64,AAAA\"", "href='#top'", "href=\"https://example.com/x\""] {
        assert!(html.contains(kept), "{kept} stays");
    }
}

#[tokio::test]
async fn presses_and_votes_ride_with_the_revision_they_were_made_on_only() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;
    let press = PressPageAction { session_id: id.clone(), artifact: path.clone(), revision: 1, action: "discuss-1".into(), words: None };
    let pressed = Arc::clone(&rig.fleet).press_page_action(press).await.expect("pressed");
    let result = VoteResult::Pick { take: "roomy".into(), notes: BTreeMap::new() };
    let cast = CastPageVote { session_id: id.clone(), artifact: path.clone(), revision: 1, vote: "layout".into(), result };
    let voted = Arc::clone(&rig.fleet).cast_page_vote(cast).await.expect("voted");
    rewritten(&rig, &id, at, &path, &PAGE.replace("Flaky tests</h1>", "Flaky tests, again</h1>"), 2).await;

    let newest = page(&rig, &id, &path, None).await.expect("the newest");
    let first = page(&rig, &id, &path, Some(1)).await.expect("the first");

    assert_eq!(newest.revision, 2);
    let record = record_of(&newest.html);
    assert_eq!((record.artifact.as_str(), record.revision, record.presses.len(), record.votes.len()), (path.as_str(), 2, 0, 0));
    assert!(newest.html.contains("Flaky tests, again"));
    let record = record_of(&first.html);
    assert_eq!((record.revision, record.presses, record.votes), (1, vec![pressed], vec![voted]));
}

#[tokio::test]
async fn a_revision_never_kept_is_refused_and_so_is_an_artifact_not_held() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;

    for revision in [0, 2] {
        let refused = page(&rig, &id, &path, Some(revision)).await.expect_err("refused");
        assert_eq!(refused.error().code, "fleet.artifact_revision_unknown");
        assert!(refused.error().message.contains(&format!("revision {revision} of `Flaky tests` is not kept")), "{}", refused.error().message);
    }
    let refused = page(&rig, &id, "/no/such/page.html", None).await.expect_err("refused");
    assert_eq!(refused.error().code, "fleet.artifact_not_held");
}

#[tokio::test]
async fn an_archived_page_still_serves() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;
    let archive = ArtifactRef { artifact: path.clone(), session_id: Some(id.clone()) };
    Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Owner, archive).await.expect("archived");

    let served = page(&rig, &id, &path, None).await.expect("served");

    assert_eq!(served.revision, 1);
    assert!(served.html.contains("Flaky tests"));
}

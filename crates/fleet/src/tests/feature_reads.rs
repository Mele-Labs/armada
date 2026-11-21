//! The docs and tickets a Job read, filed under the features it then changed: **a read becomes a link only when a write
//! of the same Job to a file the feature owns comes after it.** Run through the facts a Job's mod tells and the page a
//! feature serves.

use api::{Features, Sessions};
use ipc::{FeaturePage, SessionFact, SessionId, SessionReport};

use super::agent_starts::{rig, Rig};
use super::feature_library::{a_project, an_agent_for, seeded, VOTING};
use super::slot_leases::{lease_record, By};

const FILE: &str = "crates/pages/voting/ranks.rs";

/// The Job's mod tells Fleet a read: the call, what it named, and the cut of its result.
pub(super) async fn read(rig: &Rig, id: &SessionId, tool: &str, target: &str, title: &str, gist: &str) {
    told(rig, id, SessionFact::Consulted { tool: tool.into(), target: target.into(), title: title.into(), gist: gist.into() }).await;
}

/// The Job holds slot `slot` of the rig's repository, as the pool's lease record says.
fn held(rig: &Rig, id: &SessionId, slot: u32) {
    let root = rig.fleet.repositories().first().expect("served").root().to_string();
    std::fs::create_dir_all(adapter_traits::slot_path(&root, slot)).expect("the slot");
    lease_record(&root, slot, &format!("branch-{slot}"), By::Job(id.as_str()));
}

/// The Job writes a file of the voting feature, from the slot it holds.
pub(super) async fn edit(rig: &Rig, id: &SessionId, slot: u32) {
    let root = rig.fleet.repositories().first().expect("served").root().to_string();
    told(rig, id, SessionFact::Touched { paths: vec![format!("{}/{FILE}", adapter_traits::slot_path(&root, slot))], by: None, act: None }).await;
}

async fn told(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact };
    rig.fleet.report_session(report).await.expect("reported");
}

/// A project that holds the voting feature, and a Job on it that holds slot 4.
pub(super) async fn a_job(rig: &Rig) -> (String, SessionId) {
    let project = a_project(rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(rig, &project).await;
    held(rig, &id, 4);
    (project, id)
}

pub(super) async fn page(rig: &Rig, project: &str, slug: &str) -> FeaturePage {
    rig.fleet.get_feature_page(None, Some(project.to_string()), slug.to_string(), None, false).await.expect("the page")
}

#[tokio::test]
async fn a_doc_read_before_the_edit_is_linked_with_its_title_and_first_sentence() {
    let rig = rig(None);
    let (project, id) = a_job(&rig).await;
    read(&rig, &id, "WebFetch", "https://example.com/spec", "Voting spec", "A take is ranked by votes.").await;
    read(&rig, &id, "Read", "docs/concepts/votes.md", "Votes", "A vote is one per person.").await;
    read(&rig, &id, "Read", "crates/pages/voting/ranks.rs", "", "").await;
    edit(&rig, &id, 4).await;

    let voting = page(&rig, &project, VOTING).await;
    let mut docs: Vec<_> = voting.docs.iter().map(|one| (one.target.as_str(), one.title.as_str(), one.gist.as_str(), one.job.as_str())).collect();
    docs.sort();
    assert_eq!(
        docs,
        [
            ("docs/concepts/votes.md", "Votes", "A vote is one per person.", id.as_str()),
            ("https://example.com/spec", "Voting spec", "A take is ranked by votes.", id.as_str()),
        ],
        "a source file is not a doc"
    );
    assert!(voting.tickets.is_empty());
    assert!(page(&rig, &project, "page-actions").await.docs.is_empty(), "another feature's files were not written");
}

#[tokio::test]
async fn a_doc_read_after_the_edit_is_not_linked() {
    let rig = rig(None);
    let (project, id) = a_job(&rig).await;
    edit(&rig, &id, 4).await;
    read(&rig, &id, "WebFetch", "https://example.com/late", "Late", "Read once the file was written.").await;
    assert!(page(&rig, &project, VOTING).await.docs.is_empty());

    // The next write comes after the read, so it is that write that the read shaped.
    edit(&rig, &id, 4).await;
    assert_eq!(page(&rig, &project, VOTING).await.docs.len(), 1);
}

#[tokio::test]
async fn a_read_in_another_job_is_not_linked() {
    let rig = rig(None);
    let (project, reader) = a_job(&rig).await;
    let writer = an_agent_for(&rig, &project).await;
    held(&rig, &writer, 5);
    read(&rig, &reader, "WebFetch", "https://example.com/spec", "Voting spec", "").await;
    edit(&rig, &writer, 5).await;
    assert!(page(&rig, &project, VOTING).await.docs.is_empty());
}

#[tokio::test]
async fn a_ticket_is_kind_ticket_and_stays_out_of_the_docs() {
    let rig = rig(None);
    let (project, id) = a_job(&rig).await;
    read(&rig, &id, "mcp__plugin_productivity_linear__get_issue", "ENG-12", "Ranks tie", "Two takes with equal votes swap places.").await;
    read(&rig, &id, "Bash", "armada-a3f2", "", "").await;
    read(&rig, &id, "mcp__plugin_productivity_notion__notion-update-page", "page-1", "Not a read", "").await;
    edit(&rig, &id, 4).await;

    let voting = page(&rig, &project, VOTING).await;
    assert!(voting.docs.is_empty(), "{:?}", voting.docs);
    let mut tickets: Vec<_> = voting.tickets.iter().map(|one| (one.target.as_str(), one.title.as_str(), one.gist.as_str())).collect();
    tickets.sort();
    assert_eq!(tickets, [("ENG-12", "Ranks tie", "Two takes with equal votes swap places."), ("armada-a3f2", "armada-a3f2", "")], "a tool that writes is no read, and a result not seen is named by its id");
}

#[tokio::test]
async fn a_repeat_read_and_a_repeat_edit_keep_one_link() {
    let rig = rig(None);
    let (project, id) = a_job(&rig).await;
    read(&rig, &id, "WebFetch", "https://example.com/spec", "Voting spec", "First.").await;
    edit(&rig, &id, 4).await;
    read(&rig, &id, "WebFetch", "https://example.com/spec", "Voting spec", "Changed.").await;
    edit(&rig, &id, 4).await;

    let voting = page(&rig, &project, VOTING).await;
    assert_eq!(voting.docs.len(), 1);
    assert_eq!(voting.docs[0].gist, "First.", "the first sighting stays");
    let store = rig.fleet.store().lock().await;
    let feature = store.feature_named(&project, VOTING).expect("read").expect("the feature");
    assert_eq!(store.feature_links(&feature.id).expect("read").iter().filter(|link| link.kind == "doc").count(), 1);
}

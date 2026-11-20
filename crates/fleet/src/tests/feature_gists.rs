//! A Job's own gist of a doc it read, written with its sections: **it replaces the first sentence on the Job's read links
//! and wins over any heard later, it is refused for a doc the Job never read and linked, and `all_done` is never held for
//! one.** Run through the facts a Job's mod tells and the page a feature serves.

use api::{Features, HostedSessions, Sessions};
use ipc::{AllDone, DocGist, FeatureSectionKind, SessionId, WriteFeatureSections};

use super::agent_starts::caller;
use super::feature_library::{named, pushed, refused, write_all, VOTING};
use super::feature_reads::{a_job, edit, page, read};

const SPEC: &str = "https://example.com/spec";

fn gists(id: &SessionId, gists: &[(&str, &str)]) -> WriteFeatureSections {
    WriteFeatureSections {
        feature: VOTING.to_string(),
        sections: Vec::new(),
        glossary: Vec::new(),
        gists: gists.iter().map(|(target, gist)| DocGist { target: target.to_string(), gist: gist.to_string() }).collect(),
        project: None,
        session_id: Some(id.clone()),
    }
}

#[tokio::test]
async fn a_jobs_gist_replaces_the_first_sentence_on_its_read_link_and_is_served_as_the_jobs() {
    let rig = super::agent_starts::rig(None);
    let (project, id) = a_job(&rig).await;
    read(&rig, &id, "WebFetch", SPEC, "Voting spec", "A take is ranked by votes.").await;
    edit(&rig, &id, 4).await;
    assert!(!page(&rig, &project, VOTING).await.docs[0].gist_by_job);

    let written = rig.fleet.write_feature_sections(caller(), gists(&id, &[(SPEC, "Ties go to  the older\ntake; rank is recomputed on every vote.")])).await.expect("a gists-only call");
    assert_eq!((written.gists, written.written.is_empty()), (1, true));

    let voting = page(&rig, &project, VOTING).await;
    assert_eq!(voting.docs.len(), 1);
    assert_eq!(voting.docs[0].gist, "Ties go to the older take; rank is recomputed on every vote.", "one line");
    assert!(voting.docs[0].gist_by_job);
}

#[tokio::test]
async fn a_first_sentence_heard_after_the_jobs_gist_does_not_replace_it() {
    let rig = super::agent_starts::rig(None);
    let (project, id) = a_job(&rig).await;
    read(&rig, &id, "WebFetch", SPEC, "Voting spec", "First.").await;
    edit(&rig, &id, 4).await;
    rig.fleet.write_feature_sections(caller(), gists(&id, &[(SPEC, "What the Job says.")])).await.expect("written");

    read(&rig, &id, "WebFetch", SPEC, "Voting spec", "A later first sentence.").await;
    edit(&rig, &id, 4).await;

    let voting = page(&rig, &project, VOTING).await;
    assert_eq!((voting.docs[0].gist.as_str(), voting.docs[0].gist_by_job), ("What the Job says.", true));
    let store = rig.fleet.store().lock().await;
    assert_eq!(store.feature_job_reads(id.as_str()).expect("read")[0].gist, "What the Job says.", "a link made later carries it");
}

#[tokio::test]
async fn a_gist_for_a_target_the_job_never_read_and_linked_or_a_blank_one_is_refused_and_nothing_is_kept() {
    let rig = super::agent_starts::rig(None);
    let (project, id) = a_job(&rig).await;
    read(&rig, &id, "WebFetch", SPEC, "Voting spec", "First.").await;
    // Read after the write, so not linked yet.
    edit(&rig, &id, 4).await;
    read(&rig, &id, "WebFetch", "https://example.com/late", "Late", "").await;

    for target in ["https://example.com/never", "https://example.com/late"] {
        let refusal = rig.fleet.write_feature_sections(caller(), gists(&id, &[(SPEC, "Kept?"), (target, "A line.")])).await.expect_err("not read and linked");
        assert_eq!(refusal.error().code, "fleet.gist_not_read");
        assert!(refusal.error().message.contains(target), "{}", refusal.error().message);
    }
    let blank = rig.fleet.write_feature_sections(caller(), gists(&id, &[(SPEC, "  ")])).await.expect_err("blank");
    assert_eq!(blank.error().code, "fleet.gist_blank");
    assert_eq!(page(&rig, &project, VOTING).await.docs[0].gist, "First.", "a refused call keeps none of its gists");
}

#[tokio::test]
async fn the_owed_answer_and_the_refusal_name_the_docs_read_and_a_gist_never_holds_all_done() {
    let rig = super::agent_starts::rig(None);
    let (_, id) = a_job(&rig).await;
    for n in 0..10 {
        read(&rig, &id, "WebFetch", &format!("https://example.com/doc-{n}"), &format!("Doc {n}"), "").await;
    }
    edit(&rig, &id, 4).await;
    named(&rig, &id, VOTING).await;
    pushed(&rig, &id, "812").await;

    let (code, said) = refused(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await);
    assert_eq!(code, "fleet.feature_sections_owed");
    assert!(said.contains("gist is welcome, never required"), "{said}");
    assert_eq!(said.matches("\n- https://example.com/doc-").count(), 8, "at most eight docs: {said}");

    let one = write_all(&id, VOTING, &[FeatureSectionKind::Test]);
    let written = rig.fleet.write_feature_sections(caller(), one).await.expect("written");
    assert_eq!(written.owed[0].docs.len(), 8);
    let named_doc = written.owed[0].docs[0].clone();

    let written = rig.fleet.write_feature_sections(caller(), gists(&id, &[(&named_doc, "Said.")])).await.expect("written");
    assert!(!written.owed[0].docs.contains(&named_doc), "a doc with a gist is not asked for again");

    let written = rig.fleet.write_feature_sections(caller(), write_all(&id, VOTING, &[FeatureSectionKind::Runs, FeatureSectionKind::Observe, FeatureSectionKind::Debt])).await.expect("written");
    assert!(written.owed.is_empty(), "{:?}", written.owed);
    assert!(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.is_ok(), "no gist was given for the other docs");
}

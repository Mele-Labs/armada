//! The feature catalog's Decisions, written before the code: **what the owner settled in a Job is filed on the feature
//! pages by Fleet, with no model.** An artifact that names features files its decisions under them and under no other; one
//! that names none files them under the features the Job changed, whenever those were linked. `docs/concepts/feature-catalog.md`.

use std::sync::Arc;

use api::{Features, HostedSessions, Sessions};
use ipc::{AnswerPart, AnswerSketch, FeatureDecision, PartHow, SessionId, ShowSketch, SketchHow};

use super::feature_catalog::{a_job_in, a_project, commits_filed, libraried, rig, trailer, ACTIONS, VOTING};
use super::session_host::Rig;

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
const TAKE_A: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Left"}],"edges":[]}"#;
const TAKE_B: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Right"}],"edges":[]}"#;

fn drawn(id: &SessionId, title: &str, feature: Option<&str>) -> ShowSketch {
    let feature = feature.map(|slug| format!(r#","feature":"{slug}""#)).unwrap_or_default();
    ipc::decode("a sketch", format!(r#"{{"title":"{title}","session_id":"{}","scene":{SCENE}{feature}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn takes(id: &SessionId, title: &str) -> ipc::AddPart {
    let body = format!(
        r#"{{"session_id":"{}","title":"{title}","part":{{"id":"layout","kind":"takes","question":"Which layout?","takes":[{{"id":"a","title":"Left","scene":{TAKE_A}}},{{"id":"b","title":"Right","scene":{TAKE_B}}}]}}}}"#,
        id.as_str()
    );
    ipc::decode("a part", body.as_bytes()).unwrap()
}

async fn decisions(rig: &Rig, project: &str, slug: &str) -> Vec<FeatureDecision> {
    rig.fleet.get_feature_page(None, Some(project.to_string()), slug.to_string(), None, false).await.expect("page").decisions
}

#[tokio::test]
async fn a_pick_on_takes_is_filed_under_the_feature_its_sketch_names_and_no_other() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    // The Job changed the other feature, which the sketch does not name.
    commits_filed(&rig, &job, &[trailer(ACTIONS, "abc1", &[])]).await;

    rig.fleet.show_sketch(None, drawn(&job, "Vote layout", Some(VOTING))).await.unwrap();
    rig.fleet.add_part(None, takes(&job, "Vote layout")).await.unwrap();
    let layout = rig.fleet.get_job_artifacts(job.clone()).await.unwrap().remove(0).parts.into_iter().find(|part| part.id == "layout").unwrap();
    let pick = AnswerPart {
        session_id: job.clone(),
        artifact: "sketch:Vote layout".into(),
        part: "layout".into(),
        drawn: layout.drawn,
        how: PartHow::Picked,
        take: Some("b".into()),
        words: Some("Right reads better.".into()),
        notes: Default::default(),
        extras: Default::default(),
    };
    Arc::clone(&rig.fleet).answer_part(pick).await.unwrap();

    let filed = decisions(&rig, &project, VOTING).await;
    assert_eq!(filed.len(), 1, "{filed:?}");
    assert_eq!((filed[0].question.as_str(), filed[0].choice.as_str(), filed[0].note.as_deref()), ("Which layout?", "Right", Some("Right reads better.")));
    assert_eq!(filed[0].job, job.as_str());
    assert!(decisions(&rig, &project, ACTIONS).await.is_empty(), "the sketch names a feature, so the Job's other one is not given it");
}

#[tokio::test]
async fn a_verdict_on_a_sketch_naming_no_feature_is_filed_under_the_features_the_job_changed_even_when_linked_later() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer(VOTING, "abc1", &[])]).await;

    let record = rig.fleet.show_sketch(None, drawn(&job, "Vote results", None)).await.unwrap();
    let drawing = record.attachments.iter().find(|one| one.target == "sketch:Vote results").and_then(|one| one.detail.get("drawn").cloned()).unwrap();
    let said = AnswerSketch { session_id: job.clone(), sketch: "sketch:Vote results".into(), drawn: drawing, how: SketchHow::SentBack, words: Some("Fewer columns.".into()), extras: Default::default() };
    Arc::clone(&rig.fleet).answer_sketch(said).await.unwrap();

    let filed = decisions(&rig, &project, VOTING).await;
    assert_eq!(filed.len(), 1, "{filed:?}");
    assert_eq!((filed[0].question.as_str(), filed[0].choice.as_str(), filed[0].note.as_deref()), ("Vote results", "Asked for changes", Some("Fewer columns.")));
    assert!(decisions(&rig, &project, ACTIONS).await.is_empty(), "the Job had not changed it yet");

    commits_filed(&rig, &job, &[trailer(ACTIONS, "abc2", &[])]).await;
    assert_eq!(decisions(&rig, &project, ACTIONS).await, filed, "a feature the Job changes later shows what he settled before");
}

#[tokio::test]
async fn hearing_an_answer_again_files_it_once() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer(VOTING, "abc1", &[])]).await;

    let record = rig.fleet.show_sketch(None, drawn(&job, "Vote results", None)).await.unwrap();
    let drawing = record.attachments.iter().find(|one| one.target == "sketch:Vote results").and_then(|one| one.detail.get("drawn").cloned()).unwrap();
    let said = AnswerSketch { session_id: job.clone(), sketch: "sketch:Vote results".into(), drawn: drawing, how: SketchHow::Approved, words: None, extras: Default::default() };
    let settled = Arc::clone(&rig.fleet).answer_sketch(said).await.unwrap();

    // Fleet hears the same settled sketch again, as it does each time the Job's record is reported.
    let again = settled.attachments.iter().find(|one| one.target == "sketch:Vote results").unwrap().clone();
    let now = rig.fleet.now().as_str().to_string();
    for _ in 0..2 {
        rig.fleet.artifact_decided(&mut *rig.fleet.store().lock().await, job.as_str(), &again.target, &again.detail, &now);
    }
    commits_filed(&rig, &job, &[trailer(VOTING, "abc1", &[])]).await;

    let filed = decisions(&rig, &project, VOTING).await;
    assert_eq!(filed.len(), 1, "{filed:?}");
    assert_eq!(filed[0].choice, "Approved");
}

#[tokio::test]
async fn the_page_serves_decisions_newest_first() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer(VOTING, "abc1", &[])]).await;
    let kept = |key: &str, question: &str, at: &str| store::KeptFeatureDecision {
        job: job.as_str().into(),
        key: key.into(),
        artifact: String::new(),
        question: question.into(),
        choice: "Looks good".into(),
        note: String::new(),
        at: at.into(),
    };
    {
        let mut store = rig.fleet.store().lock().await;
        store.keep_feature_decision(&kept("one", "First", "2026-11-18T09:00:00.000Z")).unwrap();
        store.keep_feature_decision(&kept("three", "Third", "2026-11-18T11:00:00.000Z")).unwrap();
        store.keep_feature_decision(&kept("two", "Second", "2026-11-18T10:00:00.000Z")).unwrap();
    }
    let order: Vec<String> = decisions(&rig, &project, VOTING).await.into_iter().map(|one| one.question).collect();
    assert_eq!(order, ["Third", "Second", "First"]);
}

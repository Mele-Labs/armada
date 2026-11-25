//! Judge types through a real Fleet: which one a press reads as, an unknown one refused, and a Kit save and
//! delete moving what the next press asks. `docs/concepts/judge.md`, *Judge types*; the files' own rules are
//! `config`'s.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use api::{JudgeReadings, Refusal};
use ipc::{DeleteJudgeType, JudgeTypes, ReviewToolScope, ReviewToolSource, SaveJudgeType};

use super::judge::{a_job_with_a_plan, answered_with, ask, asked_of, code, ended, readings, the_plan_artifact, ASKED};
use super::session_host::Rig;

const NOTHING: &str = r#"{"notes": []}"#;
const ADVERSARIAL_WORDS: &str = "Find what the person or the agent missed in this work";
const SYSTEM_DESIGN_WORDS: &str = "Read the work as the person who will run it a year from now";

fn kit_judges(rig: &Rig) -> PathBuf {
    PathBuf::from(&rig.fleet.host().kit_home).join("judges")
}

fn plant(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(path, text).expect("written");
}

async fn listed(rig: &Rig, project: Option<&str>) -> Result<JudgeTypes, Refusal> {
    Arc::clone(&rig.fleet).list_judge_types(Some(rig.manifest.clone()), project.map(str::to_string)).await
}

fn saving(name: &str, prompt: &str, overwrite: bool) -> SaveJudgeType {
    SaveJudgeType { scope: ReviewToolScope::Kit, project: None, name: name.into(), prompt: prompt.into(), overwrite }
}

async fn asked_as(rig: &Rig, job: &ipc::SessionId, judge: Option<&str>, line: &str) -> Result<ipc::JudgeReading, Refusal> {
    Arc::clone(&rig.fleet)
        .ask_judge(ipc::AskJudge { session_id: job.clone(), artifact: the_plan_artifact(), prompt: line.into(), judge: judge.map(str::to_string) })
        .await
}

#[tokio::test]
async fn a_press_with_no_type_reads_as_adversarial_and_a_line_to_verify_is_laid_after_it() {
    let (rig, judge) = answered_with(NOTHING);
    let job = a_job_with_a_plan(&rig).await;

    let reading = asked_as(&rig, &job, None, "").await.expect("asked");
    assert_eq!((reading.judge.as_deref(), reading.judge_name.as_deref()), (Some("adversarial"), Some("Adversarial")));
    let kept = ended(&rig, &job, ipc::JudgeKind::Plan).await;
    assert_eq!((kept.judge.as_deref(), kept.judge_name.as_deref()), (Some("adversarial"), Some("Adversarial")), "kept with the reading");
    let brief = &asked_of(&judge)[0].0;
    assert!(brief.contains(&format!("## What to look for\n\n{ADVERSARIAL_WORDS}")), "{brief}");
    assert!(!brief.contains("asked you to verify"), "no line, no section");

    asked_as(&rig, &job, None, "Check the lock.").await.expect("again");
    ended(&rig, &job, ipc::JudgeKind::Plan).await;
    let brief = &asked_of(&judge)[1].0;
    let (looks, verify) = (brief.find(ADVERSARIAL_WORDS), brief.find("## What the person asked you to verify\n\nCheck the lock."));
    assert!(looks.is_some() && verify.is_some() && looks < verify, "the type's words, then his line: {brief}");
}

#[tokio::test]
async fn a_press_names_a_type_and_the_brief_carries_its_prompt_and_the_reading_its_name() {
    let (rig, judge) = answered_with(NOTHING);
    let job = a_job_with_a_plan(&rig).await;

    let reading = asked_as(&rig, &job, Some("system-design"), "").await.expect("asked");
    assert_eq!((reading.judge.as_deref(), reading.judge_name.as_deref()), (Some("system-design"), Some("System design")));
    let kept = ended(&rig, &job, ipc::JudgeKind::Plan).await;
    assert_eq!(kept.judge_name.as_deref(), Some("System design"));
    assert_eq!(readings(&rig, &job).await[0].judge.as_deref(), Some("system-design"), "on the Job's record");
    let brief = &asked_of(&judge)[0].0;
    assert!(brief.contains(SYSTEM_DESIGN_WORDS), "{brief}");
    assert!(!brief.contains(ADVERSARIAL_WORDS), "the default's words are not laid beside it");
    assert!(brief.starts_with(ASKED));
}

#[tokio::test]
async fn a_type_nobody_defines_refuses_the_ask_naming_it_and_runs_nothing() {
    let (rig, judge) = answered_with(NOTHING);
    let job = a_job_with_a_plan(&rig).await;

    let refusal = asked_as(&rig, &job, Some("nope"), "").await.expect_err("refused");
    assert!(refusal.error().message.contains("`nope`"), "{}", refusal.error().message);
    assert_eq!(code(refusal), "fleet.judge_type_unknown");
    assert!(readings(&rig, &job).await.is_empty(), "no reading was kept");
    assert!(asked_of(&judge).is_empty(), "no call was made");
}

#[tokio::test]
async fn a_kit_save_of_an_armada_id_edits_its_prompt_and_deleting_it_restores_armadas() {
    let (rig, judge) = answered_with(NOTHING);
    let job = a_job_with_a_plan(&rig).await;
    let armada = listed(&rig, None).await.expect("listed");
    let ids: Vec<&str> = armada.types.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["adversarial", "code", "design", "system-design", "api-review"]);

    let saved = Arc::clone(&rig.fleet).save_judge_type(rig.manifest.clone(), saving("Code", "Look at the locks only.", false)).await.expect("saved");
    let code_type = saved.types.iter().find(|one| one.id == "code").expect("code");
    assert_eq!((code_type.source, code_type.replaces, code_type.prompt.as_str()), (ReviewToolSource::Kit, Some(ReviewToolSource::Armada), "Look at the locks only."));
    assert!(kit_judges(&rig).join("code.md").exists());
    asked_as(&rig, &job, Some("code"), "").await.expect("asked");
    ended(&rig, &job, ipc::JudgeKind::Plan).await;
    assert!(asked_of(&judge)[0].0.contains("## What to look for\n\nLook at the locks only."));

    let again = Arc::clone(&rig.fleet).save_judge_type(rig.manifest.clone(), saving("Code", "Another.", false)).await.expect_err("exists");
    assert_eq!(code(again), "fleet.judge_type_exists");
    Arc::clone(&rig.fleet).save_judge_type(rig.manifest.clone(), saving("Code", "Another.", true)).await.expect("overwritten");

    let deleted = Arc::clone(&rig.fleet)
        .delete_judge_type(rig.manifest.clone(), DeleteJudgeType { scope: ReviewToolScope::Kit, project: None, id: "code".into() })
        .await
        .expect("deleted");
    assert_eq!(deleted, armada, "Armada's prompt answers for it again, as if it had never been saved");
    assert!(!kit_judges(&rig).join("code.md").exists());
    asked_as(&rig, &job, Some("code"), "").await.expect("asked");
    ended(&rig, &job, ipc::JudgeKind::Plan).await;
    assert!(asked_of(&judge)[1].0.contains("Read the change as the person who has to maintain it"), "Armada's again");

    let missing = Arc::clone(&rig.fleet)
        .delete_judge_type(rig.manifest.clone(), DeleteJudgeType { scope: ReviewToolScope::Kit, project: None, id: "code".into() })
        .await
        .expect_err("nothing left");
    assert_eq!(code(missing), "fleet.judge_type_not_kept");
}

#[tokio::test]
async fn nothing_is_written_into_the_repository_and_a_blank_or_wordless_save_is_refused() {
    let (rig, _) = answered_with(NOTHING);
    let into_repository = SaveJudgeType { scope: ReviewToolScope::Repository, ..saving("Mine", "Words.", false) };
    let refused = Arc::clone(&rig.fleet).save_judge_type(rig.manifest.clone(), into_repository).await.expect_err("refused");
    assert_eq!(code(refused), "fleet.judge_type_misplaced");
    assert!(!Path::new(&rig.root).join(".armada/judges").exists());
    let repository = DeleteJudgeType { scope: ReviewToolScope::Repository, project: None, id: "code".into() };
    assert_eq!(code(Arc::clone(&rig.fleet).delete_judge_type(rig.manifest.clone(), repository).await.expect_err("refused")), "fleet.judge_type_misplaced");
    for (name, prompt, said) in [("", "Words.", "fleet.judge_type_blank"), ("Mine", "  ", "fleet.judge_type_blank"), ("!!!", "Words.", "fleet.judge_type_not_a_name")] {
        let refused = Arc::clone(&rig.fleet).save_judge_type(rig.manifest.clone(), saving(name, prompt, false)).await.expect_err("refused");
        assert_eq!(code(refused), said, "{name:?} {prompt:?}");
    }
    assert!(!kit_judges(&rig).exists(), "nothing was written");
}

#[tokio::test]
async fn a_projects_folder_answers_for_that_project_only_and_a_file_that_does_not_fit_is_named() {
    let (rig, _) = answered_with(NOTHING);
    let repository = Path::new(&rig.root).join(".armada/judges/design.md");
    plant(&repository, "---\nname: Design\n---\nThe repository's design.\n");
    plant(&kit_judges(&rig).join("design.md"), "---\nname: Design\n---\nKit's design.\n");
    plant(&kit_judges(&rig).join("projects/storefront/design.md"), "---\nname: Design\n---\nThe storefront's design.\n");
    plant(&kit_judges(&rig).join("broken.md"), "no front matter");

    let all = listed(&rig, None).await.expect("listed");
    let design = all.types.iter().find(|one| one.id == "design").expect("design");
    assert_eq!((design.source, design.project.as_deref(), design.prompt.as_str()), (ReviewToolSource::Kit, None, "Kit's design."));
    assert_eq!(design.hides, Some(ReviewToolSource::Repository), "Kit's copy hides the repository's");
    assert_eq!(all.left_out.len(), 1);
    assert!(all.left_out[0].file.ends_with("judges/broken.md") && all.left_out[0].said.contains("was left out"), "{:?}", all.left_out);

    let storefront = listed(&rig, Some("storefront")).await.expect("listed");
    let design = storefront.types.iter().find(|one| one.id == "design").expect("design");
    assert_eq!((design.source, design.project.as_deref(), design.prompt.as_str()), (ReviewToolSource::Kit, Some("storefront"), "The storefront's design."));
    let other = listed(&rig, Some("backoffice")).await.expect("listed");
    assert_eq!(other.types.iter().find(|one| one.id == "design").expect("design").prompt, "Kit's design.");
    assert_eq!(code(listed(&rig, Some("../outside")).await.expect_err("not a folder")), "fleet.judge_type_misplaced");
}

#[tokio::test]
async fn a_job_in_a_folder_with_no_manifest_sees_armadas_types_and_kits_and_no_repositorys() {
    let (rig, _) = answered_with(NOTHING);
    plant(&Path::new(&rig.root).join(".armada/judges/design.md"), "---\nname: Design\n---\nThe repository's design.\n");
    plant(&kit_judges(&rig).join("mine.md"), "---\nname: Mine\n---\nKit's own.\n");
    plant(&kit_judges(&rig).join("projects/storefront/mine.md"), "---\nname: Mine\n---\nThe storefront's own.\n");

    let unplaced = Arc::clone(&rig.fleet).list_judge_types(None, None).await.expect("listed with no repository");
    let ids: Vec<&str> = unplaced.types.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["adversarial", "code", "design", "system-design", "api-review", "mine"]);
    let design = unplaced.types.iter().find(|one| one.id == "design").expect("design");
    assert_eq!(design.source, ReviewToolSource::Armada, "the repository's copy is not read where there is no repository");
    assert_eq!(unplaced.types.iter().find(|one| one.id == "mine").expect("mine").prompt, "Kit's own.");
    let storefront = Arc::clone(&rig.fleet).list_judge_types(None, Some("storefront".into())).await.expect("listed");
    assert_eq!(storefront.types.iter().find(|one| one.id == "mine").expect("mine").prompt, "The storefront's own.");

    // The Job's own record names no Manifest, and an ask resolves its type the same way.
    let job = a_job_with_a_plan(&rig).await;
    let mut hosting = rig.fleet.store().lock().await.hosting(job.as_str()).expect("read").expect("hosted");
    hosting.manifest_id = String::new();
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    let reading = asked_as(&rig, &job, Some("mine"), "").await.expect("a Kit type answers for it");
    assert_eq!((reading.judge.as_deref(), reading.judge_name.as_deref()), (Some("mine"), Some("Mine")));
    ended(&rig, &job, ipc::JudgeKind::Plan).await;
    let none = asked_as(&rig, &job, Some("nope"), "").await.expect_err("refused");
    assert_eq!(code(none), "fleet.judge_type_unknown");
}

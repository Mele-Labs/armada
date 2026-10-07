//! Fleet fires a Job's Triggers at their moments, records each, and a failure
//! moves nothing. Commands are real programs (`echo`, `true`, `false`) run in
//! the Job's worktree; the Drone and the pull request are fakes.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use adapter_traits::Opened;
use config::{Manifest, TriggerWritten};
use core_model::{JobId, JobStatus, StepState, TriggerState, TriggerWhen};
use testkit::{Delivering, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{
    Catalogued, Located, Locating, NotLocated, SavedWorkflow, WorkflowNotSaved,
};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, worktree_directory,
};
use crate::tests::repositories::Fixture;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// The Trigger files a case plants, as the composition root would read them.
#[derive(Default)]
pub(super) struct Files(Mutex<Vec<TriggerWritten>>);

impl Files {
    pub(super) fn say(&self, files: Vec<TriggerWritten>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = files;
    }
}

impl Locating for Arc<Files> {
    fn triggers(&self, _root: &Path, _base: Option<&str>) -> Vec<TriggerWritten> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
    fn save_trigger(
        &self,
        _: &Path,
        manifest: &Manifest,
        asked: &ipc::SaveTrigger,
    ) -> Result<crate::repositories::SavedTrigger, crate::repositories::TriggerNotSaved> {
        let file = Path::new("/home/user/.armada/machine/triggers/saved.yml");
        let fitted = config::fit_trigger(file, &asked.definition, manifest).map_err(|why| {
            crate::repositories::TriggerNotSaved::Unfit {
                why: why.to_string(),
            }
        })?;
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(TriggerWritten::on_machine(file.into(), asked.definition.clone()));
        Ok(crate::repositories::SavedTrigger {
            trigger: fitted,
            file: file.display().to_string(),
            replaced: false,
        })
    }
    fn save_workflow(
        &self,
        _: &Path,
        _: &Manifest,
        _: &ipc::SaveWorkflow,
    ) -> Result<SavedWorkflow, WorkflowNotSaved> {
        unreachable!("no case saves a workflow")
    }
    fn workflows(&self, _: &Path, _: &Manifest, _: &[Manifest]) -> Catalogued {
        Catalogued::default()
    }
    fn located(&self, _: &Path) -> Result<Located, NotLocated> {
        unreachable!("no case adds a repository")
    }
    fn serving(&self, _: &str) {}
}

pub(super) fn machine(file: &str, text: &str) -> TriggerWritten {
    TriggerWritten::on_machine(
        Path::new("/home/user/.armada/machine/triggers").join(file),
        text.to_string(),
    )
}

pub(super) fn deploys() -> TriggerWritten {
    machine(
        "deploy.yml",
        "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\n",
    )
}

pub(super) fn manifest(deploy_qa: Option<&str>) -> Manifest {
    let mut text = String::from("version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  fmt:\n    run: \"true\"\n  wipe:\n    run: \"true\"\n    destructive: true\n");
    if let Some(run) = deploy_qa {
        text.push_str(&format!("  deploy_qa:\n    run: \"{run}\"\n"));
    }
    Manifest::parse(Path::new("armada.yml"), &text).expect("a Manifest")
}

fn a_fleet(
    home: &TempDir,
    files: &Arc<Files>,
    manifest: Manifest,
    delivering: Delivering,
) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().manifest = manifest;
    fittings.vcs = FakeVcs::new().delivering(delivering);
    fittings.locating = Arc::new(Arc::clone(files));
    Fleet::assembled(fittings)
}

/// Approve, work the first step, and enter the delivering one.
pub(super) async fn to_the_delivering_step(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    job.id().clone()
}

pub(super) async fn firings(fleet: &Fixture, job: &JobId) -> Vec<(String, TriggerWhen, String, TriggerState)> {
    let store = fleet.store().lock().await;
    store
        .trigger_firings(job)
        .unwrap()
        .into_iter()
        .map(|f| (f.name, f.when, f.step.as_str().to_string(), f.state))
        .collect()
}

fn log(fleet: &Fixture, home: &TempDir, job: &core_model::Job) -> String {
    let _ = fleet;
    let path = crate::transcript::log_of(&home.path().to_string_lossy(), &job.handle());
    std::fs::read_to_string(path).unwrap_or_default()
}

#[tokio::test]
async fn each_moment_fires_what_applies_and_the_pull_request_one_fires_after_it_opens() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![
        deploys(),
        machine("begin.yml", "name: begin\nwhen: step_starts\ncommand: fmt\n"),
        machine(
            "tidy.yml",
            "name: tidy\nwhen: step_passes\nstep: implement\ncommand: fmt\n",
        ),
    ]);
    let fleet = a_fleet(
        &home,
        &files,
        manifest(Some("echo deployed-to-qa")),
        Delivering::default(),
    );
    let id = to_the_delivering_step(&fleet, &home).await;

    use TriggerState::Passed;
    assert_eq!(
        firings(&fleet, &id).await,
        [
            ("begin", TriggerWhen::StepStarts, "implement", Passed),
            ("tidy", TriggerWhen::StepPasses, "implement", Passed),
            ("begin", TriggerWhen::StepStarts, "summarise", Passed),
            ("deploy", TriggerWhen::PrOpened, "summarise", Passed),
        ]
        .map(|(n, w, s, st)| (n.to_string(), w, s.to_string(), st)),
    );
    let job = fleet.load(&id).await.unwrap();
    let delivery = fleet.store().lock().await.delivery_for(&id).unwrap();
    assert!(delivery.pull_request.is_some(), "it opened before it fired");
    assert!(
        log(&fleet, &home, &job).contains("deployed-to-qa"),
        "what the Command printed is in the Job's log"
    );
}

#[tokio::test]
async fn a_command_that_fails_is_recorded_and_the_job_goes_on_as_it_would_have() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![deploys()]);
    let fleet = a_fleet(&home, &files, manifest(Some("false")), Delivering::default());
    let id = to_the_delivering_step(&fleet, &home).await;

    let held = fleet.store().lock().await.trigger_firings(&id).unwrap();
    let [only] = held.as_slice() else {
        panic!("one firing: {held:?}");
    };
    assert_eq!((only.state, only.exit_code), (TriggerState::Failed, Some(1)));

    let job = fleet.load(&id).await.unwrap();
    assert_eq!(job.status(), JobStatus::Running);
    assert_eq!(job.current_step_id().map(|s| s.as_str()), Some("summarise"));
    assert_eq!(
        job.step(job.current_step_id().unwrap()).unwrap().state(),
        StepState::Running
    );
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
}

#[tokio::test]
async fn a_repository_without_the_command_skips_it_and_a_destructive_one_waits() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![
        deploys(),
        machine("wipe.yml", "name: wipe\nwhen: pr_opened\ncommand: wipe\n"),
    ]);
    let fleet = a_fleet(&home, &files, manifest(None), Delivering::default());
    let id = to_the_delivering_step(&fleet, &home).await;

    let held = fleet.store().lock().await.trigger_firings(&id).unwrap();
    let state = |name: &str| held.iter().find(|f| f.name == name).unwrap().state;
    assert_eq!(state("deploy"), TriggerState::Skipped);
    assert_eq!(state("wipe"), TriggerState::AwaitingOwner);
}

#[tokio::test]
async fn no_pull_request_means_no_pr_opened_trigger() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![deploys()]);
    let delivering = Delivering {
        review: Opened::NoTool {
            why: String::from("no forge here"),
        },
        ..Delivering::default()
    };
    let fleet = a_fleet(&home, &files, manifest(Some("true")), delivering);
    let id = to_the_delivering_step(&fleet, &home).await;

    assert!(firings(&fleet, &id).await.is_empty());
}

#[tokio::test]
async fn a_trigger_saved_after_the_approval_does_not_touch_the_job() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![deploys()]);
    let fleet = a_fleet(&home, &files, manifest(Some("true")), Delivering::default());
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    files.say(vec![machine(
        "late.yml",
        "name: late\nwhen: pr_opened\ncommand: fmt\n",
    )]);
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let fired: Vec<_> = firings(&fleet, job.id()).await.into_iter().map(|f| f.0).collect();
    assert_eq!(fired, ["deploy"]);
}

#[tokio::test]
async fn the_job_lists_what_is_frozen_and_what_fired_and_each_change_is_published() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![
        machine("begin.yml", "name: begin\nwhen: step_starts\ncommand: fmt\n"),
        deploys(),
    ]);
    let fleet = a_fleet(&home, &files, manifest(Some("false")), Delivering::default());
    let mut heard = fleet.events().subscribe();
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    let wire = ipc::JobId::from(job.id());
    let detail = fleet.job_detail(wire.clone()).await.unwrap();
    let states: Vec<_> = detail
        .triggers
        .iter()
        .map(|t| (t.name.as_str(), t.step.as_str(), t.state))
        .collect();
    use ipc::TriggerFiringState::{Passed, Pending};
    assert_eq!(
        states,
        [
            ("begin", "implement", Passed),
            ("begin", "summarise", Pending),
            ("deploy", "summarise", Pending),
        ]
    );

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    let detail = fleet.job_detail(wire.clone()).await.unwrap();
    let deploy = detail.triggers.iter().find(|t| t.name == "deploy").unwrap();
    assert_eq!(deploy.state, ipc::TriggerFiringState::Failed);
    assert_eq!(deploy.exit_code, Some(1));
    assert!(deploy.log_at.is_some());

    let seen = crate::tests::under_review::published(&mut heard).await;
    let moves: Vec<_> = seen
        .iter()
        .filter_map(|event| match event {
            ipc::Event::JobTriggerChanged(changed) if changed.trigger.name == "deploy" => {
                Some(changed.trigger.state)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        moves,
        [ipc::TriggerFiringState::Running, ipc::TriggerFiringState::Failed],
        "one message to open and one to end: {seen:?}"
    );
    let line = log(&fleet, &home, &job);
    assert!(
        line.contains(deploy.log_at.as_ref().unwrap().as_str()),
        "the log line carries the instant the detail points at"
    );
}

#[tokio::test]
async fn a_saved_trigger_is_on_the_next_list_and_says_what_runs() {
    use api::{Authoring, Queries};
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![TriggerWritten::in_repository(
        ".armada/triggers/tidy.yml".into(),
        "name: tidy\nwhen: step_passes\ncommand: fmt\n".into(),
    )]);
    let fleet = a_fleet(&home, &files, manifest(None), Delivering::default());

    let mine = "name: tidy\nwhen: step_passes\ncommand: fmt\non_failure: {repair: true}\n";
    let saved = fleet
        .save_trigger(
            ipc::SaveTrigger {
                scope: ipc::TriggerScope::Machine,
                definition: mine.into(),
                overwrite: false,
            },
            None,
        )
        .await
        .expect("saved");
    assert_eq!(saved.runs_from, Some(ipc::TriggerLevel::Machine));
    assert!(!saved.waits_for_main, "a machine's is not waiting on anything");

    let listed = fleet.list_triggers(None).await.expect("listed");
    let [one] = listed.triggers.as_slice() else {
        panic!("one identity: {listed:?}");
    };
    assert_eq!(one.level, ipc::TriggerLevel::Machine);
    assert!(one.repair);
    assert_eq!(one.overrides.len(), 1);

    let theirs = fleet
        .get_trigger(
            ipc::TriggerMoment::StepPasses,
            None,
            "tidy".into(),
            Some(ipc::TriggerLevel::Repository),
            None,
        )
        .await
        .expect("the replaced copy can still be read");
    assert_eq!(theirs.overridden_by, Some(ipc::TriggerLevel::Machine));
    assert!(theirs.definition.contains("command: fmt"));
}

//! The values a project and Kit set: kept and listed key by key, read for a project and a repository with
//! the layer of each, and frozen on a Job when it starts. A Job in a project follows the project over its
//! repository, and keeps what it took when the project is edited later.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Review;
use api::{HostedSessions, Projects, Sessions};
use ipc::{
    EditProject, LookSet, ManifestId, NewProject, ProjectMember, SaveKitValues, SavePreference, SessionRecord,
    StartSession, ValueLayer, ValuesSet,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::daemon::fittings;
use super::main_ci::MANIFEST;
use super::projects::code_of;
use super::projects_kept::file;
use super::session_host::{Shared, StandIn};
use super::tmp::TempDir;
use crate::daemon::Fleet;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

struct Rig {
    fleet: Arc<Hosted>,
    home: TempDir,
}

/// A Fleet serving one repository whose `armada.yml` also says `repository`.
fn a_rig(repository: &str) -> Rig {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), &format!("{MANIFEST}{repository}")).expect("parses");
    let fleet = Fleet::assembled(fittings).hosting_sessions_on(Arc::new(Shared(Arc::new(StandIn::default()))), Duration::from_secs(600));
    Rig { fleet: Arc::new(fleet), home }
}

impl Rig {
    fn manifest(&self) -> ManifestId {
        ManifestId::carried(self.fleet.repositories().first().expect("served").manifest().id().as_str())
    }

    async fn project(&self, values: Option<ValuesSet>) -> String {
        let members = Some(vec![ProjectMember { manifest_id: self.manifest(), folder: None }]);
        let new = NewProject { name: "Shop".to_string(), members, place: None, values };
        self.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
    }

    async fn job(&self, project: Option<&str>) -> SessionRecord {
        let start = StartSession {
            project: project.map(str::to_string),
            manifest_id: self.manifest(),
            title: Some("Ship it".to_string()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
        };
        self.fleet.start_session(start).await.expect("started")
    }

    async fn auto_land(&self, job: &SessionRecord) -> bool {
        self.fleet.store().lock().await.hosting(job.id.as_str()).expect("read").expect("hosted").auto_land
    }

    fn kit_values(&self, text: &str) {
        std::fs::create_dir_all(self.home.path().join("kit")).expect("a folder");
        std::fs::write(self.home.path().join("kit/values.yml"), text).expect("Kit's values");
    }
}

fn set(pr_mode: Option<core_model::PrMode>, auto_merge: Option<&str>) -> ValuesSet {
    ValuesSet { pr_mode: pr_mode.map(Into::into), auto_merge: auto_merge.map(str::to_string), ..ValuesSet::default() }
}

fn from_of(values: &ipc::ResolvedValues) -> [ValueLayer; 6] {
    [values.pr_mode.from, values.auto_merge.from, values.review_model.from, values.layout.from, values.theme.from, values.density.from]
}

#[tokio::test]
async fn a_project_s_values_are_listed_as_set_replaced_whole_by_an_edit_and_kept_by_one_that_names_none() {
    let rig = a_rig("");
    let listed = |projects: &ipc::Projects| projects.projects.last().expect("a project").values.clone();
    let id = rig.project(Some(ValuesSet { look: Some(LookSet { density: Some("roomy".into()), ..LookSet::default() }), ..set(Some(core_model::PrMode::Draft), Some("checks-pass")) })).await;
    let listed_now = rig.fleet.list_projects().await.expect("listed");
    let first = listed(&listed_now).expect("its values");
    assert_eq!((first.pr_mode.map(|mode| mode.as_wire()), first.auto_merge.as_deref(), first.review_model.clone()), (Some("draft"), Some("checks-pass"), None));
    assert_eq!(first.look.and_then(|look| look.density).as_deref(), Some("roomy"), "only what it sets");

    // It is in the file, and read again from it.
    let text = std::fs::read_to_string(file(&rig.home, &id)).expect("its file");
    assert!(text.contains("values:") && text.contains("pr_mode: draft") && text.contains("density: roomy"), "{text}");
    rig.fleet.projects_reload().await;
    assert_eq!(listed(&rig.fleet.list_projects().await.expect("listed")), listed(&listed_now));

    // A rename leaves them.
    let renamed = rig.fleet.edit_project(id.clone(), EditProject { name: Some("Store".into()), ..Default::default() }).await.expect("edited");
    assert_eq!(listed(&renamed), listed(&listed_now), "an edit that names none leaves the values");

    // Values replace the whole block: the keys it does not name are taken away.
    let replaced = rig
        .fleet
        .edit_project(id.clone(), EditProject { values: Some(ValuesSet { review_model: Some("big-model".into()), ..ValuesSet::default() }), ..Default::default() })
        .await
        .expect("edited");
    let now = listed(&replaced).expect("its values");
    assert_eq!((now.review_model.as_deref(), now.pr_mode, now.auto_merge, now.look), (Some("big-model"), None, None, None));

    // An empty block takes every value back, and the file carries none.
    let cleared = rig.fleet.edit_project(id.clone(), EditProject { values: Some(ValuesSet::default()), ..Default::default() }).await.expect("edited");
    assert_eq!(listed(&cleared), None);
    assert!(!std::fs::read_to_string(file(&rig.home, &id)).expect("its file").contains("values"));
}

#[tokio::test]
async fn a_word_a_key_does_not_take_is_refused_and_the_file_is_not_written_over() {
    let rig = a_rig("");
    let id = rig.project(Some(set(None, Some("never")))).await;
    let before = std::fs::read_to_string(file(&rig.home, &id)).expect("its file");
    for (said, why) in [
        (set(None, Some("sometimes")), "an auto merge"),
        (ValuesSet { look: Some(LookSet { density: Some("huge".into()), ..LookSet::default() }), ..ValuesSet::default() }, "a density"),
        (ValuesSet { review_model: Some("  ".into()), ..ValuesSet::default() }, "a blank model"),
        (reviews_set("review", "sometimes", "off"), "a review"),
        (reviews_set("off", "off", "review_and_approve"), "a walk that is approved"),
        (agent_starts_set("later"), "an agent start"),
    ] {
        let refused = rig.fleet.edit_project(id.clone(), EditProject { values: Some(said), ..Default::default() }).await.expect_err(why);
        assert_eq!(code_of(refused), (422, "fleet.project_values_refused".to_string()), "{why}");
    }
    assert_eq!(std::fs::read_to_string(file(&rig.home, &id)).expect("its file"), before);
    let new = NewProject { name: "Other".into(), members: None, place: None, values: Some(set(None, Some("sometimes"))) };
    assert_eq!(code_of(rig.fleet.add_project(new).await.expect_err("refused")).1, "fleet.project_values_refused", "a new project is refused the same");
}

#[tokio::test]
async fn a_task_that_makes_a_repository_a_member_keeps_the_values() {
    let rig = a_rig("");
    let new = NewProject { name: "Shop".to_string(), members: None, place: None, values: Some(set(Some(core_model::PrMode::Draft), None)) };
    let id = rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone();
    assert!(rig.fleet.list_projects().await.expect("listed").projects[0].members.is_empty());
    let task = super::projects::new_task(rig.manifest(), Some(id), "Elsewhere");
    rig.fleet.add_project_task(task).await.expect("added");
    let read = rig.fleet.list_projects().await.expect("listed");
    assert_eq!(read.projects[0].members.len(), 1, "the task's repository became a member");
    assert_eq!(read.projects[0].values.as_ref().and_then(|values| values.pr_mode).map(|mode| mode.as_wire()), Some("draft"));
}

#[tokio::test]
async fn get_values_names_the_layer_each_key_came_from_for_a_project_a_repository_or_both() {
    let rig = a_rig("pr_mode: draft\nauto_merge: never\n");
    // The Machine says ready, which the repository's draft is nearer than.
    rig.fleet.save_preference_now(SavePreference { name: "draft_pull_requests".into(), value: false, text: None }).await.expect("saved");
    rig.kit_values("version: 1\nvalues:\n  auto_merge: always\n  look:\n    theme: paper\n    density: roomy\n");
    let values = ValuesSet { look: Some(LookSet { density: Some("compact".into()), ..LookSet::default() }), ..set(None, Some("checks-pass")) };
    let project = rig.project(Some(values)).await;
    let (repository, ask) = (rig.manifest(), |project: Option<&str>, repository: Option<&ManifestId>| {
        rig.fleet.get_values(project.map(str::to_string), repository.cloned())
    });

    let neither = ask(None, None).await.expect("values");
    assert_eq!((neither.pr_mode.value.as_deref(), neither.pr_mode.from), (Some("ready"), ValueLayer::Machine));
    assert_eq!((neither.auto_merge.value.as_deref(), neither.auto_merge.from), (Some("always"), ValueLayer::Kit));
    assert_eq!((neither.theme.value.as_deref(), neither.theme.from), (Some("paper"), ValueLayer::Kit));
    assert_eq!((neither.density.value.as_deref(), neither.density.from), (Some("roomy"), ValueLayer::Kit));
    assert_eq!(neither.layout, ipc::ResolvedValue { value: None, from: ValueLayer::Default }, "a key nothing sets and nothing ships");
    assert!(neither.autonomy.is_empty(), "no repository was asked about");

    let project_only = ask(Some(&project), None).await.expect("values");
    assert_eq!((project_only.auto_merge.value.as_deref(), project_only.auto_merge.from), (Some("checks-pass"), ValueLayer::Project));
    assert_eq!((project_only.density.value.as_deref(), project_only.density.from), (Some("compact"), ValueLayer::Project));
    assert_eq!(project_only.theme.from, ValueLayer::Kit, "a key the project does not set is Kit's");

    let repository_only = ask(None, Some(&repository)).await.expect("values");
    assert_eq!((repository_only.pr_mode.value.as_deref(), repository_only.pr_mode.from), (Some("draft"), ValueLayer::Repository));
    assert_eq!((repository_only.auto_merge.value.as_deref(), repository_only.auto_merge.from), (Some("never"), ValueLayer::Repository), "a repository that says never is nearer than Kit's always");
    assert_eq!(repository_only.autonomy.len(), core_model::rule::ActKind::ALL.len());

    let both = ask(Some(&project), Some(&repository)).await.expect("values");
    assert_eq!((both.pr_mode.from, both.auto_merge.from, both.density.from, both.theme.from), (ValueLayer::Repository, ValueLayer::Project, ValueLayer::Project, ValueLayer::Kit));
    assert_eq!(both.auto_merge.value.as_deref(), Some("checks-pass"), "the project's word replaces the repository's");
    assert_eq!(from_of(&both)[2], ValueLayer::Default);
}

#[tokio::test]
async fn autonomy_is_named_for_the_layer_of_the_piece_that_sets_it() {
    let rig = a_rig("");
    let project = rig.project(None).await;
    let repository = rig.manifest();
    let piece = |id: &str, scope: Option<&str>, body: &str| {
        let scope = scope.map(|one| format!("scope: {one}\n")).unwrap_or_default();
        let folder = rig.home.path().join("kit/autonomy");
        std::fs::create_dir_all(&folder).expect("a folder");
        std::fs::write(folder.join(format!("{id}.yml")), format!("version: 1\nautonomy_id: {id}\n{scope}defaults:\n{body}")).expect("a piece");
    };
    piece("everywhere", None, "  tracker_write: acts\n  forge_write: acts\n");
    piece("mine", Some(&project), "  forge_write: asks\n  draft: off\n");
    piece("here", Some(repository.as_str()), "  draft: asks\n");

    let values = rig.fleet.get_values(Some(project.clone()), Some(repository.clone())).await.expect("values");
    let of = |kind: &str| values.autonomy.iter().find(|one| one.kind == kind).map(|one| (one.value.as_str(), one.from)).expect("answered");
    assert_eq!(of("tracker_write"), ("acts", ValueLayer::Kit));
    assert_eq!(of("forge_write"), ("asks", ValueLayer::Project));
    assert_eq!(of("draft"), ("asks", ValueLayer::Repository));
    assert_eq!(of("start_job").1, ValueLayer::Default);
    let without = rig.fleet.get_values(None, Some(repository)).await.expect("values");
    assert_eq!(without.autonomy.iter().find(|one| one.kind == "forge_write").map(|one| one.from), Some(ValueLayer::Kit), "no project, no project layer");
}

#[tokio::test]
async fn get_values_refuses_a_project_there_is_not_one_that_does_not_read_and_one_that_does_not_hold_the_repository() {
    let rig = a_rig("");
    let missing = rig.fleet.get_values(Some("nope".into()), None).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project".to_string()));
    let elsewhere = rig.fleet.add_project(NewProject { name: "Elsewhere".into(), members: Some(vec![ProjectMember { manifest_id: ManifestId::carried("other"), folder: None }]), place: None, values: None }).await.expect("added").projects[0].id.clone();
    let not_member = rig.fleet.get_values(Some(elsewhere.clone()), Some(rig.manifest())).await.expect_err("refused");
    assert_eq!(code_of(not_member), (422, "fleet.project_not_member".to_string()));
    std::fs::remove_file(file(&rig.home, &elsewhere)).expect("its file");
    rig.fleet.projects_reload().await;
    let unread = rig.fleet.get_values(Some(elsewhere), None).await.expect_err("refused");
    assert_eq!(code_of(unread), (422, "fleet.project_unreadable".to_string()));
}

#[tokio::test]
async fn kit_s_values_are_saved_whole_read_back_and_a_bad_word_writes_nothing() {
    let rig = a_rig("");
    let saved = rig.fleet.save_kit_values(SaveKitValues { values: set(Some(core_model::PrMode::Draft), Some("always")) }).await.expect("saved");
    assert_eq!(saved.values.auto_merge.as_deref(), Some("always"));
    let text = std::fs::read_to_string(rig.home.path().join("kit/values.yml")).expect("its file");
    assert!(text.contains("auto_merge: always") && text.contains("pr_mode: draft"), "{text}");
    let read = rig.fleet.get_values(None, None).await.expect("values");
    assert_eq!((read.pr_mode.value.as_deref(), read.pr_mode.from), (Some("draft"), ValueLayer::Kit));
    let silent = rig.fleet.get_values(None, Some(rig.manifest())).await.expect("values");
    assert_eq!((silent.auto_merge.value.as_deref(), silent.auto_merge.from), (Some("always"), ValueLayer::Kit), "a repository that writes nothing does not beat Kit");

    let whole = rig.fleet.save_kit_values(SaveKitValues { values: set(None, Some("never")) }).await.expect("saved");
    assert_eq!(whole.values.pr_mode, None, "a key left out is taken away");
    let refused = rig.fleet.save_kit_values(SaveKitValues { values: set(None, Some("sometimes")) }).await.expect_err("refused");
    assert_eq!(code_of(refused), (422, "fleet.project_values_refused".to_string()));
    assert!(std::fs::read_to_string(rig.home.path().join("kit/values.yml")).expect("its file").contains("never"), "the file is as it was");
    std::fs::write(rig.home.path().join("kit/values.yml"), "version: 9\n").expect("a file that will not read");
    assert_eq!(rig.fleet.get_values(None, None).await.expect("values").pr_mode.from, ValueLayer::Default, "a file that will not read sets nothing");
}

#[tokio::test]
async fn a_job_freezes_its_values_at_start_and_a_later_edit_of_the_project_does_not_move_them() {
    let rig = a_rig("");
    let project = rig.project(Some(set(Some(core_model::PrMode::Draft), Some("checks-pass")))).await;
    let job = rig.job(Some(&project)).await;
    let frozen = job.hosted.as_ref().and_then(|hosted| hosted.values.clone()).expect("the Job's values");
    assert_eq!((frozen.pr_mode.value.as_deref(), frozen.pr_mode.from), (Some("draft"), ValueLayer::Project));
    assert_eq!((frozen.auto_merge.value.as_deref(), frozen.auto_merge.from), (Some("checks-pass"), ValueLayer::Project));
    assert_eq!(frozen.autonomy.len(), core_model::rule::ActKind::ALL.len());

    rig.fleet.edit_project(project.clone(), EditProject { values: Some(set(Some(core_model::PrMode::Ready), Some("never"))), ..Default::default() }).await.expect("edited");
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed");
    let again = listed.sessions.iter().find(|one| one.id == job.id).and_then(|one| one.hosted.as_ref()).and_then(|hosted| hosted.values.clone());
    assert_eq!(again, Some(frozen.clone()), "the Job keeps what it took");
    let next = rig.job(Some(&project)).await;
    let taken = next.hosted.and_then(|hosted| hosted.values).expect("the next Job's values");
    assert_eq!((taken.pr_mode.value.as_deref(), taken.auto_merge.value.as_deref()), (Some("ready"), Some("never")), "a Job started after takes the edit");
    assert_eq!(rig.fleet.job_values(job.id.as_str()).await.and_then(|held| held.pr_mode()), Some(core_model::PrMode::Draft));
}

#[tokio::test]
async fn a_job_in_no_project_freezes_its_repository_s_values() {
    let rig = a_rig("pr_mode: draft\n");
    let job = rig.job(None).await;
    let frozen = job.hosted.and_then(|hosted| hosted.values).expect("the Job's values");
    assert_eq!((frozen.pr_mode.value.as_deref(), frozen.pr_mode.from), (Some("draft"), ValueLayer::Repository));
    assert_eq!(frozen.auto_merge.from, ValueLayer::Default, "a repository that says nothing leaves the default");
}

/// A member may be any folder of a repository: the folder carries no `armada.yml` of its own, so the
/// repository's word is the one read for it, and a Job started for the project freezes that word.
#[tokio::test]
async fn a_folder_member_with_no_armada_yml_takes_its_repositorys_values() {
    let rig = a_rig("pr_mode: draft\n");
    let root = rig.fleet.served_named(None).expect("served").root().to_string();
    std::fs::create_dir_all(Path::new(&root).join("services/api")).expect("a folder");
    assert!(!Path::new(&root).join("services/api/armada.yml").exists());
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest(), folder: Some("services/api".to_string()) }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    let project = rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone();

    let values = rig.fleet.get_values(Some(project.clone()), Some(rig.manifest())).await.expect("values");
    assert_eq!((values.pr_mode.value.as_deref(), values.pr_mode.from), (Some("draft"), ValueLayer::Repository));

    let frozen = rig.job(Some(&project)).await.hosted.and_then(|hosted| hosted.values).expect("the Job's values");
    assert_eq!((frozen.pr_mode.value.as_deref(), frozen.pr_mode.from), (Some("draft"), ValueLayer::Repository));
}

#[tokio::test]
async fn a_job_in_a_project_offers_its_pull_request_as_the_project_says_over_the_repository() {
    let offered = |rig: &Rig, job: &SessionRecord| {
        let (fleet, id) = (Arc::clone(&rig.fleet), job.id.as_str().to_string());
        async move { fleet.review_as_offered_by(&id, Review::assembled("Title", "Body")).await.draft() }
    };
    // The repository says ready; the project says draft.
    let rig = a_rig("pr_mode: ready\n");
    let project = rig.project(Some(set(Some(core_model::PrMode::Draft), None))).await;
    assert!(offered(&rig, &rig.job(Some(&project)).await).await, "the project's draft is over the repository's ready");
    assert!(!offered(&rig, &rig.job(None).await).await, "a Job in no project runs on the repository's");
    // The repository says draft; the project says ready.
    let rig = a_rig("pr_mode: draft\n");
    let project = rig.project(Some(set(Some(core_model::PrMode::Ready), None))).await;
    assert!(!offered(&rig, &rig.job(Some(&project)).await).await, "the project's ready is over the repository's draft");
    assert!(offered(&rig, &rig.job(None).await).await);
    // Nothing above the Machine speaks: the Machine's word stands, and is read as it is now.
    let rig = a_rig("");
    let job = rig.job(None).await;
    assert!(!offered(&rig, &job).await);
    rig.fleet.save_preference_now(SavePreference { name: "draft_pull_requests".into(), value: true, text: None }).await.expect("saved");
    assert!(offered(&rig, &job).await, "the Machine's setting stays live for a Job");
}

#[tokio::test]
async fn a_job_starts_with_auto_land_off_whatever_auto_merge_resolves_to() {
    let rig = a_rig("auto_merge: checks-pass\n");
    assert!(!rig.auto_land(&rig.job(None).await).await, "a repository that says checks-pass does not turn it on");
    let always = rig.project(Some(set(None, Some("always")))).await;
    let job = rig.job(Some(&always)).await;
    assert!(!rig.auto_land(&job).await, "nor does a project that says always");
    let frozen = rig.fleet.job_values(job.id.as_str()).await.expect("frozen");
    assert_eq!(frozen.wire().auto_merge.value.as_deref(), Some("always"), "the value is still frozen, and read where something reads it");
}

#[tokio::test]
async fn a_job_reviews_on_the_model_its_project_set_and_a_job_with_none_reads_models_review() {
    let rig = a_rig("");
    let project = rig.project(Some(ValuesSet { review_model: Some("project-model".into()), ..ValuesSet::default() })).await;
    let job = rig.job(Some(&project)).await;
    assert_eq!(rig.fleet.review_model_of_job(job.id.as_str()).await.as_deref(), Some("project-model"));
    rig.fleet.edit_project(project, EditProject { values: Some(ValuesSet::default()), ..Default::default() }).await.expect("edited");
    assert_eq!(rig.fleet.review_model_of_job(job.id.as_str()).await.as_deref(), Some("project-model"), "it keeps what it took");
    let plain = rig.job(None).await;
    assert_eq!(rig.fleet.review_model_of_job(plain.id.as_str()).await, None, "nothing above the Machine set one, so `models.review` stands");
}

fn reviews_set(plans: &str, pull_requests: &str, walks: &str) -> ValuesSet {
    let word = |word: &str| Some(word.to_string());
    ValuesSet { reviews: Some(ipc::ReviewsSet { plans: word(plans), pull_requests: word(pull_requests), walks: word(walks) }), ..ValuesSet::default() }
}

#[tokio::test]
async fn a_job_freezes_how_its_project_reviews_work_and_keeps_it_when_the_project_is_edited() {
    let rig = a_rig("");
    let project = rig.project(Some(reviews_set("review_and_approve", "review", "off"))).await;
    let job = rig.job(Some(&project)).await;
    let said = |job: &SessionRecord| job.hosted.clone().and_then(|hosted| hosted.values).and_then(|values| values.reviews).expect("reviews");
    let first = said(&job);
    assert_eq!((first.plans.as_str(), first.pull_requests.as_str(), first.walks.as_str(), first.from), ("review_and_approve", "review", "off", ValueLayer::Project));
    rig.fleet.edit_project(project, EditProject { values: Some(reviews_set("off", "off", "review")), ..Default::default() }).await.expect("edited");
    let frozen = rig.fleet.job_values(job.id.as_str()).await.expect("frozen").reviews();
    assert_eq!((frozen.plans, frozen.pull_requests), (core_model::ReviewMode::ReviewAndApprove, core_model::ReviewMode::Review), "it keeps what it took");
    let plain = rig.job(None).await;
    assert_eq!(said(&plain).plans, "off", "a Job in no project is reviewed by nothing");
    assert_eq!(said(&plain).from, ValueLayer::Default);
}

fn agent_starts_set(word: &str) -> ValuesSet {
    ValuesSet { agent_starts: Some(word.to_string()), ..ValuesSet::default() }
}

#[tokio::test]
async fn a_project_that_says_ask_me_holds_the_starts_made_for_it_and_every_other_case_starts_at_once() {
    let rig = a_rig("");
    let repository = rig.manifest();
    let asks = rig.project(Some(agent_starts_set("ask_me"))).await;
    assert!(rig.fleet.agent_start_held(Some(&asks), repository.as_str()).await, "the project's word");
    assert!(!rig.fleet.agent_start_held(None, repository.as_str()).await, "no project, and the shipped word is at once");
    assert!(!rig.fleet.agent_start_held(Some("no-such-project"), repository.as_str()).await, "a project that is not there falls back to none");
    let plain = rig.project(Some(agent_starts_set("at_once"))).await;
    assert!(!rig.fleet.agent_start_held(Some(&plain), repository.as_str()).await);
    rig.kit_values("version: 1\nvalues:\n  agent_starts: ask_me\n");
    assert!(rig.fleet.agent_start_held(None, repository.as_str()).await, "Kit's word where no project speaks");
    let listed = rig.fleet.values_asked(Some(asks), Some(repository)).await.expect("read").agent_starts.expect("the key");
    assert_eq!((listed.value.as_deref(), listed.from), (Some("ask_me"), ValueLayer::Project));
}

const KIT_REVIEWS: &str = "version: 1\nvalues:\n  reviews:\n    pull_requests: review\n    walks: review\n";

fn plans_only(word: &str) -> ValuesSet {
    ValuesSet { reviews: Some(ipc::ReviewsSet { plans: Some(word.to_string()), ..Default::default() }), ..ValuesSet::default() }
}

async fn reviews_of(rig: &Rig, project: &str) -> ipc::ResolvedReviews {
    rig.fleet.get_values(Some(project.to_string()), None).await.expect("values").reviews.expect("reviews")
}

fn kinds(reviews: &ipc::ResolvedReviews) -> [(&str, Option<ValueLayer>); 3] {
    [(reviews.plans.as_str(), reviews.plans_from), (reviews.pull_requests.as_str(), reviews.pull_requests_from), (reviews.walks.as_str(), reviews.walks_from)]
}

#[tokio::test]
async fn a_project_setting_one_review_kind_leaves_the_others_to_kit() {
    let rig = a_rig("");
    rig.kit_values(KIT_REVIEWS);
    let project = rig.project(Some(plans_only("review_and_approve"))).await;
    let reviews = reviews_of(&rig, &project).await;
    assert_eq!(
        kinds(&reviews),
        [("review_and_approve", Some(ValueLayer::Project)), ("review", Some(ValueLayer::Kit)), ("review", Some(ValueLayer::Kit))]
    );
    assert_eq!(reviews.from, ValueLayer::Project, "from is the nearest layer that set any kind");
}

#[tokio::test]
async fn a_kind_no_layer_sets_is_off_from_the_default() {
    let rig = a_rig("");
    let project = rig.project(Some(plans_only("review"))).await;
    let reviews = reviews_of(&rig, &project).await;
    assert_eq!(kinds(&reviews), [("review", Some(ValueLayer::Project)), ("off", Some(ValueLayer::Default)), ("off", Some(ValueLayer::Default))]);
    let bare = rig.project(None).await;
    let none = reviews_of(&rig, &bare).await;
    assert_eq!(kinds(&none), [("off", Some(ValueLayer::Default)); 3]);
    assert_eq!(none.from, ValueLayer::Default);
}

#[tokio::test]
async fn an_all_three_block_reads_as_before() {
    let rig = a_rig("");
    rig.kit_values(KIT_REVIEWS);
    let project = rig.project(Some(reviews_set("review_and_approve", "off", "off"))).await;
    let reviews = reviews_of(&rig, &project).await;
    assert_eq!(kinds(&reviews), [("review_and_approve", Some(ValueLayer::Project)), ("off", Some(ValueLayer::Project)), ("off", Some(ValueLayer::Project))], "a word of off is a word, and Kit's is not read");
    assert_eq!(reviews.from, ValueLayer::Project);
}

#[tokio::test]
async fn clearing_one_kind_on_a_project_falls_back_to_kit() {
    let rig = a_rig("");
    rig.kit_values(KIT_REVIEWS);
    let project = rig.project(Some(reviews_set("review_and_approve", "off", "off"))).await;
    let without = ValuesSet { reviews: Some(ipc::ReviewsSet { plans: Some("review_and_approve".into()), pull_requests: None, walks: Some("off".into()) }), ..ValuesSet::default() };
    rig.fleet.edit_project(project.clone(), EditProject { values: Some(without), ..Default::default() }).await.expect("edited");
    let reviews = reviews_of(&rig, &project).await;
    assert_eq!(
        kinds(&reviews),
        [("review_and_approve", Some(ValueLayer::Project)), ("review", Some(ValueLayer::Kit)), ("off", Some(ValueLayer::Project))]
    );
    let text = std::fs::read_to_string(file(&rig.home, &project)).expect("its file");
    assert!(!text.contains("pull_requests"), "the cleared kind is not in the file: {text}");
}

#[tokio::test]
async fn a_block_with_no_kinds_is_dropped_from_the_file() {
    let rig = a_rig("");
    let empty = ValuesSet { reviews: Some(ipc::ReviewsSet::default()), ..set(None, Some("never")) };
    let project = rig.project(Some(empty)).await;
    let text = std::fs::read_to_string(file(&rig.home, &project)).expect("its file");
    assert!(text.contains("auto_merge") && !text.contains("reviews"), "{text}");
    let listed = rig.fleet.list_projects().await.expect("listed").projects.last().expect("a project").values.clone().expect("its values");
    assert_eq!(listed.reviews, None, "a block naming no kind is no block");
}

#[tokio::test]
async fn a_job_freezes_each_kind_with_its_own_layer() {
    let rig = a_rig("");
    rig.kit_values(KIT_REVIEWS);
    let project = rig.project(Some(plans_only("review_and_approve"))).await;
    let job = rig.job(Some(&project)).await;
    let said = job.hosted.clone().and_then(|hosted| hosted.values).and_then(|values| values.reviews).expect("reviews");
    assert_eq!(
        kinds(&said),
        [("review_and_approve", Some(ValueLayer::Project)), ("review", Some(ValueLayer::Kit)), ("review", Some(ValueLayer::Kit))]
    );
    assert_eq!(said.from, ValueLayer::Project);
    let frozen = rig.fleet.job_values(job.id.as_str()).await.expect("frozen");
    let words = frozen.reviews();
    assert_eq!((words.plans, words.pull_requests, words.walks), (core_model::ReviewMode::ReviewAndApprove, core_model::ReviewMode::Review, core_model::WalkReview::Review));
    assert_eq!(frozen.wire().reviews.expect("reviews"), said, "the frozen rows read back as they were said");
    let plain = rig.job(None).await;
    let plain = plain.hosted.and_then(|hosted| hosted.values).and_then(|values| values.reviews).expect("reviews");
    assert_eq!(
        kinds(&plain),
        [("off", Some(ValueLayer::Default)), ("review", Some(ValueLayer::Kit)), ("review", Some(ValueLayer::Kit))],
        "a Job in no project still has Kit's kinds"
    );
}

fn delivery_set(word: &str) -> ValuesSet {
    ValuesSet { delivery: Some(word.to_string()), ..ValuesSet::default() }
}

#[tokio::test]
async fn delivery_takes_four_words_and_refuses_another() {
    let rig = a_rig("");
    let id = rig.project(None).await;
    for word in ["auto_land", "open_pr", "keep_local", "ask"] {
        let edited = rig.fleet.edit_project(id.clone(), EditProject { values: Some(delivery_set(word)), ..Default::default() }).await.expect(word);
        let held = edited.projects.last().and_then(|one| one.values.clone()).and_then(|values| values.delivery);
        assert_eq!(held.as_deref(), Some(word), "{word} is kept");
    }
    let before = std::fs::read_to_string(file(&rig.home, &id)).expect("its file");
    for word in ["land", "at_once", "AUTO_LAND", " "] {
        let refused = rig.fleet.edit_project(id.clone(), EditProject { values: Some(delivery_set(word)), ..Default::default() }).await.expect_err(word);
        assert_eq!(code_of(refused), (422, "fleet.project_values_refused".to_string()), "{word:?}");
    }
    assert_eq!(std::fs::read_to_string(file(&rig.home, &id)).expect("its file"), before, "nothing was written");
    let saved = rig.fleet.save_kit_values(SaveKitValues { values: delivery_set("later") }).await.expect_err("Kit refuses it too");
    assert_eq!(code_of(saved).1, "fleet.project_values_refused");
}

#[tokio::test]
async fn a_projects_delivery_beats_kits_and_the_answer_names_the_layer() {
    let rig = a_rig("");
    let repository = rig.manifest();
    let unset = rig.project(None).await;
    let set = rig.project(Some(delivery_set("auto_land"))).await;
    let asked = |project: Option<String>| rig.fleet.values_asked(project, Some(repository.clone()));

    let nothing = asked(Some(unset.clone())).await.expect("read").delivery.expect("the key");
    assert_eq!((nothing.value, nothing.from), (None, ValueLayer::Default), "no layer sets it");
    rig.kit_values("version: 1\nvalues:\n  delivery: keep_local\n");
    let kits = asked(Some(unset)).await.expect("read").delivery.expect("the key");
    assert_eq!((kits.value.as_deref(), kits.from), (Some("keep_local"), ValueLayer::Kit));
    let projects = asked(Some(set)).await.expect("read").delivery.expect("the key");
    assert_eq!((projects.value.as_deref(), projects.from), (Some("auto_land"), ValueLayer::Project));
}

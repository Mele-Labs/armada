//! What applies to a repository and what a Job holds, read through `api::Lifecycles`: the right folders,
//! the files set aside with their sentences, a Job's instances with their labels, links and moves, and
//! `job_lifecycles.changed` raised when what a Job holds changes and at no other time. **No forge,
//! tracker or process is reached**: the fakes say what the sources say.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::PullRequestStanding;
use api::{Lifecycles, Next};
use ipc::{AutonomyToSave, Event, InstanceStanding, LifecyclePiece, PieceScope, SaveLifecyclePiece, JobLifecycles, ManifestId, RuleAct, RuleAmong, RuleWhen, SessionId, StateReadFrom};
use store::{rule_act_standing, AttachmentState, Holder, KeptAttachment, NewRuleAct};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::pull_requesting::{facts, forge_ran, passed};
use super::session_host::{rig_placing, Rig};
use crate::daemon::Fleet;
use crate::repositories::{Located, Served};
use crate::tests::daemon::a_fleet;
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn write(path: PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
    std::fs::write(path, text).expect("its file");
}

fn kit(home: &TempDir, kind: &str, file: &str) -> PathBuf {
    home.path().join("kit").join(kind).join(file)
}

fn armada(root: &str, kind: &str, file: &str) -> PathBuf {
    Path::new(root).join(".armada").join(kind).join(file)
}

fn first(fleet: &Fixture) -> Served {
    fleet.repositories().first().expect("the fixture's repository")
}

fn another(fleet: &Fixture, home: &TempDir, name: &str) -> Served {
    let root = home.path().join(name).to_string_lossy().to_string();
    std::fs::create_dir_all(&root).expect("a folder");
    fleet.repositories().add(Located { records_root: format!("{root}-records"), root: root.clone(), set_up: None }).expect("served");
    Served::hosting(&fleet.repositories().at(&root).expect("held"))
}

fn named(served: &Served) -> Option<ManifestId> {
    Some(ManifestId::from(served.manifest().id()))
}

fn wire_id(served: &Served) -> ManifestId {
    ManifestId::from(served.manifest().id())
}

fn ticket(scope: Option<&str>, word: &str) -> String {
    let scope = scope.map(|one| format!("scope: {one}\n")).unwrap_or_default();
    format!(
        "version: 1\nlifecycle_id: ticket\nname: Ticket\n{scope}states:\n  - {{ id: in_review, label: In review, read: {{ tracker: [{word}] }} }}\n  - {{ id: closed, label: Closed, read: {{ tracker: [Done] }} }}\nends: [closed]\n"
    )
}

fn my_store(holding: &[&Served]) -> String {
    let listed: String = holding.iter().map(|one| format!("  - {}\n", one.manifest().id().as_str())).collect();
    format!("version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n{listed}")
}

fn moves_tickets(scope: Option<&str>, to: &str, autonomy: Option<&str>) -> String {
    let scope = scope.map(|one| format!("scope: {one}\n")).unwrap_or_default();
    let autonomy = autonomy.map(|one| format!("autonomy: {one}\n")).unwrap_or_default();
    format!(
        "version: 1\nrule_id: plan_moves_tickets\nname: Move tickets\n{scope}when: {{ step_starts: plan }}\ndo:\n  - tracker_write: {{ move: {to}, on: {{ held: ticket }} }}\n{autonomy}"
    )
}

#[tokio::test]
async fn a_member_is_told_the_workspace_and_the_repository_it_is_read_from() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (web, api, back_office) = (first(&fleet), another(&fleet, &home, "store-api"), another(&fleet, &home, "back-office"));
    write(armada(web.root(), "workspaces", "my-store.yml"), &my_store(&[&web, &api]));

    let seen = fleet.list_repository_workspaces(named(&api)).await.expect("read");
    assert_eq!(seen.manifest_id, wire_id(&api));
    let [store] = seen.workspaces.as_slice() else { panic!("one: {:?}", seen.workspaces) };
    assert_eq!((store.id.as_str(), store.name.as_str(), store.source.as_str()), ("my-store", "My store", "repository"));
    assert_eq!(store.repositories, [wire_id(&web), wire_id(&api)], "in the order the file lists them");
    assert_eq!(store.home, Some(wire_id(&web)), "the member reads it from the repository that commits it");
    assert!(store.path.ends_with("my-store.yml") && store.overridden_by.is_none() && store.hides.is_none());

    let outside = fleet.list_repository_workspaces(named(&back_office)).await.expect("read");
    assert!(outside.workspaces.is_empty(), "{:?}", outside.workspaces);
    let at_home = fleet.list_repository_workspaces(named(&web)).await.expect("read");
    assert_eq!(at_home.workspaces.len(), 1);
    assert_eq!(at_home.workspaces[0].home, Some(wire_id(&web)));
}

#[tokio::test]
async fn yours_in_kit_runs_over_the_repositorys_workspace_and_a_broken_file_is_named_with_its_sentence() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (web, api) = (first(&fleet), another(&fleet, &home, "store-api"));
    write(armada(web.root(), "workspaces", "my-store.yml"), &my_store(&[&web, &api]));
    write(kit(&home, "workspaces", "my-store.yml"), &my_store(&[&web]));
    write(kit(&home, "workspaces", "broken.yml"), "not: a workspace\n");

    let seen = fleet.list_repository_workspaces(named(&web)).await.expect("read");
    let (mine, theirs) = {
        let by = |source: &str| seen.workspaces.iter().find(|one| one.source == source).unwrap_or_else(|| panic!("no {source}: {:?}", seen.workspaces));
        (by("kit"), by("repository"))
    };
    assert_eq!(seen.workspaces.len(), 2);
    assert_eq!((mine.home.as_ref(), mine.overridden_by.as_deref(), mine.hides.as_deref()), (None, None, Some("repository")));
    assert_eq!(theirs.home, Some(wire_id(&web)));
    assert_eq!(theirs.overridden_by.as_deref(), Some("kit"));

    let [left] = seen.left_out.as_slice() else { panic!("one set aside: {:?}", seen.left_out) };
    assert!(left.source == "kit" && left.path.ends_with("broken.yml") && !left.said.is_empty(), "{left:?}");
}

#[tokio::test]
async fn the_lifecycles_that_apply_say_where_each_was_kept_and_which_one_runs() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let repository = first(&fleet);
    write(kit(&home, "lifecycles", "tickets.yml"), &ticket(None, "Mine"));
    write(armada(repository.root(), "lifecycles", "tickets.yml"), &ticket(None, "Theirs"));
    write(kit(&home, "lifecycles", "broken.yml"), "version: 1\nlifecycle_id: ticket\n");

    let read = fleet.list_lifecycles(named(&repository)).await.expect("read");
    assert_eq!(read.manifest_id, wire_id(&repository));
    let mut ids: Vec<&str> = read.lifecycles.iter().map(|one| one.lifecycle.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids, ["deploy", "job", "pr", "ticket"], "what Armada carries is listed beside what was kept");

    let tickets: Vec<_> = read.lifecycles.iter().filter(|one| one.lifecycle.id == "ticket").collect();
    let places: Vec<&str> = tickets.iter().map(|one| one.kept.rank.source.as_str()).collect();
    assert_eq!(places, ["kit", "repository", "armada"], "the one that runs first");
    let running = tickets[0];
    assert_eq!(running.lifecycle.states[0].read, StateReadFrom::Tracker { words: vec!["Mine".to_string()] });
    assert_eq!(running.lifecycle.ends, ["closed"]);
    assert_eq!(running.kept.rank.reach, "everywhere");
    assert_eq!(running.kept.hides.as_deref(), Some("repository"));
    assert!(running.kept.overridden_by.is_none() && running.kept.path.as_deref().is_some_and(|path| path.ends_with("tickets.yml")));
    assert_eq!(tickets[1].kept.overridden_by.as_ref().map(|rank| rank.source.as_str()), Some("kit"));
    assert!(tickets[2].kept.path.is_none(), "what Armada carries is built and not read from a file");

    let [left] = read.left_out.as_slice() else { panic!("one set aside: {:?}", read.left_out) };
    assert!(left.source == "kit" && left.path.ends_with("broken.yml") && !left.said.is_empty(), "{left:?}");
}

#[tokio::test]
async fn a_lifecycle_scoped_to_a_workspace_says_so_and_a_member_reads_it_from_the_home() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (web, api) = (first(&fleet), another(&fleet, &home, "store-api"));
    write(armada(web.root(), "workspaces", "my-store.yml"), &my_store(&[&web, &api]));
    write(armada(web.root(), "lifecycles", "shared.yml"), &ticket(Some("my-store"), "Shared"));

    let read = fleet.list_lifecycles(named(&api)).await.expect("read");
    let shared = read.lifecycles.iter().find(|one| one.kept.rank.home).expect("listed as the home's");
    assert_eq!(shared.lifecycle.scope, ipc::PieceScope::Within { id: "my-store".to_string() });
    assert_eq!(shared.kept.rank.reach, "workspace");
    assert!(shared.kept.path.as_deref().is_some_and(|path| path.starts_with(web.root())));
}

#[tokio::test]
async fn the_rules_that_apply_say_their_moment_their_acts_and_how_far_each_goes_on_its_own() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let repository = first(&fleet);
    write(kit(&home, "rules", "moves.yml"), &moves_tickets(None, "closed", Some("acts")));
    write(armada(repository.root(), "rules", "moves.yml"), &moves_tickets(None, "in_review", None));
    write(kit(&home, "rules", "typo.yml"), &moves_tickets(None, "in_progres", None).replace("plan_moves_tickets", "typo"));

    let read = fleet.list_lifecycle_rules(named(&repository)).await.expect("read");
    assert_eq!(read.manifest_id, wire_id(&repository));
    let [running, beaten] = read.rules.as_slice() else { panic!("both are listed: {:?}", read.rules) };
    assert_eq!(running.kept.rank.source, "kit");
    assert_eq!(running.kept.hides.as_deref(), Some("repository"));
    assert_eq!(beaten.kept.overridden_by.as_ref().map(|rank| rank.source.as_str()), Some("kit"));

    let rule = &running.rule;
    assert_eq!((rule.id.as_str(), rule.when.clone()), ("plan_moves_tickets", RuleWhen::StepStarts { step: "plan".to_string() }));
    assert_eq!(rule.autonomy.as_deref(), Some("acts"));
    let [doing] = rule.does.as_slice() else { panic!("one act: {:?}", rule.does) };
    let RuleAct::TrackerMove { to, target } = &doing.what else { panic!("a move: {:?}", doing.what) };
    assert_eq!(to, "closed");
    assert_eq!(target.among, RuleAmong::Held { lifecycle: "ticket".to_string() });
    assert_eq!((doing.kind.as_str(), doing.autonomy.as_str()), ("tracker_write", "acts"), "the rule's own word");
    assert_eq!(beaten.rule.does[0].autonomy, "asks", "a write to a tracker asks unless the rule says otherwise");
    assert!(beaten.rule.autonomy.is_none());

    let [left] = read.left_out.as_slice() else { panic!("one set aside: {:?}", read.left_out) };
    assert!(left.path.ends_with("typo.yml") && left.said.contains("`in_progres`"), "{left:?}");
}

#[tokio::test]
async fn a_repository_nobody_serves_is_the_callers_to_correct() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let nobody = Some(ManifestId::carried("01NOBODY"));
    for refused in [
        fleet.list_repository_workspaces(nobody.clone()).await.err(),
        fleet.list_lifecycles(nobody.clone()).await.err(),
        fleet.list_lifecycle_rules(nobody.clone()).await.err(),
        fleet.list_autonomy_defaults(nobody).await.err(),
    ] {
        assert_eq!(refused.expect("refused").status(), 422);
    }
}

/// Saves one layer of defaults in Kit through the route's own write, as Bridge will.
async fn saved_layer(fleet: &Fixture, id: &str, scope: PieceScope, set: &[(&str, &str)]) {
    let defaults = set.iter().map(|(kind, autonomy)| (kind.to_string(), autonomy.to_string())).collect();
    let piece = LifecyclePiece::Autonomy { autonomy: AutonomyToSave { id: id.to_string(), scope, defaults } };
    fleet.save_lifecycle_piece(SaveLifecyclePiece { piece, overwrite: false }).await.expect("saved");
}

fn layers_of(read: &ipc::RepositoryAutonomy) -> Vec<(&str, &str, &str)> {
    read.layers.iter().map(|one| (one.id.as_str(), one.kept.rank.reach.as_str(), one.kept.rank.source.as_str())).collect()
}

fn set_by(read: &ipc::RepositoryAutonomy, id: &str) -> Vec<(String, String)> {
    let layer = read.layers.iter().find(|one| one.id == id).unwrap_or_else(|| panic!("no {id}: {:?}", read.layers));
    layer.defaults.iter().map(|one| (one.kind.clone(), one.autonomy.clone())).collect()
}

#[tokio::test]
async fn the_autonomy_defaults_are_served_layer_by_layer_with_where_each_was_kept_and_only_what_it_set() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (web, api, back_office) = (first(&fleet), another(&fleet, &home, "store-api"), another(&fleet, &home, "back-office"));
    write(armada(web.root(), "workspaces", "my-store.yml"), &my_store(&[&web, &api]));
    let (store_scope, web_scope) = (PieceScope::Within { id: "my-store".to_string() }, PieceScope::Within { id: web.manifest().id().as_str().to_string() });
    saved_layer(&fleet, "base", PieceScope::Everywhere, &[("tracker_write", "acts"), ("run_command", "off")]).await;
    saved_layer(&fleet, "store", store_scope, &[("forge_write", "asks")]).await;
    saved_layer(&fleet, "mine", web_scope, &[("tracker_write", "asks")]).await;
    write(
        armada(web.root(), "autonomy", "committed.yml"),
        &format!("version: 1\nautonomy_id: committed\nscope: {}\ndefaults:\n  start_job: acts\n", web.manifest().id().as_str()),
    );
    write(kit(&home, "autonomy", "broken.yml"), "defaults: nothing\n");

    let read = fleet.list_autonomy_defaults(named(&web)).await.expect("read");
    assert_eq!(read.manifest_id, wire_id(&web));
    assert_eq!(
        layers_of(&read),
        [("base", "everywhere", "kit"), ("store", "workspace", "kit"), ("committed", "repository", "repository"), ("mine", "repository", "kit")],
        "widest first, in the order they are laid, the repository's committed copy before yours"
    );
    assert_eq!(set_by(&read, "base"), [("tracker_write".to_string(), "acts".to_string()), ("run_command".to_string(), "off".to_string())], "only what it set, in the order of the kinds");
    assert_eq!(set_by(&read, "store"), [("forge_write".to_string(), "asks".to_string())]);
    assert_eq!(set_by(&read, "mine"), [("tracker_write".to_string(), "asks".to_string())]);
    let committed = read.layers.iter().find(|one| one.id == "committed").expect("committed");
    assert!(committed.kept.path.as_deref().is_some_and(|path| path.ends_with(".armada/autonomy/committed.yml")), "{committed:?}");
    assert!(read.layers.iter().all(|one| one.kept.overridden_by.is_none()));
    assert_eq!(read.layers[1].scope, PieceScope::Within { id: "my-store".to_string() });
    assert_eq!(read.layers[0].scope, PieceScope::Everywhere);

    let kinds: Vec<(&str, &str)> = read.armada.iter().map(|one| (one.kind.as_str(), one.autonomy.as_str())).collect();
    assert_eq!(
        kinds,
        [
            ("tracker_write", "asks"), ("forge_write", "asks"), ("draft", "asks"), ("start_job", "asks"), ("start_watch", "acts"),
            ("tell_agent", "acts"), ("run_command", "acts"), ("run_agent", "acts"),
        ],
        "Armada's own default for every kind"
    );
    let [left] = read.left_out.as_slice() else { panic!("one set aside: {:?}", read.left_out) };
    assert!(left.source == "kit" && left.path.ends_with("broken.yml") && !left.said.is_empty(), "{left:?}");

    let member = fleet.list_autonomy_defaults(named(&api)).await.expect("read");
    assert_eq!(layers_of(&member), [("base", "everywhere", "kit"), ("store", "workspace", "kit")], "a member reads its Workspace's layer, and not the repository's it is not");
    let outside = fleet.list_autonomy_defaults(named(&back_office)).await.expect("read");
    assert_eq!(layers_of(&outside), [("base", "everywhere", "kit")]);
}

async fn holds(rig: &Rig, id: &SessionId, kind: &str, target: &str, state: AttachmentState, detail: &[(&str, &str)]) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: kind.into(),
                manifest_id: rig.manifest.as_str().to_string(),
                target: target.into(),
                state,
                detail: detail.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .expect("attached");
}

async fn record(rig: &Rig, id: &SessionId) -> ipc::SessionRecord {
    use api::HostedSessions;
    std::sync::Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}

async fn held_by(rig: &Rig, id: &SessionId) -> JobLifecycles {
    rig.fleet.get_job_lifecycles(None, Some(id.clone())).await.expect("read")
}

#[tokio::test]
async fn a_job_that_holds_nothing_answers_empty_lists_and_a_job_nobody_holds_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let held = held_by(&rig, &id).await;
    assert_eq!(held.session_id, id);
    assert!(held.lifecycles.is_empty() && held.instances.is_empty() && held.moves.is_empty(), "{held:?}");

    let unknown = rig.fleet.get_job_lifecycles(None, Some(SessionId::carried("01NOBODY"))).await.unwrap_err();
    assert_eq!(unknown.status(), 404);
    let unplaced = rig.fleet.get_job_lifecycles(None, None).await.unwrap_err();
    assert_eq!((unplaced.status(), unplaced.error().code.as_str()), (422, "fleet.job_lifecycles_unplaced"));
}

#[tokio::test]
async fn a_job_names_what_it_holds_with_what_a_person_reads_for_each_and_what_it_came_from() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "issue", "ARM-41", AttachmentState::Standing, &[("pull_requests", "812")]).await;
    holds(&rig, &id, "pr", "812", AttachmentState::Standing, &[("state", "open")]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;

    let held = held_by(&rig, &id).await;
    let mut seen: Vec<_> = held.instances.iter().map(|one| (one.lifecycle.as_str(), one.key.as_str(), one.label.as_str())).collect();
    seen.sort_unstable();
    assert_eq!(seen, [("pr", "812", "PR #812"), ("ticket", "ARM-41", "ARM-41")]);
    let pull = held.instances.iter().find(|one| one.lifecycle == "pr").expect("held");
    assert_eq!(pull.links.iter().map(|link| (link.lifecycle.as_str(), link.key.as_str())).collect::<Vec<_>>(), [("ticket", "ARM-41")]);
    assert_eq!(pull.standing, InstanceStanding::Unread, "nothing has read it yet");
    let mut sent: Vec<&str> = held.lifecycles.iter().map(|one| one.id.as_str()).collect();
    sent.sort_unstable();
    assert_eq!(sent, ["pr", "ticket"], "the definitions its instances are of, and no others");
    assert!(held.moves.is_empty(), "taken on is not a move");
}

#[tokio::test]
async fn the_acts_a_jobs_rules_planned_are_read_oldest_first_with_where_each_stands_and_what_is_empty_is_left_out() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "issue", "ARM-41", AttachmentState::Standing, &[]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;
    assert!(held_by(&rig, &id).await.acts.is_empty(), "no rule planned anything");

    {
        let mut store = rig.fleet.store().lock().await;
        let act = |position, course, state, text| NewRuleAct {
            rule_id: "plan_moves_tickets",
            position,
            on: Some(("ticket", "ARM-41")),
            entered: Some(("pr", "812")),
            course,
            state,
            text,
            said: "",
        };
        let kept = store
            .keep_rule_plan(
                id.as_str(),
                "occurrence-1",
                &["plan_moves_tickets"],
                &[act(0, "acts", rule_act_standing::PLANNED, "Move ARM-41 to In review"), act(1, "asks", rule_act_standing::ASKED, "Ask Maya for review")],
                "2026-10-30T09:00:00.000Z",
            )
            .expect("kept");
        store.move_rule_act(kept[0], &[rule_act_standing::PLANNED], rule_act_standing::DONE, "", "2026-10-30T09:00:05.000Z").expect("settled");
    }

    let acts = held_by(&rig, &id).await.acts;
    let [done, asked] = acts.as_slice() else { panic!("two acts: {acts:?}") };
    assert_eq!((done.rule.as_str(), done.text.as_str(), done.course.as_str(), done.standing.as_str()), ("plan_moves_tickets", "Move ARM-41 to In review", "acts", "done"));
    assert_eq!((done.at.as_str(), done.settled_at.as_ref().map(|at| at.as_str())), ("2026-10-30T09:00:00.000Z", Some("2026-10-30T09:00:05.000Z")));
    assert_eq!(done.on.as_ref().map(|one| (one.lifecycle.as_str(), one.key.as_str())), Some(("ticket", "ARM-41")));
    assert_eq!(done.entered.as_ref().map(|one| (one.lifecycle.as_str(), one.key.as_str())), Some(("pr", "812")));
    assert_eq!((asked.standing.as_str(), asked.course.as_str()), ("asked", "asks"));
    assert!(asked.settled_at.is_none() && asked.said.is_none(), "nothing settled it and nothing was said: {asked:?}");
}

#[tokio::test]
async fn a_job_another_job_started_is_labelled_with_the_title_it_was_given() {
    let rig = rig_placing(None);
    let parent = rig.start().await;
    let child = rig.start_titled(Some("Build the report call")).await;
    holds(&rig, &child, "started_by", parent.as_str(), AttachmentState::Spent, &[]).await;
    rig.fleet.lifecycles_moved(record(&rig, &child).await).await;

    let held = held_by(&rig, &parent).await;
    let [one] = held.instances.as_slice() else { panic!("the child: {:?}", held.instances) };
    assert_eq!((one.lifecycle.as_str(), one.key.as_str(), one.label.as_str()), ("job", child.as_str(), "Build the report call"));
    assert_eq!(one.links.iter().map(|link| link.key.as_str()).collect::<Vec<_>>(), [parent.as_str()]);
}

#[tokio::test]
async fn a_pull_requests_moves_are_listed_oldest_first_with_where_it_stood_before_and_after() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "pr", "12", AttachmentState::Standing, &[("state", "open")]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;
    forge_ran(&rig.fleet, passed());
    let (manifest_id, root) = (rig.manifest.clone(), rig.root.clone());

    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Draft)));
    rig.fleet.pull_ledger_kept_current(&manifest_id, &root).await;
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Merged)));
    rig.fleet.pull_ledger_kept_current(&manifest_id, &root).await;

    let held = held_by(&rig, &id).await;
    let moved: Vec<_> = held.moves.iter().map(|one| (one.key.as_str(), one.from.clone(), one.to.clone(), one.source.as_str())).collect();
    assert_eq!(
        moved,
        [
            ("12", InstanceStanding::Unread, InstanceStanding::In { state: "draft".to_string() }, "forge"),
            ("12", InstanceStanding::In { state: "draft".to_string() }, InstanceStanding::In { state: "merged".to_string() }, "forge"),
        ]
    );
    assert!(held.moves.windows(2).all(|pair| pair[0].at <= pair[1].at), "oldest first");
    assert_eq!(held.instances[0].standing, InstanceStanding::In { state: "merged".to_string() });
}

/// The Job an agent asks about is the one holding its connection, and it needs to name nothing.
#[tokio::test]
async fn an_agent_is_answered_with_its_own_jobs_lifecycles_by_the_connection_it_calls_from() {
    let peers = Placing::nothing();
    let rig = rig_placing(Some(std::sync::Arc::clone(&peers)));
    let id = rig.start().await;
    rig.send(&id, "hi").await;
    peers.holding(2000, 52000, 12345);
    peers.started(1000, 2000);
    holds(&rig, &id, "issue", "ARM-41", AttachmentState::Standing, &[]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;

    let own = api::Caller::at("127.0.0.1:52000".parse().unwrap());
    let held = rig.fleet.get_job_lifecycles(Some(own), None).await.expect("read");
    assert_eq!(held.session_id, id);
    assert_eq!(held.instances.len(), 1);
}

/// Every `job_lifecycles.changed` the stream carries in the next moments.
async fn lifecycle_events(watching: &mut api::Subscription) -> Vec<JobLifecycles> {
    let mut found = Vec::new();
    while let Ok(next) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => {
                if let Event::JobLifecyclesChanged(held) = delivered.event {
                    found.push(held);
                }
            }
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    found
}

#[tokio::test]
async fn bridge_is_told_when_a_job_takes_something_on_or_one_of_its_instances_moves_and_only_then() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "pr", "12", AttachmentState::Standing, &[("state", "open")]).await;
    let row = record(&rig, &id).await;
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.lifecycles_moved(row.clone()).await;
    let taken = lifecycle_events(&mut watching).await;
    assert_eq!(taken.len(), 1, "{taken:?}");
    assert_eq!((taken[0].session_id.clone(), taken[0].instances.len()), (id.clone(), 1));
    assert_eq!(taken[0].instances[0].label, "PR #12");

    rig.fleet.lifecycles_moved(row).await;
    assert!(lifecycle_events(&mut watching).await.is_empty(), "the same row again changes nothing");

    forge_ran(&rig.fleet, passed());
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Draft)));
    rig.fleet.pull_ledger_kept_current(&rig.manifest, &rig.root).await;
    let moved = lifecycle_events(&mut watching).await;
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!(moved[0].moves.len(), 1);
    assert_eq!(moved[0].instances[0].standing, InstanceStanding::In { state: "draft".to_string() });

    rig.fleet.pull_ledger_kept_current(&rig.manifest, &rig.root).await;
    assert!(lifecycle_events(&mut watching).await.is_empty(), "a reading that finds it where it was says nothing");
}

#[tokio::test]
async fn a_job_that_holds_nothing_is_never_announced() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut before = record(&rig, &id).await;
    before.id = SessionId::carried("01RUNNINGBEFORE");
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.lifecycles_moved(before).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;

    assert!(lifecycle_events(&mut watching).await.is_empty());
}

#[tokio::test]
async fn the_starter_is_told_when_a_job_it_started_is_taken_on_and_again_when_that_job_is_found_running() {
    let rig = rig_placing(None);
    let parent = rig.start().await;
    let child = rig.start_titled(Some("Build the report call")).await;
    holds(&rig, &child, "started_by", parent.as_str(), AttachmentState::Spent, &[]).await;
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.lifecycles_moved(record(&rig, &child).await).await;

    let told = lifecycle_events(&mut watching).await;
    assert!(told.iter().all(|one| one.session_id == parent), "only the starter holds it: {told:?}");
    let [taken, running] = told.as_slice() else { panic!("taken on, then read: {told:?}") };
    assert_eq!((taken.instances[0].label.as_str(), &taken.instances[0].standing), ("Build the report call", &InstanceStanding::Unread));
    assert!(taken.moves.is_empty());
    assert_eq!(running.instances[0].standing, InstanceStanding::In { state: "running".to_string() });
    assert_eq!(running.moves.len(), 1);
}

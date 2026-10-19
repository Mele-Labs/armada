//! The Cockpit's flight controls, `~/.armada/flight-controls.toml`: read from a file, saved whole
//! and refused with the field named. `docs/concepts/kit.md`.
//!
//! **A file that will not read is never the defaults.** Each way it can fail is a case here, for
//! the machine's file and for a repository's own, kept at the place its runs are.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use api::Commands;
use ipc::{
    FlightControl, FlightControlKind, FlightControls, FlightControlsRead, RepositoryFlightControls,
    RunCloud, RunMemoryPlace, RunPlace, SaveRepositoryFlightControls, SessionId,
};
use testkit::{FakeBucket, FakeHarness, FakeVcs, FakeWorkProduct};

use super::red_claim::Planted;
use super::session_host::{rig_fitted, rig_placing, Rig};
use crate::flight_controls::repository::FOLDER as REPOSITORY_FOLDER;
use crate::flight_controls::FILE;
use crate::run_memory::places;
use crate::tests::tmp::TempDir;

fn the_file(home: &TempDir) -> PathBuf {
    home.path().join("kit").join(FILE)
}

/// A person writing the file by hand.
fn written(home: &TempDir, text: &str) {
    let path = the_file(home);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("Kit's home");
    std::fs::write(path, text).expect("the file");
}

fn tile(id: &str) -> FlightControl {
    FlightControl {
        id: id.into(),
        name: None,
        glyph: None,
        key: None,
        hidden: false,
        text: None,
        on_text: None,
        off_text: None,
        kind: None,
    }
}

fn not_read(read: FlightControlsRead) -> String {
    match read {
        FlightControlsRead::NotRead { why } => why,
        FlightControlsRead::Read { controls } => panic!("read as {controls:?}"),
    }
}

#[tokio::test]
async fn a_missing_file_reads_as_armadas_own() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));

    let FlightControlsRead::Read { controls } = fleet.flight_controls_read().await else {
        panic!("an absent file is the defaults, and it was looked at");
    };
    assert_eq!(controls, FlightControls::default());
    assert!(controls.job.is_empty() && controls.deck.is_empty());
}

#[tokio::test]
async fn a_file_a_person_wrote_reads_field_for_field() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    written(
        &home,
        "version = 1\n\n[[job]]\nid = \"ask\"\nname = \"Update\"\nkey = \"1\"\n\n\
         [[job]]\nid = \"pull\"\nkind = \"shell\"\ntext = \"git pull\"\nhidden = true\n\n\
         [[deck]]\nid = \"dispatch\"\nglyph = \"plus\"\n",
    );

    let FlightControlsRead::Read { controls } = fleet.flight_controls_read().await else {
        panic!("the file is well formed");
    };
    assert_eq!(controls.job[0].name.as_deref(), Some("Update"));
    assert_eq!(controls.job[0].key.as_deref(), Some("1"));
    assert_eq!(controls.job[1].kind, Some(FlightControlKind::Shell));
    assert!(controls.job[1].hidden);
    assert_eq!(controls.deck[0].glyph.as_deref(), Some("plus"));
}

/// **Each way a file can fail is `not_read` with a reason**, and the reason names what to fix.
#[tokio::test]
async fn a_file_that_will_not_read_is_not_read_and_says_why() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));

    written(&home, "version = 1\n[[job]\nid = \"ask\"\n");
    let why = not_read(fleet.flight_controls_read().await);
    assert!(why.contains(FILE), "names the file: {why}");

    written(&home, "version = 1\n[[job]]\nid = \"ask\"\nhiden = true\n");
    let why = not_read(fleet.flight_controls_read().await);
    assert!(why.contains("hiden"), "a key Fleet does not know is named: {why}");

    written(&home, "version = 2\n");
    let why = not_read(fleet.flight_controls_read().await);
    assert!(why.contains("version"), "{why}");

    written(&home, "[[job]]\nid = \"ask\"\n[[job]]\nid = \"ask\"\n");
    let why = not_read(fleet.flight_controls_read().await);
    assert!(why.contains("job[1].id"), "a rule a save would refuse is held on a read: {why}");

    std::fs::remove_file(the_file(&home)).expect("a file to replace");
    std::fs::create_dir(the_file(&home)).expect("a directory where the file is");
    not_read(fleet.flight_controls_read().await);
}

#[tokio::test]
async fn a_save_is_read_back_whole() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let saved = FlightControls {
        version: 1,
        job: vec![
            FlightControl { name: Some("Update".into()), key: Some("1".into()), ..tile("ask") },
            FlightControl {
                kind: Some(FlightControlKind::Shell),
                text: Some("git pull && echo \"done\"".into()),
                glyph: Some("git-pull-request".into()),
                hidden: true,
                ..tile("pull")
            },
            FlightControl {
                on_text: Some("land it".into()),
                off_text: Some("hold".into()),
                ..tile("land")
            },
        ],
        deck: vec![FlightControl { key: Some("d".into()), ..tile("dispatch") }],
    };

    let answered = fleet.flight_controls_saved(saved.clone()).await.expect("saved");
    assert_eq!(answered, FlightControlsRead::Read { controls: saved.clone() });
    assert_eq!(fleet.flight_controls_read().await, answered, "the file holds what was sent");
    let mut staged = the_file(&home).into_os_string();
    staged.push(".writing");
    assert!(!PathBuf::from(staged).exists(), "the staging file was renamed away");

    let again = fleet.flight_controls_saved(FlightControls::default()).await.expect("saved");
    assert_eq!(again, FlightControlsRead::Read { controls: FlightControls::default() });
}

/// **A refusal names the field and writes nothing**: the file that was there stays as it was.
#[tokio::test]
async fn a_save_that_breaks_a_rule_is_refused_with_the_field_named() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let refused = |controls: FlightControls| {
        let fleet = &fleet;
        async move {
            match fleet.flight_controls_saved(controls).await {
                Err(api::Refusal::Unacceptable(error)) => error.message,
                other => panic!("expected a refusal, got {other:?}"),
            }
        }
    };
    let list = |job: Vec<FlightControl>| FlightControls { version: 1, job, deck: Vec::new() };

    let said = refused(list(vec![tile("ask"), tile("land"), tile("ask")])).await;
    assert!(said.contains("job[2].id") && said.contains("job[0]"), "{said}");

    let said = refused(FlightControls {
        version: 1,
        job: Vec::new(),
        deck: vec![tile("a"), tile("a")],
    })
    .await;
    assert!(said.contains("deck[1].id"), "{said}");

    let keyed = |id: &str, key: &str| FlightControl { key: Some(key.into()), ..tile(id) };
    let said = refused(list(vec![keyed("a", "1"), keyed("b", "1")])).await;
    assert!(said.contains("job[1].key"), "{said}");
    let said = refused(list(vec![keyed("a", "x"), keyed("b", "X")])).await;
    assert!(said.contains("job[1].key"), "a letter is one key in either case: {said}");
    let said = refused(list(vec![keyed("a", "12")])).await;
    assert!(said.contains("job[0].key"), "{said}");
    let said = refused(list(vec![keyed("a", "-")])).await;
    assert!(said.contains("job[0].key"), "{said}");
    let said = refused(list(vec![tile(" ")])).await;
    assert!(said.contains("job[0].id"), "{said}");
    let said = refused(FlightControls { version: 2, ..FlightControls::default() }).await;
    assert!(said.contains("version"), "{said}");

    assert!(!the_file(&home).exists(), "nothing was written");

    // The same key in the two lists is two tiles in two places, which is allowed.
    fleet
        .flight_controls_saved(FlightControls {
            version: 1,
            job: vec![keyed("a", "1")],
            deck: vec![keyed("b", "1")],
        })
        .await
        .expect("a key is once per list");
}

// A repository's own tiles, kept where its runs are: `crate::flight_controls::repository`.

fn served_root(fleet: &crate::daemon::Fleet<FakeHarness, FakeVcs, FakeWorkProduct>) -> String {
    fleet.repositories().first().expect("a served repository").root().to_string()
}

/// A person's own action, the only tile a repository holds.
fn own(id: &str) -> FlightControl {
    FlightControl { kind: Some(FlightControlKind::Shell), text: Some("bin/rspec".into()), ..tile(id) }
}

fn saving(root: &str, job: Vec<FlightControl>) -> SaveRepositoryFlightControls {
    SaveRepositoryFlightControls { repository: root.to_string(), controls: FlightControls { version: 1, job, deck: Vec::new() }, session_id: None }
}

fn refused_with(refusal: api::Refusal) -> (String, String) {
    match refusal {
        api::Refusal::Unacceptable(error) => (error.code, error.message),
        other => panic!("not a 422: {other:?}"),
    }
}

fn kit_file(home: &TempDir, root: &str) -> PathBuf {
    places::file_for(&home.path().join("kit"), REPOSITORY_FOLDER, root)
}

#[tokio::test]
async fn a_repositorys_file_is_read_from_kit_and_a_missing_one_is_empty_lists() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = served_root(&fleet);

    let missing = fleet.repository_flight_controls_read(root.clone(), None).await.expect("read");
    assert_eq!(
        missing,
        RepositoryFlightControls { repository: root.clone(), place: RunPlace::ThisMac, read: FlightControlsRead::Read { controls: FlightControls::default() } }
    );

    let path = kit_file(&home, &root);
    assert!(path.ends_with(format!("kit/flight-controls/{}", path.file_name().expect("a name").to_string_lossy())));
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(&path, "version = 1\n[[job]]\nid = \"specs\"\nkind = \"shell\"\ntext = \"bin/rspec\"\n").expect("written");
    let RepositoryFlightControls { read: FlightControlsRead::Read { controls }, .. } =
        fleet.repository_flight_controls_read(root.clone(), None).await.expect("read")
    else {
        panic!("the file is well formed");
    };
    assert_eq!((controls.job[0].id.as_str(), controls.job[0].kind), ("specs", Some(FlightControlKind::Shell)));
}

#[tokio::test]
async fn a_repositorys_save_is_written_whole_beside_its_runs_and_not_in_the_machine_file() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = served_root(&fleet);
    let sent = saving(&root, vec![FlightControl { key: Some("r".into()), ..own("specs") }]);

    let answered = fleet.repository_flight_controls_saved(sent.clone()).await.expect("saved");
    assert_eq!(answered.place, RunPlace::ThisMac);
    assert_eq!(answered.read, FlightControlsRead::Read { controls: sent.controls.clone() });
    assert_eq!(fleet.repository_flight_controls_read(root.clone(), None).await.expect("read"), answered);
    let path = kit_file(&home, &root);
    assert!(path.exists());
    assert!(!path.with_extension("toml.writing").exists(), "the staging file was renamed away");
    assert!(!the_file(&home).exists(), "the machine's own file is not touched");
}

/// **A repository holds a person's own actions only**, so a tile with no `kind` is refused on a
/// save and not read from a file, with the field named either way.
#[tokio::test]
async fn a_repositorys_tile_with_no_kind_is_refused_and_a_file_holding_one_is_not_read() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = served_root(&fleet);

    let (code, said) =
        refused_with(fleet.repository_flight_controls_saved(saving(&root, vec![own("specs"), tile("ask")])).await.expect_err("refused"));
    assert_eq!(code, "fleet.flight_controls_refused");
    assert!(said.contains("job[1].kind is missing"), "{said}");
    assert!(!kit_file(&home, &root).exists(), "nothing was written");

    let path = kit_file(&home, &root);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(&path, "version = 1\n[[deck]]\nid = \"dispatch\"\n").expect("written");
    let why = not_read(fleet.repository_flight_controls_read(root.clone(), None).await.expect("read").read);
    assert!(why.contains("deck[0].kind"), "{why}");

    std::fs::write(&path, "version = 1\n[[job]\n").expect("written");
    let why = not_read(fleet.repository_flight_controls_read(root, None).await.expect("read").read);
    assert!(why.contains(REPOSITORY_FOLDER), "names the file: {why}");
}

#[tokio::test]
async fn a_synced_repository_keeps_its_file_under_the_synced_folder_and_a_folder_gone_reads_and_saves_nothing() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = served_root(&fleet);
    let synced = TempDir::new();
    let folder = synced.path().join("Dropbox");
    std::fs::create_dir_all(&folder).expect("a synced folder");
    fleet
        .set_run_memory_place(RunMemoryPlace {
            repository: root.clone(),
            place: RunPlace::Synced,
            synced_folder: Some(folder.to_string_lossy().to_string()),
            cloud: None,
        })
        .await
        .expect("chosen");

    let answered = fleet.repository_flight_controls_saved(saving(&root, vec![own("specs")])).await.expect("saved");
    assert_eq!(answered.place, RunPlace::Synced);
    assert!(places::file_for(&folder, REPOSITORY_FOLDER, &root).exists());
    assert!(!kit_file(&home, &root).exists(), "not in Kit's folder");

    std::fs::remove_dir_all(&folder).expect("the folder went");
    let read = fleet.repository_flight_controls_read(root.clone(), None).await.expect("read");
    assert_eq!(read.place, RunPlace::Synced);
    assert!(not_read(read.read).contains("is not there"));
    let (code, _) = refused_with(fleet.repository_flight_controls_saved(saving(&root, vec![own("specs")])).await.expect_err("refused"));
    assert_eq!(code, "fleet.flight_controls_unplaced");
    assert!(!folder.exists(), "a save does not make the synced folder");
}

#[tokio::test]
async fn a_bucket_keeps_a_repositorys_file_as_an_object_beside_its_runs() {
    let bucket = Arc::new(FakeBucket::new());
    let held = Arc::clone(&bucket);
    let rig = rig_fitted(move |fittings| fittings.reaching = Arc::new(Planted::with_bucket(held)));
    let root = rig.root.clone();
    rig.fleet
        .set_run_memory_place(RunMemoryPlace {
            repository: root.clone(),
            place: RunPlace::Cloud,
            synced_folder: None,
            cloud: Some(RunCloud::Bucket("s3://shop-runs/armada".into())),
        })
        .await
        .expect("chosen");
    let object = places::object_for(REPOSITORY_FOLDER, &root);
    assert!(object.starts_with("flight-controls/"), "{object}");

    let empty = rig.fleet.repository_flight_controls_read(root.clone(), None).await.expect("read");
    assert_eq!((empty.place, empty.read), (RunPlace::Cloud, FlightControlsRead::Read { controls: FlightControls::default() }));

    let sent = saving(&root, vec![own("specs")]);
    let answered = rig.fleet.repository_flight_controls_saved(sent.clone()).await.expect("saved");
    assert_eq!(answered.read, FlightControlsRead::Read { controls: sent.controls });
    let text = String::from_utf8(bucket.object(&object).expect("an object")).expect("text");
    assert!(text.contains("bin/rspec"), "{text}");

    bucket.plant(&object, b"version = 1\n[[job]]\nid = \"ask\"\n");
    let why = not_read(rig.fleet.repository_flight_controls_read(root.clone(), None).await.expect("read").read);
    assert!(why.contains("job[0].kind") && why.contains(&object), "{why}");

    bucket.set_read_only(true);
    let (code, _) = refused_with(rig.fleet.repository_flight_controls_saved(saving(&root, vec![own("specs")])).await.expect_err("refused"));
    assert_eq!(code, crate::run_memory::CLOUD_REFUSED);
    bucket.set_unreachable(true);
    let why = not_read(rig.fleet.repository_flight_controls_read(root, None).await.expect("read").read);
    assert!(why.contains("could not be reached"), "{why}");
}

const IN_REPOSITORY: &str = ".armada/flight-controls.toml";

async fn in_the_repository(rig: &Rig) {
    rig.fleet
        .set_run_memory_place(RunMemoryPlace { repository: rig.root.clone(), place: RunPlace::Repository, synced_folder: None, cloud: None })
        .await
        .expect("chosen");
}

/// A Job in the rig's repository holding `lease`, a slot and its branch, with the slot's tree made.
async fn a_job_holding(rig: &Rig, lease: Option<(u32, &str)>) -> (SessionId, PathBuf) {
    let id = rig.start().await;
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.lease_slot = lease.map(|(slot, _)| slot);
    hosting.lease_branch = lease.map(|(_, branch)| branch.to_string());
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    let tree = PathBuf::from(adapter_traits::slot_path(&rig.root, lease.map_or(0, |(slot, _)| slot)));
    std::fs::create_dir_all(&tree).expect("a tree");
    (id, tree)
}

fn ids(read: RepositoryFlightControls) -> Vec<String> {
    match read.read {
        FlightControlsRead::Read { controls } => controls.job.into_iter().map(|tile| tile.id).collect(),
        FlightControlsRead::NotRead { why } => panic!("not read: {why}"),
    }
}

/// **Never the checkout**: the file is written in the Job's tree and committed on its branch, so it
/// lands with the pull request as `armada.yml`'s remembered runs do.
#[tokio::test]
async fn the_repository_place_commits_the_file_on_the_jobs_branch_and_leaves_the_checkout_alone() {
    let rig = rig_placing(None);
    in_the_repository(&rig).await;
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;
    let before = rig.fleet.vcs().committed().len();
    let sent = SaveRepositoryFlightControls { session_id: Some(id), ..saving(&rig.root, vec![own("specs")]) };

    let answered = rig.fleet.repository_flight_controls_saved(sent.clone()).await.expect("saved");
    assert_eq!((answered.place, answered.read), (RunPlace::Repository, FlightControlsRead::Read { controls: sent.controls }));
    let written = std::fs::read_to_string(tree.join(IN_REPOSITORY)).expect("in the Job's tree");
    assert!(written.contains("bin/rspec"), "{written}");
    let commits = rig.fleet.vcs().committed();
    assert_eq!(commits.len(), before + 1);
    assert_eq!(commits[before].branch, "job/one");
    assert_eq!(commits[before].scope, testkit::CommitScope::Paths(vec![IN_REPOSITORY.to_string()]));
    assert!(!Path::new(&rig.root).join(IN_REPOSITORY).exists(), "the checkout is untouched");
}

#[tokio::test]
async fn the_repository_place_with_no_jobs_branch_refuses_a_save_and_writes_nothing() {
    let rig = rig_placing(None);
    in_the_repository(&rig).await;
    let before = rig.fleet.vcs().committed().len();
    let (unleased, _) = a_job_holding(&rig, None).await;

    for session_id in [None, Some(unleased)] {
        let sent = SaveRepositoryFlightControls { session_id, ..saving(&rig.root, vec![own("specs")]) };
        let (code, said) = refused_with(rig.fleet.repository_flight_controls_saved(sent).await.expect_err("refused"));
        assert_eq!(code, "fleet.flight_controls_unplaced");
        assert!(said.contains("waits for a Job's branch to write it on"), "{said}");
    }
    assert_eq!(rig.fleet.vcs().committed().len(), before, "nothing committed");
    assert!(!Path::new(&rig.root).join(IN_REPOSITORY).exists(), "nothing written in the checkout");
}

#[tokio::test]
async fn the_repository_place_reads_the_named_jobs_tree_and_else_the_checkout() {
    let rig = rig_placing(None);
    in_the_repository(&rig).await;
    let checkout = Path::new(&rig.root).join(IN_REPOSITORY);
    std::fs::create_dir_all(checkout.parent().expect("a folder")).expect("made");
    std::fs::write(&checkout, "version = 1\n[[job]]\nid = \"on-main\"\nkind = \"shell\"\ntext = \"make\"\n").expect("written");
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;
    std::fs::create_dir_all(tree.join(".armada")).expect("made");
    std::fs::write(tree.join(IN_REPOSITORY), "version = 1\n[[job]]\nid = \"on-branch\"\nkind = \"shell\"\ntext = \"make\"\n").expect("written");

    let read = rig.fleet.repository_flight_controls_read(rig.root.clone(), None).await.expect("read");
    assert_eq!(read.place, RunPlace::Repository);
    assert_eq!(ids(read), ["on-main"]);
    let read = rig.fleet.repository_flight_controls_read(rig.root.clone(), Some(id)).await.expect("read");
    assert_eq!(ids(read), ["on-branch"]);
}

/// Armada's store is not built yet, and says so.
#[tokio::test]
async fn armadas_store_says_it_is_not_built_and_refuses_a_save() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = served_root(&fleet);

    fleet.store().lock().await.set_run_memory_place(&root, "cloud", None, Some("store")).expect("the store chosen");
    let read = fleet.repository_flight_controls_read(root.clone(), None).await.expect("read");
    assert_eq!(read.place, RunPlace::Cloud);
    assert!(not_read(read.read).contains("yet"));
    let (code, _) = refused_with(fleet.repository_flight_controls_saved(saving(&root, vec![own("specs")])).await.expect_err("refused"));
    assert_eq!(code, "fleet.flight_controls_unplaced");
}

#[tokio::test]
async fn a_repository_fleet_does_not_serve_is_not_found() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let not_served = |refusal: api::Refusal| matches!(refusal, api::Refusal::NotFound(error) if error.code == crate::run_memory::NOT_SERVED);

    assert!(not_served(fleet.repository_flight_controls_read("/not/served".into(), None).await.expect_err("refused")));
    assert!(not_served(fleet.repository_flight_controls_saved(saving("/not/served", vec![own("specs")])).await.expect_err("refused")));
}

//! The Cockpit's flight controls, `~/.armada/flight-controls.toml`: read from a file, saved whole
//! and refused with the field named. `docs/concepts/kit.md`.
//!
//! **A file that will not read is never the defaults.** Each way it can fail is a case here.

use std::path::PathBuf;

use ipc::{FlightControl, FlightControlKind, FlightControls, FlightControlsRead};
use testkit::FakeWorkProduct;

use crate::flight_controls::FILE;
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

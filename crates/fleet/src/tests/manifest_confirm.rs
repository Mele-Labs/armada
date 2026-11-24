//! A Command flagged `confirm: true` reaches Bridge by name in `get_manifest`, so the dashboard asks before it runs one.

use ipc::ManifestId;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

const CONFIRMING: &str = "01CONFIRMINGMANIFEST";

#[test]
fn a_command_flagged_confirm_is_named_in_the_manifest_config_and_one_that_is_not_is_left_out() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&[]));
    let workflows = fittings.starting().workflows.clone();
    let fleet = Fleet::assembled(fittings);
    let root = home.path().join("confirming");
    let text = format!(
        "version: 1\nid: {CONFIRMING}\ncommands:\n  migrate:\n    run: make migrate\n    confirm: true\n  fmt:\n    run: make fmt\n  web:\n    serve: make web\n    confirm: true\n"
    );
    let manifest = config::Manifest::parse(&root.join("armada.yml"), &text).expect("loads");
    fleet
        .repositories()
        .add(Located {
            root: root.to_string_lossy().to_string(),
            records_root: home.path().join("confirming-records").to_string_lossy().to_string(),
            set_up: Some(SetUp::of(manifest, workflows)),
        })
        .expect("served");

    let read = fleet.manifest_config(ManifestId::carried(CONFIRMING)).expect("read");

    assert_eq!(read.commands, ["fmt", "migrate"]);
    assert_eq!(read.confirm_commands, ["migrate", "web"]);
}

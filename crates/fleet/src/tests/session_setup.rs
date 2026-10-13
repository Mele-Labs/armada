//! A hosted session started to set a repository up: in a served root with no
//! `armada.yml` yet, where Setup's button starts one to write it.

use api::{HostedSessions, Sessions};
use ipc::{GateAnswer, ManifestId, StartSession};

use crate::tests::session_host::{rig_placing, Rig};
use crate::tests::tmp::TempDir;

/// A folder served without a Manifest, as Add serves one Setup has not written.
fn unset_up(rig: &Rig, home: &TempDir) -> String {
    let root = home.path().join("unset-up");
    std::fs::create_dir_all(&root).expect("a checkout");
    let root = root
        .canonicalize()
        .expect("resolved")
        .to_string_lossy()
        .to_string();
    let located = crate::repositories::Located {
        root: root.clone(),
        records_root: home.path().join("records").to_string_lossy().to_string(),
        set_up: None,
    };
    rig.fleet
        .repositories()
        .add(located)
        .expect("served without a Manifest");
    root
}

fn setting_up(root: &str) -> StartSession {
    StartSession {
        manifest_id: ManifestId::carried(""),
        title: Some(String::from("Set up unset-up")),
        model: None,
        effort: None,
        mode: None,
        pilot: None,
        root: Some(root.to_string()),
        fork: None,
    }
}

/// **The session stands in the repository's own root**, carries no Manifest, and its
/// process starts there; the owner asked that it write `armada.yml` for him.
#[tokio::test]
async fn a_setup_session_starts_in_a_root_with_no_manifest_and_runs_there() {
    let rig = rig_placing(None);
    let home = TempDir::new();
    let root = unset_up(&rig, &home);

    let started = rig
        .fleet
        .start_session(setting_up(&root))
        .await
        .expect("started");
    assert_eq!(started.cwd, root);
    assert_eq!(started.manifest_id, None, "no Manifest to carry yet");

    rig.send(&started.id, "set this repository up").await;
    let starts = rig.stand_in.starts();
    assert_eq!(starts.len(), 1);
    assert_eq!(
        starts[0].directory, root,
        "the process runs in the root, not a slot"
    );

    let file = format!(r#"{{"file_path":"{root}/armada.yml","content":"version: 1\n"}}"#);
    let answer = rig.gate_in(&root, &started.id, "Write", &file).await;
    assert_eq!(
        answer,
        GateAnswer::pass(),
        "it may write armada.yml where Fleet reads it"
    );
}

/// **A root Fleet does not serve is refused**, as any start naming nothing is.
#[tokio::test]
async fn a_setup_session_in_a_folder_fleet_does_not_serve_is_refused() {
    let rig = rig_placing(None);
    let refused = rig.fleet.start_session(setting_up("/not/served")).await;
    assert!(refused.is_err());
    assert!(rig
        .fleet
        .list_sessions(None, None, None)
        .await
        .unwrap()
        .sessions
        .is_empty());
}

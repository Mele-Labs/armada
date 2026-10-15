//! A hosted session started to set a repository up: in a served root with no
//! `armada.yml` yet, where Setup's button starts one to write it.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{GateAnswer, ManifestId, SessionTurn, StartSession};

use crate::repositories::{Locating, SetUp};
use crate::tests::repositories::Planted;
use crate::tests::session_host::{denied, eventually, reason, rig_fitted, rig_placing, Rig};
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

/// **Only `armada.yml` and `.gitignore` at any depth, and `.git/info/exclude`, until the
/// Manifest is served** (the owner, 14 Oct 2026). Anything else, by a file tool or a shell
/// line, is refused naming them; a read still runs.
#[tokio::test]
async fn a_setup_session_writes_only_its_setup_files_under_its_root() {
    let rig = rig_placing(None);
    let home = TempDir::new();
    let root = unset_up(&rig, &home);
    let id = rig
        .fleet
        .start_session(setting_up(&root))
        .await
        .expect("started")
        .id;

    let write = |path: &str| format!(r#"{{"file_path":"{path}","content":"x"}}"#);
    let gitignore = r#"{"file_path":".gitignore","old_string":"a","new_string":"b"}"#;
    assert_eq!(
        rig.gate_in(&root, &id, "Write", &write(&format!("{root}/armada.yml")))
            .await,
        GateAnswer::pass()
    );
    assert_eq!(
        rig.gate_in(&root, &id, "Edit", gitignore).await,
        GateAnswer::pass()
    );
    assert_eq!(
        rig.gate_in(&root, &id, "Bash", r#"{"command":"armada check test"}"#)
            .await,
        GateAnswer::pass()
    );

    let source = rig
        .gate_in(&root, &id, "Write", &write(&format!("{root}/src/x.ts")))
        .await;
    assert!(denied(&source), "{source:?}");
    for said in ["armada.yml", ".gitignore", ".git/info/exclude"] {
        assert!(reason(&source).contains(said), "{}", reason(&source));
    }
    for allowed in [
        format!("{root}/ops/armada.yml"),
        format!("{root}/services/x/.gitignore"),
        format!("{root}/.git/info/exclude"),
        String::from("services/TermsService/armada.yml"),
    ] {
        let answer = rig.gate_in(&root, &id, "Write", &write(&allowed)).await;
        assert_eq!(answer, GateAnswer::pass(), "{allowed}");
    }
    for refused in [
        format!("{root}/ops/app.rb"),
        format!("{root}/.git/config"),
        format!("{root}/.git/hooks/armada.yml"),
        format!("{root}/../other/armada.yml"),
        format!("{root}/ops/armada.yml.bak"),
        String::from("../other/armada.yml"),
    ] {
        let answer = rig.gate_in(&root, &id, "Write", &write(&refused)).await;
        assert!(denied(&answer), "{refused}: {answer:?}");
    }
    let shell = rig
        .gate_in(&root, &id, "Bash", r#"{"command":"touch src/x.ts"}"#)
        .await;
    assert!(denied(&shell), "{shell:?}");
}

/// **Once the root's Manifest is served, the session leases a slot like any other**, save
/// its setup files: its turn's end loads the armada.yml it wrote, an app's armada.yml still
/// lands in the checkout, and a source file leases a slot.
#[tokio::test]
async fn once_its_manifest_is_served_a_setup_session_leases_a_slot_like_any_other() {
    let planted = Arc::new(Planted::nothing());
    let locating = Arc::clone(&planted);
    let rig = rig_fitted(move |fittings| fittings.locating = locating as Arc<dyn Locating>);
    let home = TempDir::new();
    let root = unset_up(&rig, &home);
    let id = rig
        .fleet
        .start_session(setting_up(&root))
        .await
        .expect("started")
        .id;
    rig.send(&id, "set this repository up").await;
    rig.stand_in.init(0);

    let text = "version: 1\nid: 01SETUPMANIFEST\n";
    let manifest =
        config::Manifest::parse(&Path::new(&root).join("armada.yml"), text).expect("loads");
    let located = crate::repositories::Located {
        root: root.clone(),
        records_root: home.path().join("records").to_string_lossy().to_string(),
        set_up: Some(SetUp::of(manifest, BTreeMap::new())),
    };
    planted.plant(&root, located);
    rig.stand_in.finishes(0, "armada.yml is written");
    eventually(|| async { rig.turn(&id).await == SessionTurn::Idle }).await;
    let repository = rig.fleet.repositories().at(&root).expect("served");
    assert!(repository.manifest().is_some(), "the turn's end loaded it");

    let write = |path: &str| format!(r#"{{"file_path":"{path}","content":"x"}}"#);
    for setup in ["ops/armada.yml", "armada.yml", ".git/info/exclude"] {
        let answer = rig
            .gate_in(&root, &id, "Write", &write(&format!("{root}/{setup}")))
            .await;
        assert_eq!(answer, GateAnswer::pass(), "{setup} stays in the checkout");
    }
    let answer = rig
        .gate_in(&root, &id, "Write", &write(&format!("{root}/src/x.ts")))
        .await;
    assert!(denied(&answer), "{answer:?}");
    assert!(reason(&answer).contains("Slot"), "{}", reason(&answer));
    let leased = rig
        .gate_in(
            &root,
            &id,
            "Write",
            &write(&format!("{root}/consumer/armada.yml")),
        )
        .await;
    assert_eq!(leased, GateAnswer::pass(), "and still once it holds a slot");
    let record = Arc::clone(&rig.fleet)
        .get_session(id.clone())
        .await
        .unwrap()
        .session;
    assert_eq!(
        record.manifest_id,
        Some(ManifestId::carried("01SETUPMANIFEST"))
    );
}

/// **The window is Setup's alone**: a session started on a served Manifest leases a slot for
/// an app's armada.yml as for any other write.
#[tokio::test]
async fn a_session_not_started_by_setup_leases_a_slot_for_an_armada_yml() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(0);
    let file = format!(
        r#"{{"file_path":"{}/ops/armada.yml","content":"x"}}"#,
        rig.root
    );
    let answer = rig.gate(&id, "Write", &file).await;
    assert!(denied(&answer), "{answer:?}");
    assert!(reason(&answer).contains("Slot"), "{}", reason(&answer));
}

/// **Its door places it and answers its permission prompt with no Manifest**: the
/// agent CLI ended at the first such call when the door offered it nothing.
#[tokio::test]
async fn a_setup_sessions_permission_prompt_is_answered_with_no_manifest() {
    use api::{Admitting, Conversations};

    let rig = rig_placing(None);
    let home = TempDir::new();
    let root = unset_up(&rig, &home);
    let id = rig
        .fleet
        .start_session(setting_up(&root))
        .await
        .expect("started")
        .id;
    rig.send(&id, "set this repository up").await;

    let caller = api::Caller::at("127.0.0.1:50999".parse().expect("an address"));
    let reach = rig.fleet.helm_at(caller).expect("its connection is placed");
    assert!(reach.is_a_hosted_session() && reach.may("ask_the_person"));
    let asking: ipc::AskingToRun = ipc::decode(
        "an ask",
        br#"{"tool_name":"Bash","input":{"command":"git branch --show-current"}}"#,
    )
    .expect("an ask");
    let decided = rig
        .fleet
        .ask_the_person_at(asking, None, caller)
        .await
        .expect("answered");
    assert!(
        matches!(decided, ipc::RunOrNot::Allow { .. }),
        "{decided:?}"
    );
}

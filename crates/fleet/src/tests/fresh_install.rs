//! A Fleet on a Mac with nothing but the app: an empty home, and a repository with no `armada.yml`
//! and no `plugins/`. A hosted Job there has the mod, filled from the bundle beside the binary, and
//! its door answers the acts its rules name. `crates/acceptance/tests/fresh_install.rs` holds the
//! claim; this is the half that needs a Fleet.

use std::net::SocketAddr;
use std::sync::Arc;

use api::{HostedSessions, Sessions};
use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::RunId;
use tower::ServiceExt;

use super::session_host::{rig_fitted, Rig};
use crate::repositories::Located;
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

const RELAY: u16 = 52010;

/// The mod as an app carries it: `plugins/`'s contents, with one skill.
fn bundle_in(app: &TempDir) -> String {
    let mod_dir = app.path().join("mod");
    for (file, text) in [
        (adapters::agent_mod::COPY_MARKETPLACE, "{}"),
        (adapters::MOD_INSTALLED_MANIFEST, r#"{"name":"armada","version":"1.0.0"}"#),
        ("armada/skills/reading-a-brief/SKILL.md", "Read the brief."),
    ] {
        let at = mod_dir.join(file);
        std::fs::create_dir_all(at.parent().expect("a folder")).expect("its folder");
        std::fs::write(at, text).expect("a file");
    }
    mod_dir.to_string_lossy().to_string()
}

async fn at_the_door(app: &Router, tool: &str, arguments: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{RELAY}").parse().expect("an address");
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    );
    let request = Request::builder()
        .method("POST")
        .uri(api::door_without_a_manifest())
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(call))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let body = response.into_body().collect().await.expect("a readable body").to_bytes();
    String::from_utf8_lossy(&body).to_string()
}

/// A Fleet with an empty home and a bundle, serving a repository that has nothing of Armada's, and a
/// Job started in it.
async fn a_job_in_a_bare_repository(app: &TempDir) -> (Rig, ipc::SessionId, Router) {
    let peers = Placing::nothing();
    let bundle = bundle_in(app);
    let rig = {
        let peers = Arc::clone(&peers);
        rig_fitted(move |fittings| {
            fittings.peers = peers;
            fittings.host.mod_bundle = Some(bundle);
        })
    };
    let root = app.path().join("shop");
    std::fs::create_dir_all(&root).expect("a repository");
    let added = rig
        .fleet
        .repositories()
        .add(Located {
            root: root.to_string_lossy().to_string(),
            records_root: app.path().join("shop-records").to_string_lossy().to_string(),
            set_up: None,
        })
        .expect("added");
    let id = Arc::clone(&rig.fleet)
        .start_session(ipc::StartSession {
            manifest_id: ipc::ManifestId::carried(added.implicit_id()),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
        })
        .await
        .expect("started")
        .id;
    rig.send(&id, "hello").await;
    rig.stand_in.init(0);
    peers.holding(1000, RELAY, rig.fleet.host().port);
    let door = api::router(api::Served::sharing(Arc::clone(&rig.fleet), RunId::carried("01RUN"), rig.fleet.events()));
    (rig, id, door)
}

#[tokio::test]
async fn a_job_in_a_bare_repository_on_an_empty_home_has_the_mod_and_answers_its_rules_acts() {
    let app = TempDir::new();
    let (rig, _, door) = a_job_in_a_bare_repository(&app).await;

    // The copy was filled from the bundle when the Job launched.
    let copy = crate::runtime::mod_dir(&rig.fleet.host().home);
    assert!(copy.join("armada/skills/reading-a-brief/SKILL.md").is_file(), "{}", copy.display());

    // Each act is answered by its own route, and none by the door's refusal for a root with no
    // `armada.yml`. A call that cannot succeed here says why in the route's own code.
    let served = r#""isError":false"#;
    for (tool, arguments, expected) in [
        ("list_manifests", "{}", served),
        ("list_watches", "{}", served),
        ("list_watch_recipes", "{}", served),
        ("defer_later", r#"{"body":{"sentence":"file the slow fixture load"}}"#, served),
        ("settle_later", r#"{"body":{"id":"later-1","how":"done"}}"#, served),
        ("finish_up", "{}", served),
        ("report_bug", r#"{"body":{"title":"a title","what":"it broke"}}"#, served),
        ("start_another_job", r#"{"body":{"title":"Check the reader","message":"Read crates/reader."}}"#, served),
        ("dispute_fleet", r#"{"body":{"why":"a notice that is not so"}}"#, "fleet.dispute_refused"),
        ("stop_watch", r#"{"body":{"watch":"w-none"}}"#, "fleet.no_such_watch"),
        ("weigh_watch", r#"{"body":{"watch":"w-none","verdict":"ours","sentence":"mine"}}"#, "fleet.no_such_watch"),
        ("delete_watch", r#"{"body":{"watch":"w-none"}}"#, "fleet.no_such_watch"),
        ("start_watch", r#"{"body":{"name":"x","in_terminal":"done"}}"#, "fleet.watch_terminal_closed"),
        ("try_watch", r#"{"body":{}}"#, "name a `recipe`"),
        ("save_watch_recipe", r#"{"body":{}}"#, "missing field `name`"),
    ] {
        let answered = at_the_door(&door, tool, arguments).await;
        assert!(answered.contains(expected), "{tool}: {answered}");
        assert!(!answered.contains("no armada.yml yet"), "{tool} is refused by the door: {answered}");
    }
}

/// A Job in a repository with no `armada.yml` dispatches without naming a repository, and what it
/// starts is in that repository and waits on its start card.
#[tokio::test]
async fn a_job_with_no_armada_yml_dispatches_into_its_own_repository_and_the_new_job_is_held() {
    let app = TempDir::new();
    let (rig, id, _) = a_job_in_a_bare_repository(&app).await;
    let mine = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session;
    let caller = api::Caller::at(format!("127.0.0.1:{RELAY}").parse().expect("an address"));

    let held = Arc::clone(&rig.fleet)
        .start_another_session(
            Some(caller),
            ipc::StartAnotherSession { manifest_id: None, title: "Check the reader".into(), message: "Read crates/reader.".into(), group: None, after: None },
        )
        .await
        .expect("started");

    let new = Arc::clone(&rig.fleet).get_session(held.session_id).await.expect("read").session;
    assert!(new.manifest_id.is_some());
    assert_eq!(new.manifest_id, mine.manifest_id, "the caller's own repository");
    assert!(new.hosted.expect("hosted").start.is_some(), "held on its start card");
}

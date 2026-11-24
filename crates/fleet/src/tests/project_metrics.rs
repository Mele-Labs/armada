//! A project's live metrics: the recipes are kept in its file and read with no model, fetched only when stale, and a
//! token is in no answer. Cli recipes are `sh` scripts that count their own calls; one case serves HTTP from a socket.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use api::Projects;
use ipc::{
    BoardBlock, BoardLayout, ManifestId, MetricCall, MetricCredential, MetricHeader, MetricRecipe, MetricState, MetricValue, NewProject,
    ProjectMember, SaveMetricToken, SaveProjectBoard, TryMetric,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::daemon::fittings;
use super::main_ci::MANIFEST;
use super::planted::Held as MovableClock;
use super::projects::code_of;
use super::projects_kept::file;
use super::requesting::Held as Keychain;
use super::session_host::{Shared, StandIn};
use super::tmp::TempDir;
use crate::daemon::Fleet;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

struct Rig {
    fleet: Arc<Hosted>,
    home: TempDir,
    project: String,
    clock: Arc<MovableClock>,
    keychain: Arc<Keychain>,
}

async fn a_rig() -> Rig {
    let home = TempDir::new();
    let clock = Arc::new(MovableClock::started());
    let keychain = Arc::new(Keychain::default());
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("parses");
    fittings.clock = clock.clone();
    fittings.metric_tokens = keychain.clone();
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_sessions_on(Arc::new(Shared(Arc::new(StandIn::default()))), Duration::from_secs(600)));
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let members = Some(vec![ProjectMember { manifest_id: ManifestId::carried(&manifest), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    let project = fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone();
    Rig { fleet, home, project, clock, keychain }
}

/// A cli recipe that adds a line to `calls` each time it runs and answers one row, `7`.
fn counting(id: &str, calls: &Path) -> MetricRecipe {
    let script = format!(r#"echo x >> {}; echo '{{"data":{{"n":7}}}}'"#, calls.display());
    MetricRecipe {
        id: id.to_string(),
        title: id.to_string(),
        call: MetricCall::Cli { argv: vec!["/bin/sh".into(), "-c".into(), script] },
        map: r#"{rows: [{label: "open", value: .data.n}]}"#.to_string(),
        every_secs: Some(30),
        credential: None,
    }
}

fn calls_in(calls: &Path) -> usize {
    std::fs::read_to_string(calls).map(|text| text.lines().count()).unwrap_or(0)
}

fn calls_file(rig: &Rig) -> PathBuf {
    rig.home.path().join("calls")
}

fn ask(recipe: MetricRecipe) -> TryMetric {
    TryMetric { recipe }
}

fn block(kind: &str, y: u8) -> BoardBlock {
    BoardBlock { kind: kind.to_string(), x: 0, y, w: 3, h: 2, view: "list".to_string() }
}

#[tokio::test]
async fn a_recipe_is_kept_in_the_projects_file_and_read_back_after_the_file_is_read_again() {
    let rig = a_rig().await;
    let calls = calls_file(&rig);
    let saved = rig.fleet.save_metric(rig.project.clone(), ask(counting("errors", &calls))).await.expect("saved");
    assert_eq!(saved.reading.state, MetricState::Read);
    assert_eq!(saved.reading.answer.expect("its first reading").rows[0].value, MetricValue::Number(7.0));

    let written = std::fs::read_to_string(file(&rig.home, &rig.project)).expect("its file");
    assert!(written.contains("metrics:") && written.contains("id: errors"), "kept in the project's file: {written}");
    rig.fleet.projects_reload().await;
    let read = rig.fleet.read_metrics(rig.project.clone()).await.expect("read");
    assert_eq!(read.readings.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["errors"]);
}

#[tokio::test]
async fn a_recipe_that_cannot_read_is_refused_and_not_kept() {
    let rig = a_rig().await;
    let broken = MetricRecipe { call: MetricCall::Cli { argv: vec!["/bin/sh".into(), "-c".into(), "echo it is down >&2; exit 3".into()] }, ..counting("down", &calls_file(&rig)) };
    let refused = rig.fleet.save_metric(rig.project.clone(), ask(broken)).await.expect_err("it cannot read");
    assert_eq!(refused.status(), 422);
    assert_eq!(refused.error().code, "fleet.metric_unreadable");
    assert!(refused.error().message.contains("it is down"), "the why is said: {}", refused.error().message);
    let written = std::fs::read_to_string(file(&rig.home, &rig.project)).expect("its file");
    assert!(!written.contains("metrics:"), "nothing is kept: {written}");
    assert!(rig.fleet.read_metrics(rig.project.clone()).await.expect("read").readings.is_empty());
}

#[tokio::test]
async fn a_recipe_that_only_lacks_this_macs_token_is_kept_and_reads_once_the_token_is_saved() {
    let rig = a_rig().await;
    let calls = calls_file(&rig);
    let credential = MetricCredential { name: "sentry".into(), how: "A Sentry auth token".into(), url: None };
    let recipe = MetricRecipe { credential: Some(credential.clone()), ..counting("errors", &calls) };
    let saved = rig.fleet.save_metric(rig.project.clone(), ask(recipe)).await.expect("kept without a token");
    assert_eq!(saved.reading.state, MetricState::NoToken { credential });
    assert_eq!(calls_in(&calls), 0, "no call is made without the token");
    assert!(std::fs::read_to_string(file(&rig.home, &rig.project)).expect("its file").contains("metrics:"));

    let after = rig.fleet.save_metric_token(rig.project.clone(), SaveMetricToken { credential: "sentry".into(), token: "tok-1".into() }).await.expect("kept");
    assert_eq!(after.readings[0].state, MetricState::Read, "read again once the token is there");
    assert_eq!(calls_in(&calls), 1);
    assert!(rig.keychain.accounts().iter().any(|one| one.ends_with("/sentry")), "kept by the credential's name: {:?}", rig.keychain.accounts());
    let written = std::fs::read_to_string(file(&rig.home, &rig.project)).expect("its file");
    assert!(!written.contains("tok-1"), "a token is in no file: {written}");
}

#[tokio::test]
async fn read_metrics_fetches_a_stale_recipe_and_serves_a_fresh_one_without_a_call_and_two_asks_make_one() {
    let rig = a_rig().await;
    let calls = calls_file(&rig);
    rig.fleet.save_metric(rig.project.clone(), ask(counting("errors", &calls))).await.expect("saved");
    assert_eq!(calls_in(&calls), 1, "the try that saved it");

    let fresh = rig.fleet.read_metrics(rig.project.clone()).await.expect("read");
    assert_eq!(calls_in(&calls), 1, "a reading inside its interval is served without a call");
    assert_eq!(fresh.readings[0].kept.len(), 1);

    rig.clock.on(31);
    let stale = rig.fleet.read_metrics(rig.project.clone()).await.expect("read");
    assert_eq!(calls_in(&calls), 2, "older than its interval, it is fetched");
    assert_eq!(stale.readings[0].kept.len(), 2, "a point is kept for each reading");

    rig.clock.on(31);
    let (one, other) = tokio::join!(rig.fleet.read_metrics(rig.project.clone()), rig.fleet.read_metrics(rig.project.clone()));
    assert_eq!(calls_in(&calls), 3, "two asks at once wait on one fetch");
    assert_eq!(one.expect("read").readings[0].kept.len(), 3);
    assert_eq!(other.expect("read").readings[0].kept.len(), 3);
}

#[test]
fn kept_points_are_the_first_number_of_each_good_reading_and_the_oldest_five_hundred_at_most() {
    let mut entry = crate::project_metrics::Entry::default();
    let answer = |value: f64| ipc::MetricAnswer {
        rows: vec![
            ipc::MetricRow { label: "words".into(), value: MetricValue::Text("fine".into()), unit: None, at: None, link: None },
            ipc::MetricRow { label: "n".into(), value: MetricValue::Number(value), unit: None, at: None, link: None },
        ],
        picture: None,
    };
    for n in 0..520 {
        let now = core_model::Timestamp::from_rfc3339(format!("2026-08-26T09:{:02}:{:02}.000Z", (n / 60) % 60, n % 60));
        entry.took(&now, Ok(answer(f64::from(n))));
    }
    entry.took(&core_model::Timestamp::from_rfc3339("2026-08-26T10:00:00.000Z".to_string()), Err("down".to_string()));
    assert_eq!(entry.kept.len(), 500);
    assert_eq!(entry.kept.first().map(|point| point.value), Some(20.0), "the oldest are dropped");
    assert_eq!(entry.kept.last().map(|point| point.value), Some(519.0), "a broken fetch adds none");
}

/// One HTTP server: each connection is answered with the next of `answers`.
fn serving(answers: Vec<String>) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        for answer in answers {
            let (mut stream, _) = listener.accept().unwrap();
            let mut seen = Vec::new();
            let mut chunk = [0u8; 2048];
            while !String::from_utf8_lossy(&seen).contains("\r\n\r\n") {
                let n = stream.read(&mut chunk).unwrap();
                if n == 0 {
                    break;
                }
                seen.extend_from_slice(&chunk[..n]);
            }
            stream.write_all(answer.as_bytes()).unwrap();
        }
    });
    (url, handle)
}

fn http(status: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

#[tokio::test]
async fn a_token_in_a_failing_calls_error_is_scrubbed_and_the_last_answer_that_read_is_kept() {
    const TOKEN: &str = "tok-s3cret/+";
    let rig = a_rig().await;
    let (url, server) = serving(vec![http("200 OK", r#"{"data":{"n":3}}"#), http("401 Unauthorized", &format!(r#"{{"detail":"bad token {TOKEN}"}}"#))]);
    let recipe = MetricRecipe {
        id: "errors".into(),
        title: "Errors".into(),
        call: MetricCall::Http {
            method: "GET".into(),
            url: format!("{url}/issues"),
            headers: vec![MetricHeader { name: "Authorization".into(), value: "Bearer {credential}".into() }],
            body: None,
        },
        map: r#"{rows: [{label: "open", value: .data.n}]}"#.into(),
        every_secs: Some(30),
        credential: Some(MetricCredential { name: "sentry".into(), how: "A token".into(), url: None }),
    };
    rig.fleet.save_metric_token(rig.project.clone(), SaveMetricToken { credential: "sentry".into(), token: TOKEN.into() }).await.expect("token");
    rig.fleet.save_metric(rig.project.clone(), ask(recipe)).await.expect("it read once");

    rig.clock.on(31);
    let broken = rig.fleet.read_metrics(rig.project.clone()).await.expect("read");
    let reading = &broken.readings[0];
    let MetricState::Broken { why } = &reading.state else { panic!("a 401 breaks it: {:?}", reading.state) };
    assert!(why.contains("401") && !why.contains(TOKEN), "the token is scrubbed: {why}");
    assert_eq!(reading.answer.as_ref().map(|answer| answer.rows[0].value.clone()), Some(MetricValue::Number(3.0)), "the last answer that read stands");
    let wire = ipc::encode(&broken).expect("encoded");
    assert!(!wire.contains(TOKEN), "no answer carries the token");
    let (fixing, said) = rig.fleet.metric_for_fix(&rig.project, "errors").await.expect("the recipe");
    assert_eq!(fixing.id, "errors");
    assert_eq!(said.as_deref(), Some(why.as_str()), "the librarian is given the why it last failed with");
    server.join().unwrap();
}

#[tokio::test]
async fn a_saved_recipe_gets_a_block_at_the_bottom_of_a_saved_board_and_remove_drops_both() {
    let rig = a_rig().await;
    let calls = calls_file(&rig);
    let layout = BoardLayout { blocks: vec![block("needs", 0), block("checks", 2)] };
    rig.fleet.save_project_board(rig.project.clone(), SaveProjectBoard { layout: Some(layout) }).await.expect("a board");
    rig.fleet.save_metric(rig.project.clone(), ask(counting("errors", &calls))).await.expect("saved");
    rig.fleet.save_metric(rig.project.clone(), ask(counting("errors", &calls))).await.expect("saved again replaces");

    let board = rig.fleet.get_project_board(rig.project.clone()).await.expect("board").layout.expect("a layout");
    let added: Vec<_> = board.blocks.iter().filter(|one| one.kind == "metric:errors").collect();
    assert_eq!(added.len(), 1, "one block, however often it is saved");
    assert_eq!((added[0].x, added[0].y, added[0].w, added[0].h, added[0].view.as_str()), (0, 4, 3, 2, "list"), "at the bottom");

    let left = rig.fleet.remove_metric(rig.project.clone(), "errors".to_string()).await.expect("removed");
    assert!(left.readings.is_empty());
    let board = rig.fleet.get_project_board(rig.project.clone()).await.expect("board").layout.expect("a layout");
    assert_eq!(board.blocks.iter().map(|one| one.kind.as_str()).collect::<Vec<_>>(), ["needs", "checks"], "the block goes with the recipe");
    let written = std::fs::read_to_string(file(&rig.home, &rig.project)).expect("its file");
    assert!(!written.contains("metrics:") && !written.contains("metric:errors"), "{written}");
    rig.fleet.remove_metric(rig.project.clone(), "errors".to_string()).await.expect("removing what is not kept is not an error");
}

#[tokio::test]
async fn a_project_that_kept_no_board_keeps_none_when_a_recipe_is_saved() {
    let rig = a_rig().await;
    rig.fleet.save_metric(rig.project.clone(), ask(counting("errors", &calls_file(&rig)))).await.expect("saved");
    assert_eq!(rig.fleet.get_project_board(rig.project.clone()).await.expect("board").layout, None, "Bridge appends it to its preset");
}

#[tokio::test]
async fn a_try_keeps_nothing_and_a_missing_project_is_not_found() {
    let rig = a_rig().await;
    let calls = calls_file(&rig);
    let tried = rig.fleet.try_metric(rig.project.clone(), ask(counting("errors", &calls))).await.expect("tried");
    assert_eq!(tried.state, MetricState::Read);
    assert!(tried.kept.is_empty(), "a try keeps no point");
    assert!(rig.fleet.read_metrics(rig.project.clone()).await.expect("read").readings.is_empty(), "and no recipe");
    let refused = rig.fleet.read_metrics("nope".to_string()).await.expect_err("no such project");
    assert_eq!(code_of(refused), (404, "fleet.no_such_project".to_string()));
    let refused = rig.fleet.save_metric_token(rig.project.clone(), SaveMetricToken { credential: "no spaces".into(), token: "x".into() }).await.expect_err("a bad name");
    assert_eq!(code_of(refused), (422, "fleet.metric_token_refused".to_string()));
}

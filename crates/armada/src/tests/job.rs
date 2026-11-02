//! `armada job` against a door that is a thread on a socket: what it lists, what it sends, what it
//! prints and how it refuses. No Fleet runs; the door's own answers are written here as the door
//! writes them.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use ipc::{ManifestId, ManifestSummary};

use crate::cli::{read, Body, Fault, JobAct, Verb};
use crate::job::{called, first_sentence, help, job_lines, listing, manifest_for, offered};
use crate::loopback::Loopback;
use crate::mcp::Reached;

fn asked(line: &str) -> Result<Verb, crate::cli::Misread> {
    read(line.split_whitespace().map(str::to_string))
}

fn job(line: &str) -> JobAct {
    match asked(line).expect("a job line") {
        Verb::Job(act) => act,
        other => panic!("{other:?}"),
    }
}

const TOOLS: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[
  {"name":"list_manifests","description":"The Manifests Fleet holds. GET /manifests","inputSchema":{"type":"object","properties":{},"required":[]}},
  {"name":"start_another_job","description":"Start a new job in any repository. Body: manifest_id, title, message. POST /sessions/start_another","inputSchema":{"type":"object","properties":{"body":{"type":"object","description":"x"}},"required":[]}},
  {"name":"waiting_for","description":"Set the whole list. POST /sessions/waiting","inputSchema":{"type":"object","properties":{"body":{"type":"object","description":"x"}},"required":[]}}
]}}"#;

fn done(text: &str) -> String {
    let text = text.replace('"', "\\\"");
    format!(r#"{{"jsonrpc":"2.0","id":2,"result":{{"content":[{{"type":"text","text":"{text}"}}],"isError":false}}}}"#)
}

fn refused(text: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":2,"result":{{"content":[{{"type":"text","text":"{text}"}}],"isError":true}}}}"#)
}

/// A door on loopback that answers each message with what `answer` makes of it, and keeps what it
/// was sent. Nothing about the answers is Fleet's: only the framing is.
fn door(answer: impl Fn(&str) -> String + Send + 'static) -> (Reached, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("an address").port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    std::thread::spawn(move || {
        for socket in listener.incoming() {
            let Ok(mut socket) = socket else { break };
            let mut raw = Vec::new();
            let mut chunk = [0u8; 4096];
            let body = loop {
                let read = socket.read(&mut chunk).unwrap_or(0);
                raw.extend_from_slice(&chunk[..read]);
                let text = String::from_utf8_lossy(&raw).to_string();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .and_then(|n| n.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if body.len() >= length || read == 0 {
                        break body.to_string();
                    }
                }
                if read == 0 {
                    break String::new();
                }
            };
            kept.lock().expect("seen").push(body.clone());
            let said = answer(&body);
            let _ = socket.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{said}",
                    said.len()
                )
                .as_bytes(),
            );
        }
    });
    let reached = Reached {
        fleet: Loopback::at(port),
        door: String::from("/mcp"),
        adopted: None,
        standing: None,
    };
    (reached, seen)
}

fn manifest(id: &str, repository: &str, path: &str) -> ManifestSummary {
    ManifestSummary {
        id: ManifestId::carried(id),
        repository: repository.to_string(),
        path: path.to_string(),
        records_root: String::new(),
        version: 1,
        checks: Vec::new(),
        auto_merge: String::from("never"),
        review_gate: String::from("human_always"),
        kit: None,
    }
}

#[test]
fn the_words_after_job_are_an_act_fields_and_one_body() {
    assert_eq!(job("job"), JobAct::Acts);
    assert_eq!(job("job status"), JobAct::Status);
    assert_eq!(job("job waiting_for --help"), JobAct::Help { act: "waiting_for".into() });
    assert_eq!(
        job("job dispatch repo=web title=Fix"),
        JobAct::Call {
            act: "start_another_job".into(),
            fields: vec![("repo".into(), "web".into()), ("title".into(), "Fix".into())],
            body: None,
        },
        "`dispatch` is the act that starts a Job"
    );
    assert_eq!(
        job("job finish-up"),
        JobAct::Call { act: "finish_up".into(), fields: vec![], body: None },
        "a dash in an act's name is read as an underscore"
    );
    assert_eq!(
        job(r#"job waiting_for {"items":[{"text":"a=b"}]}"#),
        JobAct::Call {
            act: "waiting_for".into(),
            fields: vec![],
            body: Some(Body::Inline(r#"{"items":[{"text":"a=b"}]}"#.into())),
        },
        "an equals sign inside a JSON body does not make it a field"
    );
    assert_eq!(
        job("job record_plan -"),
        JobAct::Call { act: "record_plan".into(), fields: vec![], body: Some(Body::Stdin) }
    );
}

#[test]
fn a_word_that_is_none_of_the_three_is_refused_by_name() {
    let said = asked("job waiting_for nonsense").expect_err("refused").to_string();
    assert!(said.contains("`nonsense` is not `name=value`"), "{said}");
    let said = asked("job record_plan - {}").expect_err("two bodies").to_string();
    assert!(said.contains("takes one JSON body"), "{said}");
    assert!(matches!(
        asked("job status now").expect_err("refused").faults[0],
        Fault::JobStatusTakesNothing { .. }
    ));
}

#[test]
fn the_list_comes_from_the_door_and_names_status_and_dispatch() {
    let (reached, seen) = door(|_| TOOLS.to_string());
    let all = offered(&reached).expect("the list");
    let said = listing(&all);
    assert!(said.contains("status") && said.contains("list_manifests"), "{said}");
    assert!(said.contains("start_another_job") && said.contains("(also `dispatch`)"), "{said}");
    assert!(seen.lock().expect("seen")[0].contains(r#""method":"tools/list""#));
    assert_eq!(first_sentence("Start a new job. Body: x. POST /y"), "Start a new job");
    assert!(help(&all[1]).contains("armada job dispatch [name=value ...] [<json> | -]"));
}

#[test]
fn a_call_prints_the_act_s_answer_and_a_refusal_is_fleet_s_sentence() {
    let (reached, seen) = door(|asked| match asked.contains("tools/list") {
        true => TOOLS.to_string(),
        false if asked.contains("no_such_thing") => refused("that is not yours"),
        false => done(r#"{"ok":true}"#),
    });
    let all = offered(&reached).expect("the list");
    let waiting = all.iter().find(|act| act.name == "waiting_for").expect("an act");

    let said = called(&reached, waiting, &[], Some(r#"{"items":[]}"#), None).expect("an answer");
    assert_eq!(said, r#"{"ok":true}"#);
    let sent = seen.lock().expect("seen").last().cloned().expect("a call");
    assert!(sent.contains(r#""name":"waiting_for""#) && sent.contains(r#""items":[]"#), "{sent}");

    let why = called(&reached, waiting, &[("no_such_thing".into(), "1".into())], None, None)
        .expect_err("refused");
    assert_eq!(why, "that is not yours");
}

#[test]
fn a_terminal_job_is_named_and_an_act_that_reads_no_session_is_sent_again_without_it() {
    let (reached, seen) = door(|asked| match asked.contains("tools/list") {
        true => TOOLS.to_string(),
        false if asked.contains("\"session_id\"") => refused("unknown field `session_id`"),
        false => done("{}"),
    });
    let all = offered(&reached).expect("the list");
    let waiting = all.iter().find(|act| act.name == "waiting_for").expect("an act");

    called(&reached, waiting, &[], None, Some("abc-123")).expect("answered the second time");
    let seen = seen.lock().expect("seen");
    let calls: Vec<&String> = seen.iter().filter(|body| body.contains("tools/call")).collect();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert!(calls[0].contains("abc-123"), "the first names the Job: {}", calls[0]);
    assert!(!calls[1].contains("abc-123"), "the second does not: {}", calls[1]);
}

#[test]
fn fleet_being_down_is_said_and_not_an_empty_answer() {
    let port = TcpListener::bind("127.0.0.1:0").expect("a port").local_addr().expect("address").port();
    let reached = Reached {
        fleet: Loopback::at(port),
        door: String::from("/mcp"),
        adopted: None,
        standing: None,
    };
    let why = offered(&reached).expect_err("nothing listens");
    assert!(why.starts_with("Armada stopped answering:"), "{why}");
}

#[test]
fn a_repository_name_is_the_manifest_it_names_and_an_unknown_one_lists_what_exists() {
    let held = [
        manifest("01WEB", "web", "/r/web/armada.yml"),
        manifest("01API", "api", "/r/api/armada.yml"),
        manifest("01TWIN", "web", "/other/web/armada.yml"),
    ];
    assert_eq!(manifest_for("API", &held).as_deref(), Ok("01API"), "any case");
    assert_eq!(manifest_for("01WEB", &held).as_deref(), Ok("01WEB"), "or by id");
    let why = manifest_for("docs", &held).expect_err("unknown");
    assert!(why.contains("no repository is named `docs`") && why.contains("web, api, web"), "{why}");
    let why = manifest_for("web", &held).expect_err("two");
    assert!(why.contains("/r/web/armada.yml") && why.contains("/other/web/armada.yml"), "{why}");
}

#[test]
fn status_tells_a_mod_that_never_reported_from_one_that_did() {
    let row = |mod_version: Option<&str>, missing: &[ipc::ToolMissing]| ipc::SessionSummary {
        id: ipc::SessionId::carried("s1"),
        title: Some("Fix it".into()),
        manifest_id: None,
        origin: ipc::SessionOrigin::Bridge,
        state: ipc::SessionState::Live,
        turn: None,
        last_turn_at: None,
        last_seen_at: ipc::Instant::carried("2026-11-02T09:00:00.000Z"),
        slot: None,
        branch: None,
        pull_requests: Vec::new(),
        mod_version: mod_version.map(str::to_string),
        tools_missing: missing.to_vec(),
    };
    let none = job_lines(&row(None, &[ipc::ToolMissing::Mod])).join("\n");
    assert!(none.contains("no mod has reported") && none.contains("the armada mod did not load"), "{none}");
    let some = job_lines(&row(Some("1.4.0"), &[])).join("\n");
    assert!(some.contains("version 1.4.0, reported") && some.contains("missing     nothing"), "{some}");
}

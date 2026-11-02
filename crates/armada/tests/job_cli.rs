//! `armada job`, the binary, against a Fleet that is a thread on a socket.
//!
//! **The real program, run as an agent's Bash would run it**: from a repository's folder, with the
//! runtime file where Fleet publishes it and the door answering on the port in it. What is proved
//! is what a unit test of the parts cannot: the exit code, which stream each answer is on, and that
//! the CLI finds Fleet, scopes itself to a Manifest and speaks the door's two methods in order.
//!
//! | Proved | Not proved |
//! |---|---|
//! | `armada job` lists the acts the door lists and calls one, its JSON on stdout and exit 0 | That Fleet places the caller by its process tree. `fleet::peer` and `api::tests::door_scope` |
//! | A refusal is Fleet's sentence on stderr and a non-zero exit, stdout empty | That the door cuts a long answer. `ipc::tests::door` |
//! | Fleet not running is said on stderr with a non-zero exit | A hosted Job's own agent running the command. Needs a Fleet hosting a real agent |

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};

use fleet::runtime::{self, RuntimeFile};
use ipc::{ManifestId, ManifestSummary, ProtocolId};

const TOOLS: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[
  {"name":"waiting_for","description":"Set the whole list. POST /sessions/waiting","inputSchema":{"type":"object","properties":{"body":{"type":"object","description":"x"}},"required":[]}},
  {"name":"finish_up","description":"Give back what this job is finished with. POST /sessions/finish_up","inputSchema":{"type":"object","properties":{"body":{"type":"object","description":"x"}},"required":[]}}
]}}"#;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!("armada-job-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch folder");
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A repository folder with a Manifest, and a home to publish the runtime file under.
fn world(scratch: &Scratch) -> (PathBuf, PathBuf) {
    let repository = scratch.0.join("web");
    let home = scratch.0.join("home");
    std::fs::create_dir_all(&repository).expect("a repository");
    std::fs::create_dir_all(&home).expect("a home");
    std::fs::write(repository.join("armada.yml"), "version: 1\nid: 01FIXTUREMANIFEST\n").expect("a Manifest");
    (repository, home)
}

/// Fleet, as far as the CLI meets it: the Manifests it serves and the door. Returns the port and the
/// door messages it was sent.
fn fleet(answer: impl Fn(&str) -> String + Send + 'static) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("an address").port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    std::thread::spawn(move || {
        for socket in listener.incoming() {
            let Ok(mut socket) = socket else { break };
            let mut raw = Vec::new();
            let mut chunk = [0u8; 4096];
            let (head, body) = loop {
                let read = socket.read(&mut chunk).unwrap_or(0);
                raw.extend_from_slice(&chunk[..read]);
                let text = String::from_utf8_lossy(&raw).to_string();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .and_then(|n| n.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if body.len() >= length {
                        break (head.to_string(), body.to_string());
                    }
                }
                if read == 0 {
                    break (String::new(), String::new());
                }
            };
            let said = match head.starts_with("GET /manifests") {
                true => manifests(),
                false => {
                    kept.lock().expect("seen").push(body.clone());
                    answer(&body)
                }
            };
            let _ = socket.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{said}",
                    said.len()
                )
                .as_bytes(),
            );
        }
    });
    (port, seen)
}

fn manifests() -> String {
    ipc::encode(&vec![ManifestSummary {
        id: ManifestId::carried("01FIXTUREMANIFEST"),
        repository: String::from("web"),
        path: String::from("/repos/web/armada.yml"),
        records_root: String::new(),
        version: 1,
        checks: Vec::new(),
        auto_merge: String::from("never"),
        review_gate: String::from("human_always"),
        kit: None,
    }])
    .expect("a list encodes")
}

/// Publish this process as the Fleet at `port`, where `home` says the runtime file is.
fn published(home: &Path, port: u16) -> runtime::Published {
    let at = home.join("Library/Application Support/Armada/fleet.json");
    let vacancy = runtime::read(&at).expect("a read").vacancy(&at).expect("a vacant path");
    RuntimeFile::publish(vacancy, port, ProtocolId::current()).expect("published")
}

fn armada(home: &Path, in_folder: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut run = Command::new(env!("CARGO_BIN_EXE_armada"));
    run.args(args)
        .current_dir(in_folder)
        .env("HOME", home)
        .env_remove("ARMADA_SESSION")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = run.spawn().expect("the binary starts");
    if let Some(text) = stdin {
        child.stdin.take().expect("stdin").write_all(text.as_bytes()).expect("a body");
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("it ends")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn door(asked: &str, refuse: &str) -> String {
    match asked {
        ping if ping.contains(r#""method":"ping""#) => String::from(r#"{"jsonrpc":"2.0","id":0,"result":{}}"#),
        list if list.contains("tools/list") => TOOLS.to_string(),
        call if call.contains(refuse) => {
            String::from(r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"nothing is waiting on you"}],"isError":true}}"#)
        }
        _ => String::from(r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"ok\":true}"}],"isError":false}}"#),
    }
}

#[test]
fn armada_job_lists_the_acts_and_calls_one() {
    let scratch = Scratch::new("calls");
    let (repository, home) = world(&scratch);
    let (port, seen) = fleet(|asked| door(asked, "NEVER"));
    let _published = published(&home, port);

    let listed = armada(&home, &repository, &["job"], None);
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    let said = text(&listed.stdout);
    assert!(said.contains("status") && said.contains("waiting_for") && said.contains("finish_up"), "{said}");

    let called = armada(&home, &repository, &["job", "waiting-for", r#"{"items":[{"id":"a","text":"Restart"}]}"#], None);
    assert!(called.status.success(), "{}", text(&called.stderr));
    assert_eq!(text(&called.stdout).trim(), r#"{"ok":true}"#);
    let sent = seen.lock().expect("seen").iter().find(|body| body.contains("tools/call")).cloned().expect("a call");
    assert!(sent.contains(r#""name":"waiting_for""#) && sent.contains("Restart"), "{sent}");

    let piped = armada(&home, &repository, &["job", "finish_up", "-"], Some("{}"));
    assert!(piped.status.success(), "{}", text(&piped.stderr));

    let help = armada(&home, &repository, &["job", "finish_up", "--help"], None);
    assert!(help.status.success() && text(&help.stdout).contains("Give back what this job is finished with"));
}

#[test]
fn a_refusal_is_fleet_s_sentence_on_stderr_and_a_failing_exit() {
    let scratch = Scratch::new("refused");
    let (repository, home) = world(&scratch);
    let (port, _seen) = fleet(|asked| door(asked, "finish_up"));
    let _published = published(&home, port);

    let refused = armada(&home, &repository, &["job", "finish_up"], None);
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty(), "{}", text(&refused.stdout));
    assert!(text(&refused.stderr).contains("nothing is waiting on you"), "{}", text(&refused.stderr));

    let unknown = armada(&home, &repository, &["job", "no_such_act"], None);
    assert!(!unknown.status.success());
    assert!(text(&unknown.stderr).contains("`no_such_act` is not an act this Job may call"), "{}", text(&unknown.stderr));
}

#[test]
fn fleet_not_running_is_said_on_stderr_with_a_failing_exit() {
    let scratch = Scratch::new("down");
    let (repository, home) = world(&scratch);

    let down = armada(&home, &repository, &["job"], None);
    assert!(!down.status.success());
    assert!(down.stdout.is_empty());
    assert!(text(&down.stderr).contains("Armada is not running"), "{}", text(&down.stderr));
}

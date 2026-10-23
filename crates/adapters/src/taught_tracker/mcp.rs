//! One tool called on an MCP server that is started for the call and stopped after it: the
//! handshake, the call, and the answer as JSON for the mod's `map`.
//!
//! **A server per call, not one kept.** A tracker is read a few times a minute at most, and a kept
//! server is a process to supervise, restart and clean up across a Fleet restart for the sake of
//! one process start. Each call also gets its own `env`, so a credential is never one a server
//! kept from the call before.
//!
//! **Nothing here reads JSON but `ipc::decode`**, for a message's `id`, `method` and `error`;
//! what the tool said is read by `jq`, which is also how `map` reads it.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::mapping;
use super::running::{self, MOST_OUTPUT};

/// How long a server is given to end on its own once its input is closed.
const GRACE: Duration = Duration::from_secs(1);

/// How long a server that has stopped is waited on for what it said on its error output.
const LAST_WORDS: Duration = Duration::from_millis(200);

/// How much of a server's error output is kept.
const MOST_STDERR: usize = 16 * 1024;

const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"armada","version":"0"}}}"#;
const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

/// What the call answered, as one line of JSON: the structured content where there is any,
/// otherwise the first text part read as JSON. An error result is the tool's, with its text.
const ANSWER: &str = r#"
.result as $r
| if $r == null then error("the server's answer has no result")
  elif $r.isError == true then
    error("the tool reported an error: " + ([$r.content[]? | select(.type == "text") | .text] | join("\n")))
  elif $r.structuredContent != null then $r.structuredContent
  else
    ([$r.content[]? | select(.type == "text") | .text][0]) as $text
    | if $text == null then error("the server's answer has neither structured content nor a text part")
      else ($text | try fromjson catch error("the server's answer has no structured content and its text is not JSON"))
      end
  end
"#;

/// The three things in a message that say whose it is.
#[derive(Deserialize)]
struct Frame {
    id: Option<u64>,
    method: Option<String>,
    error: Option<Refusal>,
}

#[derive(Deserialize)]
struct Refusal {
    message: Option<String>,
}

/// What came off the server's output.
enum Heard {
    Line(Vec<u8>),
    /// More than [`MOST_OUTPUT`] bytes in all.
    Over,
    Closed,
}

/// Why an exchange stopped. A silent server's error output is worth saying with it; a server that
/// answered with a refusal has said what it meant.
enum Stop {
    Said(String),
    Silent(String),
}

/// `tool` called with `arguments` on `server` started with `env`, and what it answered as JSON.
/// The whole exchange is bounded by `within` and, in what the server says, by [`MOST_OUTPUT`].
pub(crate) fn call(
    server: &[String],
    env: &[(String, String)],
    tool: &str,
    arguments: &str,
    within: Duration,
) -> Result<Vec<u8>, String> {
    let program = server.first().map(String::as_str).unwrap_or_default();
    let mut child = Command::new(program)
        .args(&server[1..])
        .envs(env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| running::not_started(program, &why))?;
    let (Some(mut stdin), Some(stdout), Some(stderr)) = (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        let _ = child.kill();
        return Err(format!("`{program}` could not be talked to"));
    };
    let heard = listen(stdout);
    let (kept, keeping) = keep(stderr);
    let reply = exchange(&mut stdin, &heard, Instant::now() + within, within, tool, arguments);
    drop(stdin);
    stop(&mut child);
    match reply {
        Ok(reply) => answer_of(&reply),
        Err(Stop::Said(why)) => Err(why),
        Err(Stop::Silent(why)) => {
            let until = Instant::now() + LAST_WORDS;
            while !keeping.is_finished() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(5));
            }
            let said = kept.lock().map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string()).unwrap_or_default();
            Err(if said.is_empty() { why } else { format!("{why}: {said}") })
        }
    }
}

fn exchange(
    stdin: &mut ChildStdin,
    heard: &Receiver<Heard>,
    deadline: Instant,
    within: Duration,
    tool: &str,
    arguments: &str,
) -> Result<Vec<u8>, Stop> {
    send(stdin, INITIALIZE)?;
    awaited(heard, 1, deadline, within)?;
    send(stdin, INITIALIZED)?;
    let name = ipc::encode(&tool).map_err(|why| Stop::Said(why.to_string()))?;
    send(
        stdin,
        &format!(r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":{name},"arguments":{arguments}}}}}"#),
    )?;
    awaited(heard, 2, deadline, within)
}

fn send(stdin: &mut ChildStdin, message: &str) -> Result<(), Stop> {
    stdin
        .write_all(message.as_bytes())
        .and_then(|()| stdin.write_all(b"\n"))
        .and_then(|()| stdin.flush())
        .map_err(|why| Stop::Silent(format!("the server stopped reading: {why}")))
}

/// The reply to request `want`, as the line it came in. A line that is not JSON is a server's
/// log, and a message with a `method` is its own request or notification, neither answered.
fn awaited(heard: &Receiver<Heard>, want: u64, deadline: Instant, within: Duration) -> Result<Vec<u8>, Stop> {
    loop {
        match heard.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(Heard::Line(line)) => {
                let Ok(frame) = ipc::decode::<Frame>("an MCP message", line.trim_ascii()) else { continue };
                if frame.method.is_some() {
                    continue;
                }
                if let Some(refusal) = frame.error {
                    if frame.id.is_none() || frame.id == Some(want) {
                        let why = refusal.message.unwrap_or_else(|| "it gave no reason".to_string());
                        return Err(Stop::Said(format!("the server refused: {why}")));
                    }
                    continue;
                }
                if frame.id == Some(want) {
                    return Ok(line);
                }
            }
            Ok(Heard::Over) => return Err(Stop::Silent(format!("the server said more than {MOST_OUTPUT} bytes"))),
            Ok(Heard::Closed) | Err(RecvTimeoutError::Disconnected) => {
                return Err(Stop::Silent("the server closed its output before answering".to_string()));
            }
            Err(RecvTimeoutError::Timeout) => {
                return Err(Stop::Silent(format!("the server did not answer within {within:?}")));
            }
        }
    }
}

fn answer_of(reply: &[u8]) -> Result<Vec<u8>, String> {
    let jq = ["jq".to_string(), "-c".to_string(), ANSWER.to_string()];
    let ran = running::run(&jq, Some(reply))?;
    match ran.succeeded {
        true => Ok(ran.stdout),
        false => Err(mapping::refusal_of(&ran.stderr)),
    }
}

/// The server's output a line at a time from its own thread, so a deadline can pass while a read
/// waits. The thread ends when the server closes its output, or at the first byte past the bound.
fn listen(stdout: ChildStdout) -> Receiver<Heard> {
    let (send, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout.take(MOST_OUTPUT as u64 + 1));
        loop {
            let mut line = Vec::new();
            let read = reader.read_until(b'\n', &mut line);
            if reader.get_ref().limit() == 0 {
                let _ = send.send(Heard::Over);
                return;
            }
            match read {
                Ok(n) if n > 0 => {
                    if send.send(Heard::Line(line)).is_err() {
                        return;
                    }
                }
                _ => {
                    let _ = send.send(Heard::Closed);
                    return;
                }
            }
        }
    });
    heard
}

/// The head of the server's error output, kept as it arrives and read by whoever needs it. Not
/// joined: a grandchild holding the pipe open must not hold the call.
fn keep(mut stderr: ChildStderr) -> (Arc<Mutex<Vec<u8>>>, std::thread::JoinHandle<()>) {
    let kept = Arc::new(Mutex::new(Vec::new()));
    let into = Arc::clone(&kept);
    let keeping = std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut chunk) {
            if n == 0 {
                return;
            }
            if let Ok(mut bytes) = into.lock() {
                let room = MOST_STDERR.saturating_sub(bytes.len());
                bytes.extend_from_slice(&chunk[..n.min(room)]);
            }
        }
    });
    (kept, keeping)
}

/// The server, whose input is closed, given [`GRACE`] to end and killed after it.
fn stop(child: &mut Child) {
    let until = Instant::now() + GRACE;
    while Instant::now() < until {
        match child.try_wait() {
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            _ => return,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

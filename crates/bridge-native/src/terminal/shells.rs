//! Where a terminal pane's shell lives: Fleet's, on the routes Electron's `job-shell.ts` uses, or a
//! walk's, which a small fake shell answers. The pane asks the same things of either.
//!
//! | Act | Fleet |
//! |---|---|
//! | open or attach | `POST /sessions/terminal/open` |
//! | what it prints | `WS /sessions/run/:run_id/observe` |
//! | typing | `POST /sessions/run/write` |
//! | its size | `POST /sessions/run/resize` |
//! | End shell | `POST /sessions/run/stop` |
//! | the Job's shells | `GET /sessions/terminal/list` |
//! | Rename | `POST /sessions/terminal/rename` |
//! | Run a command an agent showed | `POST /sessions/run/start` |
//! | A terminal Job | `POST /sessions/terminal/start` |

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use async_channel::{Receiver, Sender};
use gpui::{App, Task};
use ipc::{
    JobTerminals, ListedTerminal, ManifestId, OpenTerminal, RenameTerminal, ResizeSessionRun, SessionId, SessionRunMessage, SessionRunUnderway, StartSessionRun,
    StartSideJob, StopSessionRun, SideJobStarted, WireError, WriteSessionRun,
};

use super::screen::Cells;

/// Which of a Job's shells a pane opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Which {
    /// The Job's first, `t1` where it has none.
    First,
    /// This one: its running shell, or a new shell under the same id where it ended.
    Named(String),
    /// The next id, with a new shell.
    New,
    /// No shell: this command alone, on a terminal of its own where the Job works, as Run on a block runs it.
    Command(String),
}

/// Where a terminal Job starts: a Manifest (or a repository's implicit id), or any folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Repository(String),
    Folder(String),
}

/// What a quick Job is started with: where, and the size a shell would open at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    pub place: Place,
    /// Its first shell's size.
    pub size: Cells,
    /// The project it is started for, by id: a New project's first Job. None where ⌘T starts it.
    pub project: Option<String>,
}

/// A terminal Job Fleet started: the Job and its first shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Started {
    pub job: String,
    pub terminal: String,
    pub run: String,
}

pub trait Shells {
    fn open(&self, job: &str, which: Which, size: Cells, cx: &mut App) -> Task<Result<SessionRunUnderway, String>>;
    /// What the run printed so far, then what it prints next, then its end. Closed when the socket is.
    fn observe(&self, run: &str, cx: &mut App) -> Receiver<SessionRunMessage>;
    /// Keys for the shell. A refusal is the sentence Fleet or the link gave, and the keys are gone: nothing holds
    /// them to type later (`share_job_terminal.toml`).
    fn write(&self, run: &str, data: String, cx: &mut App) -> Task<Result<(), String>>;
    fn resize(&self, run: &str, size: Cells, cx: &mut App);
    fn stop(&self, run: &str, cx: &mut App);
    fn list(&self, job: &str, cx: &mut App) -> Task<Result<JobTerminals, String>>;
    fn rename(&self, job: &str, terminal: &str, name: &str, cx: &mut App) -> Task<Result<ListedTerminal, String>>;
    /// A terminal Job: no words, its first shell open.
    fn start(&self, start: Start, cx: &mut App) -> Task<Result<Started, String>>;
    /// Whether `start` can start a quick Job here: ⌘T and the palette's New Job here are offered only where
    /// it can.
    fn starts(&self) -> bool;
    /// A command run where Job `job` works, as `$SHELL -lc` runs it, on a terminal of `size`.
    fn run(&self, job: &str, command: &str, size: Cells, cx: &mut App) -> Task<Result<SessionRunUnderway, String>>;
}

/// Fleet's shells, on the port its connection holds.
pub struct Live {
    pub port: Rc<dyn Fn(&App) -> Option<u16>>,
}

/// One request on a runtime of its own, off the main thread, read as `T`. A refusal is Fleet's sentence.
fn ask<T: serde::de::DeserializeOwned + Send + 'static>(port: Option<u16>, path: String, body: Option<String>, what: &'static str, cx: &mut App) -> Task<Result<T, String>> {
    cx.background_executor().spawn(async move {
        let port = port.ok_or_else(|| "Fleet is not connected".to_string())?;
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = match body {
            Some(body) => runtime.block_on(crate::transport::post(port, &path, body))?,
            None => runtime.block_on(crate::transport::get(port, &path))?,
        };
        if !(200..300).contains(&status) {
            return Err(ipc::decode::<WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message));
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| why.to_string())
    })
}

fn encoded<B: serde::Serialize>(body: &B) -> Option<String> {
    ipc::encode(body).ok()
}

impl Live {
    fn port(&self, cx: &App) -> Option<u16> {
        (self.port)(cx)
    }

    /// A request whose answer nothing waits on: typing, a size, Stop. A refusal is logged.
    fn tell<B: serde::Serialize>(&self, path: &'static str, body: &B, cx: &mut App) {
        let task: Task<Result<SessionRunUnderway, String>> = ask(self.port(cx), path.into(), encoded(body), "SessionRunUnderway", cx);
        cx.background_executor()
            .spawn(async move {
                if let Err(why) = task.await {
                    crate::log::Note::warn("a terminal request was refused").field("route", path).field("why", why).emit();
                }
            })
            .detach();
    }
}

impl Shells for Live {
    fn open(&self, job: &str, which: Which, size: Cells, cx: &mut App) -> Task<Result<SessionRunUnderway, String>> {
        let (terminal, new) = match which {
            Which::First => (None, false),
            Which::Named(id) => (Some(id), false),
            Which::New => (None, true),
            Which::Command(command) => return self.run(job, &command, size, cx),
        };
        let body = OpenTerminal { session_id: SessionId::carried(job.to_string()), terminal, new, cols: Some(size.cols), rows: Some(size.rows) };
        ask(self.port(cx), "/sessions/terminal/open".into(), encoded(&body), "SessionRunUnderway", cx)
    }

    fn observe(&self, run: &str, cx: &mut App) -> Receiver<SessionRunMessage> {
        let (tx, rx) = async_channel::unbounded();
        let port = self.port(cx);
        let path = format!("/sessions/run/{run}/observe");
        // The socket is read on a thread of its own, for as long as it is open.
        std::thread::spawn(move || {
            let Some(port) = port else { return };
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return };
            runtime.block_on(follow(port, path, tx));
        });
        rx
    }

    fn write(&self, run: &str, data: String, cx: &mut App) -> Task<Result<(), String>> {
        let writing: Task<Result<SessionRunUnderway, String>> = ask(self.port(cx), "/sessions/run/write".into(), encoded(&WriteSessionRun { run_id: run.to_string(), data }), "SessionRunUnderway", cx);
        cx.background_executor().spawn(async move { writing.await.map(|_| ()) })
    }

    fn resize(&self, run: &str, size: Cells, cx: &mut App) {
        self.tell("/sessions/run/resize", &ResizeSessionRun { run_id: run.to_string(), cols: size.cols, rows: size.rows }, cx);
    }

    fn stop(&self, run: &str, cx: &mut App) {
        self.tell("/sessions/run/stop", &StopSessionRun { run_id: run.to_string() }, cx);
    }

    fn list(&self, job: &str, cx: &mut App) -> Task<Result<JobTerminals, String>> {
        ask(self.port(cx), format!("/sessions/terminal/list?session_id={job}"), None, "JobTerminals", cx)
    }

    fn rename(&self, job: &str, terminal: &str, name: &str, cx: &mut App) -> Task<Result<ListedTerminal, String>> {
        let body = RenameTerminal { session_id: SessionId::carried(job.to_string()), terminal: terminal.to_string(), name: name.to_string() };
        ask(self.port(cx), "/sessions/terminal/rename".into(), encoded(&body), "ListedTerminal", cx)
    }

    /// A refusal is Fleet's sentence.
    fn start(&self, start: Start, cx: &mut App) -> Task<Result<Started, String>> {
        let body = start_body(start);
        let asking: Task<Result<SideJobStarted, String>> = ask(self.port(cx), "/sessions/side_job/start".into(), encoded(&body), "SideJobStarted", cx);
        cx.background_executor().spawn(async move {
            let started = asking.await?;
            Ok(Started { job: started.session_id.as_str().to_string(), terminal: started.terminal, run: started.run_id })
        })
    }

    fn starts(&self) -> bool {
        true
    }

    fn run(&self, job: &str, command: &str, size: Cells, cx: &mut App) -> Task<Result<SessionRunUnderway, String>> {
        let body = StartSessionRun { session_id: SessionId::carried(job.to_string()), command: command.to_string(), cols: Some(size.cols), rows: Some(size.rows) };
        ask(self.port(cx), "/sessions/run/start".into(), encoded(&body), "SessionRunUnderway", cx)
    }
}

async fn follow(port: u16, path: String, tx: Sender<SessionRunMessage>) {
    let io = match crate::transport::upgrade(port, &path).await {
        Ok(io) => io,
        Err(why) => {
            crate::log::Note::warn("a terminal's output could not be watched").field("why", why).emit();
            return;
        }
    };
    let mut messages = crate::frames::Messages::new(io);
    while let Some(text) = messages.next().await {
        match ipc::decode::<SessionRunMessage>("SessionRunMessage", text.as_bytes()) {
            Ok(message) => {
                let ended = matches!(message, SessionRunMessage::Exited { .. });
                if tx.send(message).await.is_err() || ended {
                    return;
                }
            }
            Err(why) => crate::log::Note::warn("a terminal message could not be read").field("why", why.to_string()).emit(),
        }
    }
}

/// A walk's shells: each prints a prompt, echoes what is typed, and answers a few commands with
/// what they print. No process runs.
#[derive(Default)]
pub struct Mock {
    state: RefCell<MockState>,
}

#[derive(Default)]
struct MockState {
    /// Each Job's shells, in the order opened: id, name, run.
    jobs: HashMap<String, Vec<(String, String, String)>>,
    runs: HashMap<String, MockRun>,
    next_run: u32,
    next_job: u32,
    /// The Jobs of the walk's own list a start becomes, in turn, so the Job it opens is one the list holds.
    starts_as: Vec<String>,
    /// The Jobs held on another Mac, whose shells go through the link.
    elsewhere: Vec<String>,
    /// The link to that Mac does not carry: an open and a key are refused in its words.
    link_down: bool,
}

/// What the link says when it cannot reach the other Mac, as the Gateway words it.
pub const UNREACHED: &str = "That Mac is not answering. It may be asleep.";

struct MockRun {
    tx: Option<Sender<SessionRunMessage>>,
    /// What it printed, replayed to a viewer that attaches.
    printed: String,
    line: Vec<char>,
    /// Where in the line the cursor stands.
    at: usize,
    folder: String,
    ended: bool,
    /// A program holds it, as `pnpm dev` does until ⌃C: Fleet lists it running.
    busy: bool,
    /// How it exited: the code, or none where it was stopped.
    code: Option<i32>,
}

/// The start of the word before `at`, as zsh's backward-word finds it.
fn word_left(line: &[char], at: usize) -> usize {
    let mut to = at;
    while to > 0 && line[to - 1] == ' ' {
        to -= 1;
    }
    while to > 0 && line[to - 1] != ' ' {
        to -= 1;
    }
    to
}

/// The end of the word after `at`.
fn word_right(line: &[char], at: usize) -> usize {
    let mut to = at;
    while to < line.len() && line[to] == ' ' {
        to += 1;
    }
    while to < line.len() && line[to] != ' ' {
        to += 1;
    }
    to
}

/// The prompt a walk's shell prints.
fn prompt(folder: &str) -> String {
    format!("\x1b[38;5;110m{folder}\x1b[0m \x1b[32m%\x1b[0m ")
}

/// What a walk's shell prints for a command, in the colours a real one would.
fn answer(command: &str) -> String {
    match command.trim() {
        "" => String::new(),
        "ls" => "\x1b[1;34mcrates\x1b[0m  \x1b[1;34mdocs\x1b[0m  \x1b[1;34mpackages\x1b[0m  Cargo.toml  README.md\r\n".into(),
        "git status" => "On branch \x1b[1mmain\x1b[0m\r\nChanges not staged for commit:\r\n\t\x1b[31mmodified:   crates/fleet/src/lib.rs\x1b[0m\r\n".into(),
        "cargo build" => "\x1b[1;32m   Compiling\x1b[0m fleet v0.0.0\r\n\x1b[1;32m    Finished\x1b[0m `dev` profile in 4.21s\r\n".into(),
        other => format!("zsh: command not found: {}\r\n", other.split_whitespace().next().unwrap_or(other)),
    }
}

impl Mock {
    fn print(state: &mut MockState, run: &str, text: String) {
        if let Some(one) = state.runs.get_mut(run) {
            one.printed.push_str(&text);
            if let Some(tx) = &one.tx {
                let _ = tx.try_send(SessionRunMessage::Output { text });
            }
        }
    }

    fn shell(state: &mut MockState, job: &str, id: String, folder: &str) -> String {
        state.next_run += 1;
        let run = format!("run-{}", state.next_run);
        let shells = state.jobs.entry(job.to_string()).or_default();
        let name = format!("Terminal {}", id.trim_start_matches('t'));
        match shells.iter_mut().find(|(one, _, _)| *one == id) {
            Some(held) => held.2 = run.clone(),
            None => shells.push((id, name, run.clone())),
        }
        state.runs.insert(run.clone(), MockRun { tx: None, printed: prompt(folder), line: Vec::new(), at: 0, folder: folder.to_string(), ended: false, busy: false, code: None });
        run
    }

    /// The Jobs a walk opens with shells already running in them, each with `count` shells.
    pub fn with(jobs: &[(&str, usize)]) -> Mock {
        let mock = Mock::default();
        {
            let mut state = mock.state.borrow_mut();
            for (job, count) in jobs {
                for at in 1..=*count {
                    Self::shell(&mut state, job, format!("t{at}"), "~/Development/armada");
                }
            }
        }
        mock
    }

    /// A start becomes these Jobs of the walk's list, in turn.
    pub fn starting_as(self, jobs: &[&str]) -> Mock {
        self.state.borrow_mut().starts_as = jobs.iter().map(|job| job.to_string()).collect();
        self
    }

    /// These Jobs are held on another Mac: their shells go through the link.
    pub fn elsewhere(self, jobs: &[&str]) -> Mock {
        self.state.borrow_mut().elsewhere = jobs.iter().map(|job| job.to_string()).collect();
        self
    }

    /// The link to the other Mac carries again, or stops carrying: a key or an open on its Jobs is refused.
    pub fn link(&self, up: bool) {
        self.state.borrow_mut().link_down = !up;
    }

    /// The link's stream from the other Mac drops: each of its shells that is watched ends here as Fleet ends it,
    /// `exited` with no code and not stopped, while the shell itself runs on there.
    pub fn streams_drop(&self) {
        let mut state = self.state.borrow_mut();
        let runs: Vec<String> = state.jobs.iter().filter(|(job, _)| state.elsewhere.contains(job)).flat_map(|(_, shells)| shells.iter().map(|(_, _, run)| run.clone())).collect();
        for run in runs {
            if let Some(tx) = state.runs.get_mut(&run).and_then(|one| one.tx.take()) {
                let _ = tx.try_send(SessionRunMessage::Exited { code: None, stopped: false });
            }
        }
    }

    /// Whether the link refuses what goes to Job `job`'s shells now.
    fn cut_off(state: &MockState, job: &str) -> bool {
        state.link_down && state.elsewhere.iter().any(|one| one == job)
    }

    /// The Job whose shell `run` is.
    fn job_of(state: &MockState, run: &str) -> Option<String> {
        state.jobs.iter().find(|(_, shells)| shells.iter().any(|(_, _, one)| one == run)).map(|(job, _)| job.clone())
    }

    /// End a shell as `exit` would, so a walk shows Shell ended.
    pub fn end(&self, job: &str, terminal: &str) {
        let mut state = self.state.borrow_mut();
        let Some(run) = state.jobs.get(job).and_then(|shells| shells.iter().find(|(id, _, _)| id == terminal)).map(|(_, _, run)| run.clone()) else { return };
        Self::finish(&mut state, &run, false);
    }

    fn finish(state: &mut MockState, run: &str, stopped: bool) {
        Self::exit(state, run, if stopped { None } else { Some(0) });
    }

    fn exit(state: &mut MockState, run: &str, code: Option<i32>) {
        if let Some(one) = state.runs.get_mut(run) {
            if one.ended {
                return;
            }
            one.ended = true;
            one.code = code;
            if let Some(tx) = one.tx.take() {
                let _ = tx.try_send(SessionRunMessage::Exited { code, stopped: code.is_none() });
            }
        }
    }
}

impl Mock {
    /// See [`Shells::open`]: the mock answers at once.
    pub fn opened(&self, job: &str, which: Which) -> SessionRunUnderway {
        let mut state = self.state.borrow_mut();
        let held = state.jobs.get(job).cloned().unwrap_or_default();
        let id = match which {
            Which::First => held.first().map_or_else(|| "t1".to_string(), |(id, _, _)| id.clone()),
            Which::Named(id) => id,
            Which::New => format!("t{}", held.iter().filter_map(|(id, _, _)| id[1..].parse::<u32>().ok()).max().unwrap_or(0) + 1),
            // `open` hands a command to `run`.
            Which::Command(_) => return SessionRunUnderway { run_id: String::new(), terminal: None },
        };
        let running = held.iter().find(|(one, _, _)| *one == id).map(|(_, _, run)| run.clone()).filter(|run| state.runs.get(run).is_some_and(|one| !one.ended));
        let run = running.unwrap_or_else(|| Self::shell(&mut state, job, id.clone(), "~/Development/armada"));
        SessionRunUnderway { run_id: run, terminal: Some(id) }
    }
}

impl Shells for Mock {
    fn open(&self, job: &str, which: Which, size: Cells, cx: &mut App) -> Task<Result<SessionRunUnderway, String>> {
        if let Which::Command(command) = &which {
            return self.run(job, command, size, cx);
        }
        if Self::cut_off(&self.state.borrow(), job) {
            return Task::ready(Err(UNREACHED.into()));
        }
        Task::ready(Ok(self.opened(job, which)))
    }

    fn observe(&self, run: &str, _cx: &mut App) -> Receiver<SessionRunMessage> {
        let (tx, rx) = async_channel::unbounded();
        let mut state = self.state.borrow_mut();
        if let Some(one) = state.runs.get_mut(run) {
            let _ = tx.try_send(SessionRunMessage::Output { text: one.printed.clone() });
            if one.ended {
                let _ = tx.try_send(SessionRunMessage::Exited { code: one.code, stopped: one.code.is_none() });
            } else {
                one.tx = Some(tx);
            }
        }
        rx
    }

    fn write(&self, run: &str, data: String, _cx: &mut App) -> Task<Result<(), String>> {
        let mut state = self.state.borrow_mut();
        if Self::job_of(&state, run).is_some_and(|job| Self::cut_off(&state, &job)) {
            return Task::ready(Err(UNREACHED.into()));
        }
        let Some(one) = state.runs.get_mut(run) else { return Task::ready(Err("No such run".into())) };
        if one.ended {
            return Task::ready(Err("The run has ended".into()));
        }
        let mut echo = String::new();
        let mut done = Vec::new();
        let mut chars = data.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\r' => {
                    done.push(std::mem::take(&mut one.line).into_iter().collect::<String>());
                    one.at = 0;
                }
                '\x7f' if one.at > 0 => {
                    one.at -= 1;
                    one.line.remove(one.at);
                    let rest: String = one.line[one.at..].iter().collect();
                    echo.push_str(&format!("\x08{rest} \x1b[{}D", rest.chars().count() + 1));
                }
                '\x03' => {
                    one.busy = false;
                    one.line.clear();
                    one.at = 0;
                    echo.push_str("^C\r\n");
                    echo.push_str(&prompt(&one.folder));
                }
                // A cursor key, as zsh's line editor reads it: ← and → move a cell, ⌥← and ⌥→ a word. A bare Esc does nothing.
                '\x1b' if chars.peek() == Some(&'[') => {
                    chars.next();
                    let mut sequence = String::new();
                    for next in chars.by_ref() {
                        sequence.push(next);
                        if next.is_ascii_alphabetic() || next == '~' {
                            break;
                        }
                    }
                    let to = match sequence.as_str() {
                        "D" => one.at.saturating_sub(1),
                        "C" => (one.at + 1).min(one.line.len()),
                        "1;3D" => word_left(&one.line, one.at),
                        "1;3C" => word_right(&one.line, one.at),
                        _ => one.at,
                    };
                    if to < one.at {
                        echo.push_str(&format!("\x1b[{}D", one.at - to));
                    } else if to > one.at {
                        echo.push_str(&format!("\x1b[{}C", to - one.at));
                    }
                    one.at = to;
                }
                // A program holding the shell takes the keys; the walk's ones print nothing back.
                _ if one.busy => {}
                ch if !ch.is_control() => {
                    one.line.insert(one.at, ch);
                    one.at += 1;
                    let rest: String = one.line[one.at..].iter().collect();
                    echo.push(ch);
                    if !rest.is_empty() {
                        echo.push_str(&format!("{rest}\x1b[{}D", rest.chars().count()));
                    }
                }
                _ => {}
            }
        }
        let folder = one.folder.clone();
        Self::print(&mut state, run, echo);
        for command in done {
            if command.trim() == "exit" {
                Self::print(&mut state, run, "\r\n".into());
                Self::finish(&mut state, run, false);
                return Task::ready(Ok(()));
            }
            if command.trim() == "pnpm dev" {
                let (said, _) = ran("pnpm dev");
                if let Some(one) = state.runs.get_mut(run) {
                    one.busy = true;
                }
                Self::print(&mut state, run, said);
                return Task::ready(Ok(()));
            }
            Self::print(&mut state, run, format!("\r\n{}{}", answer(&command), prompt(&folder)));
        }
        Task::ready(Ok(()))
    }

    fn resize(&self, _run: &str, _size: Cells, _cx: &mut App) {}

    fn stop(&self, run: &str, _cx: &mut App) {
        Self::finish(&mut self.state.borrow_mut(), run, true);
    }

    fn list(&self, job: &str, _cx: &mut App) -> Task<Result<JobTerminals, String>> {
        let state = self.state.borrow();
        let terminals = state
            .jobs
            .get(job)
            .map(|shells| {
                shells
                    .iter()
                    .filter(|(_, _, run)| state.runs.get(run).is_some_and(|one| !one.ended))
                    .map(|(id, name, run)| ListedTerminal { terminal: id.clone(), name: name.clone(), run_id: run.clone(), running: state.runs.get(run).is_some_and(|one| one.busy) })
                    .collect()
            })
            .unwrap_or_default();
        Task::ready(Ok(JobTerminals { terminals }))
    }

    fn rename(&self, job: &str, terminal: &str, name: &str, _cx: &mut App) -> Task<Result<ListedTerminal, String>> {
        let mut state = self.state.borrow_mut();
        let Some(held) = state.jobs.get_mut(job).and_then(|shells| shells.iter_mut().find(|(id, _, _)| id == terminal)) else {
            return Task::ready(Err("No such terminal".into()));
        };
        held.1 = if name.trim().is_empty() { format!("Terminal {}", terminal.trim_start_matches('t')) } else { name.trim().to_string() };
        Task::ready(Ok(ListedTerminal { terminal: held.0.clone(), name: held.1.clone(), run_id: held.2.clone(), running: false }))
    }

    fn start(&self, start: Start, _cx: &mut App) -> Task<Result<Started, String>> {
        let mut state = self.state.borrow_mut();
        state.next_job += 1;
        let job = if state.starts_as.is_empty() { format!("terminal-{}", state.next_job) } else { state.starts_as.remove(0) };
        let folder = match &start.place {
            Place::Repository(name) => format!("~/Development/{name}"),
            Place::Folder(path) => path.replace(&std::env::var("HOME").unwrap_or_default(), "~"),
        };
        let run = Self::shell(&mut state, &job, "t1".into(), &folder);
        Task::ready(Ok(Started { job, terminal: "t1".into(), run }))
    }

    fn starts(&self) -> bool {
        true
    }

    fn run(&self, _job: &str, command: &str, _size: Cells, _cx: &mut App) -> Task<Result<SessionRunUnderway, String>> {
        let mut state = self.state.borrow_mut();
        state.next_run += 1;
        let run = format!("run-{}", state.next_run);
        let (printed, code) = ran(command);
        let ended = code.is_some();
        state.runs.insert(run.clone(), MockRun { tx: None, printed, line: Vec::new(), at: 0, folder: String::new(), ended, busy: !ended, code });
        Task::ready(Ok(SessionRunUnderway { run_id: run, terminal: None }))
    }
}

/// What a walk's run prints and how it exits: none where it keeps running until Stop, as a server does.
fn ran(command: &str) -> (String, Option<i32>) {
    match command.trim() {
        "pnpm dev" => ("\r\n  \x1b[1;32mVITE\x1b[0m v6.2.0  ready in 412 ms\r\n\r\n  \x1b[32m➜\x1b[0m  Local:   \x1b[36mhttp://localhost:5173/\x1b[0m\r\n".into(), None),
        "armada check test chip" => ("\x1b[2mchip_tone_reads_behind_by\x1b[0m \x1b[32mok\x1b[0m\r\n\x1b[2mchip_tooltip_says_behind_main\x1b[0m \x1b[32mok\x1b[0m\r\n\r\n2 passed\r\n".into(), Some(0)),
        "cargo test -p fleet" => (
            "\x1b[1;32m   Compiling\x1b[0m fleet v0.0.0\r\n\x1b[1;32m    Finished\x1b[0m `test` profile in 6.02s\r\n\r\nrunning 42 tests\r\ntest sessions::a_run_is_settled ... \x1b[31mFAILED\x1b[0m\r\n\r\ntest result: \x1b[31mFAILED\x1b[0m. 41 passed; 1 failed\r\n".into(),
            Some(101),
        ),
        other => match answer(other) {
            said if said.starts_with("zsh: command not found") => (said, Some(127)),
            said => (said, Some(0)),
        },
    }
}

/// The shells the app's panes use: Fleet's live, a walk's mock.
#[derive(Clone)]
pub struct Held(pub Rc<dyn Shells>);

impl gpui::Global for Held {}

/// Fleet's shells, reached on the port the connection holds now.
pub fn hold_live(fleet: &gpui::Entity<crate::fleet::FleetConnection>, cx: &mut App) {
    super::tabs::follow(fleet, cx);
    let fleet = fleet.clone();
    cx.set_global(Held(Rc::new(Live { port: Rc::new(move |cx: &App| fleet.read(cx).held().port()) })));
}

/// A walk's shells, kept beside [`Held`] so a scene can reach the link.
struct WalkShells(Rc<Mock>);

impl gpui::Global for WalkShells {}

/// A walk's shells: one open on the first mock Job, and each terminal Job started is the scenario's fresh one.
/// The Jobs the pairing walks put on Air go through its link.
pub fn hold_walk(cx: &mut App) {
    let mock = Rc::new(Mock::with(&[("mock-01", 1)]).starting_as(&["s-7c41e2a9"; 8]).elsewhere(&crate::mock::AIR_JOBS));
    cx.set_global(Held(mock.clone()));
    cx.set_global(WalkShells(mock));
}

/// Whether a walk's link to Air refuses what goes to its Jobs now: a page of one opens to the link's sentence too.
pub fn walk_link_down(cx: &App) -> bool {
    cx.try_global::<WalkShells>().is_some_and(|held| held.0.state.borrow().link_down)
}

/// A walk's scene for the link to Air: `up`, `down` (keys and opens refused), or `drops` (the watched shells end here).
pub fn walk_link(scene: &str, cx: &mut App) -> Result<(), String> {
    let mock = cx.try_global::<WalkShells>().map(|held| held.0.clone()).ok_or("no walk shells")?;
    match scene {
        "up" => mock.link(true),
        "down" => mock.link(false),
        "drops" => mock.streams_drop(),
        other => return Err(format!("no link scene named {other}")),
    }
    Ok(())
}

pub fn of(cx: &App) -> Option<Rc<dyn Shells>> {
    cx.try_global::<Held>().map(|held| held.0.clone())
}

/// What `POST /sessions/terminal/start` is sent for `start`. No title is sent: the Job takes one from its
/// first message.
fn start_body(start: Start) -> StartSideJob {
    let project = start.project;
    let (manifest_id, dir) = match start.place {
        Place::Repository(id) => (Some(ManifestId::carried(id)), None),
        Place::Folder(path) => (None, Some(path)),
    };
    StartSideJob {
        manifest_id,
        dir,
        title: None,
        model: None,
        effort: None,
        mode: None,
        project,
        cols: Some(start.size.cols),
        rows: Some(start.size.rows),
    }
}

/// Whether a terminal Job can be started here (`Shells::starts`).
pub fn starts(cx: &App) -> bool {
    cx.try_global::<Held>().is_some_and(|held| held.0.starts())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn a_mock_shell_echoes_and_answers_a_command(cx: &mut gpui::TestAppContext) {
        let mock = Mock::with(&[("job", 1)]);
        let run = mock.opened("job", Which::First).run_id;
        let rx = cx.update(|cx| mock.observe(&run, cx));
        cx.update(|cx| mock.write(&run, "ls\r".into(), cx)).detach();
        let printed: String = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|message| match message {
                SessionRunMessage::Output { text } => Some(text),
                _ => None,
            })
            .collect();
        assert!(printed.contains("ls\r\n"), "{printed:?}");
        assert!(printed.contains("Cargo.toml"), "{printed:?}");
    }

    #[gpui::test]
    async fn a_link_that_does_not_carry_refuses_keys_and_opens_on_another_macs_job(cx: &mut gpui::TestAppContext) {
        let mock = Mock::with(&[("air", 1), ("here", 1)]).elsewhere(&["air"]);
        let air = mock.opened("air", Which::First).run_id;
        let here = mock.opened("here", Which::First).run_id;
        mock.link(false);
        assert_eq!(cx.update(|cx| mock.write(&air, "x".into(), cx)).await, Err(UNREACHED.to_string()));
        assert_eq!(cx.update(|cx| mock.write(&here, "x".into(), cx)).await, Ok(()));
        let size = Cells { cols: 80, rows: 24 };
        assert_eq!(cx.update(|cx| mock.open("air", Which::First, size, cx)).await.map(|_| ()), Err(UNREACHED.to_string()));
        mock.link(true);
        assert_eq!(cx.update(|cx| mock.write(&air, "x".into(), cx)).await, Ok(()));
        assert!(cx.update(|cx| mock.open("air", Which::First, size, cx)).await.is_ok());
    }

    #[gpui::test]
    fn a_dropped_stream_ends_only_the_other_macs_shells_with_no_code(cx: &mut gpui::TestAppContext) {
        let mock = Mock::with(&[("air", 1), ("here", 1)]).elsewhere(&["air"]);
        let air = mock.opened("air", Which::First).run_id;
        let here = mock.opened("here", Which::First).run_id;
        let (air_rx, here_rx) = cx.update(|cx| (mock.observe(&air, cx), mock.observe(&here, cx)));
        mock.streams_drop();
        let ended = |rx: &Receiver<SessionRunMessage>| std::iter::from_fn(|| rx.try_recv().ok()).find_map(|message| match message {
            SessionRunMessage::Exited { code, stopped } => Some((code, stopped)),
            _ => None,
        });
        assert_eq!(ended(&air_rx), Some((None, false)));
        assert_eq!(ended(&here_rx), None);
    }

    fn start(place: Place) -> Start {
        Start { place, size: Cells { cols: 120, rows: 32 }, project: None }
    }

    #[test]
    fn a_live_fleet_starts_terminal_jobs() {
        assert!(Live { port: Rc::new(|_| None) }.starts());
    }

    #[test]
    fn a_folder_start_sends_the_folder_and_leaves_the_title_to_fleet() {
        let body = start_body(start(Place::Folder("/work/shop".into())));
        assert_eq!((body.manifest_id, body.dir.as_deref(), body.title), (None, Some("/work/shop"), None));
        assert_eq!((body.cols, body.rows), (Some(120), Some(32)));
    }

    #[test]
    fn a_repository_start_sends_its_manifest() {
        let body = start_body(start(Place::Repository("armada".into())));
        assert_eq!((body.manifest_id.as_ref().map(|id| id.as_str()), body.dir, body.title), (Some("armada"), None, None));
    }
}

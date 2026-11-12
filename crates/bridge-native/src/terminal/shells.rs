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

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use async_channel::{Receiver, Sender};
use gpui::{App, Task};
use ipc::{
    JobTerminals, ListedTerminal, OpenTerminal, RenameTerminal, ResizeSessionRun, SessionId, SessionRunMessage, SessionRunUnderway, StopSessionRun,
    WireError, WriteSessionRun,
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
}

/// Where a terminal Job starts: a Manifest (or a repository's implicit id), or any folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Repository(String),
    Folder(String),
}

/// What a terminal Job is started with: where, and the sheet's title, model and effort where it had them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    pub place: Place,
    pub title: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// Its first shell's size.
    pub size: Cells,
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
    fn write(&self, run: &str, data: String, cx: &mut App);
    fn resize(&self, run: &str, size: Cells, cx: &mut App);
    fn stop(&self, run: &str, cx: &mut App);
    fn list(&self, job: &str, cx: &mut App) -> Task<Result<JobTerminals, String>>;
    fn rename(&self, job: &str, terminal: &str, name: &str, cx: &mut App) -> Task<Result<ListedTerminal, String>>;
    /// A terminal Job: no words, its first shell open.
    fn start(&self, start: Start, cx: &mut App) -> Task<Result<Started, String>>;
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

    fn write(&self, run: &str, data: String, cx: &mut App) {
        self.tell("/sessions/run/write", &WriteSessionRun { run_id: run.to_string(), data }, cx);
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

    fn start(&self, _start: Start, _cx: &mut App) -> Task<Result<Started, String>> {
        // `POST /sessions/terminal/start` is the quick terminal Fleet Job's, not landed yet.
        Task::ready(Err("This Fleet does not start terminal Jobs yet".into()))
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
}

struct MockRun {
    tx: Option<Sender<SessionRunMessage>>,
    /// What it printed, replayed to a viewer that attaches.
    printed: String,
    line: String,
    folder: String,
    ended: bool,
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
        state.runs.insert(run.clone(), MockRun { tx: None, printed: prompt(folder), line: String::new(), folder: folder.to_string(), ended: false });
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

    /// End a shell as `exit` would, so a walk shows Shell ended.
    pub fn end(&self, job: &str, terminal: &str) {
        let mut state = self.state.borrow_mut();
        let Some(run) = state.jobs.get(job).and_then(|shells| shells.iter().find(|(id, _, _)| id == terminal)).map(|(_, _, run)| run.clone()) else { return };
        Self::finish(&mut state, &run, false);
    }

    fn finish(state: &mut MockState, run: &str, stopped: bool) {
        if let Some(one) = state.runs.get_mut(run) {
            one.ended = true;
            if let Some(tx) = one.tx.take() {
                let _ = tx.try_send(SessionRunMessage::Exited { code: if stopped { None } else { Some(0) }, stopped });
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
        };
        let running = held.iter().find(|(one, _, _)| *one == id).map(|(_, _, run)| run.clone()).filter(|run| state.runs.get(run).is_some_and(|one| !one.ended));
        let run = running.unwrap_or_else(|| Self::shell(&mut state, job, id.clone(), "~/Development/armada"));
        SessionRunUnderway { run_id: run, terminal: Some(id) }
    }
}

impl Shells for Mock {
    fn open(&self, job: &str, which: Which, _size: Cells, _cx: &mut App) -> Task<Result<SessionRunUnderway, String>> {
        Task::ready(Ok(self.opened(job, which)))
    }

    fn observe(&self, run: &str, _cx: &mut App) -> Receiver<SessionRunMessage> {
        let (tx, rx) = async_channel::unbounded();
        let mut state = self.state.borrow_mut();
        if let Some(one) = state.runs.get_mut(run) {
            let _ = tx.try_send(SessionRunMessage::Output { text: one.printed.clone() });
            if one.ended {
                let _ = tx.try_send(SessionRunMessage::Exited { code: Some(0), stopped: false });
            } else {
                one.tx = Some(tx);
            }
        }
        rx
    }

    fn write(&self, run: &str, data: String, _cx: &mut App) {
        let mut state = self.state.borrow_mut();
        let Some(one) = state.runs.get_mut(run) else { return };
        if one.ended {
            return;
        }
        let mut echo = String::new();
        let mut done = Vec::new();
        for ch in data.chars() {
            match ch {
                '\r' => done.push(std::mem::take(&mut one.line)),
                '\x7f' if one.line.pop().is_some() => echo.push_str("\x08 \x08"),
                '\x03' => {
                    one.line.clear();
                    echo.push_str("^C\r\n");
                    echo.push_str(&prompt(&one.folder));
                }
                ch if !ch.is_control() => {
                    one.line.push(ch);
                    echo.push(ch);
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
                return;
            }
            Self::print(&mut state, run, format!("\r\n{}{}", answer(&command), prompt(&folder)));
        }
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
                    .map(|(id, name, run)| ListedTerminal { terminal: id.clone(), name: name.clone(), run_id: run.clone(), running: false })
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
}

/// The shells the app's panes use: Fleet's live, a walk's mock.
#[derive(Clone)]
pub struct Held(pub Rc<dyn Shells>);

impl gpui::Global for Held {}

/// Fleet's shells, reached on the port the connection holds now.
pub fn hold_live(fleet: &gpui::Entity<crate::fleet::FleetConnection>, cx: &mut App) {
    let fleet = fleet.clone();
    cx.set_global(Held(Rc::new(Live { port: Rc::new(move |cx: &App| fleet.read(cx).held().port()) })));
}

/// A walk's shells: one open on the first mock Job, and each terminal Job started opens the fourth.
pub fn hold_walk(cx: &mut App) {
    cx.set_global(Held(Rc::new(Mock::with(&[("mock-01", 1)]).starting_as(&["mock-04"; 4]))));
}

pub fn of(cx: &App) -> Option<Rc<dyn Shells>> {
    cx.try_global::<Held>().map(|held| held.0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn a_mock_shell_echoes_and_answers_a_command(cx: &mut gpui::TestAppContext) {
        let mock = Mock::with(&[("job", 1)]);
        let run = mock.opened("job", Which::First).run_id;
        let rx = cx.update(|cx| mock.observe(&run, cx));
        cx.update(|cx| mock.write(&run, "ls\r".into(), cx));
        let printed: String = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|message| match message {
                SessionRunMessage::Output { text } => Some(text),
                _ => None,
            })
            .collect();
        assert!(printed.contains("ls\r\n"), "{printed:?}");
        assert!(printed.contains("Cargo.toml"), "{printed:?}");
    }
}

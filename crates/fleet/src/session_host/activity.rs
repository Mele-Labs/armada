//! What a hosted session is doing, read off the tool calls its gate sees. `docs/concepts/session.md`,
//! *Working on*. Since 14 Oct 2026.
//!
//! **Detected, never reported.** No tool exists for the agent to say what it is doing. The gate runs
//! before every tool call, so a call's name and input are enough to word `now`, and a `TodoWrite`
//! carries the agent's own list. Memory only, beside the rest of a session's runtime state.
//!
//! **The row is large and every publish sends it whole**, so a call never publishes by itself. A
//! change to `now` or `todos` publishes at once if the row has not gone out within [`PUBLISH_EVERY`],
//! and otherwise once, when that time is up. `last_call_at` alone never publishes: it rides the next.

use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{SessionActivity, SessionGate, SessionTodo, SessionTodoState};

use crate::daemon::Fleet;

/// The most often a session's row is published for what it is doing.
pub(crate) const PUBLISH_EVERY: Duration = Duration::from_secs(2);

/// The longest word `now` carries from a command or a file name, and the longest todo.
const WORD_AT_MOST: usize = 40;
const NAME_AT_MOST: usize = 80;
const TODO_AT_MOST: usize = 200;

/// What a session did last, and when its row last went out for it.
#[derive(Default)]
pub(crate) struct Activity {
    now: Option<String>,
    todos: Vec<SessionTodo>,
    last_call_at: Option<String>,
    /// When the row last went out, or was scheduled to by this.
    sent_at: Option<Instant>,
    /// A change not yet on a published row.
    unsent: bool,
    /// A task is waiting to publish `unsent`. At most one.
    trailing: bool,
}

/// What a call asks of the publisher.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Publish {
    Not,
    Now,
    After(Duration),
}

impl Activity {
    /// Take one call. `at` is when Fleet's clock says it was made.
    pub(crate) fn saw(&mut self, gate: &SessionGate, at: String, every: Duration) -> Publish {
        self.last_call_at = Some(at);
        let changed = match gate.tool_name.as_str() {
            "TodoWrite" => match todos_of(gate) {
                Some(todos) if todos != self.todos => {
                    self.todos = todos;
                    true
                }
                _ => false,
            },
            _ => match now_of(gate) {
                Some(now) if self.now.as_deref() != Some(now.as_str()) => {
                    self.now = Some(now);
                    true
                }
                _ => false,
            },
        };
        if !changed {
            return Publish::Not;
        }
        self.unsent = true;
        if self.trailing {
            return Publish::Not;
        }
        match self.sent_at.map(|sent| sent.elapsed()) {
            Some(since) if since < every => {
                self.trailing = true;
                Publish::After(every - since)
            }
            _ => {
                self.unsent = false;
                self.sent_at = Some(Instant::now());
                Publish::Now
            }
        }
    }

    /// The wait is over. Whether there is a change to publish.
    pub(crate) fn trailed(&mut self) -> bool {
        self.trailing = false;
        let owed = std::mem::take(&mut self.unsent);
        if owed {
            self.sent_at = Some(Instant::now());
        }
        owed
    }

    /// The session ended: nothing is kept, and a waiting task finds nothing owed.
    pub(crate) fn ended(&mut self) {
        *self = Activity::default();
    }

    /// As the row carries it. Absent where no call has been seen.
    pub(crate) fn wire(&self) -> Option<SessionActivity> {
        self.last_call_at.as_ref().map(|at| SessionActivity {
            now: self.now.clone(),
            todos: self.todos.clone(),
            last_call_at: Some(ipc::Instant::carried(at.clone())),
        })
    }
}

/// The agent's `TodoWrite` list. `None` for an input with no list in it, which changes nothing.
fn todos_of(gate: &SessionGate) -> Option<Vec<SessionTodo>> {
    let list = gate.tool_input.get("todos")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|one| {
                let text = one.get("content")?.as_str()?.trim();
                if text.is_empty() {
                    return None;
                }
                let state = match one.get("status").and_then(|said| said.as_str()) {
                    Some("completed") => SessionTodoState::Done,
                    Some("in_progress") => SessionTodoState::Doing,
                    _ => SessionTodoState::Todo,
                };
                Some(SessionTodo { text: text.chars().take(TODO_AT_MOST).collect(), state })
            })
            .collect(),
    )
}

/// The one short fact a call makes of `now`. `None` for a tool Fleet does not name, which leaves it as it was.
pub(crate) fn now_of(gate: &SessionGate) -> Option<String> {
    let text = |key: &str| gate.tool_input.get(key).and_then(|said| said.as_str());
    match gate.tool_name.as_str() {
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => {
            let path = text("file_path").or_else(|| text("notebook_path"))?;
            Some(format!("Editing {}", file_name(path)?))
        }
        "Read" => Some(format!("Reading {}", file_name(text("file_path")?)?)),
        "Grep" | "Glob" => Some("Searching".into()),
        "WebFetch" | "WebSearch" => Some("Fetching".into()),
        "Agent" | "Task" => Some("Running a subagent".into()),
        "Bash" => Some(format!("Running {}", command_name(text("command")?)?)),
        _ => None,
    }
}

fn file_name(path: &str) -> Option<String> {
    let name = std::path::Path::new(path).file_name()?.to_str()?;
    Some(name.chars().take(NAME_AT_MOST).collect())
}

/// Commands that take a subcommand worth naming: `cargo check`, `git diff`, `pnpm test`.
const WITH_SUBCOMMAND: [&str; 17] = [
    "git", "cargo", "pnpm", "npm", "yarn", "npx", "bun", "armada", "gh", "kubectl", "make", "go", "uv",
    "pip", "brew", "rustup", "just",
];
/// Words that only run another command, whose own flags and values are skipped with them.
const WRAPPERS: [&str; 6] = ["env", "time", "sudo", "nohup", "exec", "command"];
const WRAPPERS_WITH_VALUES: [&str; 2] = ["taskpolicy", "nice"];
/// Lines that set the scene for the one after them.
const SETTING_THE_SCENE: [&str; 7] = ["cd", "pushd", "popd", "export", "set", "unset", "source"];

/// The first one or two words of what a shell line runs, after any `cd x &&`, environment
/// assignment or wrapper. `None` where nothing in it is a plain command name.
fn command_name(line: &str) -> Option<String> {
    segments(line).into_iter().find_map(|segment| {
        let words = run_words(&segment);
        let first = *words.first()?;
        if SETTING_THE_SCENE.contains(&first) {
            return None;
        }
        let second = words.get(1).copied().filter(|word| {
            WITH_SUBCOMMAND.contains(&first) && !word.starts_with('-') && !word.contains('/') && simple(word)
        });
        Some(match second {
            Some(second) => format!("{first} {second}"),
            None => first.to_string(),
        })
    })
}

/// A line cut at `&&`, `||`, `;` and a newline that are outside quotes.
fn segments(line: &str) -> Vec<String> {
    let (mut cut, mut from) = (Vec::new(), 0);
    let (mut single, mut double) = (false, false);
    let bytes = line.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let here = bytes[at];
        let pair = bytes.get(at..at + 2);
        match here {
            b'\'' if !double => single = !single,
            b'"' if !single => double = !double,
            _ if single || double => {}
            b';' | b'\n' => {
                cut.push(line[from..at].to_string());
                from = at + 1;
            }
            b'&' | b'|' if pair.is_some_and(|two| two[0] == here && two[1] == here) => {
                cut.push(line[from..at].to_string());
                from = at + 2;
                at += 1;
            }
            _ => {}
        }
        at += 1;
    }
    cut.push(line[from..].to_string());
    cut
}

/// The words of one segment from the command on, with the command's own path trimmed to its name.
fn run_words(segment: &str) -> Vec<&str> {
    let mut words = segment.split_whitespace().peekable();
    let mut ran = Vec::new();
    while let Some(word) = words.next() {
        if ran.is_empty() {
            if assigns(word) {
                continue;
            }
            let name = word.rsplit('/').next().unwrap_or(word);
            if WRAPPERS.contains(&name) {
                continue;
            }
            if WRAPPERS_WITH_VALUES.contains(&name) {
                while words.next_if(|next| next.starts_with('-')).is_some() {
                    words.next();
                }
                continue;
            }
            if !simple(name) {
                return Vec::new();
            }
            ran.push(name);
        } else {
            ran.push(word);
        }
    }
    ran
}

fn assigns(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

fn simple(word: &str) -> bool {
    !word.is_empty()
        && word.len() <= WORD_AT_MOST
        && word.chars().all(|c| c.is_ascii_alphanumeric() || "._+-:".contains(c))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A hosted session's tool call, before it runs. **Takes a std lock for a moment and awaits
    /// nothing**: the gate answers the call, and the publish, when there is one, is a task of its own.
    pub(super) fn call_seen(self: &Arc<Self>, gate: &SessionGate) {
        let id = gate.session_id.as_str();
        let runtime = self.hosts().of(id);
        let at = self.clock().now().as_str().to_string();
        let step = runtime.state().activity.saw(gate, at, self.hosts().activity_every());
        let (fleet, id) = (Arc::clone(self), id.to_string());
        match step {
            Publish::Not => {}
            Publish::Now => {
                tokio::spawn(async move {
                    let _ = fleet.published_hosted(&id).await;
                });
            }
            Publish::After(wait) => {
                tokio::spawn(async move {
                    tokio::time::sleep(wait).await;
                    let owed = fleet.hosts().of(&id).state().activity.trailed();
                    if owed {
                        let _ = fleet.published_hosted(&id).await;
                    }
                });
            }
        }
    }

    /// What the row carries of it.
    pub(crate) fn activity_of(&self, id: &str) -> Option<SessionActivity> {
        self.hosts().of(id).state().activity.wire()
    }

    /// The session ended.
    pub(crate) fn activity_ended(&self, id: &str) {
        self.hosts().of(id).state().activity.ended();
    }
}

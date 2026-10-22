//! What a hosted session is doing, read off the tool calls its gate sees. `docs/concepts/session.md`,
//! *Working on*. Since 14 Oct 2026.
//!
//! **Detected, never reported.** No tool exists for the agent to say what it is doing. The gate runs
//! before every tool call, so a call's name and input are enough to word `now`, and a `TodoWrite`
//! carries the agent's own list. Memory only, beside the rest of a session's runtime state.
//!
//! **The row is large and every publish sends it whole**, so a call is not a row of its own. A
//! change to `now` or `todos` publishes at once if the row has not gone out within [`PUBLISH_EVERY`],
//! and otherwise once, when that time is up. A call that changes neither publishes nothing. A turn
//! starting clears `now` and the subagents' and keeps `todos`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{ReportedTask, ReportedTaskStatus, SessionActivity, SessionGate, SessionTodo, SessionTodoState, SubagentActivity};

use crate::build_keeping::build_heard;
use crate::build_record::Observed;

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
    /// What each subagent at work is doing, in the order they were first heard.
    subagents: Vec<SubagentActivity>,
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
    /// Take one call. `every` is the most often a row goes out for it.
    pub(crate) fn saw(&mut self, gate: &SessionGate, every: Duration) -> Publish {
        if let Some(agent) = gate.agent_id.as_deref() {
            return match self.subagent_saw(agent, gate) {
                true => self.changed(every),
                false => Publish::Not,
            };
        }
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
        match changed {
            true => self.changed(every),
            false => Publish::Not,
        }
    }

    /// A call a subagent made: its own `now`, and not the Session's. Its `TodoWrite` is its own list,
    /// which the Session's is not. Whether anything changed.
    fn subagent_saw(&mut self, agent: &str, gate: &SessionGate) -> bool {
        let Some(now) = now_of(gate) else { return false };
        match self.subagents.iter_mut().find(|one| one.id == agent) {
            Some(one) if one.now == now => false,
            Some(one) => {
                one.now = now;
                true
            }
            None => {
                self.subagents.push(SubagentActivity { id: agent.to_string(), now });
                true
            }
        }
    }

    /// A subagent ended: its line goes. Whether it had one.
    pub(crate) fn subagent_ended(&mut self, agent: &str) -> bool {
        let held = self.subagents.len();
        self.subagents.retain(|one| one.id != agent);
        self.subagents.len() != held
    }

    /// Something the row carries changed, though not through a call this holds: the row goes out
    /// at most once in `every`, as for a call.
    pub(crate) fn changed(&mut self, every: Duration) -> Publish {
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
                self.went_out();
                Publish::Now
            }
        }
    }

    /// The wait is over. Whether there is a change to publish.
    pub(crate) fn trailed(&mut self) -> bool {
        self.trailing = false;
        let owed = self.unsent;
        if owed {
            self.went_out();
        }
        owed
    }

    /// The row goes out now, carrying everything held.
    fn went_out(&mut self) {
        self.unsent = false;
        self.sent_at = Some(Instant::now());
    }

    /// A turn began, and Fleet publishes the row at once. What the last turn was doing is not what
    /// this one is: `now` goes, and the agent's list stays, true across turns.
    pub(crate) fn turn_began(&mut self) {
        self.now = None;
        self.subagents.clear();
        self.unsent = false;
        self.sent_at = Some(Instant::now());
    }

    /// The session ended: nothing is kept, and a waiting task finds nothing owed.
    pub(crate) fn ended(&mut self) {
        *self = Activity::default();
    }

    /// As the row carries it. Absent where it holds nothing.
    pub(crate) fn wire(&self) -> Option<SessionActivity> {
        let held = SessionActivity {
            now: self.now.clone(),
            todos: self.todos.clone(),
            subagents: self.subagents.clone(),
        };
        (held != SessionActivity::default()).then_some(held)
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

/// What a call says of the plan: the agent's `TodoWrite` list, whole, or its leaving plan mode.
fn plan_of(gate: &SessionGate) -> Option<Observed> {
    match gate.tool_name.as_str() {
        "TodoWrite" => Some(Observed::Tasks(
            todos_of(gate)?
                .into_iter()
                .enumerate()
                .map(|(place, todo)| ReportedTask {
                    id: (place + 1).to_string(),
                    text: todo.text,
                    status: match todo.state {
                        SessionTodoState::Done => ReportedTaskStatus::Completed,
                        SessionTodoState::Doing => ReportedTaskStatus::InProgress,
                        SessionTodoState::Todo => ReportedTaskStatus::Pending,
                    },
                    by: None,
                })
                .collect(),
        )),
        "ExitPlanMode" => Some(Observed::PlanLeft),
        _ => None,
    }
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
        let step = runtime.state().activity.saw(gate, self.hosts().activity_every());
        let (fleet, id) = (Arc::clone(self), id.to_string());
        // The build record hears the agent's plan the way a terminal's mod does: its list, and leaving plan mode.
        if let Some(heard) = plan_of(gate) {
            let (fleet, id) = (Arc::clone(&fleet), id.clone());
            tokio::spawn(async move {
                let now = fleet.now().as_str().to_string();
                let shown = build_heard(&mut *fleet.store().lock().await, &id, &heard, &now).unwrap_or(false);
                if shown {
                    let _ = fleet.published_hosted(&id).await;
                }
            });
        }
        fleet.publish_step(id, step);
    }

    /// Publish the row as [`Activity`] said: now, after a wait, or not.
    pub(super) fn publish_step(self: &Arc<Self>, id: String, step: Publish) {
        let fleet = Arc::clone(self);
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

    /// A subagent of the session ended: its line goes. Whether the row changed, which the caller publishes.
    pub(crate) fn activity_subagent_over(&self, id: &str, agent: &str) -> bool {
        self.hosts().of(id).state().activity.subagent_ended(agent)
    }

    /// The session ended.
    pub(crate) fn activity_ended(&self, id: &str) {
        self.hosts().of(id).state().activity.ended();
    }
}

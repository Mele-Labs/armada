//! The one interface a Helm conversation's process sits behind, and the host
//! that starts one per message under Fleet. `#939`.
//!
//! **Built to change, which `#73` asked for by name.** What a message resumes,
//! and what to do when it cannot, is `super::serving`'s; a [`Hosting`] carries
//! one message to a session and says what came back. A host in Bridge is a
//! second implementation of this trait, and nothing above it moves.
//!
//! **No process idles.** [`ProcessHost`] starts one for a message, closes its
//! input behind the message and reads until it exits — spike 016's shape.

use std::future::Future;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use adapter_traits::{AgentHarness, DroneEvent, Launch, McpConfig, Model};
use adapters::{Commanding, ConversationRefused, Conversing, HeadlessAgent};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::daemon::Host;
use crate::detach::Detached;
use crate::drone::{environment, HostPaths};
use crate::session::Turn;
use crate::session_host::UserLine;

use super::unanswered;

/// Whose conversation a message is in, and so which launch carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seat {
    /// A repository's Helm: opened in the owner's checkout, the owner answering
    /// what its settings do not cover.
    Helm,
    /// A Job's Commander: opened in the Job's worktree with its write tools cut
    /// at launch, because there is no owner to ask. `#2166`.
    Commander(Commander),
}

/// What a Commander's launch is told that Helm's is not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commander {
    /// The Job it steers, as the record holds the id. **What places its
    /// connection**: the door narrows its acts to this Job.
    pub job: String,
    /// The model it runs on, and how hard it thinks. Fleet resolves them (the
    /// Machine's setting, the Job's own over it) and the host does not choose.
    pub model: Model,
}

/// One message, as a host is asked to carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Carry {
    /// Where the session opens, which scopes its door: the repository's root
    /// for Helm, the Job's worktree for a Commander.
    pub directory: String,
    /// The turn whole: the brief and the message on a new session, the message
    /// alone on a resumed one.
    pub turn: String,
    /// The session to resume, or `None` to start one.
    pub resuming: Option<String>,
    /// `(media type, base64)` for each picture sent with the message. They reach the session as
    /// pictures; any other file is a path in `turn`.
    pub pictures: Vec<(String, String)>,
    /// The model and effort a person chose for this conversation. `None` runs on the host's own.
    pub model: Option<String>,
    pub effort: Option<String>,
    /// Where this conversation's uploads are kept, for the session to read without being asked.
    pub readable: Option<String>,
    pub seat: Seat,
}

/// What came of carrying a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Carried {
    /// The session answered, under the id the next message resumes.
    Answered { session: String },
    /// No session by the id asked for. **Nothing was heard.**
    NoSuchSession,
    /// No answer, and why.
    Failed { why: String },
}

/// Where a host puts what a session says, as it says it.
pub trait Heard: Send + Sync {
    /// Every event one line decoded to. **Must not block**, for
    /// `crate::transcript::Tap`'s reason.
    fn heard(&self, events: &[DroneEvent]);

    /// The commands the agent said it has, off its `init` line. A host with none to hear never
    /// calls it, and a listener that wants none ignores it.
    fn commands(&self, _names: Vec<String>) {}
}

pub type Carrying<'a> = Pin<Box<dyn Future<Output = Carried> + Send + 'a>>;

/// Something that can carry one message to a Helm session.
pub trait Hosting: Send + Sync + 'static {
    /// **Nothing reaches `heard` until the session has started**, so a resume
    /// the agent refused leaves no row behind it.
    fn carry<'a>(&'a self, carry: Carry, heard: &'a dyn Heard) -> Carrying<'a>;

    /// The model each message's process is started with. **The host's own
    /// fact**: nothing above it decides the model, and a record that named
    /// the one Armada asked for would be naming a guess (`#1367`).
    fn model(&self) -> String;

    /// The process each message is being answered in right now, by pid.
    ///
    /// **What places a Helm session at the agent's door** (`#941`): its relay is
    /// started inside this process's tree. A host running no process of Fleet's
    /// answers none, and the door treats its sessions as any other agent.
    fn running(&self) -> Vec<u32>;

    /// The process each Commander turn is being answered in right now, by pid
    /// and the Job it steers. **What places a Commander at the door**, as
    /// [`running`](Hosting::running) places Helm: its relay is started inside
    /// one of these. A host that starts no process answers none.
    fn running_commanders(&self) -> Vec<(u32, String)> {
        Vec::new()
    }
}

/// A session's pid, listed while its process runs and taken off however the
/// reply ends.
struct Listed<'a> {
    running: &'a Mutex<Vec<(u32, Option<String>)>>,
    pid: Option<u32>,
}

impl<'a> Listed<'a> {
    /// `job` is the Job a Commander turn steers, and `None` for Helm's.
    fn while_running(
        running: &'a Mutex<Vec<(u32, Option<String>)>>,
        pid: Option<u32>,
        job: Option<String>,
    ) -> Listed<'a> {
        if let (Some(pid), Ok(mut listed)) = (pid, running.lock()) {
            listed.push((pid, job));
        }
        Listed { running, pid }
    }
}

impl Drop for Listed<'_> {
    fn drop(&mut self) {
        if let (Some(pid), Ok(mut listed)) = (self.pid, self.running.lock()) {
            listed.retain(|(held, _)| *held != pid);
        }
    }
}

/// How long one reply may take before its process is ended, as it ships;
/// `timeouts.helmReplySeconds` moves it. A reply that reads a dozen Jobs is a
/// dozen tool calls, so it is generous.
///
/// **Not an idle timeout, and `#943` found none to add.** [`ProcessHost`]
/// starts one process per message and closes its input the moment the message
/// is written, so nothing here ever sits idle waiting for one — the shape the
/// module doc names. This bounds one reply's own length instead, which is a
/// different question and, like the model above, has no settings row of its
/// own to read.
pub(crate) const REPLY_BUDGET: Duration = Duration::from_secs(15 * 60);

/// The agent's door as a repository's `.mcp.json` names it —
/// `crates/armada/src/mcp.rs`. **Spelled twice**, because `fleet` cannot name
/// the crate that owns it.
const DOOR_PROGRAM: &str = "armada";
const DOOR_ARGS: &[&str] = &["mcp"];

/// The door's configuration file, beside the one a Drone is bound to.
const DOOR_FILE: &str = "helm-mcp.json";

/// How much of the CLI's own complaint a failure carries.
const COMPLAINT: u64 = 2048;

/// The agent CLI, one process per message.
pub struct ProcessHost {
    agent: HeadlessAgent,
    model: String,
    door: PathBuf,
    path: String,
    home: String,
    user: String,
    budget: Duration,
    /// How hard the model thinks. `None` passes no effort, the harness's own.
    effort: Option<adapter_traits::Effort>,
    /// Every session process started and not yet ended. See [`Hosting::running`].
    running: Mutex<Vec<(u32, Option<String>)>>,
}

impl ProcessHost {
    pub fn new(
        agent: HeadlessAgent,
        model: &str,
        door: PathBuf,
        host: HostPaths<'_>,
    ) -> ProcessHost {
        ProcessHost {
            agent,
            model: model.to_string(),
            door,
            path: host.path.to_string(),
            home: host.home.to_string(),
            user: host.user.to_string(),
            budget: REPLY_BUDGET,
            effort: None,
            running: Mutex::new(Vec::new()),
        }
    }

    /// The same host under a shorter budget, so a test can watch one run out
    /// rather than wait [`REPLY_BUDGET`] for it.
    #[cfg(test)]
    pub(crate) fn answering_within(mut self, budget: Duration) -> ProcessHost {
        self.budget = budget;
        self
    }

    /// The CLI on the Drone's `PATH`, with the Drone's environment, and the
    /// door's file beside the Drone's own.
    ///
    /// **The program is `host.agent_binary`, not a fresh `PATH` search.**
    /// `#943`: the composition root resolves the `AGENT_BINARY` override once,
    /// for a Drone, the Judge and Helm alike, and hands the same program name
    /// down here — a machine that names an override no longer leaves Helm's
    /// host to find whatever the daemon's own `PATH` happens to hold.
    ///
    /// **The model stays the adapter's default.** There is no settings row for
    /// it to read: `settings.toml` names three Helm rows and a model is not
    /// one of them, so this is a constant until a row exists to say
    /// otherwise.
    pub(crate) fn on_this_machine(host: &Host) -> ProcessHost {
        let mut process = ProcessHost::new(
            HeadlessAgent::at(host.agent_binary.clone()),
            HeadlessAgent::default_model(),
            Path::new(&host.mcp_config).with_file_name(DOOR_FILE),
            HostPaths {
                path: &host.path,
                home: &host.home,
                user: &host.user,
            },
        );
        // `timeouts.helmReplySeconds` and `effort.helm`, read at start.
        process.budget = host.helm_reply_budget;
        process.effort = host.helm_effort;
        process
    }

    fn launch(&self, carry: &Carry) -> Result<Launch, Carried> {
        let failed = |why: String| Carried::Failed {
            why: unanswered::no_session_started(why),
        };
        let door = McpConfig::only_these(&self.door.to_string_lossy())
            .map_err(|why| failed(why.said()))?;
        let paths = HostPaths {
            path: &self.path,
            home: &self.home,
            user: &self.user,
        };
        let env = environment(paths, &[]).map_err(|why| failed(why.said()))?;
        if let Seat::Commander(commander) = &carry.seat {
            let mut commanding =
                Commanding::in_worktree(&carry.directory, commander.model.clone(), door, env)
                    .map_err(|why| failed(why.to_string()))?;
            if let Some(session) = &carry.resuming {
                commanding = commanding.resuming(session).map_err(|why| match why {
                    ConversationRefused::SessionNotPortable { .. } => Carried::NoSuchSession,
                    other => failed(other.to_string()),
                })?;
            }
            return self
                .agent
                .render_commander(&commanding)
                .map_err(|why| failed(why.to_string()));
        }
        // A choice a person made for this conversation, else the host's own.
        let effort = match carry.effort.as_deref() {
            Some(chosen) => crate::tuning::effort_of(chosen).or(self.effort),
            None => self.effort,
        };
        let model = Model::named_at(carry.model.as_deref().unwrap_or(&self.model), effort)
            .map_err(|why| failed(why.said()))?;
        let mut conversing = Conversing::in_repository(&carry.directory, model, door, env)
            .map_err(|why| failed(why.to_string()))?;
        if let Some(uploads) = &carry.readable {
            conversing = conversing
                .reading(uploads)
                .map_err(|why| failed(why.to_string()))?;
        }
        if let Some(session) = &carry.resuming {
            // An id the CLI could not have named is a session it does not have.
            conversing = conversing.resuming(session).map_err(|why| match why {
                ConversationRefused::SessionNotPortable { .. } => Carried::NoSuchSession,
                other => failed(other.to_string()),
            })?;
        }
        self.agent
            .render_conversation(&conversing)
            .map_err(|why| failed(why.to_string()))
    }

    async fn carried(&self, carry: Carry, heard: &dyn Heard) -> Carried {
        let failed = |why: String| Carried::Failed { why };
        if let Err(why) = adapters::publish_the_agents_door(&self.door, DOOR_PROGRAM, DOOR_ARGS) {
            return failed(unanswered::door_unconfigured(why));
        }
        let launch = match self.launch(&carry) {
            Ok(launch) => launch,
            Err(carried) => return carried,
        };
        let mut child = match Detached::launching(&launch)
            .piping_input()
            .capturing_output()
            .spawn()
        {
            Ok(child) => child,
            Err(why) => return failed(unanswered::would_not_start(launch.program(), why)),
        };
        let pid = child.id();
        let steering = match &carry.seat {
            Seat::Helm => None,
            Seat::Commander(commander) => Some(commander.job.clone()),
        };
        let _listed = Listed::while_running(&self.running, pid, steering);
        let (Some(mut input), Some(output), Some(mut complaints)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return failed(unanswered::lost_before_held());
        };
        let turn = match turn_line(&carry) {
            Ok(turn) => turn + "\n",
            Err(why) => return failed(unanswered::message_not_encoded(why)),
        };
        // A session that exits before reading is the unknown-id case, and its
        // own stdout says so, so a broken pipe is read past rather than raised.
        let written = input.write_all(turn.as_bytes()).await;
        drop(input);
        if let Err(why) = written.as_ref() {
            if why.kind() != std::io::ErrorKind::BrokenPipe {
                return failed(unanswered::message_not_written(why));
            }
        }
        let complained = tokio::spawn(async move {
            let mut kept = Vec::new();
            let _ = (&mut complaints)
                .take(COMPLAINT)
                .read_to_end(&mut kept)
                .await;
            let _ = tokio::io::copy(&mut complaints, &mut tokio::io::sink()).await;
            String::from_utf8_lossy(&kept).trim().to_string()
        });
        let reading = async {
            let mut session: Option<String> = None;
            let mut lines = BufReader::new(output).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(names) = adapters::init_commands(&line) {
                    heard.commands(names);
                }
                let events = self.agent.read(&line);
                if session.is_none() {
                    session = events.iter().find_map(|event| match event {
                        DroneEvent::Started { session, .. } if !session.is_empty() => {
                            Some(session.clone())
                        }
                        _ => None,
                    });
                }
                if session.is_some() {
                    heard.heard(&events);
                }
            }
            (session, child.wait().await)
        };
        match tokio::time::timeout(self.budget, reading).await {
            Err(_) => {
                if let Some(group) = pid.and_then(NonZeroU32::new) {
                    crate::group::end_the_group(group);
                }
                let _ = child.wait().await;
                failed(unanswered::nothing_came_back(self.budget))
            }
            Ok((Some(session), _)) => Carried::Answered { session },
            Ok((None, _)) if carry.resuming.is_some() => Carried::NoSuchSession,
            Ok((None, status)) => {
                let complaint = complained.await.unwrap_or_default();
                let exited = status.map(|status| status.to_string()).unwrap_or_default();
                failed(unanswered::never_started(&exited, &complaint))
            }
        }
    }
}

/// The line written to the session's input. **A words-only turn is [`Turn::first`]'s bytes**, the
/// shape every turn here was before pictures: `UserLine` writes `content` as a list of blocks,
/// which is a different line, so it is used only when there is a picture to carry.
pub(crate) fn turn_line(carry: &Carry) -> Result<String, ipc::Unencodable> {
    match carry.pictures.is_empty() {
        true => ipc::encode(&Turn::first(&carry.turn)),
        false => ipc::encode(&UserLine::of(carry.turn.clone(), carry.pictures.clone())),
    }
}

impl Hosting for ProcessHost {
    fn carry<'a>(&'a self, carry: Carry, heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(self.carried(carry, heard))
    }

    fn model(&self) -> String {
        self.model.clone()
    }

    fn running(&self) -> Vec<u32> {
        self.running
            .lock()
            .map(|listed| {
                listed
                    .iter()
                    .filter(|(_, job)| job.is_none())
                    .map(|(pid, _)| *pid)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn running_commanders(&self) -> Vec<(u32, String)> {
        self.running
            .lock()
            .map(|listed| {
                listed
                    .iter()
                    .filter_map(|(pid, job)| Some((*pid, job.clone()?)))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> ProcessHost {
        let mut host = ProcessHost::new(
            HeadlessAgent::at("/usr/local/bin/agent"),
            "the-hosts-model",
            PathBuf::from("/var/armada/helm-mcp.json"),
            HostPaths {
                path: "/usr/bin:/bin",
                home: "/Users/user",
                user: "someone",
            },
        );
        host.effort = Some(adapter_traits::Effort::Low);
        host
    }

    fn words(turn: &str) -> Carry {
        Carry {
            directory: "/repos/armada".to_string(),
            turn: turn.to_string(),
            resuming: None,
            pictures: Vec::new(),
            model: None,
            effort: None,
            readable: None,
            seat: Seat::Helm,
        }
    }

    fn after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        let at = args.iter().position(|arg| arg == flag)?;
        args.get(at + 1).map(String::as_str)
    }

    /// A conversation nobody tuned runs on the host's own model and the settings' effort.
    #[test]
    fn an_untuned_launch_is_the_hosts_own() {
        let launch = host().launch(&words("hi")).expect("renders");
        assert_eq!(after(launch.args(), "--model"), Some("the-hosts-model"));
        assert_eq!(after(launch.args(), "--effort"), Some("low"));
        assert_eq!(after(launch.args(), "--add-dir"), None);
    }

    /// What a person chose wins, each on its own: a model alone leaves the effort the host's.
    #[test]
    fn a_chosen_model_and_effort_replace_the_hosts_own_each_by_itself() {
        let mut chosen = words("hi");
        chosen.model = Some("a-chosen-model".to_string());
        let launch = host().launch(&chosen).expect("renders");
        assert_eq!(after(launch.args(), "--model"), Some("a-chosen-model"));
        assert_eq!(after(launch.args(), "--effort"), Some("low"));

        chosen.effort = Some("high".to_string());
        let launch = host().launch(&chosen).expect("renders");
        assert_eq!(after(launch.args(), "--effort"), Some("high"));
    }

    #[test]
    fn the_uploads_directory_is_readable_without_an_ask() {
        let mut carry = words("hi");
        carry.readable = Some("/var/armada/helm/armada".to_string());
        let launch = host().launch(&carry).expect("renders");
        assert_eq!(after(launch.args(), "--add-dir"), Some("/var/armada/helm/armada"));
    }

    /// `UserLine` writes `content` as blocks and `Turn::first` as a string, so they are not the
    /// same line; a words-only turn keeps the second, and a picture takes the first.
    #[test]
    fn a_words_only_turn_is_turn_firsts_line_and_a_picture_makes_it_blocks() {
        let carry = words("why did 77 stop");
        let line = turn_line(&carry).expect("encodes");
        assert_eq!(line, ipc::encode(&Turn::first("why did 77 stop")).expect("encodes"));
        let blocks = ipc::encode(&UserLine::of(carry.turn.clone(), Vec::new())).expect("encodes");
        assert_ne!(line, blocks, "the two shapes are not one line");

        let mut with_picture = words("look");
        with_picture.pictures = vec![("image/png".to_string(), "aGk=".to_string())];
        let line = turn_line(&with_picture).expect("encodes");
        assert!(line.contains(r#""type":"image""#), "{line}");
        assert!(line.contains(r#""media_type":"image/png""#), "{line}");
        assert!(line.contains(r#""text":"look""#), "{line}");
    }
}

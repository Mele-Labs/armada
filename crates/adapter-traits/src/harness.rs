//! What a Drone is started as, in a crate that cannot start one.
//!
//! **The harness renders; Fleet starts.**
//! [`AgentHarness`](super::AgentHarness) does not spawn. It turns a
//! [`DroneSpawnConfig`] into a [`Launch`] — a program, an argument list, an
//! environment and a directory — and Fleet starts that, detached, through the
//! one type in the workspace that can start anything. Three things fall out of
//! the split, and each was a reason to take it.
//!
//! **Every confinement property becomes a pure assertion.** Whether the strict
//! flag is on the argument list, whether an ambient server can appear, whether
//! a credential is in the environment — all of it is a value a test reads, with
//! no process, no timing and no machine involved. The alternative is a suite
//! that can only check confinement by spawning the thing being confined.
//!
//! **The environment is built by Fleet, once, and not by whoever spawns.** v1's
//! Drone spawn inherited the operator's environment wholesale, and v1's own
//! design table rejected that channel in the same repository. Here there is no
//! inherit: [`Environment::nothing`] is where every Drone environment starts.
//!
//! **A harness cannot start an attached process**, because it cannot start one.
//! And there is no escape hatch at any of the three levels this is built from:
//! [`DroneSpawnConfig`], [`Launch`] and [`McpConfig`] each say what they
//! refuse.

//! **Over 500 lines, on purpose.** Every type here is a field of
//! [`DroneSpawnConfig`] or what one renders into, and "no escape hatch at any
//! level" is a claim a reader should be able to check in one file.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::Worktree;

/// The model a Drone runs on.
///
/// **This is `core_model::ModelName`'s string, and it is deliberately not that
/// type.** `cargo tree -p adapter-traits` shows only this crate, so the seam
/// every adapter implements cannot name the domain crate. The composition root
/// is the one place both types are in scope, and `ModelName::as_str` is the
/// only thing that should ever be handed to [`Model::named`].
///
/// **It carries the effort the model thinks at**, because the two are one
/// answer to what the Drone runs as. `None` is the harness's own default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    name: String,
    effort: Option<Effort>,
}

/// How hard the model thinks, as a person set it on the step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effort {
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl Model {
    pub fn named(name: &str) -> Result<Model, SpawnConfigRefused> {
        Model::named_at(name, None)
    }

    /// [`named`](Model::named), thinking at `effort`.
    pub fn named_at(name: &str, effort: Option<Effort>) -> Result<Model, SpawnConfigRefused> {
        if name.trim().is_empty() {
            return Err(SpawnConfigRefused::ModelUnnamed);
        }
        Ok(Model {
            name: String::from(name),
            effort,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    pub fn effort(&self) -> Option<Effort> {
        self.effort
    }
}

/// The assembled prompt: six layers of text, already in order.
///
/// This type does not assemble it and does not know what the layers are — that
/// is the Agent Prompt Contract's, and Fleet's. What it holds is the finished
/// string that becomes the session's first turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt(String);

impl Prompt {
    pub fn assembled(text: &str) -> Result<Prompt, SpawnConfigRefused> {
        if text.trim().is_empty() {
            return Err(SpawnConfigRefused::PromptEmpty);
        }
        Ok(Prompt(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The MCP configuration file Fleet wrote for one Drone.
///
/// Non-optional on [`DroneSpawnConfig`], because the Evidence tool is the only
/// sanctioned completion path and a Drone without it has no way to finish. The
/// two failures are not alike: a harness that cannot inject the server fails
/// immediately and visibly, while a harness that injects but cannot exclude
/// looks exactly like success — the Drone works, holding the operator's whole
/// toolbelt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpConfig {
    path: String,
}

impl McpConfig {
    /// The file at `path`, and nothing but the servers it names.
    ///
    /// The name says what the value means, so a call site reads as the
    /// guarantee rather than as a path being handed over.
    pub fn only_these(path: &str) -> Result<McpConfig, SpawnConfigRefused> {
        if path.trim().is_empty() {
            return Err(SpawnConfigRefused::McpConfigPathEmpty);
        }
        if !path.starts_with('/') {
            return Err(SpawnConfigRefused::McpConfigPathNotAbsolute {
                given: String::from(path),
            });
        }
        Ok(McpConfig {
            path: String::from(path),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

/// The Drone's whole environment, built one variable at a time.
///
/// **Nothing is inherited.** `env_clear` appears nowhere in v1's production
/// code, so a token exported in the operator's shell reached every Drone v1
/// spawned — the one place v1's Drone spawn was worse than its own check
/// spawn, and the thing its own handoff table said not to do. Here the starting
/// point is [`Environment::nothing`] and there is no method that adds the
/// caller's own environment to it.
///
/// **Nothing brokered goes in here yet.** Nothing in this workspace resolves a
/// secret, so there is no named exit through which a [`Secret`](crate::Secret)
/// becomes a value on this list. When one arrives it is one method, and this
/// type loses its `Debug` the day it does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    vars: Vec<(String, String)>,
}

impl Environment {
    /// An empty environment. **The only starting point there is.**
    pub fn nothing() -> Environment {
        Environment::default()
    }

    /// One variable, by name.
    ///
    /// A name that is empty, holds `=`, or holds a NUL is refused: the first
    /// two are not names the operating system can express, and a NUL truncates
    /// the string somebody else reads.
    pub fn and(mut self, name: &str, value: &str) -> Result<Environment, SpawnConfigRefused> {
        if name.is_empty() {
            return Err(SpawnConfigRefused::EnvNameEmpty);
        }
        if name.contains('=') || name.contains('\0') || value.contains('\0') {
            return Err(SpawnConfigRefused::EnvNameNotPortable {
                name: String::from(name),
            });
        }
        if self.vars.iter().any(|(held, _)| held == name) {
            return Err(SpawnConfigRefused::EnvNameTwice {
                name: String::from(name),
            });
        }
        self.vars.push((String::from(name), String::from(value)));
        Ok(self)
    }

    /// The variable that says this process runs unattended: a Drone, a Judge
    /// call or a scout. The `armada` mod reads it and reports nothing, so the
    /// Sessions list holds only the operator's own sessions.
    pub const UNATTENDED: &'static str = "ARMADA_DRONE";

    /// This environment with [`Environment::UNATTENDED`] set. Infallible: the
    /// name is fixed, and one already held is left as it is.
    pub fn unattended(mut self) -> Environment {
        if !self.vars.iter().any(|(held, _)| held == Self::UNATTENDED) {
            self.vars
                .push((String::from(Self::UNATTENDED), String::from("1")));
        }
        self
    }

    /// Every variable, in the order it was named.
    pub fn vars(&self) -> &[(String, String)] {
        &self.vars
    }

    /// The names only, never the values. What a log line or a failure journal
    /// is allowed to carry — v1's `EnvDelta::names()`, kept for the same reason.
    pub fn names(&self) -> Vec<&str> {
        self.vars.iter().map(|(name, _)| name.as_str()).collect()
    }
}

/// Everything one Drone is started with.
///
/// **Private fields, one constructor, no `Default`, no setter, no raw argument
/// builder and no escape-hatch constructor.** A future flag is a new field and
/// a new parameter, never a string that goes around the fields already here.
/// Every parameter of [`DroneSpawnConfig::spawn_in`] is a distinct type, so no
/// two of them can be passed in the wrong order.
///
/// The failure this refuses is the config that is correct today because
/// whoever built it remembered the flag, and wrong the day somebody builds one
/// by a different path. There is no different path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneSpawnConfig {
    worktree: Worktree,
    model: Model,
    prompt: Prompt,
    mcp: McpConfig,
    environment: Environment,
}

impl DroneSpawnConfig {
    /// The only way to make one.
    pub fn spawn_in(
        worktree: &Worktree,
        model: Model,
        prompt: Prompt,
        mcp: McpConfig,
            environment: Environment,
    ) -> DroneSpawnConfig {
        DroneSpawnConfig {
            worktree: worktree.clone(),
            model,
            prompt,
            mcp,
            environment,
        }
    }

    pub fn worktree(&self) -> &Worktree {
        &self.worktree
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The session's first turn. **Not an argument** — see this module's note
    /// on why nothing readable goes in argv.
    pub fn prompt(&self) -> &Prompt {
        &self.prompt
    }

    pub fn mcp(&self) -> &McpConfig {
        &self.mcp
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }
}

/// A value that could not have produced a confined Drone, refused where it was
/// written rather than where it would have failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnConfigRefused {
    ModelUnnamed,
    PromptEmpty,
    McpConfigPathEmpty,
    /// A relative path would resolve against the Drone's working directory,
    /// which is the worktree — so the file Fleet wrote would not be the file
    /// the Drone read.
    McpConfigPathNotAbsolute {
        given: String,
    },
    EnvNameEmpty,
    EnvNameNotPortable {
        name: String,
    },
    /// The same name twice. Refused rather than resolved, because which one
    /// wins is a rule nobody would find and both callers believe theirs is set.
    EnvNameTwice {
        name: String,
    },
    /// A Judge's checkout was relative, so it would read wherever the call
    /// happened to start. [`crate::Reading::checkout`].
    CheckoutNotAbsolute {
        given: String,
    },
}

/// What a harness turns a [`DroneSpawnConfig`] into: a process, not yet started.
///
/// **There is no stdio on it.** Where the Drone's input, output and errors go
/// is fixed by whoever starts it — stdin holds the session, stdout carries the
/// transcript — and a field here would be a way to ask for something else.
///
/// **And no field for the prompt's text on the argument list.** Argv is
/// world-readable through `ps`, measured on darwin 27 — where the environment
/// of a same-uid child is *not* — so anything on it is public to every process
/// on the machine, and a Job's brief is not something to publish. The task text
/// goes in on stdin as the session's first turn instead, which is also what a
/// live session needs: stdin held open, one JSON object per line, the mechanism
/// spike 4 measured. The prompt and every later injected turn take one path,
/// and there is not a second one to keep correct.
///
/// It takes its environment and its directory from the [`DroneSpawnConfig`]
/// rather than from its own caller, so an implementation cannot render a Drone
/// into a different directory or a different environment than the one it was
/// given — bar [`Launch::waiting_on_permission`], whose value is fixed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    program: String,
    args: Vec<String>,
    directory: String,
    environment: Environment,
}

impl Launch {
    /// Render a config into a startable process.
    ///
    /// **The directory and the environment come from the config**, not from the
    /// caller, so an implementation cannot put a Drone somewhere else or hand
    /// it something else. What an implementation supplies is the two things
    /// only it can know: which program, and how that program spells the
    /// config's guarantees.
    pub fn rendered(config: &DroneSpawnConfig, program: &str, args: Vec<String>) -> Launch {
        Launch {
            program: String::from(program),
            args,
            directory: String::from(config.worktree().path()),
            environment: config.environment().clone(),
        }
    }

    /// Marked as running unattended, for the `armada` mod to read.
    pub fn unattended(mut self) -> Launch {
        self.environment = self.environment.unattended();
        self
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The worktree. A Drone's working directory is its own checkout and
    /// nothing else, which is what makes a relative path in its own output
    /// mean something.
    pub fn directory(&self) -> &str {
        &self.directory
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }
}

/// A Drone that is running: what Fleet knows about the process itself.
///
/// **The pid, and nothing that can act on it.** Killing a Drone is
/// `LiveSession::terminate` in `fleet`, which is held by the one caller allowed
/// to end one; a handle with a `kill` on it would put that call everywhere the
/// handle goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneHandle {
    pid: u32,
}

impl DroneHandle {
    /// Record a process that started. Called by whoever started it.
    pub fn started(pid: u32) -> DroneHandle {
        DroneHandle { pid }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl SpawnConfigRefused {
    /// A sentence for a person, built here rather than by each caller.
    pub fn said(&self) -> String {
        match self {
            SpawnConfigRefused::ModelUnnamed => "no model was named".to_string(),
            SpawnConfigRefused::PromptEmpty => "the assembled prompt is empty".to_string(),
            SpawnConfigRefused::McpConfigPathEmpty => {
                "no MCP configuration file was named".to_string()
            }
            SpawnConfigRefused::McpConfigPathNotAbsolute { given } => {
                let mut said = String::from("the MCP configuration path `");
                said.push_str(given);
                said.push_str("` is relative, and would resolve inside the worktree");
                said
            }
            SpawnConfigRefused::EnvNameEmpty => {
                "an environment variable was given no name".to_string()
            }
            SpawnConfigRefused::EnvNameNotPortable { name } => {
                let mut said = String::from("`");
                said.push_str(name);
                said.push_str("` is not a name the operating system can carry");
                said
            }
            SpawnConfigRefused::EnvNameTwice { name } => {
                let mut said = String::from("`");
                said.push_str(name);
                said.push_str("` was named twice, and which one wins is not decided here");
                said
            }
            SpawnConfigRefused::CheckoutNotAbsolute { given } => {
                let mut said = String::from("the checkout `");
                said.push_str(given);
                said.push_str("` is relative, so a Judge would read wherever it started");
                said
            }
        }
    }
}

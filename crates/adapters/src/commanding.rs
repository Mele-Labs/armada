//! A Job's Commander, as the headless CLI is started for one turn. `#2166`.
//!
//! **A third launch, apart from a Drone's, Helm's and a scout's.** It is
//! `crate::conversing`'s process (one per turn, the session resumed by id) with
//! the opposite posture on the tools: the door alone (`--strict-mcp-config`),
//! the three reads, and every other built-in denied by name.
//!
//! **There is no owner to ask, so a write tool is not available to be asked
//! about.** Helm edits its checkout because a person said to and answers each
//! call; a Commander steers Drones and does not code the step. The toolset is
//! cut with `--tools` and `--disallowedTools` both, for [`crate::scouting`]'s
//! reason: a tool left out and not denied comes back through an allow rule
//! somebody adds. Which of the door's tools it may call is `api`'s allowlist.

use adapter_traits::{
    DroneSpawnConfig, Environment, Launch, McpConfig, Model, Prompt, Toolbelt, Worktree,
};

use crate::conversing::{door_tools, portable_session, ConversationRefused};
use crate::harness::HeadlessAgent;
use crate::scouting::{NOT_A_READ, READ_TOOLS};

/// Everything one Commander turn's process is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commanding {
    directory: String,
    model: Model,
    door: McpConfig,
    environment: Environment,
    resuming: Option<String>,
}

impl Commanding {
    /// A new session in `directory`, **the Job's own worktree** and never the
    /// owner's checkout: what a Commander reads is what its Drones wrote.
    pub fn in_worktree(
        directory: &str,
        model: Model,
        door: McpConfig,
        environment: Environment,
    ) -> Result<Commanding, ConversationRefused> {
        if !directory.starts_with('/') {
            return Err(ConversationRefused::DirectoryNotAbsolute {
                given: directory.to_string(),
            });
        }
        Ok(Commanding {
            directory: directory.to_string(),
            model,
            door,
            environment,
            resuming: None,
        })
    }

    /// The same, resuming a session the CLI named on an earlier turn. An id is
    /// refused on `crate::conversing`'s terms: it is argv.
    pub fn resuming(mut self, session: &str) -> Result<Commanding, ConversationRefused> {
        if !portable_session(session) {
            return Err(ConversationRefused::SessionNotPortable {
                given: session.to_string(),
            });
        }
        self.resuming = Some(session.to_string());
        Ok(self)
    }
}

impl HeadlessAgent {
    /// One Commander turn's process. The turn is not on it: it goes in on stdin.
    pub fn render_commander(&self, commanding: &Commanding) -> Result<Launch, ConversationRefused> {
        let mut allowed: Vec<&str> = READ_TOOLS.to_vec();
        let door = door_tools();
        allowed.push(&door);
        let mut args: Vec<String> = vec![
            "-p".into(),
            "--input-format".into(),
            "stream-json".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--model".into(),
            commanding.model.as_str().into(),
            // Not `default`: that mode puts an uncovered call to somebody, and
            // nobody is there. `dontAsk` refuses it, which is the posture.
            "--permission-mode".into(),
            "dontAsk".into(),
            "--strict-mcp-config".into(),
            "--mcp-config".into(),
            commanding.door.path().into(),
            "--tools".into(),
            READ_TOOLS.join(","),
            "--allowedTools".into(),
            allowed.join(","),
            "--disallowedTools".into(),
            NOT_A_READ.join(","),
        ];
        crate::harness::thinking(&mut args, commanding.model.effort());
        if let Some(session) = &commanding.resuming {
            args.push("--resume".into());
            args.push(session.clone());
        }
        // `Launch`'s one constructor reads a spawn config; only the directory
        // and the environment are taken from it, as a conversation's is.
        let borrowed = DroneSpawnConfig::spawn_in(
            &Worktree::at(commanding.directory.clone(), ""),
            commanding.model.clone(),
            Prompt::assembled("carried on stdin").map_err(ConversationRefused::Unassembled)?,
            commanding.door.clone(),
            Toolbelt::evidence_only(),
            commanding.environment.clone(),
        );
        Ok(Launch::rendered(&borrowed, self.program(), args).unattended())
    }
}

/// The built-ins a Commander's launch denies, for a test to hold every write to.
pub fn denied_to_a_commander() -> &'static [&'static str] {
    NOT_A_READ
}

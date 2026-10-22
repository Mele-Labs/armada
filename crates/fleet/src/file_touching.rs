//! Which files a Session and its subagents touched, read off the tool calls Fleet hears, for the
//! Field. `docs/concepts/session.md`, *Who touched which file*.
//!
//! **Pure.** The listeners that already hear a tool call (`session_host::hearing` for a hosted
//! Session's stream, `sessioning` for a terminal Session's mod) pass what they heard here, and
//! `plan_keeping` writes it. Nothing is asked of the agent: a subagent is told from the stream by
//! the call that started it, and a file by the call that changed it.

use std::collections::VecDeque;

use adapter_traits::{AgentHarness, DroneEvent, FileAct, ToolUse};

/// How many touches a Session keeps. They ride on the record, which goes out whole on every change.
pub(crate) const WINDOW: usize = 60;

/// The most subagent calls remembered, oldest forgotten first: enough for any one turn's.
const SPAWNS_AT_MOST: usize = 64;

/// The most paths one `rm` is read for.
const REMOVED_AT_MOST: usize = 8;

/// What a subagent's call was asked to be, by the id of the call. In memory: a Fleet that restarted
/// names a subagent whose call it did not hear by what it is, a subagent.
#[derive(Default)]
pub(crate) struct Spawns(VecDeque<(String, String)>);

impl Spawns {
    pub(crate) fn told(&mut self, call: &str, kind: &str) {
        self.0.retain(|(held, _)| held != call);
        self.0.push_back((call.to_string(), kind.to_string()));
        while self.0.len() > SPAWNS_AT_MOST {
            self.0.pop_front();
        }
    }

    pub(crate) fn kind_of(&self, call: &str) -> Option<&str> {
        self.0.iter().find(|(held, _)| held == call).map(|(_, kind)| kind.as_str())
    }
}

/// What a call did to a file: `write`, `edit` or `remove`, as the store keeps it.
pub(crate) type Act = &'static str;

/// The files an event changed, as the call named them, and how. A file changed by an edit tool is the
/// harness's to name; one taken away is read from a plain `rm` or `git rm`, and a command that
/// removes by a pattern or a variable names nothing.
pub(crate) fn touches_of<H: AgentHarness>(harness: &H, event: &DroneEvent) -> Vec<(String, Act)> {
    if let Some(path) = harness.edited(event) {
        let act = match harness.edit_act(event) {
            FileAct::Write => "write",
            FileAct::Edit => "edit",
        };
        return vec![(path, act)];
    }
    match harness.used(event) {
        Some(ToolUse::Ran(command)) => removed_by(&command).into_iter().map(|path| (path, "remove")).collect(),
        _ => Vec::new(),
    }
}

/// The paths a shell line removes with `rm` or `git rm`: its plain words after the flags.
pub(crate) fn removed_by(command: &str) -> Vec<String> {
    let mut found = Vec::new();
    for segment in command.replace("&&", ";").replace("||", ";").replace('\n', ";").split(';') {
        let mut words = segment.split_whitespace().skip_while(|word| matches!(*word, "sudo" | "command") || word.contains('='));
        let removing = match (words.next(), words.clone().next()) {
            (Some("rm" | "unlink"), _) => true,
            (Some("git"), Some("rm")) => {
                words.next();
                true
            }
            _ => false,
        };
        if !removing {
            continue;
        }
        for word in words {
            let word = word.trim_matches(['"', '\'']);
            let plain = !word.is_empty()
                && !word.starts_with('-')
                && !word.contains(['*', '?', '[', ']', '$', '`', '(', ')', '{', '}', '<', '>', '|', '&', '\\']);
            if plain && found.len() < REMOVED_AT_MOST {
                found.push(word.to_string());
            }
        }
    }
    found
}

/// A path from the repository's root, for a file the Session's agent touched. `root` is the
/// worktree and `cwd` the directory the Session works in, which may be below it. `~/` is `home`, an
/// absolute path loses `root`, and a relative one is from `cwd`, so it gains `cwd`'s way down from
/// `root`. A path that lies outside `root`, climbs out with `..`, or names the directory itself is none.
pub(crate) fn rooted(root: &str, cwd: &str, home: &str, path: &str) -> Option<String> {
    let path = match path.strip_prefix("~/") {
        Some(rest) if !home.is_empty() => format!("{}/{rest}", home.trim_end_matches('/')),
        _ => path.to_string(),
    };
    let root = root.trim_end_matches('/');
    let relative = match path.strip_prefix('/') {
        Some(_) => path.strip_prefix(&format!("{root}/"))?.to_string(),
        None => {
            let way = cwd.trim_end_matches('/').strip_prefix(root).unwrap_or_default().trim_matches('/');
            let path = path.trim_start_matches("./");
            match way.is_empty() {
                true => path.to_string(),
                false => format!("{way}/{path}"),
            }
        }
    };
    let relative = relative.trim_start_matches("./").trim_end_matches('/').to_string();
    (!relative.is_empty() && !relative.split('/').any(|part| part == "..")).then_some(relative)
}

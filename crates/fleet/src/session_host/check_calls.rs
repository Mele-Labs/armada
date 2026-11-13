//! The agent's own shell calls that run `armada check`, held while they run, so a Check run that
//! starts names the thread row of the call that ran it (`SessionRow::Check::tool_row`).
//!
//! **Detected off the session's own stream, never reported**: the CLI's `armada check` tells Fleet
//! nothing of the call it runs in. A call is out from the moment it is heard until it answers, or,
//! where it was backgrounded, until the task it started ends. The harness answers a backgrounded
//! shell call before its command has begun, so an answer alone does not end one. A subagent's calls
//! are no rows of the thread and are not held. Memory only: a replaced process ends every one.

use std::collections::VecDeque;

/// How many calls are held. One whose end is never heard is dropped oldest first.
const HELD_AT_MOST: usize = 64;

struct Open {
    call: String,
    /// The thread row drawn for the call.
    row: String,
    answered: bool,
    /// The background task the call started, which ends it.
    task: Option<String>,
}

impl Open {
    fn running(&self) -> bool {
        !self.answered || self.task.is_some()
    }
}

#[derive(Default)]
pub(crate) struct CheckCalls(VecDeque<Open>);

impl CheckCalls {
    pub(crate) fn called(&mut self, call: &str, row: &str) {
        self.0.push_back(Open {
            call: call.to_string(),
            row: row.to_string(),
            answered: false,
            task: None,
        });
        while self.0.len() > HELD_AT_MOST {
            self.0.pop_front();
        }
    }

    pub(crate) fn answered(&mut self, call: &str) {
        if let Some(open) = self.0.iter_mut().find(|open| open.call == call) {
            open.answered = true;
        }
    }

    pub(crate) fn backgrounded(&mut self, call: &str, task: &str) {
        if let Some(open) = self.0.iter_mut().find(|open| open.call == call) {
            open.task = Some(task.to_string());
        }
    }

    pub(crate) fn task_ended(&mut self, task: &str) {
        self.0.retain(|open| open.task.as_deref() != Some(task));
    }

    pub(crate) fn ended_all(&mut self) {
        self.0.clear();
    }

    /// The row of the one call running now. **None where none is or several are**: two runs at
    /// once cannot be told apart by anything the run says.
    pub(crate) fn running_row(&self) -> Option<String> {
        let mut running = self.0.iter().filter(|open| open.running());
        match (running.next(), running.next()) {
            (Some(one), None) => Some(one.row.clone()),
            _ => None,
        }
    }
}

/// Whether a shell command runs `armada check`: the word `armada`, or a path ending in it, then
/// `check`, anywhere in a pipeline or a list.
pub(crate) fn runs_armada_check(command: &str) -> bool {
    let words: Vec<&str> = command
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '(' | ')'))
        .filter(|word| !word.is_empty())
        .collect();
    words
        .windows(2)
        .any(|pair| pair[1] == "check" && pair[0].rsplit('/').next() == Some("armada"))
}

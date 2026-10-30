//! The writes a tracker mod may teach: `[move]`, `[assign]` and `[close]`, each a call that
//! changes one issue and answers nothing Armada reads. `docs/concepts/mods.md`, *tracker.toml*.
//!
//! **Optional, and a mod without one cannot make that write.** A saved rule's act that needs it is
//! then planned and recorded as not carried, with the sentence [`Writes::lacks`] gives, and the
//! owner is never asked to press it. **A try never runs one**: it writes into the tracker.

use adapter_traits::IssueWrite;
use toml::{Table, Value};

use super::file::{transport_of, Reads, Transport, ASSIGNEE, REFERENCE, STATUS};

/// One write, as the call that makes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Write {
    pub(crate) transport: Transport,
}

/// The writes a mod teaches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Writes {
    /// `{reference}` is the issue and `{status}` the tracker's word for where it goes.
    pub(crate) move_to: Option<Write>,
    /// `{reference}` is the issue and `{assignee}` whoever it goes to.
    pub(crate) assign: Option<Write>,
    /// `{reference}` is the issue.
    pub(crate) close: Option<Write>,
}

impl Writes {
    /// The call that makes `write`, or why the mod cannot. **A close with no `[close]` is a move to
    /// the word the Workspace gives Closed**, so a tracker that closes by status teaches one section.
    pub(crate) fn call_for(&self, write: &IssueWrite<'_>) -> Result<&Transport, String> {
        let found = match write {
            IssueWrite::Move { .. } => self.move_to.as_ref(),
            IssueWrite::Assign { .. } => self.assign.as_ref(),
            IssueWrite::Close { status } => self.close.as_ref().or((*status).and(self.move_to.as_ref())),
        };
        found.map(|one| &one.transport).ok_or_else(|| self.lacks(write))
    }

    fn lacks(&self, write: &IssueWrite<'_>) -> String {
        match write {
            IssueWrite::Move { .. } => String::from("this tracker mod teaches no `[move]`, so it cannot move an issue"),
            IssueWrite::Assign { .. } => String::from("this tracker mod teaches no `[assign]`, so it cannot assign an issue"),
            IssueWrite::Close { status: Some(_) } => {
                String::from("this tracker mod teaches neither `[close]` nor `[move]`, so it cannot close an issue")
            }
            IssueWrite::Close { status: None } if self.move_to.is_some() => {
                String::from("this tracker mod closes by a move, and the ticket lifecycle names no word for Closed")
            }
            IssueWrite::Close { status: None } => String::from("this tracker mod teaches no `[close]`, so it cannot close an issue"),
        }
    }
}

/// The three sections, read from `table`. A problem is pushed for each that is wrong.
pub(super) fn read(table: &Table, has_credential: bool, problems: &mut Vec<String>) -> Writes {
    Writes {
        move_to: one(table, "move", &[REFERENCE, STATUS], has_credential, problems),
        assign: one(table, "assign", &[REFERENCE, ASSIGNEE], has_credential, problems),
        close: one(table, "close", &[REFERENCE], has_credential, problems),
    }
}

fn one(table: &Table, name: &str, fills: &[&str], has_credential: bool, problems: &mut Vec<String>) -> Option<Write> {
    match table.get(name) {
        None => None,
        Some(Value::Table(it)) => {
            let before = problems.len();
            let transport = transport_of(name, it, fills, Reads::Nothing, has_credential, problems)?;
            (problems.len() == before).then_some(Write { transport })
        }
        Some(_) => {
            problems.push(format!("`{name}` must be a section, `[{name}]`"));
            None
        }
    }
}

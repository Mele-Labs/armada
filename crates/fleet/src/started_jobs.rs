//! What a Job may do to the Jobs it started, decided on values. `docs/concepts/session/a-job-that-started-jobs.md`.
//!
//! **A child is a Job the caller's own `started_to` attachments name**, and only those: a grandchild,
//! a fork and another Job's child are none of its. The effects are `session_host::commanding`'s;
//! what is decided here needs no store, so the acceptance test calls it.

use ipc::{Attachment, AttachmentState, StartBase, StartCondition, StartedAct, StartedCommand};

/// The attachment on a starter that names the Job it started.
pub const STARTED_TO: &str = "started_to";

/// An entry names a Job the caller did not start.
pub const NOT_YOURS: &str = "fleet.started_not_yours";
/// A withdraw of a Job that has begun.
pub const BEGUN: &str = "fleet.started_begun";
/// A withdraw of a Job that has ended.
pub const ENDED: &str = "fleet.started_ended";
/// An end with no reason to tell the Job.
pub const NEEDS_WHY: &str = "fleet.started_needs_why";
/// A change that changes nothing.
pub const NEEDS_CHANGE: &str = "fleet.started_needs_change";

/// One Job the caller started, and the two facts a ruling reads of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Child {
    pub id: String,
    /// It has not ended.
    pub live: bool,
    /// An agent has run in it: it is neither held on its start card nor waiting on another Job.
    pub begun: bool,
}

/// What Fleet does for one command, once it is ruled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Do {
    Withdraw,
    End { why: String },
    Pause,
    Resume,
    Change { condition: Option<StartCondition>, base: Option<StartBase> },
    Drop,
}

/// Why a command is not done: a code of its own and a sentence that says so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub code: &'static str,
    pub said: String,
}

/// The ids of the Jobs `attachments`, a starter's, name as started, oldest first and once each.
pub fn children_in(attachments: &[Attachment]) -> Vec<String> {
    let mut named: Vec<&Attachment> = attachments.iter().filter(|one| one.kind == STARTED_TO).collect();
    named.sort_by(|a, b| a.since.as_str().cmp(b.since.as_str()));
    let mut ids: Vec<String> = Vec::new();
    for one in named {
        // A link is `spent`, a fact that stays; one given back names no child.
        if one.state != AttachmentState::GivenBack && !ids.contains(&one.target) {
            ids.push(one.target.clone());
        }
    }
    ids
}

/// `command` against the caller's `children`: what to do, or why not. **The first thing asked is whether
/// the Job is the caller's**, so nothing is learned of another Job's state from the refusal.
pub fn ruled(children: &[Child], command: &StartedCommand) -> Result<Do, Refused> {
    let id = command.session_id.as_str();
    let act = command.act;
    let Some(child) = children.iter().find(|one| one.id == id) else {
        return Err(Refused {
            code: NOT_YOURS,
            said: format!("job {id} was not started by this job, so it is not yours to {}", act.word()),
        });
    };
    match act {
        StartedAct::Withdraw if !child.live => Err(Refused { code: ENDED, said: format!("job {id} has ended, so there is nothing to withdraw") }),
        StartedAct::Withdraw if child.begun => {
            Err(Refused { code: BEGUN, said: format!("job {id} has begun, so it cannot be withdrawn; end it instead") })
        }
        StartedAct::Withdraw => Ok(Do::Withdraw),
        StartedAct::End => match command.why.as_deref().map(str::trim).filter(|why| !why.is_empty()) {
            Some(why) => Ok(Do::End { why: why.to_string() }),
            None => Err(Refused { code: NEEDS_WHY, said: format!("ending job {id} needs a why, which the job is told") }),
        },
        StartedAct::Pause => Ok(Do::Pause),
        StartedAct::Resume => Ok(Do::Resume),
        StartedAct::Change if command.condition.is_none() && command.base.is_none() => {
            Err(Refused { code: NEEDS_CHANGE, said: format!("changing job {id} needs a condition or a base") })
        }
        StartedAct::Change => Ok(Do::Change { condition: command.condition.clone(), base: command.base }),
        StartedAct::Drop => Ok(Do::Drop),
    }
}

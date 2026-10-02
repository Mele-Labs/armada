//! A plan's groups, and Fleet's record of each group's runs. Spike 022, slice 2.
//!
//! **A group is what runs.** Its tasks are worked one at a time, and the
//! step's gate runs at the group's end rather than once over the step. A red
//! group goes round on its own while the step's `retry_limit` lasts, and its
//! tasks read `failed` only once the retries run out (answer 9).
//!
//! **Which tasks are in which group is the plan's**, in
//! [`WorkPlan`](crate::WorkPlan), because a person moves them there. **How each
//! group's runs went is Fleet's**, here, because nobody but the gate can say:
//! it is appended as [`GroupMove`]s and folded, the plan's own shape, so a
//! restart reads back exactly what the gate wrote.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::num::NonZeroU32;

use crate::envelope::Timestamp;
use crate::job::attempt::{Attempt, Spent};
use crate::job::escalation::{EscalationTrigger, StepLevelTrigger};
use crate::job::ids::StepId;
use crate::job::step::StepVerdict;

/// A group's stable name within one plan: `G1`, `G2`, … in the order minted.
/// **Minted by Fleet when the plan is recorded**, and never renumbered by a
/// move, so a group's runs stay its own wherever a person puts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(NonZeroU32);

impl GroupId {
    /// The first group, which is every task of a plan recorded without groups.
    pub const FIRST: GroupId = GroupId(NonZeroU32::MIN);

    pub fn numbered(number: NonZeroU32) -> GroupId {
        GroupId(number)
    }

    /// `G` and a positive number with no leading zero, or `None`.
    pub fn read(spelling: &str) -> Option<GroupId> {
        let digits = spelling.strip_prefix('G')?;
        if digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse::<NonZeroU32>().ok().map(GroupId)
    }

    pub fn number(self) -> u32 {
        self.0.get()
    }

    /// The group after the highest of these, for a mint.
    pub(crate) fn after(highest: u32) -> GroupId {
        GroupId(NonZeroU32::MIN.saturating_add(highest))
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "G{}", self.0)
    }
}

/// Where a group is. The registry's eight words (`group_state` in
/// `domain/enum-verbs.toml`), of which Fleet writes five: `joining` waits for
/// tasks that run at once (slice 5), and `checking` and `landed` are drawn from
/// the step's own `checking` and the delivery rather than written here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupState {
    Pending,
    Running,
    Joining,
    Checking,
    Passed,
    Failed,
    Retrying,
    Landed,
}

impl GroupState {
    pub const ALL: &'static [GroupState] = &[
        GroupState::Pending,
        GroupState::Running,
        GroupState::Joining,
        GroupState::Checking,
        GroupState::Passed,
        GroupState::Failed,
        GroupState::Retrying,
        GroupState::Landed,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            GroupState::Pending => "pending",
            GroupState::Running => "running",
            GroupState::Joining => "joining",
            GroupState::Checking => "checking",
            GroupState::Passed => "passed",
            GroupState::Failed => "failed",
            GroupState::Retrying => "retrying",
            GroupState::Landed => "landed",
        }
    }

    pub fn from_wire(value: &str) -> Option<GroupState> {
        GroupState::ALL
            .iter()
            .copied()
            .find(|s| s.as_wire() == value)
    }
}

/// One row of Fleet's record of a group's runs, as it is appended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupMove {
    /// A run of the group began: its first task's Drone, a round Fleet sent it
    /// on, or a person restarting one of its tasks.
    Started {
        group: GroupId,
        /// Which run of the group, from one. Never chosen by a caller:
        /// [`GroupRuns::opening`] counts it.
        run: u32,
        step: StepId,
        /// The step's run this group's run is filed under, which is what its
        /// Checks, Judge and evidence are kept against.
        step_attempt: Attempt,
        at: Timestamp,
    },
    /// The group's gate answered that run.
    Ended {
        group: GroupId,
        run: u32,
        verdict: StepVerdict,
        /// The commit the group made, on a green run that committed.
        commit: Option<String>,
        at: Timestamp,
    },
}

/// One run of one group, as the record leaves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupAttempt {
    pub group: GroupId,
    pub run: u32,
    pub step: StepId,
    pub step_attempt: Attempt,
    pub started_at: Timestamp,
    /// `None` while the run is open.
    pub ended: Option<GroupEnded>,
}

/// What the gate said about one run of a group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEnded {
    pub at: Timestamp,
    pub verdict: StepVerdict,
    pub commit: Option<String>,
}

/// Every run of every group of one Job's plan, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupRuns {
    attempts: Vec<GroupAttempt>,
}

impl GroupRuns {
    /// The record a list of moves leaves. **An end with no open run is
    /// dropped** rather than refused: it is Fleet's own row, written after the
    /// start it closes, and the only way to reach one is a store edited by hand.
    pub fn fold(moves: &[GroupMove]) -> GroupRuns {
        let mut attempts: Vec<GroupAttempt> = Vec::new();
        for moved in moves {
            match moved {
                GroupMove::Started {
                    group,
                    run,
                    step,
                    step_attempt,
                    at,
                } => attempts.push(GroupAttempt {
                    group: *group,
                    run: *run,
                    step: step.clone(),
                    step_attempt: *step_attempt,
                    started_at: at.clone(),
                    ended: None,
                }),
                GroupMove::Ended {
                    group,
                    run,
                    verdict,
                    commit,
                    at,
                } => {
                    if let Some(open) = attempts
                        .iter_mut()
                        .rev()
                        .find(|a| a.group == *group && a.run == *run && a.ended.is_none())
                    {
                        open.ended = Some(GroupEnded {
                            at: at.clone(),
                            verdict: *verdict,
                            commit: commit.clone(),
                        });
                    }
                }
            }
        }
        GroupRuns { attempts }
    }

    /// Every run of one group, oldest first.
    pub fn attempts(&self, group: GroupId) -> impl Iterator<Item = &GroupAttempt> {
        self.attempts.iter().filter(move |a| a.group == group)
    }

    /// Every run of every group, oldest first.
    pub fn all(&self) -> &[GroupAttempt] {
        &self.attempts
    }

    fn last(&self, group: GroupId) -> Option<&GroupAttempt> {
        self.attempts(group).last()
    }

    /// The run of this group the gate has not answered yet.
    pub fn open_attempt(&self, group: GroupId) -> Option<&GroupAttempt> {
        self.last(group).filter(|a| a.ended.is_none())
    }

    /// Whether the group's last run passed and none is open since.
    pub fn passed(&self, group: GroupId) -> bool {
        self.last(group)
            .and_then(|a| a.ended.as_ref())
            .is_some_and(|ended| ended.verdict == StepVerdict::Passed)
    }

    /// Whether the group's last run, with none open since, stopped on
    /// `gate_failure`: a Judge refusal after green Checks, or a red run on
    /// the last its retries allow. Its tasks' states tell the two apart.
    pub fn stopped_on_gate_failure(&self, group: GroupId) -> bool {
        let gate_failure = StepLevelTrigger::of(EscalationTrigger::GateFailure);
        self.last(group)
            .and_then(|a| a.ended.as_ref())
            .is_some_and(|ended| match ended.verdict {
                StepVerdict::Failed(trigger) => Some(trigger) == gate_failure,
                _ => false,
            })
    }

    /// Which run the group is on, which is what its retry budget is asked
    /// against: the open one, or the one a gate with none open would start.
    /// **Not reset by a person's restart**, as a step's is not today.
    pub fn spent(&self, group: GroupId) -> Spent {
        let run = match self.last(group) {
            Some(open) if open.ended.is_none() => open.run,
            Some(ended) => ended.run + 1,
            None => 1,
        };
        Spent::runs_this_pass(run)
    }

    /// The row that starts the group's next run.
    pub fn opening(
        &self,
        group: GroupId,
        step: &StepId,
        step_attempt: Attempt,
        at: Timestamp,
    ) -> GroupMove {
        GroupMove::Started {
            group,
            run: self.last(group).map_or(1, |a| a.run + 1),
            step: step.clone(),
            step_attempt,
            at,
        }
    }

    /// The row that answers the group's open run, or `None` with none open.
    pub fn closing(
        &self,
        group: GroupId,
        verdict: StepVerdict,
        commit: Option<String>,
        at: Timestamp,
    ) -> Option<GroupMove> {
        self.open_attempt(group).map(|open| GroupMove::Ended {
            group,
            run: open.run,
            verdict,
            commit,
            at,
        })
    }

    /// When the group's first run began.
    pub fn started_at(&self, group: GroupId) -> Option<&Timestamp> {
        self.attempts(group).next().map(|a| &a.started_at)
    }

    /// When the group's last run was answered, with none open since: passed,
    /// failed for good, or stopped for a person.
    pub fn ended_at(&self, group: GroupId) -> Option<&Timestamp> {
        self.last(group)
            .and_then(|a| a.ended.as_ref())
            .map(|ended| &ended.at)
    }

    /// Where the group is, from its runs and its tasks' states.
    pub fn state(&self, group: GroupId, plan: &crate::WorkPlan) -> GroupState {
        use crate::TaskState;
        let mut tasks = plan.tasks_in(group).map(|task| task.state());
        let any_working = tasks.clone().any(|s| s == TaskState::Working);
        let any_open = tasks.any(|s| s == TaskState::Open);
        let open = self.open_attempt(group);
        match (open, self.last(group)) {
            (Some(open), _) if open.run > 1 => GroupState::Retrying,
            (Some(_), _) => GroupState::Running,
            (None, _) if any_working => GroupState::Running,
            (None, Some(last)) => match last.ended.as_ref().map(|ended| ended.verdict) {
                Some(StepVerdict::Passed) if any_open => GroupState::Pending,
                Some(StepVerdict::Passed) => GroupState::Passed,
                Some(StepVerdict::Failed(_)) => GroupState::Failed,
                _ => GroupState::Pending,
            },
            (None, None) => GroupState::Pending,
        }
    }
}

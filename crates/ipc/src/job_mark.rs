//! The state a Jobs-list row draws, read by Fleet so a client does not work it out. **States only**:
//! the words a row says beside its mark stay with the client. Carried on `SessionRecord::mark`.

use serde::{Deserialize, Serialize};

/// What a Job is doing, as a row's mark draws it. Bridge's `SessionState`, value for value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Blank,
    Working,
    Running,
    Subagent,
    Stuck,
    Stopped,
    StoppedAsks,
    Waiting,
    Replied,
    Done,
    Failing,
    Idle,
    Quiet,
    Ended,
    Limited,
    After,
    Draft,
    Held,
    Paused,
    Unstarted,
}

/// The row's lamp: needs him, failed, running, done; anything else is idle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobLamp {
    Needs,
    Failed,
    Running,
    Done,
    Idle,
}

/// The group a list files the row under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobGroup {
    Needs,
    Working,
    Held,
    Done,
}

/// How a Job's row reads: its state, lamp and group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobMark {
    pub state: JobState,
    pub lamp: JobLamp,
    pub group: JobGroup,
}

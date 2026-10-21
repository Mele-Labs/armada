//! A Fleet notice a person turned off — `set_notice_mute` in `crates/ipc/operations/`.

use serde::{Deserialize, Serialize};

/// A notice Fleet sends a Session that can be turned off. The word is what a thread row's
/// `notice` carries and what the store keeps the mute under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Notice {
    /// "slot-N is N commits behind the base."
    BehindBase,
}

impl Notice {
    pub fn word(self) -> &'static str {
        match self {
            Notice::BehindBase => "behind_base",
        }
    }
}

/// What a mute covers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NoticeScope {
    /// One Job, until a person lifts it.
    Job { session_id: String },
    /// Every Job in a repository, until a person lifts it.
    Repo { manifest_id: String },
    /// One Job, for an hour from the press. Lifting it lifts the Job's mute too.
    Hour { session_id: String },
}

/// `set_notice_mute`' body and answer: mute the notice, or lift the mute.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetNoticeMute {
    pub notice: Notice,
    pub scope: NoticeScope,
    pub muted: bool,
}

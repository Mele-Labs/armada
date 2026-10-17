//! Who asked for a Check run, as a typed value a client can follow to the place
//! that asked. Since 23.40.
//!
//! **`kind` is an opaque string**, the way a Record row's kind is: a new
//! kind of requester is then a minor bump, and a client that has not heard of
//! one draws the ids it carries. The ids each kind needs are beside it, and
//! absent on every kind that does not use them.
//!
//! | `kind` | Who | Ids |
//! |---|---|---|
//! | `merge_line` | The merge line, for one branch | `branch` |
//! | `session` | A Session's agent running `armada check` in the slot it holds. Since 23.75 | `session_id`, `slot` |
//! | `outside` | Nothing in Armada: a person's press, a bare `armada check` | none |
//!
//! **`outside` is a value and never an absence.** A record written before this
//! field existed reads as `outside`, which is also what it was.

use serde::{Deserialize, Serialize};

pub const MERGE_LINE: &str = "merge_line";
pub const SESSION: &str = "session";
pub const OUTSIDE: &str = "outside";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requester {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The Session whose agent ran it. Since 23.75.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The pool slot it ran in. Since 23.75.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

impl Requester {
    fn of(kind: &str) -> Requester {
        Requester {
            kind: kind.to_string(),
            branch: None,
            session_id: None,
            slot: None,
        }
    }

    pub fn merge_line(branch: &str) -> Requester {
        Requester {
            branch: Some(branch.to_string()),
            ..Requester::of(MERGE_LINE)
        }
    }

    pub fn session(session_id: &str, slot: u32) -> Requester {
        Requester {
            session_id: Some(session_id.to_string()),
            slot: Some(slot),
            ..Requester::of(SESSION)
        }
    }

    pub fn outside() -> Requester {
        Requester::of(OUTSIDE)
    }
}

impl Default for Requester {
    fn default() -> Requester {
        Requester::outside()
    }
}

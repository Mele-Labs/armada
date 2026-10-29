//! The Job browser: a page in Bridge that a Job's agent and the owner drive together.
//!
//! **Fleet holds each act and Bridge does it.** An agent's call is parked under a request id,
//! published as `browser.request`, and answered when Bridge posts `answer_browser_request`.
//! `docs/practices/protocol/the-job-browser.md`.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;
use crate::walk_steps::WalkTarget;

/// `browser_open`: the agent loads a page. `POST /sessions/browser/open`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserOpen {
    pub url: String,
    /// Which Job. **Read only where the connection places no hosted Job**, as `TypeInTerminal`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `browser_click`: the agent presses something on the page. `POST /sessions/browser/click`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserClick {
    pub target: WalkTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `browser_type`: the agent types into something on the page. `POST /sessions/browser/type`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserType {
    pub target: WalkTarget,
    pub text: String,
    /// Whether Enter follows `text`. Absent is `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub submit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `browser_snapshot` and `browser_screenshot` take nothing but the Job.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `browser_wait_for`: the agent waits for words or a target to appear. Exactly one of `text` and
/// `target`. `POST /sessions/browser/wait_for`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserWaitFor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<WalkTarget>,
    /// Absent is 5000; at most 30000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `browser_evaluate`: the agent runs an expression in the page. `POST /sessions/browser/evaluate`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserEvaluate {
    pub expression: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// One act Bridge is asked to do, as `browser.request` carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserAct {
    Open {
        url: String,
    },
    Click {
        target: WalkTarget,
    },
    Type {
        target: WalkTarget,
        text: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        submit: bool,
    },
    Snapshot,
    /// **`path` is Fleet's**: a file in the Job's scratch folder Bridge writes the PNG to. Whatever
    /// a caller set is replaced.
    Screenshot {
        #[serde(default)]
        path: String,
    },
    /// `timeout_ms` as Fleet held it, so Bridge waits as long as Fleet expects.
    WaitFor {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<WalkTarget>,
        timeout_ms: u64,
    },
    Evaluate {
        expression: String,
    },
}

/// `browser.request` on `/events`: Fleet asks Bridge to do one act in a Job's browser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserRequest {
    pub session_id: SessionId,
    pub request_id: String,
    pub act: BrowserAct,
}

/// What a done act answers: Bridge's to Fleet, and Fleet's to the agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserAnswer {
    /// After `open`, `click`, `type`, `snapshot` and `wait_for`: the page's accessibility tree as
    /// text.
    Page {
        url: String,
        title: String,
        snapshot: String,
        /// Fleet cut `snapshot`. Bridge sends none.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
    /// After `screenshot`. `path` is the act's, set by Fleet; Bridge sends none.
    Screenshot {
        url: String,
        title: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        path: String,
    },
    /// After `evaluate`: the value as `JSON.stringify` writes it, absent where it has no JSON.
    Evaluated {
        url: String,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<String>,
        /// Fleet cut `result`. Bridge sends none.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
}

/// Why Bridge could not do an act. Fleet words each for the agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserError {
    /// The owner took the browser over.
    OwnerDriving,
    /// The act's target is not on the page.
    NotFound,
    /// `wait_for` ran out, or a page did not load in Bridge's own time.
    TimedOut,
    /// Anything else, in Bridge's words: an expression that threw, an address that would not load.
    Failed { why: String },
}

/// `answer_browser_request`: Bridge answers one `browser.request`, with exactly one of `answer` and
/// `error`. `POST /sessions/browser/answer`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerBrowserRequest {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<BrowserAnswer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<BrowserError>,
}

/// What `answer_browser_request` is answered: the call it completed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserRequestAnswered {
    pub request_id: String,
    pub session_id: SessionId,
}

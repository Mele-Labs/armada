//! `show_window`'s `steps`: a scripted walk a session passes with the page it shows.
//!
//! **The shape is the mock walks' own** (`apps/desktop/src/renderer/src/mock/walk.ts`): one verb per
//! step, a target found the way a test finds one, and a `say` caption. Bridge plays them from
//! outside the page (`docs/practices/capture-window.md`, *A walk in the page*), so a session in any
//! repository can walk the owner through its changes on a page that knows nothing of Armada.
//!
//! **A step is one struct and not an enum** so the JSON a session writes is the JSON the mock
//! writes: the verb is the key that is present. Which keys may stand together is checked in Fleet,
//! where a refusal can name the step.

use serde::{Deserialize, Serialize};

/// Something on the page, found by role and accessible name, or by the words it draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkTarget {
    /// An ARIA role such as `button`, `tab` or `textbox`, or `text` for an element found by the
    /// words it draws.
    pub role: String,
    /// A string inside the accessible name (or inside the words, for `text`), in any case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The whole name and nothing else, rather than a string inside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact: Option<bool>,
    /// Looked for only inside this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within: Option<Box<WalkTarget>>,
}

/// A distance in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkPoint {
    pub x: i32,
    pub y: i32,
}

/// One step of a walk: exactly one of `press`, `look`, `hover`, `later`, `type`, `key`, `select`,
/// `paste` or `drag`, with the key that goes with it, and the caption the owner reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkStep {
    /// What the owner reads while the step is on screen.
    pub say: String,
    /// Pressed with a pointer: down, up, at the target's middle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press: Option<WalkTarget>,
    /// Ringed and nothing more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<WalkTarget>,
    /// The pointer put on the target and held there while the step is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<WalkTarget>,
    /// Ringed, as `look`. A mock uses it to let its scenario move on; a page of its own has nothing to move.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub later: Option<WalkTarget>,
    /// Typed into `into`; words ending in a newline end with Enter.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub typed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into: Option<WalkTarget>,
    /// A key sent to `on` as the keyboard sends one: `n`, `Tab`, or `Meta+n` for a chord.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<WalkTarget>,
    /// These words, inside `in`, selected as a pointer drag leaves them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(default, rename = "in", skip_serializing_if = "Option::is_none")]
    pub within: Option<WalkTarget>,
    /// The clipboard as it stands, pasted into the target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paste: Option<WalkTarget>,
    /// Picked up by its middle and put down `by` this far away.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag: Option<WalkTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<WalkPoint>,
}

impl WalkStep {
    /// The steps as the window's artifact on the ledger keeps them, one string in its `detail`,
    /// which holds strings only. Bridge reads it back to reopen the window with its steps.
    pub fn as_detail(steps: &[WalkStep]) -> String {
        serde_json::to_string(steps).unwrap_or_default()
    }
}

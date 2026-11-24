//! The Cockpit home's layout: the one grid of blocks Bridge draws when no Job is selected, kept by Fleet per user.
//! `docs/practices/protocol/projects.md`, *The Cockpit home's layout*.
//!
//! **The wire shape is the project board's** ([`BoardLayout`]), so Bridge draws both with one grid. A block's `kind`
//! and `view` are text kept as written, known or not. Fleet stores the layout as the JSON text of
//! `bridge.homeLayout`, and the text reads back here because `serde_json::from_*` may appear only in `ipc` and `store`.

use serde::{Deserialize, Serialize};

use crate::BoardLayout;

/// `get_home_layout` and `save_home_layout`'s answer: the layout he keeps. **`layout` is absent where he saved
/// none**, and Bridge draws its preset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<BoardLayout>,
}

/// `save_home_layout`: the layout to keep, whole. **Absent forgets it**, so Bridge draws the preset again.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveHomeLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<BoardLayout>,
}

/// The most blocks one layout may hold, so a runaway save cannot fill settings.json.
pub const MOST_BLOCKS: usize = 64;

/// Every problem in `layout`, or empty where Fleet may keep it. Only what makes a block unspeakable is refused:
/// whether a kind or a view exists is Bridge's registry.
pub fn layout_problems(layout: &BoardLayout) -> Vec<String> {
    let mut found = Vec::new();
    if layout.blocks.len() > MOST_BLOCKS {
        found.push(format!("more than {MOST_BLOCKS} blocks"));
    }
    for (at, block) in layout.blocks.iter().enumerate() {
        if block.kind.trim().is_empty() || block.view.trim().is_empty() {
            found.push(format!("block {} names its kind and its view, and one of them is blank", at + 1));
        }
    }
    found
}

/// Every problem in `text`, the text of a layout, or empty where Fleet may read it.
pub fn problems(text: &str) -> Vec<String> {
    match serde_json::from_str::<BoardLayout>(text) {
        Ok(layout) => layout_problems(&layout),
        Err(why) => vec![format!("not a layout of blocks: {why}")],
    }
}

/// The text Fleet keeps for `layout`.
pub fn text(layout: &BoardLayout) -> String {
    serde_json::to_string(layout).expect("blocks of text and numbers are JSON")
}

/// The layout `text` holds, or none where it is empty or is not one: a half-written value is never half drawn.
pub fn read(text: &str) -> Option<BoardLayout> {
    serde_json::from_str(text).ok()
}

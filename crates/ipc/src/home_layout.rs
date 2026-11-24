//! The Cockpit home's layout: the one grid of blocks Bridge draws when no Job is selected, kept by Fleet per user.
//! `docs/practices/protocol/projects.md`, *The Cockpit home's layout*.
//!
//! **The wire shape is the project board's** ([`BoardLayout`]), so Bridge draws both with one grid. A block's `kind`
//! and `view` are text kept as written, known or not. Fleet stores the layout as the JSON text of
//! `bridge.homeLayout`, and the text reads back here because `serde_json::from_*` may appear only in `ipc` and `store`.
//!
//! **It follows him across his Macs, the latest save winning.** A save is stamped with when it was made, and the
//! stamp is kept with the layout in `bridge.homeLayout`. A forgotten layout keeps its stamp too, so a later
//! forgetting beats an earlier layout on another Mac. A layout with no stamp, written by hand, loses to any stamped one.

use serde::{Deserialize, Serialize};

use crate::{BoardLayout, Instant};

/// `get_home_layout` and `save_home_layout`'s answer: the layout he keeps. **`layout` is absent where he saved
/// none**, and Bridge draws its preset. `saved_at` is when the save that made it was made, on whichever Mac he made
/// it; it is absent where no save did, and Bridge need not read it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<BoardLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_at: Option<Instant>,
}

impl HomeLayout {
    /// Whether this one replaces `mine` on a Mac that holds `mine`: the later save wins, a stamped one beats an
    /// unstamped one, and two saves of one instant are settled by their text, so both Macs pick the same.
    pub fn supersedes(&self, mine: &HomeLayout) -> bool {
        match (&self.saved_at, &mine.saved_at) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(theirs), Some(ours)) => theirs > ours || (theirs == ours && text_of(self) > text_of(mine)),
        }
    }
}

/// `share_home_layout`'s counterpart: what Mac `mac` keeps, handed in by this Mac's Phone Gateway.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacHomeLayout {
    pub mac: String,
    pub home: HomeLayout,
}

/// `home_layout.changed`: a save of the layout was made on this Mac. A linked Mac reads `share_home_layout` again.
/// Carries nothing, and is not published for a layout taken from another Mac, so two Macs do not tell each other forever.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeLayoutChanged {}

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

/// What `bridge.homeLayout` holds: a stamped layout, or the bare layout a hand wrote.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Kept {
    Stamped {
        saved_at: Instant,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layout: Option<BoardLayout>,
    },
    Bare(BoardLayout),
}

/// Every problem in `text`, the text of a layout, or empty where Fleet may read it.
pub fn problems(text: &str) -> Vec<String> {
    match serde_json::from_str::<Kept>(text) {
        Ok(Kept::Stamped { layout: Some(layout), .. }) | Ok(Kept::Bare(layout)) => layout_problems(&layout),
        Ok(Kept::Stamped { layout: None, .. }) => Vec::new(),
        Err(why) => vec![format!("not a layout of blocks: {why}")],
    }
}

/// The text Fleet keeps for `home`, or none where there is nothing to keep: no layout and no stamp.
pub fn text_of(home: &HomeLayout) -> Option<String> {
    match (&home.saved_at, &home.layout) {
        (None, None) => None,
        (None, Some(layout)) => Some(serde_json::to_string(layout).expect(PLAIN)),
        (Some(saved_at), layout) => {
            let kept = Kept::Stamped { saved_at: saved_at.clone(), layout: layout.clone() };
            Some(serde_json::to_string(&kept).expect(PLAIN))
        }
    }
}

const PLAIN: &str = "blocks of text and numbers are JSON";

/// The layout `text` holds, with its stamp, or none where it is empty or is not one: a half-written value is never
/// half drawn.
pub fn read(text: &str) -> HomeLayout {
    match serde_json::from_str::<Kept>(text) {
        Ok(Kept::Stamped { saved_at, layout }) => HomeLayout { layout, saved_at: Some(saved_at) },
        Ok(Kept::Bare(layout)) => HomeLayout { layout: Some(layout), saved_at: None },
        Err(_) => HomeLayout::default(),
    }
}

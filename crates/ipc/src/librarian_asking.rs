//! Asking the librarian: his words from one box, and the one thing the librarian did with them.
//! `docs/concepts/feature-catalog.md`, *Asking the librarian*.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::project_board::BoardLayout;

/// `ask_librarian`: his words to a project's librarian, from the screen he asked on. **Only on his press.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskLibrarian {
    /// A project's id.
    pub project: String,
    /// Where he asked from: `dashboard`, `features_empty`, `feature:<slug>` or `metric:<id>`. A word Fleet does not know
    /// reads as `dashboard`.
    pub context: String,
    pub words: String,
    /// Paths and links he dropped or pasted into the box, as he gave them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// The board as it is drawn, preset or kept, so a board ask starts from what he sees. Absent off the dashboard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board: Option<AskedBoard>,
}

/// The board on screen when he asked, and what this Bridge can draw on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskedBoard {
    pub layout: BoardLayout,
    /// Whether `layout` is the preset, so Undo of a board change forgets the kept one rather than keeping the preset.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub preset: bool,
    /// Each block kind this Bridge draws, with the views it draws it in: what an added or changed block may be.
    pub kinds: Vec<AskedBlockKind>,
    /// The grid's width in columns.
    pub columns: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskedBlockKind {
    pub kind: String,
    pub views: Vec<String>,
}

/// What the librarian did with one ask. **At most one act, already done**: Undo takes it back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianAnswered {
    pub project: String,
    /// The ask, by the instant Fleet took it: what Undo and Move name.
    pub asked_at: Instant,
    /// One line: what it did, or the answer.
    pub reply: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<LibrarianAct>,
    /// The model calls asks spent tonight for this project, this one included, and the night's limit.
    pub calls: u32,
    pub limit: u32,
}

/// The one thing an ask did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LibrarianAct {
    /// The board was kept as `layout`.
    Board { layout: BoardLayout },
    /// A doc or link was filed under a feature, as a read link.
    FiledDoc {
        /// The path or url, as it is kept.
        target: String,
        title: String,
        /// The feature's slug and title.
        feature: String,
        feature_title: String,
    },
    /// The might-lists were ranked by the librarian: `list_ranked_issues` answers them in its order, each with a why.
    Ranked,
}

/// `undo_librarian_ask`: the act of one ask is taken back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoLibrarianAsk {
    pub project: String,
    pub asked_at: Instant,
}

/// `refile_librarian_ask`: the doc one ask filed moves to another feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefileLibrarianAsk {
    pub project: String,
    pub asked_at: Instant,
    /// The feature's slug.
    pub feature: String,
}

/// What Undo did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianAskUndone {
    pub project: String,
    pub asked_at: Instant,
    /// One line of what was taken back.
    pub reply: String,
}

/// `rank_with_librarian` and `forget_librarian_ranking`: a project's might-lists, by the librarian or back to features.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankWithLibrarian {
    pub project: String,
}

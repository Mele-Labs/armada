//! The board a project starts with, by what kind of project it is. The kind is read from the manifest kinds Fleet's code map
//! already names (`MapRoom::kind`: `npm`, `cargo`, `go`), so no second reading is made. The table is the whole of it.

use super::layout::{BlockKind::*, Layout, Placed, View};

/// What the code map says a project is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A package.json somewhere: servers to run and pages to open come first.
    Web,
    /// A compiled workspace with no web side: the Checks come first.
    Compiled,
    /// Nothing read yet, or nothing known.
    Unknown,
}

/// The kind of project whose code map holds rooms of these manifest kinds.
pub fn kind_of<'a>(kinds: impl IntoIterator<Item = &'a str>) -> Kind {
    let mut compiled = false;
    for kind in kinds {
        match kind {
            "npm" => return Kind::Web,
            "cargo" | "go" | "gradle" | "maven" | "dotnet" | "swift" => compiled = true,
            _ => {}
        }
    }
    if compiled { Kind::Compiled } else { Kind::Unknown }
}

/// The board for a kind of project: Run and Needs you first for a web project, Checks first for a compiled one.
pub fn preset(kind: Kind) -> Layout {
    let blocks = match kind {
        Kind::Web | Kind::Unknown => vec![
            Placed::new(Run, 0, 0, 3, 2, View::List),
            Placed::new(Needs, 3, 0, 3, 2, View::List),
            Placed::new(Reviews, 0, 2, 3, 2, View::Cards),
            Placed::new(Issues, 3, 2, 3, 2, View::Table),
            Placed::new(Features, 0, 4, 3, 2, View::Cards),
            Placed::new(Checks, 3, 4, 3, 2, View::List),
        ],
        Kind::Compiled => vec![
            Placed::new(Checks, 0, 0, 3, 2, View::List),
            Placed::new(Needs, 3, 0, 3, 2, View::List),
            Placed::new(Reviews, 0, 2, 3, 2, View::Cards),
            Placed::new(Issues, 3, 2, 3, 2, View::Table),
            Placed::new(Features, 0, 4, 3, 2, View::Cards),
            Placed::new(Run, 3, 4, 3, 2, View::List),
        ],
    };
    let mut layout = Layout { blocks };
    layout.settle(None);
    layout
}

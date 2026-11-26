//! What detection keeps of one repository, apart from what goes on the wire: the parts with the modules behind them, their
//! pieces, and the edges the code names between pieces.

use ipc::{SystemLink, SystemPart};

/// One part of level one, and what detection knows of it that the wire does not carry.
#[derive(Clone, Debug)]
pub(super) struct Part {
    pub(super) wire: SystemPart,
    /// The modules of the repository's layout that are this part's own.
    pub(super) modules: Vec<usize>,
    /// The library modules attributed to it: those its own modules reach.
    pub(super) libs: Vec<usize>,
    /// It draws a screen a person looks at.
    pub(super) screen: bool,
    pub(super) port: Option<u16>,
    /// An HTTP server the code only describes: others run it.
    pub(super) api: bool,
    /// What kind of server or app it is, in a word: `native app`, `FastAPI`.
    pub(super) what: String,
}

/// One piece of a part.
#[derive(Clone, Debug)]
pub(super) struct Piece {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) part: usize,
    pub(super) paths: Vec<String>,
    pub(super) summary: String,
}

/// What an edge reaches: a piece, or a whole part where the code names no piece (a store, something outside).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Node {
    Piece(usize),
    Part(usize),
}

/// A piece reaching something, where the code says so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Edge {
    pub(super) from: usize,
    pub(super) to: Node,
    pub(super) label: Option<String>,
}

/// A repository read: its parts, their pieces and the edges between them.
#[derive(Debug, Default)]
pub(super) struct Mapped {
    pub(super) manifest: String,
    /// The folders it was read by, empty where it was read whole.
    pub(super) folders: Vec<String>,
    pub(super) parts: Vec<Part>,
    pub(super) pieces: Vec<Piece>,
    pub(super) edges: Vec<Edge>,
    /// Level one's links between parts, people left out.
    pub(super) links: Vec<SystemLink>,
    /// Every tracked file and its size.
    pub(super) files: Vec<(String, u64)>,
    /// Whether Kit declares a server for each part, by part index: what to watch it by.
    pub(super) kit: Vec<(usize, super::repo::KitServer)>,
    /// The routes the code serves, as declared, and the part each is in.
    pub(super) routes: Vec<(usize, String)>,
}

impl Mapped {
    pub(super) fn part_at(&self, id: &str) -> Option<usize> {
        self.parts.iter().position(|part| part.wire.id == id)
    }

    /// The part a repository-relative path is in: the one holding it by the longest of its paths.
    pub(super) fn part_of(&self, path: &str) -> Option<usize> {
        self.parts
            .iter()
            .enumerate()
            .filter_map(|(at, part)| part.wire.paths.iter().filter(|prefix| holds(prefix, path)).map(|prefix| prefix.trim_matches('/').len()).max().map(|len| (len, at)))
            .max_by_key(|(len, _)| *len)
            .map(|(_, at)| at)
    }

    /// The piece a path is in: the one holding it by the longest of its paths.
    pub(super) fn piece_of(&self, path: &str) -> Option<usize> {
        self.pieces
            .iter()
            .enumerate()
            .filter_map(|(at, piece)| piece.paths.iter().filter(|prefix| holds(prefix, path)).map(|prefix| prefix.trim_matches('/').len()).max().map(|len| (len, at)))
            .max_by_key(|(len, _)| *len)
            .map(|(_, at)| at)
    }
}

/// Whether `prefix`, a folder with or without its `/` or a file, holds `path`. The repository's own root, `./`, holds all.
pub(super) fn holds(prefix: &str, path: &str) -> bool {
    let prefix = prefix.trim_matches('/');
    prefix.is_empty() || prefix == "." || crate::feature_library::mapping::owns(prefix, path)
}

/// `paths` kept to `folders`: a path inside one stays, and a path that holds one is that folder. All of them where there are
/// no folders.
pub(super) fn within(folders: &[String], paths: &[String]) -> Vec<String> {
    if folders.is_empty() {
        return paths.to_vec();
    }
    let mut out: Vec<String> = Vec::new();
    for path in paths {
        let bare = path.trim_matches('/');
        let kept: Vec<String> = if super::repo::inside(folders, bare) {
            vec![path.clone()]
        } else {
            folders.iter().filter(|folder| holds(path, folder)).map(|folder| format!("{folder}/")).collect()
        };
        out.extend(kept.into_iter().filter(|one| !out.contains(one)).collect::<Vec<_>>());
    }
    out
}

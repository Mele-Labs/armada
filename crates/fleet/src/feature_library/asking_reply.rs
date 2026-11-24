//! What an ask of the librarian puts to the model, and what Fleet reads back. `docs/concepts/feature-catalog.md`.
//!
//! **Lines, never JSON.** The model answers an `ACT` line, a `REPLY` line and the act's own lines, and anything else it
//! says is left unread. A reply that follows none of it is read as an answer.

use ipc::{AskedBoard, BoardBlock};

/// The longest answer kept, in characters.
pub(super) const MOST_REPLY: usize = 200;
/// The most features the prompt names.
const MOST_FEATURES: usize = 80;

/// The screen he asked from, read from `context`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Screen {
    Dashboard,
    FeaturesEmpty,
    /// A feature's page: the feature a filed doc goes under unless his words say another.
    Feature(String),
    Metric(String),
}

impl Screen {
    /// A word Fleet does not know reads as the dashboard.
    pub(super) fn of(context: &str) -> Screen {
        let context = context.trim();
        if context == "features_empty" {
            return Screen::FeaturesEmpty;
        }
        if let Some(slug) = context.strip_prefix("feature:").map(str::trim).filter(|slug| !slug.is_empty()) {
            return Screen::Feature(slug.to_string());
        }
        if let Some(id) = context.strip_prefix("metric:").map(str::trim).filter(|id| !id.is_empty()) {
            return Screen::Metric(id.to_string());
        }
        Screen::Dashboard
    }

    /// The acts an ask from here may do. The empty Features page and a metric answer only, for now.
    pub(super) fn acts(&self) -> &'static [&'static str] {
        match self {
            Screen::Dashboard => &["board", "file", "rank", "answer"],
            Screen::Feature(_) => &["file", "answer"],
            Screen::FeaturesEmpty | Screen::Metric(_) => &["answer"],
        }
    }

    fn named(&self) -> String {
        match self {
            Screen::Dashboard => "the project's dashboard".to_string(),
            Screen::FeaturesEmpty => "the project's Features page, which holds no feature yet".to_string(),
            Screen::Feature(slug) => format!("the page of the feature `{slug}`"),
            Screen::Metric(id) => format!("the metric `{id}` on the dashboard"),
        }
    }
}

/// A feature as the prompt names it.
pub(super) struct Named {
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) area: String,
}

/// What the model is asked.
pub(super) fn question(screen: &Screen, words: &str, files: &[String], features: &[Named], board: Option<&AskedBoard>) -> String {
    let acts = screen.acts();
    let mut text = format!(
        "You are the librarian of a software project's feature catalog. Its owner typed into the Ask box on {}. \
         Do at most one thing for him, from these: {}.\n\n",
        screen.named(),
        acts.join(", ")
    );
    text.push_str("Acts:\n");
    if acts.contains(&"board") {
        text.push_str("- board: change the dashboard board. Give the whole new board, every block kept or changed.\n");
    }
    if acts.contains(&"file") {
        text.push_str("- file: file one of his files or links under the feature it is about.\n");
    }
    if acts.contains(&"rank") {
        text.push_str("- rank: rank the issues he might pick up and the pull requests he might be interested in.\n");
    }
    text.push_str("- answer: answer him and do nothing else.\n\n");
    text.push_str(
        "Answer in these lines and nothing else:\n\
         ACT <one act>\n\
         REPLY <one line, at most 200 characters: what you did, or the answer. Facts only, no dashes, no greeting>\n",
    );
    if acts.contains(&"board") {
        text.push_str("For board, one line a block: BLOCK <kind> <x> <y> <w> <h> <view>\n");
    }
    if acts.contains(&"file") {
        text.push_str("For file, one line: FILE <path or link> | <title> | <feature slug>\n");
    }
    text.push_str(&format!("\nHis words: {}\n", flat(words)));
    if !files.is_empty() {
        text.push_str(&format!("His files and links: {}\n", files.iter().map(|one| one.trim()).collect::<Vec<_>>().join(", ")));
    }
    if let Screen::Feature(slug) = screen {
        text.push_str(&format!("A file goes under `{slug}` unless his words name another feature.\n"));
    }
    if !features.is_empty() {
        text.push_str("\nFeatures (slug | title | area):\n");
        for one in features.iter().take(MOST_FEATURES) {
            text.push_str(&format!("{} | {} | {}\n", one.slug, one.title, one.area));
        }
    }
    if let (Some(board), true) = (board, acts.contains(&"board")) {
        text.push_str(&format!("\nThe board is {} columns wide. Blocks may not overlap, and w and h are at least 1.\n", board.columns));
        text.push_str("Kinds and the views each may use:\n");
        for kind in &board.kinds {
            text.push_str(&format!("{}: {}\n", kind.kind, kind.views.join(", ")));
        }
        text.push_str("The board now:\n");
        for block in &board.layout.blocks {
            text.push_str(&format!("BLOCK {} {} {} {} {} {}\n", block.kind, block.x, block.y, block.w, block.h, block.view));
        }
    }
    text
}

/// What the model said it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Said {
    /// A board, or `None` where a block line did not read.
    Board(Option<Vec<BoardBlock>>),
    File { target: String, title: String, feature: String },
    Rank,
    Answer,
}

/// The model's reply, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Reading {
    pub(super) said: Said,
    /// Its one line, or empty where it gave none.
    pub(super) reply: String,
}

/// The reply as Fleet reads it. An act the screen does not allow, or none named, reads as an answer.
pub(super) fn read(reply: &str, screen: &Screen) -> Reading {
    let mut act = None;
    let mut line = None;
    let mut blocks: Vec<Option<BoardBlock>> = Vec::new();
    let mut file = None;
    for raw in reply.lines() {
        let raw = raw.trim().trim_start_matches(['-', '*']).trim();
        let (head, rest) = raw.split_once(char::is_whitespace).map_or((raw, ""), |(head, rest)| (head, rest.trim()));
        match head.to_ascii_uppercase().as_str() {
            "ACT" if act.is_none() => act = Some(rest.to_ascii_lowercase()),
            "REPLY" if line.is_none() => line = Some(rest.to_string()),
            "BLOCK" => blocks.push(block_of(rest)),
            "FILE" if file.is_none() => file = file_of(rest),
            _ => {}
        }
    }
    let allowed = |name: &str| screen.acts().contains(&name);
    let said = match act.as_deref() {
        Some("board") if allowed("board") => Said::Board((!blocks.is_empty()).then(|| blocks.into_iter().collect::<Option<Vec<_>>>()).flatten()),
        Some("file") if allowed("file") => file.map_or(Said::Answer, |(target, title, feature)| Said::File { target, title, feature }),
        Some("rank") if allowed("rank") => Said::Rank,
        _ => Said::Answer,
    };
    let reply = match (line, act.is_some()) {
        (Some(line), _) => one_line(&line, MOST_REPLY),
        // No lines at all: the whole of it is the answer.
        (None, false) => one_line(reply, MOST_REPLY),
        (None, true) => String::new(),
    };
    Reading { said, reply }
}

fn block_of(rest: &str) -> Option<BoardBlock> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let [kind, x, y, w, h, view] = parts.as_slice() else { return None };
    Some(BoardBlock { kind: kind.to_string(), x: x.parse().ok()?, y: y.parse().ok()?, w: w.parse().ok()?, h: h.parse().ok()?, view: view.to_string() })
}

fn file_of(rest: &str) -> Option<(String, String, String)> {
    let mut parts = rest.split('|').map(str::trim);
    let target = parts.next().filter(|one| !one.is_empty())?.to_string();
    let title = parts.next().unwrap_or("").to_string();
    let feature = parts.next().unwrap_or("").trim_matches('`').to_string();
    Some((target, title, feature))
}

/// Whether `blocks` may be kept on `board`: only its kinds and their views, inside its columns, none over another, none
/// smaller than one cell.
pub(super) fn fits(blocks: &[BoardBlock], board: &AskedBoard) -> bool {
    let known = |block: &BoardBlock| board.kinds.iter().any(|kind| kind.kind == block.kind && kind.views.iter().any(|view| *view == block.view));
    let inside = |block: &BoardBlock| block.w >= 1 && block.h >= 1 && u16::from(block.x) + u16::from(block.w) <= u16::from(board.columns);
    let over = |a: &BoardBlock, b: &BoardBlock| {
        let (ax, ay, aw, ah) = (u16::from(a.x), u16::from(a.y), u16::from(a.w), u16::from(a.h));
        let (bx, by, bw, bh) = (u16::from(b.x), u16::from(b.y), u16::from(b.w), u16::from(b.h));
        ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
    };
    !blocks.is_empty()
        && blocks.iter().all(|block| known(block) && inside(block))
        && blocks.iter().enumerate().all(|(at, a)| blocks[at + 1..].iter().all(|b| !over(a, b)))
}

/// `text` on one line, whitespace folded, quotes off the ends, dashes made commas, at most `most` characters.
pub(super) fn one_line(text: &str, most: usize) -> String {
    let flat = flat(text);
    let flat = flat.trim_matches(|c: char| c == '"' || c == '\'').replace(" - ", ", ").replace(['\u{2014}', '\u{2013}'], ", ").replace(" ,", ",").replace(",,", ",");
    let flat = flat.trim().to_string();
    if flat.chars().count() <= most {
        return flat;
    }
    let cut: String = flat.chars().take(most).collect();
    // At a word, where there is one to cut at.
    cut.rsplit_once(' ').map_or(cut.clone(), |(kept, _)| kept.trim_end_matches([',', ';', ':']).to_string())
}

fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

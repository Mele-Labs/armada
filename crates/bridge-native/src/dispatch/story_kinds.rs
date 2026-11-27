//! The kinds of Job a sentence starts, read from `crates/core-model/domain/story_kinds.toml` as built in, and the
//! sentence each one writes. A kind's `sentence` is cut into text and blanks once, here; the sheet keeps only what a
//! hand typed into each blank.
//!
//! No TOML parser, as `ipc::ask_tools` keeps none: one `[kinds.<id>]` table a kind, one quoted line a field.

use std::sync::OnceLock;

/// One piece of a sentence after its opening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Words that stand as written.
    Text(&'static str),
    /// A blank a hand types into, named by what it asks for.
    Blank(&'static str),
    /// A blank filled from the project's feature catalog, with the one more row some kinds add at the top of the list.
    Feature(Option<&'static str>),
    /// A choice between two words.
    Choice(&'static str, &'static str),
}

/// One kind of Job a sentence starts.
#[derive(Clone, Debug)]
pub struct StoryKind {
    pub id: &'static str,
    pub name: &'static str,
    /// The words a person types, which lead the sentence.
    pub opening: &'static str,
    pub pieces: Vec<Piece>,
    /// The workflow it starts on, `none` for a talk Job. A start sends it as the Job's workflow, which stands over Fleet's guess.
    pub workflow: &'static str,
    pub starts: &'static str,
    /// What the line says where the choice reads its second word.
    pub later: Option<&'static str>,
    pub hue: &'static str,
    pub glyph: &'static str,
}

/// Every kind, in the order the file lists them, which is the order the openings are offered.
pub fn kinds() -> &'static [StoryKind] {
    static KINDS: OnceLock<Vec<StoryKind>> = OnceLock::new();
    KINDS.get_or_init(|| read(include_str!("../../../core-model/domain/story_kinds.toml")))
}

/// The sentence's pieces: text between braces is a blank, and what comes before and after is text.
fn pieces_of(sentence: &'static str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut rest = sentence;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|at| open + at) else { break };
        if open > 0 {
            pieces.push(Piece::Text(&rest[..open]));
        }
        let inside = &rest[open + 1..close];
        pieces.push(match (inside.strip_prefix('@'), inside.strip_prefix('=')) {
            (Some(feature), _) => Piece::Feature(feature.split_once('|').map(|(_, extra)| extra)),
            (_, Some(choice)) => match choice.split_once('|') {
                Some((first, second)) => Piece::Choice(first, second),
                None => Piece::Blank(choice),
            },
            _ => Piece::Blank(inside),
        });
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest));
    }
    pieces
}

fn read(registry: &'static str) -> Vec<StoryKind> {
    let mut tables: Vec<(&'static str, Vec<(&'static str, &'static str)>)> = Vec::new();
    for line in registry.lines().map(str::trim) {
        if let Some(id) = line.strip_prefix("[kinds.").and_then(|rest| rest.strip_suffix(']')) {
            tables.push((id, Vec::new()));
        } else if let (Some((_, fields)), Some((key, value))) = (tables.last_mut(), line.split_once('=')) {
            if !line.starts_with('#') {
                fields.push((key.trim(), value.trim().trim_matches('"')));
            }
        }
    }
    tables
        .into_iter()
        .map(|(id, fields)| {
            let get = |key: &str| fields.iter().find_map(|(named, value)| (*named == key).then_some(*value));
            let field = |key: &str| get(key).unwrap_or_else(|| panic!("story_kinds.toml: `{id}` has no `{key}`"));
            StoryKind {
                id,
                name: field("name"),
                opening: field("opening"),
                pieces: pieces_of(field("sentence")),
                workflow: field("workflow"),
                starts: field("starts"),
                later: get("later"),
                hue: field("hue"),
                glyph: field("glyph"),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_reads_six_kinds_in_order_each_with_an_opening_and_blanks() {
        let ids: Vec<_> = kinds().iter().map(|kind| kind.id).collect();
        assert_eq!(ids, ["bug", "feature", "idea", "design", "investigate", "brainstorm"]);
        for kind in kinds() {
            assert!(!kind.opening.is_empty(), "{} has an opening", kind.id);
            assert!(kind.pieces.iter().any(|piece| !matches!(piece, Piece::Text(_))), "{} has a blank to type into", kind.id);
        }
    }
}

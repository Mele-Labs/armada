//! A diff's code coloured by what each piece is: keyword, string, number, comment, function, type or
//! punctuation. syntect's grammars name the pieces; only the name is read, never a syntect theme, and
//! each kind takes an Armada token, so the colours follow the theme. A file's language is its
//! extension's; an unknown one stays plain.
//!
//! Each hunk is read on its own, off the main thread: the new side (context and added lines) and the
//! old side (context and removed lines) each as one run, so a string or comment that spans lines
//! carries over within its side.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use crate::diff::{DiffLine, LineKind};
use crate::look::{token, Token};

/// What a piece of code is, as far as its colour goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hue {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
    Punctuation,
}

impl Hue {
    /// The token each kind is drawn in. Armada has no syntax tokens yet, so these borrow the sketch
    /// hues, which alias the themed status hues and say nothing about a Job; red is left out, as it
    /// reads as a removed line.
    pub fn token(self) -> Token {
        match self {
            Hue::Keyword => token::SKETCH_ORANGE,
            Hue::String => token::SKETCH_GREEN,
            Hue::Number => token::SKETCH_AMBER,
            Hue::Comment => token::FG_SUBTLE,
            Hue::Function => token::SKETCH_BLUE,
            Hue::Type => token::SKETCH_TEAL,
            Hue::Punctuation => token::FG_MUTED,
        }
    }
}

/// One line's coloured pieces, as byte ranges of its code: the line without its sign.
pub type Pieces = Arc<[(Range<usize>, Hue)]>;

/// Each line's pieces by its text, sign and all, for one file: what a hunk, a file and a draft's
/// excerpt all look their lines up in.
pub type Read = HashMap<String, Pieces>;

/// The grammars, loaded once on first use. Loading takes a moment, so it only ever happens off the
/// main thread.
fn grammars() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn grammar_of(path: &str) -> Option<&'static SyntaxReference> {
    let (_, extension) = path.rsplit_once('.')?;
    grammars().find_syntax_by_extension(extension)
}

/// A file's lines read hunk by hunk, by each line's text. Empty where the language is unknown. Off
/// the main thread only.
pub fn read(path: &str, lines: &[DiffLine]) -> Read {
    let mut read = Read::new();
    let Some(grammar) = grammar_of(path) else { return read };
    for hunk in lines.split(|line| line.kind == LineKind::Hunk) {
        for side in [LineKind::Added, LineKind::Removed] {
            let mut state = ParseState::new(grammar);
            let mut stack = ScopeStack::new();
            for line in hunk.iter().filter(|line| line.kind == side || line.kind == LineKind::Context) {
                let code = line.text.get(1..).unwrap_or_default();
                let pieces = pieces_of(&mut state, &mut stack, code);
                // Context lines are read on both sides; the new side's reading is the one kept.
                if (line.kind == side || side == LineKind::Added) && !pieces.is_empty() {
                    read.entry(line.text.clone()).or_insert_with(|| pieces.into());
                }
            }
        }
    }
    read
}

/// One line's pieces, carrying the grammar's state on to the next line.
fn pieces_of(state: &mut ParseState, stack: &mut ScopeStack, code: &str) -> Vec<(Range<usize>, Hue)> {
    let line = format!("{code}\n");
    let Ok(ops) = state.parse_line(&line, grammars()) else { return Vec::new() };
    let mut pieces: Vec<(Range<usize>, Hue)> = Vec::new();
    let mut at = 0;
    let push = |range: Range<usize>, hue: Option<Hue>, pieces: &mut Vec<(Range<usize>, Hue)>| {
        let Some(hue) = hue else { return };
        if range.is_empty() {
            return;
        }
        match pieces.last_mut() {
            Some((last, was)) if *was == hue && last.end == range.start => last.end = range.end,
            _ => pieces.push((range, hue)),
        }
    };
    for (end, op) in ops {
        let end = end.min(code.len());
        if end > at {
            push(at..end, hue_of(stack), &mut pieces);
            at = end;
        }
        let _ = stack.apply(&op);
    }
    if at < code.len() {
        push(at..code.len(), hue_of(stack), &mut pieces);
    }
    pieces
}

/// The kind a piece is, by its scopes: inside a comment or a string it is that, whatever else it is;
/// otherwise the innermost scope that names a kind.
fn hue_of(stack: &ScopeStack) -> Option<Hue> {
    let names: Vec<String> = stack.as_slice().iter().map(|scope| scope.build_string()).collect();
    if names.iter().any(|name| name.starts_with("comment")) {
        return Some(Hue::Comment);
    }
    if names.iter().any(|name| name.starts_with("string") || name.starts_with("constant.character")) {
        return Some(Hue::String);
    }
    names.iter().rev().find_map(|name| kind_of(name))
}

fn kind_of(name: &str) -> Option<Hue> {
    let is = |prefix: &str| name.starts_with(prefix);
    if is("constant.numeric") || is("constant.language") {
        Some(Hue::Number)
    } else if is("keyword.operator") || is("punctuation") {
        Some(Hue::Punctuation)
    } else if is("keyword") || is("storage") {
        Some(Hue::Keyword)
    } else if is("entity.name.function") || is("support.function") || is("variable.function") {
        Some(Hue::Function)
    } else if is("entity.name") || is("support.type") || is("support.class") || is("entity.other.inherited-class") {
        Some(Hue::Type)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: LineKind, text: &str) -> DiffLine {
        DiffLine { kind, text: text.into(), collides: None }
    }

    #[test]
    fn a_rust_hunk_is_read_by_kind_and_an_unknown_file_stays_plain() {
        let lines = vec![
            line(LineKind::Hunk, "@@ -1,2 +1,2 @@"),
            line(LineKind::Context, " fn wait() -> u32 {"),
            line(LineKind::Removed, "-    500 // half a second"),
            line(LineKind::Added, "+    \"later\""),
        ];
        let rust = read("src/retry.rs", &lines);
        let hues = |text: &str| rust[text].iter().map(|(_, hue)| *hue).collect::<Vec<_>>();
        assert!(hues(" fn wait() -> u32 {").contains(&Hue::Keyword));
        assert!(hues(" fn wait() -> u32 {").contains(&Hue::Function));
        assert_eq!(hues("-    500 // half a second"), vec![Hue::Number, Hue::Comment]);
        assert_eq!(hues("+    \"later\""), vec![Hue::String]);
        assert!(read("notes.unknownext", &lines).is_empty());
    }
}

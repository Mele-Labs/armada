//! The authored words a Judge brief is written in, and the pieces a caller may
//! replace.
//!
//! **This crate knows nothing of settings.** Each [`Piece`] carries the text
//! Armada ships, beside the code that assembles it; a caller that owns an
//! override hands a [`Wording`] in, and one that does not uses
//! [`Wording::shipped`]. Material — a diff, a file, a request — is never a
//! piece: it is appended by the code, after the words that say how to read it.

use std::collections::BTreeMap;

/// One authored piece of a brief: its name and the words that ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub id: &'static str,
    pub shipped: &'static str,
}

/// The pieces a caller replaced, every other one as it ships.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wording(BTreeMap<&'static str, String>);

impl Wording {
    /// Every piece as Armada ships it.
    pub fn shipped() -> Wording {
        Wording::default()
    }

    /// The same wording with `piece` in `text`.
    pub fn with(mut self, piece: Piece, text: impl Into<String>) -> Wording {
        self.0.insert(piece.id, text.into());
        self
    }

    /// The words of `piece` in force.
    pub fn get(&self, piece: Piece) -> &str {
        self.0.get(piece.id).map_or(piece.shipped, String::as_str)
    }

    /// The words of `piece` with its placeholders filled. See [`fill`].
    pub fn fill(&self, piece: Piece, values: &[(&str, &str)]) -> String {
        fill(self.get(piece), values)
    }
}

/// `template` with each `{name}` in `values` replaced by its value, in one
/// pass: what fills one placeholder is never read again for another, so
/// material that happens to spell `{record}` arrives as written. A brace that
/// opens no placeholder in `values` is left alone, which is every JSON example
/// a prompt shows.
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut filled = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        filled.push_str(&rest[..open]);
        let from = &rest[open + 1..];
        let named = values.iter().find(|(name, _)| {
            from.strip_prefix(name)
                .is_some_and(|after| after.starts_with('}'))
        });
        match named {
            Some((name, value)) => {
                filled.push_str(value);
                rest = &from[name.len() + 1..];
            }
            None => {
                filled.push('{');
                rest = from;
            }
        }
    }
    filled.push_str(rest);
    filled
}

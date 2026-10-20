//! The uncased WordPiece tokenizer the pinned model was trained with, read from its `vocab.txt`.
//!
//! **Written here rather than taken from a tokenizer crate**: the one that reads `tokenizer.json`
//! brings a regex engine, a C build and a JSON parse for what is a hundred lines. Accents are kept
//! rather than stripped, which only changes the pieces of an accented word; the head was trained on
//! these same pieces, so what it reads and what it learned from agree.

use std::collections::HashMap;

/// Words longer than this are one unknown piece, as the original tokenizer has it.
const LONGEST_WORD: usize = 100;

pub(crate) struct WordPiece {
    ids: HashMap<String, u32>,
    unknown: u32,
    pub(crate) start: u32,
    pub(crate) end: u32,
}

impl WordPiece {
    /// One token a line, its id the line's number from nought.
    pub(crate) fn from_vocab(vocab: &str) -> Option<WordPiece> {
        let ids: HashMap<String, u32> = vocab.lines().enumerate().map(|(at, token)| (token.to_string(), at as u32)).collect();
        Some(WordPiece { unknown: *ids.get("[UNK]")?, start: *ids.get("[CLS]")?, end: *ids.get("[SEP]")?, ids })
    }

    /// The ids of `text`'s pieces, without the start and end marks.
    pub(crate) fn pieces(&self, text: &str) -> Vec<u32> {
        let mut out = Vec::new();
        for word in words(text) {
            self.word(&word, &mut out);
        }
        out
    }

    /// Longest match first; a word with any part not in the vocabulary is one unknown piece.
    fn word(&self, word: &str, out: &mut Vec<u32>) {
        let chars: Vec<char> = word.chars().collect();
        if chars.len() > LONGEST_WORD {
            out.push(self.unknown);
            return;
        }
        let mut pieces = Vec::new();
        let mut start = 0;
        while start < chars.len() {
            let mut end = chars.len();
            let found = loop {
                if end == start {
                    break None;
                }
                let piece: String = chars[start..end].iter().collect();
                let piece = if start > 0 { format!("##{piece}") } else { piece };
                if let Some(id) = self.ids.get(&piece) {
                    break Some(*id);
                }
                end -= 1;
            };
            match found {
                Some(id) => pieces.push(id),
                None => {
                    out.push(self.unknown);
                    return;
                }
            }
            start = end;
        }
        out.extend(pieces);
    }
}

/// Lower-cased words, with every punctuation mark and every CJK character a word of its own.
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    for c in text.chars() {
        if c == '\0' || c == '\u{FFFD}' || (c.is_control() && !c.is_whitespace()) {
            continue;
        }
        if c.is_whitespace() {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
        } else if !c.is_alphanumeric() || is_cjk(c) {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            out.push(c.to_lowercase().collect());
        } else {
            word.extend(c.to_lowercase());
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF | 0x2A700..=0x2B73F | 0x2B740..=0x2B81F | 0x2B820..=0x2CEAF | 0xF900..=0xFAFF | 0x2F800..=0x2FA1F)
}

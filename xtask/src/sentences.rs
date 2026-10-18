//! One sentence per line in the docs, as `verify-docs` holds them.
//!
//! An agent greps and reads by line, and a paragraph on one line comes back
//! whole for every hit in it. Each doc whose prose this would break is an
//! output of `verify-docs`, so a stale one fails and `--write` rewraps it.
//! Markdown joins the lines of a paragraph, so what renders does not change.

use std::fs;
use std::path::Path;

use crate::files_with_ext;

/// A prose line longer than this is broken at its sentence ends.
const LONG: usize = 200;

/// Generated from the docs, so it is left to the walk that writes it.
const GENERATED: &str = "docs/OPEN.md";

/// Words whose full stop ends no sentence.
const ABBREVIATIONS: &[&str] = &["e.g", "i.e", "vs", "etc", "cf", "No", "Fig", "Mr", "Ms", "Dr", "St", "approx"];

/// Every doc whose prose is not yet one sentence per line, as (path, rewrapped).
pub fn outputs(root: &Path) -> Vec<(String, String)> {
    files_with_ext(root, &root.join("docs"), &["md"])
        .into_iter()
        .filter(|rel| rel != GENERATED)
        .filter_map(|rel| {
            let have = fs::read_to_string(root.join(&rel)).ok()?;
            let want = rewrap(&have);
            (want != have).then_some((rel, want))
        })
        .collect()
}

/// The text with each long prose line broken at its sentence ends. Fences,
/// tables, headings, HTML and indented blocks are left as they are.
pub fn rewrap(text: &str) -> String {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in text.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            out.push(line.to_string());
        } else if fenced {
            out.push(line.to_string());
        } else {
            out.extend(split(line));
        }
    }
    out.join("\n")
}

fn split(line: &str) -> Vec<String> {
    if line.chars().count() <= LONG {
        return vec![line.to_string()];
    }
    let (lead, body, indent) = if let Some(n) = list_marker(line) {
        (&line[..n], &line[n..], " ".repeat(n))
    } else if let Some(body) = line.strip_prefix("> ") {
        ("> ", body, "> ".to_string())
    } else if line.starts_with(char::is_whitespace) || ["|", "#", "<", "---"].iter().any(|p| line.starts_with(p)) {
        return vec![line.to_string()];
    } else {
        ("", line, String::new())
    };
    let cuts = breaks(body);
    if cuts.is_empty() {
        return vec![line.to_string()];
    }
    let mut parts = Vec::new();
    let mut start = 0;
    for cut in cuts.into_iter().chain([body.len()]) {
        let part = body[start..cut].trim();
        if !part.is_empty() {
            parts.push(part);
        }
        start = cut;
    }
    parts
        .iter()
        .enumerate()
        .map(|(at, part)| if at == 0 { format!("{lead}{part}") } else { format!("{indent}{part}") })
        .collect()
}

/// The byte length of a list item's marker and the spaces after it.
fn list_marker(line: &str) -> Option<usize> {
    let rest = line.trim_start();
    let pad = line.len() - rest.len();
    let marker = if rest.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        let after = rest[digits..].chars().next()?;
        if digits == 0 || !(after == '.' || after == ')') {
            return None;
        }
        digits + 1
    };
    let spaces = rest[marker..].chars().take_while(|c| *c == ' ').count();
    (spaces > 0).then_some(pad + marker + spaces)
}

/// Byte offsets just after a sentence end, outside a code span, a link and
/// emphasis, where the line may break.
fn breaks(text: &str) -> Vec<usize> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let (mut tick, mut bracket, mut paren, mut star, mut under) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        let run = |mark: char| chars[i..].iter().take_while(|(_, x)| *x == mark).count();
        if c == '`' {
            let n = run('`');
            tick = if tick == n { 0 } else if tick == 0 { n } else { tick };
            i += n;
            continue;
        }
        if tick > 0 {
            i += 1;
            continue;
        }
        match c {
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '*' => {
                star += 1;
                i += run('*');
                continue;
            }
            '_' => {
                let before = i.checked_sub(1).map(|j| chars[j].1.is_alphanumeric()).unwrap_or(false);
                let after = chars.get(i + 1).map(|(_, x)| x.is_alphanumeric()).unwrap_or(false);
                if !before || !after {
                    under += 1;
                }
            }
            _ => {}
        }
        let next = chars.get(i + 2).map(|(_, x)| *x);
        if matches!(c, '.' | '?' | '!')
            && chars.get(i + 1).map(|(_, x)| *x) == Some(' ')
            && next.is_some_and(|x| x.is_uppercase() || "`*\"'(".contains(x))
            && bracket == 0
            && paren == 0
            && star % 2 == 0
            && under % 2 == 0
            && !abbreviated(&text[..at + 1])
        {
            out.push(at + 1);
        }
        i += 1;
    }
    out
}

/// Whether the text ends on a stop that ends no sentence: an abbreviation, an
/// initial or a numbered item.
fn abbreviated(text: &str) -> bool {
    let word = text[..text.len() - 1].rsplit(|c: char| !(c.is_alphanumeric() || c == '.')).next().unwrap_or("");
    ABBREVIATIONS.contains(&word)
        || (word.chars().count() == 1 && word.chars().all(|c| c.is_ascii_uppercase()))
        || word.chars().last().is_some_and(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests;

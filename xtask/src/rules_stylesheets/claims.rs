//! One class name belongs to one composition.
//!
//! `index.css` concatenates every stylesheet into one sheet, so two
//! compositions declaring `.armada-decision` is one rule repainting the other,
//! decided by import order and visible from neither file (#1678). The review
//! gate drew the Job-members card's padding, border and ground that way until
//! somebody said the screen looked flat.
//!
//! **A claim is a selector that styles the class on its own**: no combinator,
//! one class, any pseudo or attribute — `.x`, `.x:hover`, `.x[data-y]`. A
//! composition styling another's class inside its own (`.mine .theirs`) or in
//! combination (`.theirs.mine`) is scoped by its own name and claims nothing.
//! The key is the class rather than the whole selector, because `.x` in one
//! file and `.x:hover` in another collide just the same.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::{strip_comments, ROOT};
use crate::{files_with_ext, Report};

/// The compositions. Each directory directly under it is one composition, and
/// every stylesheet in that directory is that composition's.
const COMPOSITIONS: &str = "compositions";

/// One rule that claims a class: the stylesheet, and the line its selector
/// starts on.
#[derive(Debug, Clone)]
struct Site {
    composition: String,
    path: String,
    line: usize,
}

pub fn no_two_compositions_claim_one_class(root: &Path) -> Report {
    let mut report = Report::new("no two compositions declare one class");
    let dir = root.join(ROOT).join(COMPOSITIONS);
    let prefix = format!("{ROOT}/{COMPOSITIONS}/");

    let mut claims: BTreeMap<String, Vec<Site>> = BTreeMap::new();
    for path in files_with_ext(root, &dir, &["css"]) {
        let Some(composition) = path
            .strip_prefix(&prefix)
            .and_then(|rest| rest.split_once('/'))
            .map(|(name, _)| name.to_string())
        else {
            continue;
        };
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        for (class, line) in claimed(&strip_comments(&text)) {
            let sites = claims.entry(class).or_default();
            if sites.iter().all(|s| s.composition != composition) {
                sites.push(Site {
                    composition: composition.clone(),
                    path: path.clone(),
                    line,
                });
            }
        }
    }

    for (class, sites) in &claims {
        if sites.len() < 2 {
            continue;
        }
        let named: Vec<String> = sites
            .iter()
            .map(|s| format!("{}:{}", s.path, s.line))
            .collect();
        report.fail(format!(
            "`.{class}` is declared by {} compositions — {}. index.css joins them \
             into one sheet, so whichever it imports later repaints the other's \
             elements. One keeps the name; the other takes its own",
            sites.len(),
            named.join(" and ")
        ));
    }
    report
}

/// Every class one stylesheet claims, with the line the claiming selector
/// starts on. A rule nested inside another style rule is scoped by its parent
/// and claims nothing; an at-rule's block (`@media`, `@supports`) is not a
/// style rule and its rules are read as written at the top.
fn claimed(code: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    // For each open brace, whether it opened a style rule.
    let mut open: Vec<bool> = Vec::new();
    let mut start = 0;
    for (at, c) in code.char_indices() {
        match c {
            '{' => {
                let prelude = &code[start..at];
                let selector = prelude.trim();
                let style = !selector.starts_with('@');
                if style && !open.iter().any(|s| *s) {
                    let lead = prelude.len() - prelude.trim_start().len();
                    let line = code[..start + lead].matches('\n').count() + 1;
                    for part in split_list(selector) {
                        if let Some(class) = sole_class(part) {
                            found.push((class, line));
                        }
                    }
                }
                open.push(style);
                start = at + 1;
            }
            '}' => {
                open.pop();
                start = at + 1;
            }
            ';' => start = at + 1,
            _ => {}
        }
    }
    found
}

/// A selector list's selectors, split at the commas outside parentheses.
fn split_list(selector: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut from = 0;
    for (at, c) in selector.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&selector[from..at]);
                from = at + 1;
            }
            _ => {}
        }
    }
    parts.push(&selector[from..]);
    parts
}

/// The class a selector styles on its own: no combinator and exactly one
/// class outside any `:not(…)` or `[…]`.
fn sole_class(selector: &str) -> Option<String> {
    let mut outer = String::new();
    let mut depth = 0usize;
    for c in selector.trim().chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => outer.push(c),
            _ => {}
        }
    }
    if outer.contains(|c: char| c.is_whitespace() || matches!(c, '>' | '+' | '~')) {
        return None;
    }
    let mut classes = outer.split('.').skip(1).map(|rest| {
        rest.chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
    });
    // A keyframe's `12.5%` splits on its point too, and no class starts with
    // a digit.
    let class = classes
        .next()
        .filter(|c| c.starts_with(|c: char| !c.is_ascii_digit()))?;
    classes.next().is_none().then_some(class)
}

#[cfg(test)]
mod tests;

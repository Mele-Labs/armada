//! The one CSS shape a theme is written in: rule blocks of `--token: value;` declarations.
//!
//! Not a CSS parser. `packages/tokens/src/light.css`, the generated catalogue files and a mod's
//! `theme.css` are all `[data-theme="<id>"] { ... }` blocks, and Fleet has already checked a mod's
//! sheet before this reads it (`docs/concepts/mods.md`). What is read here is those blocks and
//! nothing else: a rule it does not understand is skipped, never guessed at.

use std::collections::BTreeMap;

/// A rule block: which themes it is for, and its declarations in the order written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub selector: Selector,
    pub declarations: Vec<(String, String)>,
}

/// Whose block it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    /// `[data-theme="id"]`, possibly one of several in a list.
    Themes(Vec<String>),
    /// `:root`, `[data-theme]` or `:root[data-theme]`: the theme in force, whichever it is.
    Bare,
    /// Anything else. Never applied.
    Other,
}

impl Block {
    /// Whether this block is part of the theme `id`.
    pub fn applies_to(&self, id: &str) -> bool {
        match &self.selector {
            Selector::Themes(ids) => ids.iter().any(|one| one == id),
            Selector::Bare => true,
            Selector::Other => false,
        }
    }

    /// The value of one declaration, the last written if it appears twice, as CSS does.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.declarations.iter().rev().find(|(one, _)| one == name).map(|(_, value)| value.as_str())
    }
}

/// Every block in `css`, without comments. An `@` rule is skipped whole, braces and all.
pub fn blocks(css: &str) -> Vec<Block> {
    let text = without_comments(css);
    let mut found = Vec::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find('{') {
        let head = rest[..open].trim();
        let Some(close) = matching_brace(&rest[open..]) else { break };
        let body = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        if head.starts_with('@') {
            continue;
        }
        found.push(Block { selector: selector_of(head), declarations: declarations_of(body) });
    }
    found
}

/// The declarations of every block that applies to `id`, laid over each other in the order written.
/// A later declaration wins over an earlier one at equal specificity, which is how a mod's sheet
/// adopted after the tokens wins in the Electron Bridge.
pub fn layer(css: &str, id: &str) -> BTreeMap<String, String> {
    let mut merged = BTreeMap::new();
    for block in blocks(css).into_iter().filter(|block| block.applies_to(id)) {
        for (name, value) in block.declarations {
            merged.insert(name, value);
        }
    }
    merged
}

/// The ids a sheet names in its `[data-theme="id"]` selectors, in the order they appear.
pub fn theme_ids(css: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for block in blocks(css) {
        if let Selector::Themes(named) = block.selector {
            for id in named {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
    ids
}

fn without_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// The offset of the brace closing the one at the start of `text`.
fn matching_brace(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (at, ch) in text.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

fn selector_of(head: &str) -> Selector {
    let mut ids = Vec::new();
    let mut bare = false;
    for part in head.split(',').map(str::trim) {
        let part = part.strip_prefix("html").unwrap_or(part);
        if matches!(part, ":root" | "[data-theme]" | ":root[data-theme]") {
            bare = true;
            continue;
        }
        let named = part
            .strip_prefix("[data-theme=")
            .and_then(|tail| tail.strip_suffix(']'))
            .map(|quoted| quoted.trim_matches(|ch| ch == '"' || ch == '\''));
        match named {
            Some(id) if !id.is_empty() => ids.push(id.to_string()),
            _ => return Selector::Other,
        }
    }
    match (ids.is_empty(), bare) {
        (false, false) => Selector::Themes(ids),
        (true, true) => Selector::Bare,
        // A list of a bare selector and a named one is the named theme's and every other's alike,
        // which is the bare block.
        (false, true) => Selector::Bare,
        (true, false) => Selector::Other,
    }
}

/// `name: value` pairs split on the semicolons outside parentheses. A pair without a colon is dropped.
fn declarations_of(body: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let mut push = |piece: &str| {
        if let Some((name, value)) = piece.split_once(':') {
            let (name, value) = (name.trim(), value.trim());
            if !name.is_empty() && !value.is_empty() {
                found.push((name.to_string(), value.to_string()));
            }
        }
    };
    for (at, ch) in body.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ';' if depth <= 0 => {
                push(&body[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    push(&body[start..]);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUSK: &str = r#"
        /* A mod's theme.css. */
        [data-theme="dusk"] {
          --bg-base: #1A1822;
          --bg-glass: rgb(35 31 46 / 0.9);
          --accent: #C1B4EC;
        }
    "#;

    #[test]
    fn a_block_is_read_with_its_declarations_in_order() {
        let found = blocks(DUSK);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].selector, Selector::Themes(vec!["dusk".into()]));
        assert_eq!(found[0].declarations[1], ("--bg-glass".to_string(), "rgb(35 31 46 / 0.9)".to_string()));
    }

    #[test]
    fn a_comment_holding_a_brace_does_not_open_a_block() {
        let found = blocks("/* [data-theme=\"x\"] { --a: 1; } */ [data-theme=\"y\"] { --b: 2; }");
        assert_eq!(theme_ids("/* [data-theme=\"x\"] { --a: 1; } */ [data-theme=\"y\"] { --b: 2; }"), vec!["y"]);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_semicolon_inside_parentheses_does_not_end_a_declaration() {
        let found = blocks("[data-theme=\"a\"] { --u: url(data:x;y); --v: 1px }");
        assert_eq!(found[0].declarations.len(), 2);
        assert_eq!(found[0].get("--v"), Some("1px"));
    }

    #[test]
    fn only_the_blocks_of_the_theme_apply_and_a_bare_one_applies_to_all() {
        let css = "[data-theme=\"a\"] { --x: 1; } [data-theme=\"b\"] { --x: 2; } :root { --y: 3; } body { --z: 4; }";
        let for_a = layer(css, "a");
        assert_eq!(for_a.get("--x").map(String::as_str), Some("1"));
        assert_eq!(for_a.get("--y").map(String::as_str), Some("3"));
        assert_eq!(for_a.get("--z"), None);
    }

    #[test]
    fn a_later_declaration_wins() {
        let css = "[data-theme=\"a\"] { --x: 1; } [data-theme=\"a\"] { --x: 2; }";
        assert_eq!(layer(css, "a").get("--x").map(String::as_str), Some("2"));
    }

    #[test]
    fn an_at_rule_is_skipped_whole() {
        let css = "@media (min-width: 1px) { [data-theme=\"a\"] { --x: 1; } } [data-theme=\"a\"] { --y: 2; }";
        let merged = layer(css, "a");
        assert_eq!(merged.get("--x"), None);
        assert_eq!(merged.get("--y").map(String::as_str), Some("2"));
    }
}

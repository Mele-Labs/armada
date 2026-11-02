//! The design tokens a style file sets, by name and value, so an agent can write rules from them
//! where nothing compiles the file, and so the walk can rank a file by how many it sets.

use ipc::{StyleKind, StyleToken};

use super::sheet::strip_comments;

/// A value is cut here: a token is a figure or a colour, and a long one is a shadow or a font stack.
const VALUE_CHARS: usize = 80;

/// The tokens of `text`, first to last, each name once.
pub(crate) fn tokens_in(kind: StyleKind, text: &str) -> Vec<StyleToken> {
    let bare = strip_comments(text);
    let mut found: Vec<StyleToken> = match kind {
        StyleKind::Scss => declared(&bare, '$', false),
        StyleKind::Less => declared(&bare, '@', false),
        StyleKind::TailwindConfig | StyleKind::ThemeModule => keyed(&bare),
        _ => declared(&bare, '-', true),
    };
    let mut names = std::collections::BTreeSet::new();
    found.retain(|token| names.insert(token.name.clone()));
    found
}

/// How many tokens set a colour or a size, which is what a search ranks a file by.
pub(crate) fn visual_count(tokens: &[StyleToken]) -> usize {
    tokens.iter().filter(|token| looks_visual(&token.value)).count()
}

fn looks_visual(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    let colour = value.starts_with('#')
        || ["rgb", "hsl", "oklch", "oklab", "lab(", "lch(", "color-mix", "color("].iter().any(|start| value.starts_with(start));
    let size = value.split_whitespace().next().is_some_and(|first| {
        let digits = first.trim_start_matches('-').trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
        first.len() > digits.len() && ["px", "rem", "em", "%", "vh", "vw", "pt"].contains(&digits)
    });
    colour || size
}

/// `--name: value;` for a CSS file (`sigil` `-`, a double dash), `$name: value;` and `@name: value;`
/// for a preprocessor's.
fn declared(text: &str, sigil: char, double: bool) -> Vec<StyleToken> {
    let mut found = Vec::new();
    for (at, ch) in text.char_indices() {
        if ch != sigil || (double && !text[at..].starts_with("--")) {
            continue;
        }
        let before = text[..at].chars().next_back();
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            continue;
        }
        let start = if double { at + 2 } else { at + 1 };
        let name: String = text[start..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        let after = text[start + name.len()..].trim_start();
        let Some(value) = after.strip_prefix(':') else { continue };
        let end = value.find([';', '}']).unwrap_or(value.len());
        let value = value[..end].trim();
        if value.is_empty() {
            continue;
        }
        let name = if double { format!("--{name}") } else { format!("{sigil}{name}") };
        found.push(StyleToken { name, value: value.chars().take(VALUE_CHARS).collect() });
    }
    found
}

/// `key: 'value'` pairs of a config or a theme module whose value is a colour or a size.
fn keyed(text: &str) -> Vec<StyleToken> {
    let mut found = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let key = key.trim().trim_matches(|c| c == '"' || c == '\'' || c == '`');
        let value = value.trim().trim_end_matches(',').trim().trim_matches(|c| c == '"' || c == '\'' || c == '`');
        let named = !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if named && looks_visual(value) {
            found.push(StyleToken { name: key.to_string(), value: value.chars().take(VALUE_CHARS).collect() });
        }
    }
    found
}

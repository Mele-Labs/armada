//! A repository's tokens by role and by theme. A custom property is a token where it is set on the
//! document (`:root`, `html`, `[data-theme]`, `@theme`) or on a theme (`[data-theme="light"]`,
//! `.dark`, `@media (prefers-color-scheme: dark)`); one set inside a component's rule is not.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use ipc::{CatalogueToken, StyleKind, TokenGroup, TokenRole};

use super::super::sheet::strip_comments;
use super::super::tokens::tokens_in;

/// The theme a document-wide rule sets.
pub(crate) const DEFAULT_THEME: &str = "default";
/// A value is cut here, after `var()` is resolved.
const VALUE_CHARS: usize = 120;
/// How deep a `var()` chain is followed.
const DEPTH_AT_MOST: usize = 8;
/// Most bytes of a sheet read for tokens.
const READ_AT_MOST: u64 = 512 * 1024;

/// One declaration of a token, where it was made.
#[derive(Clone, Debug)]
struct Declared {
    theme: String,
    name: String,
    value: String,
    /// Inside a Tailwind `@theme` block, which mostly re-exports the tokens under other names.
    in_theme_block: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Read {
    pub themes: Vec<String>,
    /// The sheets that set at least one token.
    pub sheets: Vec<String>,
    pub groups: Vec<TokenGroup>,
}

pub(crate) fn read(root: &Path, sheets: &[String]) -> Read {
    let mut declared: Vec<Declared> = Vec::new();
    let mut setting = Vec::new();
    for path in sheets {
        let file = root.join(path);
        if std::fs::metadata(&file).map_or(true, |meta| meta.len() > READ_AT_MOST) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        let before = declared.len();
        match path.rsplit('.').next() {
            Some("scss") | Some("less") => {
                let kind = if path.ends_with(".scss") { StyleKind::Scss } else { StyleKind::Less };
                declared.extend(tokens_in(kind, &text).into_iter().map(|token| Declared {
                    theme: DEFAULT_THEME.to_string(),
                    name: token.name,
                    value: token.value,
                    in_theme_block: false,
                }));
            }
            _ => scan(&strip_comments(&text), &Scope::default(), &mut declared),
        }
        if declared.len() > before {
            setting.push(path.clone());
        }
    }
    grouped(declared, setting)
}

#[derive(Clone, Default)]
struct Scope {
    /// From an enclosing `@media (prefers-color-scheme: …)`.
    scheme: Option<String>,
    in_theme_block: bool,
}

/// Every token `css` declares, walking its blocks.
fn scan(css: &str, scope: &Scope, out: &mut Vec<Declared>) {
    let mut at = 0;
    while at < css.len() {
        let rest = &css[at..];
        let Some(open) = next_open(rest) else { return };
        let prelude = rest[..open].rsplit([';', '}']).next().unwrap_or_default().trim();
        let close = matching_close(rest, open);
        let body = &rest[open + 1..close.min(rest.len())];
        at += (close + 1).min(rest.len());
        if let Some(rule) = prelude.strip_prefix('@') {
            let word = rule.split(|c: char| c.is_whitespace() || c == '(').next().unwrap_or_default();
            match word {
                "media" => {
                    let scheme = scheme_of(rule).or_else(|| scope.scheme.clone());
                    scan(body, &Scope { scheme, ..scope.clone() }, out);
                }
                "theme" => scan_declarations(body, DEFAULT_THEME, true, out),
                "layer" | "supports" | "container" | "scope" => scan(body, scope, out),
                _ => {}
            }
            continue;
        }
        if let Some(theme) = theme_of(prelude, scope.scheme.as_deref()) {
            scan_declarations(body, &theme, scope.in_theme_block, out);
        }
    }
}

/// The offset of the next `{` outside a string.
fn next_open(text: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (at, ch) in text.char_indices() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '{') => return Some(at),
            _ => {}
        }
    }
    None
}

/// The offset of the `}` closing the `{` at `open`, or the end of `text`.
fn matching_close(text: &str, open: usize) -> usize {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for (at, ch) in text[open..].char_indices() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '{') => depth += 1,
            (None, '}') => {
                depth -= 1;
                if depth == 0 {
                    return open + at;
                }
            }
            _ => {}
        }
    }
    text.len()
}

fn scheme_of(rule: &str) -> Option<String> {
    let at = rule.find("prefers-color-scheme")?;
    let after = rule[at + "prefers-color-scheme".len()..].trim_start().strip_prefix(':')?.trim_start();
    let word: String = after.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    (!word.is_empty()).then_some(word)
}

/// The theme a selector sets tokens for, or `None` where it is a component's rule.
fn theme_of(selector: &str, scheme: Option<&str>) -> Option<String> {
    let document = scheme.unwrap_or(DEFAULT_THEME).to_string();
    for part in selector.split(',').map(str::trim) {
        if let Some(at) = part.find("data-theme=") {
            let value: String = part[at + "data-theme=".len()..]
                .trim_start_matches(['"', '\''])
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !value.is_empty() {
                return Some(value);
            }
        }
        let bare = part.trim_start_matches(":root").trim_start_matches("html").trim_start_matches(":host");
        if let Some(class) = bare.strip_prefix('.').filter(|class| matches!(*class, "dark" | "light")) {
            return Some(class.to_string());
        }
        if matches!(part, ":root" | "html" | ":host" | "[data-theme]" | "*" | ":root, [data-theme]") {
            return Some(document);
        }
    }
    None
}

/// The `--name: value` declarations at the top of a rule's body, nested rules left out.
fn scan_declarations(body: &str, theme: &str, in_theme_block: bool, out: &mut Vec<Declared>) {
    let mut depth = 0usize;
    let mut start = 0;
    let mut flat = String::new();
    for (at, ch) in body.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    flat.push_str(&body[start..at]);
                    // What came before the nested block was its selector, not a declaration.
                    if let Some(cut) = flat.rfind(';') {
                        flat.truncate(cut + 1);
                    } else {
                        flat.clear();
                    }
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    start = at + 1;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        flat.push_str(&body[start.min(body.len())..]);
    }
    for statement in flat.split(';') {
        let Some((name, value)) = statement.split_once(':') else { continue };
        let name = name.trim();
        let value = value.trim();
        let named = name.len() > 2 && name.starts_with("--") && name[2..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !named || value.is_empty() || value == "initial" {
            continue;
        }
        out.push(Declared { theme: theme.to_string(), name: name.to_string(), value: value.to_string(), in_theme_block });
    }
}

/// The declarations as groups, each token once with its default value and any theme's that differs.
fn grouped(declared: Vec<Declared>, sheets: Vec<String>) -> Read {
    // A `@theme` block re-exports tokens under the names Tailwind reads: those that only point at
    // another token, and those a sheet outside the block already sets, are left out.
    let outside: std::collections::HashSet<&str> = declared.iter().filter(|one| !one.in_theme_block).map(|one| one.name.as_str()).collect();
    let kept: Vec<&Declared> = declared
        .iter()
        .filter(|one| !one.in_theme_block || (!outside.contains(one.name.as_str()) && !is_bare_alias(&one.value)))
        .collect();
    let mut themes: Vec<String> = vec![DEFAULT_THEME.to_string()];
    let mut by_theme: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
    let mut order: Vec<&str> = Vec::new();
    for one in &kept {
        if !themes.contains(&one.theme) {
            themes.push(one.theme.clone());
        }
        let values = by_theme.entry(one.theme.as_str()).or_default();
        if !values.contains_key(one.name.as_str()) {
            values.insert(one.name.as_str(), one.value.as_str());
        }
        if !order.contains(&one.name.as_str()) {
            order.push(one.name.as_str());
        }
    }
    let empty = HashMap::new();
    let defaults = by_theme.get(DEFAULT_THEME).unwrap_or(&empty);
    let mut groups: BTreeMap<(TokenRole, String), Vec<CatalogueToken>> = BTreeMap::new();
    for name in order {
        let Some(raw) = defaults.get(name).copied().or_else(|| themes.iter().find_map(|theme| by_theme.get(theme.as_str())?.get(name).copied())) else {
            continue;
        };
        let value = resolved(raw, DEFAULT_THEME, &by_theme, 0);
        let mut token = CatalogueToken {
            name: name.to_string(),
            value: cut(&value),
            alias: raw.contains("var(").then(|| cut(raw)),
            themes: BTreeMap::new(),
        };
        for theme in themes.iter().filter(|theme| theme.as_str() != DEFAULT_THEME) {
            let theirs = cut(&resolved(raw_in(name, theme, &by_theme).unwrap_or(raw), theme, &by_theme, 0));
            if theirs != token.value {
                token.themes.insert(theme.clone(), theirs);
            }
        }
        let role = role_of(name, &value);
        groups.entry((role, family_of(name))).or_default().push(token);
    }
    let groups = groups.into_iter().map(|((role, family), tokens)| TokenGroup { role, family, tokens }).collect();
    Read { themes, sheets, groups }
}

fn is_bare_alias(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("var(") && value.ends_with(')') && value.matches("var(").count() == 1 && !value.contains(',')
}

fn raw_in<'a>(name: &str, theme: &str, by_theme: &'a HashMap<&str, HashMap<&str, &str>>) -> Option<&'a str> {
    by_theme.get(theme).and_then(|values| values.get(name).copied())
}

/// `value` with each `var(--x[, fallback])` replaced by what `--x` is in `theme`, else in the
/// default theme, else the fallback; left as written where none is known.
fn resolved(value: &str, theme: &str, by_theme: &HashMap<&str, HashMap<&str, &str>>, depth: usize) -> String {
    if depth >= DEPTH_AT_MOST || !value.contains("var(") {
        return value.to_string();
    }
    let mut out = String::new();
    let mut rest = value;
    while let Some(at) = rest.find("var(") {
        out.push_str(&rest[..at]);
        let inner_start = at + 4;
        let mut depth_here = 1;
        let mut end = rest.len();
        for (offset, ch) in rest[inner_start..].char_indices() {
            match ch {
                '(' => depth_here += 1,
                ')' => {
                    depth_here -= 1;
                    if depth_here == 0 {
                        end = inner_start + offset;
                        break;
                    }
                }
                _ => {}
            }
        }
        let inner = &rest[inner_start..end.min(rest.len())];
        let (name, fallback) = match inner.split_once(',') {
            Some((name, fallback)) => (name.trim(), Some(fallback.trim())),
            None => (inner.trim(), None),
        };
        let found = raw_in(name, theme, by_theme).or_else(|| raw_in(name, DEFAULT_THEME, by_theme)).or(fallback);
        match found {
            Some(found) => out.push_str(&resolved(found, theme, by_theme, depth + 1)),
            None => out.push_str(&rest[at..(end + 1).min(rest.len())]),
        }
        rest = &rest[(end + 1).min(rest.len())..];
    }
    out.push_str(rest);
    out
}

fn cut(value: &str) -> String {
    value.chars().take(VALUE_CHARS).collect()
}

fn looks_colour(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with('#')
        || ["rgb", "hsl", "hwb", "oklch", "oklab", "lab(", "lch(", "color-mix", "color("].iter().any(|start| value.starts_with(start))
        || matches!(value.as_str(), "transparent" | "currentcolor" | "white" | "black")
}

fn looks_size(value: &str) -> bool {
    value.split_whitespace().next().is_some_and(|first| {
        let unit = first.trim_start_matches('-').trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
        first.len() > unit.len() && ["px", "rem", "em", "%", "vh", "vw", "pt", "ch", ""].contains(&unit)
    })
}

/// What a token is for: by its value where that says, else by the words in its name.
pub(crate) fn role_of(name: &str, value: &str) -> TokenRole {
    let name = name.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| name.contains(word));
    if has(&["shadow", "elevation"]) {
        TokenRole::Shadow
    } else if has(&["radius", "rounded", "corner"]) {
        TokenRole::Radius
    } else if looks_colour(value) {
        TokenRole::Colour
    } else if has(&["font", "text", "leading", "line-height", "tracking", "letter", "weight"]) {
        TokenRole::Type
    } else if has(&["ease", "duration", "delay", "motion", "transition", "animation"]) {
        TokenRole::Motion
    } else if has(&["space", "spacing", "gap", "pad", "margin", "inset", "size", "width", "height", "gutter"]) || looks_size(value) {
        TokenRole::Space
    } else {
        TokenRole::Other
    }
}

/// The family a token's name puts it in: its first word, or its first two where the first is only
/// "color".
pub(crate) fn family_of(name: &str) -> String {
    let sigil: String = name.chars().take_while(|c| matches!(c, '-' | '$' | '@')).collect();
    let words: Vec<&str> = name[sigil.len()..].split(['-', '_']).filter(|word| !word.is_empty()).collect();
    let take = match words.first() {
        Some(&"color" | &"colour" | &"c") if words.len() > 2 => 2,
        _ => 1,
    };
    format!("{sigil}{}", words.iter().take(take).copied().collect::<Vec<_>>().join("-"))
}

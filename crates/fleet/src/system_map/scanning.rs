//! Text evidence in source files: the routes a server declares, the paths a client names, the words an import line holds.
//! **Scanners, not parsers**: a shape they do not follow is a link not drawn, never a wrong one drawn.

use std::collections::BTreeSet;

use super::repo::extension_of;

/// Every string literal in `text` that closes on its line, at most 200 bytes, as it is written between its quotes.
pub(super) fn literals(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let quote = bytes[at];
        if !matches!(quote, b'"' | b'\'' | b'`') {
            at += 1;
            continue;
        }
        let start = at + 1;
        let mut end = start;
        while end < bytes.len() && end - start <= 200 && bytes[end] != quote && bytes[end] != b'\n' {
            end += if bytes[end] == b'\\' { 2 } else { 1 };
        }
        if end < bytes.len() && bytes[end] == quote && text.is_char_boundary(start) && text.is_char_boundary(end) {
            out.push(&text[start..end]);
            at = end + 1;
        } else {
            at += 1;
        }
    }
    out
}

/// A route as servers and clients compare it: no query, a placeholder segment as `*`, no trailing `/`.
pub(super) fn normal(route: &str) -> String {
    let path = route.split(['?', '#']).next().unwrap_or(route);
    let segments: Vec<String> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| if segment.starts_with([':', '<', '*']) || segment.contains(['{', '}', '$']) { "*".to_string() } else { segment.to_string() })
        .collect();
    format!("/{}", segments.join("/"))
}

/// The path a literal names, where it names one: `/api/review`, `http://localhost:8731/api/orders/ingest`, `${API}/api/x`.
pub(super) fn path_of(literal: &str) -> Option<&str> {
    let rest = if let Some(after) = literal.strip_prefix("http://").or_else(|| literal.strip_prefix("https://")) {
        &after[after.find('/')?..]
    } else if literal.starts_with("${") || literal.starts_with('{') {
        &literal[literal.find('}')? + 1..]
    } else {
        literal
    };
    (rest.starts_with('/') && rest.len() > 1 && rest.chars().skip(1).any(char::is_alphabetic) && !rest.contains(' ') && !rest.starts_with("//")).then_some(rest)
}

/// Each route the file declares, as written: Python decorators, `.method("/path"` calls in JavaScript and `.route("/path"` in Rust.
pub(super) fn declared(path: &str, text: &str) -> Vec<String> {
    const METHODS: &[&str] = &["get", "post", "put", "delete", "patch", "route", "api_route", "all", "use", "websocket"];
    let mut out: Vec<String> = Vec::new();
    let mut take = |after: &str| {
        let after = after.trim_start();
        if let Some(quote) = after.chars().next().filter(|c| matches!(c, '"' | '\'' | '`')) {
            let body = &after[1..];
            if let Some(end) = body.find(quote) {
                let route = &body[..end];
                if route.starts_with('/') && route.len() > 1 {
                    out.push(route.to_string());
                }
            }
        }
    };
    match extension_of(path) {
        "rs" => {
            for (at, found) in text.match_indices(".route(") {
                take(&text[at + found.len()..]);
            }
        }
        "py" => {
            for line in text.lines().map(str::trim).filter(|line| line.starts_with('@')) {
                for method in METHODS {
                    if let Some(at) = line.find(&format!(".{method}(")) {
                        take(&line[at + method.len() + 2..]);
                    }
                }
            }
        }
        "js" | "mjs" | "ts" | "jsx" | "tsx" => {
            for method in METHODS {
                let needle = format!(".{method}(");
                for (at, found) in text.match_indices(&needle) {
                    take(&text[at + found.len()..]);
                }
            }
        }
        _ => {}
    }
    out
}

/// The words of every import-like line of `text`, once each.
pub(super) fn imported_words(text: &str) -> BTreeSet<&str> {
    const STARTS: &[&str] = &["import ", "from ", "use ", "mod ", "pub mod ", "pub use ", "pub(crate) use ", "pub(crate) mod ", "pub(super) use ", "export "];
    let mut words = BTreeSet::new();
    for line in text.lines().map(str::trim_start) {
        if STARTS.iter().any(|start| line.starts_with(start)) || line.contains("require(") || line.contains(" from \"") || line.contains(" from '") {
            words.extend(line.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|word| word.len() >= 3));
        }
    }
    words
}

/// Whether `text` names `ident` as a whole word and then reaches into it: `ident::`, `ident.`, an import of it.
pub(super) fn refers(text: &str, ident: &str) -> bool {
    text.match_indices(ident).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + ident.len()..].chars().next();
        before.is_none_or(|c| !(c.is_alphanumeric() || matches!(c, '_' | ':'))) && after.is_some_and(|c| matches!(c, ':' | '.' | '"' | '\'' | ' ' | '\n' | ','))
    })
}

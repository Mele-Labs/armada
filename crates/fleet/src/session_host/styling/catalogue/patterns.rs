//! The house patterns a repository keeps in `.armada/catalogue/patterns.html`: one
//! `<template data-pattern="name" data-note="…">` for each, its body the HTML a mock copies. Written
//! by hand and changed like any file of the repository, so every Job and every machine reads the same.

use std::path::Path;

use ipc::CataloguePattern;

pub(crate) fn read(file: &Path) -> Vec<CataloguePattern> {
    std::fs::read_to_string(file).map(|text| patterns_in(&text)).unwrap_or_default()
}

pub(crate) fn patterns_in(text: &str) -> Vec<CataloguePattern> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("<template") {
        let after = &rest[at..];
        let Some(open_end) = after.find('>') else { break };
        let opening = &after[..open_end];
        let body_start = open_end + 1;
        let Some(close) = after[body_start..].find("</template>") else { break };
        let body = &after[body_start..body_start + close];
        rest = &after[body_start + close + "</template>".len()..];
        let Some(name) = attribute(opening, "data-pattern").filter(|name| !name.is_empty()) else { continue };
        let html = dedented(body);
        if html.is_empty() {
            continue;
        }
        found.push(CataloguePattern { name, note: attribute(opening, "data-note").filter(|note| !note.is_empty()), html });
    }
    found
}

fn attribute(opening: &str, key: &str) -> Option<String> {
    let at = opening.find(&format!("{key}="))? + key.len() + 1;
    let value = &opening[at..];
    let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let end = value[1..].find(quote)?;
    Some(value[1..1 + end].trim().to_string())
}

/// `body` with its blank edges and common indent taken off.
fn dedented(body: &str) -> String {
    let lines: Vec<&str> = body.lines().skip_while(|line| line.trim().is_empty()).collect();
    let lines: Vec<&str> = {
        let mut lines = lines;
        while lines.last().is_some_and(|line| line.trim().is_empty()) {
            lines.pop();
        }
        lines
    };
    let indent = lines.iter().filter(|line| !line.trim().is_empty()).map(|line| line.len() - line.trim_start().len()).min().unwrap_or(0);
    lines.iter().map(|line| line.get(indent..).unwrap_or(line.trim_start())).collect::<Vec<_>>().join("\n")
}

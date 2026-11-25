//! The vocabulary and the acts the native Bridge renders, emitted from the registries that own them.
//!
//! `crates/core-model/domain/enum-verbs.toml` is the authority on how a variant reads,
//! `check-outcomes.toml` on whether a Check outcome advances a step, and `actions.toml` on what
//! every act is called and bound to. They are written into `crates/bridge-native/src/generated/`
//! as checked-in Rust, because a verb list or a key map typed into a screen is the second
//! vocabulary that drifted three times before it was deleted. `crates/bridge-native` is its own
//! Cargo workspace, which is why this is a checked-in file and not a `build.rs` of its own.
//!
//! `cargo xtask verify-vocabulary` compares the files to a fresh run and `--write` rewrites them,
//! the way `verify-tokens` does. `rules_vocabulary` holds the same comparison inside
//! `verify-foundations`.
//!
//! **No `toml` crate**, for the reason [`crate::rules_actions`] has none: the registries have one
//! shape, table headers and `key = value`, and a line this reader cannot read stops the run rather
//! than reaching a surface as a blank.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

mod emit;
#[cfg(test)]
mod tests;

const VERBS: &str = "crates/core-model/domain/enum-verbs.toml";
const OUTCOMES: &str = "crates/core-model/domain/check-outcomes.toml";
const ACTS: &str = "crates/core-model/domain/actions.toml";

/// Where the two files are written, repo-relative.
pub const OUT_VOCABULARY: &str = "crates/bridge-native/src/generated/vocabulary.rs";
pub const OUT_ACTIONS: &str = "crates/bridge-native/src/generated/actions.rs";

/// The command a person runs to fix what the gate reports.
pub const WRITE: &str = "cargo xtask verify-vocabulary --write";

/// The vocabularies a surface renders. `criterion_verdict_*` and `evidence_type` are not: a Check's
/// verdict and a step's evidence were a Job's rail, and no surface draws them now.
///
/// Asking for a vocabulary is what puts the gap in this output: a wanted one with no rows in
/// `enum-verbs.toml` is an empty map and every variant of it is a gap, rather than a renderer
/// writing the missing words itself.
const WANTED: &[&str] = &[
    "queued_reason",
    "check_outcome",
    "step_state",
    "task_state",
    // A group of a plan, between a step and a task. It has no enum behind it.
    "group_state",
    "land_state",
    "advance_gate",
    "auto_merge",
    "change_kind",
];

const ACTION_KINDS: &[&str] = &["Action", "Motion"];
const ACTION_TIERS: &[&str] = &["Global", "Contextual"];
/// Why a glyph may be missing: the registry's two words, and there is no third.
const ABSENCES: &[&str] = &["undecided", "by design"];

/// One variant of one vocabulary, as the registry spells it. A blank is `""`.
pub struct Row {
    pub variant: String,
    pub verb: String,
    /// A name in `packages/icons/icons/`.
    pub glyph: String,
    pub token: String,
    /// The token without its `--status-` prefix.
    pub stem: String,
    pub hint: String,
}

/// One act of `actions.toml`.
pub struct Act {
    pub id: String,
    pub kind: String,
    pub tier: String,
    pub verb: String,
    pub icon: String,
    pub absent: String,
    pub shortcut: String,
    pub scope: String,
    pub destructive: bool,
    pub confirms: bool,
    pub issue: Option<u64>,
}

/// Every generated file as `(path from the root, contents)`.
pub fn outputs(root: &Path) -> Result<Vec<(String, String)>, String> {
    let vocabularies = vocabularies(&read(root, VERBS)?, &read(root, OUTCOMES)?)?;
    let acts = acts(&read(root, ACTS)?)?;
    Ok(vec![
        (OUT_VOCABULARY.into(), emit::vocabulary(&vocabularies)),
        (OUT_ACTIONS.into(), emit::actions(&acts)),
    ])
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    fs::read_to_string(root.join(rel)).map_err(|e| format!("cannot read {rel}: {e}"))
}

type Table = BTreeMap<String, String>;

/// `[header]` tables in file order, each a map of its `key = value` lines.
///
/// A repeated header starts its table again in place, which is the one thing a map keyed by header
/// would do to it.
fn tables(text: &str, path: &str) -> Result<Vec<(String, Table)>, String> {
    let mut found: Vec<(String, Table)> = Vec::new();
    let mut current: Option<usize> = None;
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = i + 1;
        if let Some(rest) = line.strip_prefix('[') {
            let Some(name) = rest.strip_suffix(']') else {
                return Err(format!("{path}:{at} — unreadable table header"));
            };
            match found.iter().position(|(have, _)| have == name) {
                Some(p) => {
                    found[p].1.clear();
                    current = Some(p);
                }
                None => {
                    found.push((name.to_string(), Table::new()));
                    current = Some(found.len() - 1);
                }
            }
            continue;
        }
        let Some(split) = line.find('=') else {
            return Err(format!("{path}:{at} — neither a header nor a key"));
        };
        let Some(table) = current else {
            return Err(format!("{path}:{at} — a key outside every table"));
        };
        found[table].1.insert(
            line[..split].trim().to_string(),
            unquoted(line[split + 1..].trim()),
        );
    }
    Ok(found)
}

/// A basic string, a literal string, or a bare token left as it is.
///
/// The literal form is read because one binding needs it: `⌘\` is Toggle sidebar's, and a trailing
/// backslash cannot be written in a basic string without escaping.
fn unquoted(value: &str) -> String {
    for quote in ['"', '\''] {
        if value.chars().count() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn get<'a>(table: &'a Table, key: &str) -> &'a str {
    table.get(key).map(String::as_str).unwrap_or("")
}

/// `advances` per Check outcome, from the registry that owns the set, in file order.
///
/// A surface has to tell a Check that stopped a step from one that did not, and the two are not the
/// same question as passed: `skipped` advances and is not a pass, `never_ran` neither advances nor
/// passes. The registry already answers it.
fn advances(text: &str) -> Result<Vec<(String, bool)>, String> {
    let mut found: Vec<(String, Option<bool>)> = Vec::new();
    let mut current: Option<usize> = None;
    for raw in text.split('\n') {
        let line = raw.trim();
        let header = line
            .strip_prefix("[outcomes.")
            .and_then(|rest| rest.strip_suffix(']'))
            .filter(|name| !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        if let Some(name) = header {
            match found.iter().position(|(have, _)| have == name) {
                Some(p) => {
                    found[p].1 = None;
                    current = Some(p);
                }
                None => {
                    found.push((name.to_string(), None));
                    current = Some(found.len() - 1);
                }
            }
            continue;
        }
        let Some(p) = current else { continue };
        match line {
            "advances = true" => found[p].1 = Some(true),
            "advances = false" => found[p].1 = Some(false),
            _ => {}
        }
    }
    found
        .into_iter()
        .map(|(outcome, flag)| match flag {
            Some(flag) => Ok((outcome, flag)),
            None => Err(format!(
                "{OUTCOMES} — [outcomes.{outcome}] names no advances"
            )),
        })
        .collect()
}

/// The wanted vocabularies and their rows, in the order `WANTED` lists them.
fn vocabularies(verbs: &str, outcomes: &str) -> Result<Vec<(&'static str, Vec<Row>)>, String> {
    let advances = advances(outcomes)?;
    let mut found: Vec<(&'static str, Vec<Row>)> =
        WANTED.iter().map(|name| (*name, Vec::new())).collect();

    for (header, table) in tables(verbs, "enum-verbs.toml")? {
        let parts: Vec<&str> = header.split('.').collect();
        if parts[0] != "verbs" || parts.len() != 3 {
            continue;
        }
        // Unquoted, because a wire value TOML cannot spell bare is written quoted and is still the
        // same value: `advance_gate`'s two policy forms carry a colon.
        let variant = unquoted(parts[2]);
        let Some((_, rows)) = found.iter_mut().find(|(name, _)| *name == parts[1]) else {
            continue;
        };
        let token = get(&table, "status_token");
        // Badge takes the token stem and spells `--status-{stem}` itself, so the stem comes from
        // the token and not from the variant name: `queued` renders `--status-not-started`.
        let stem = token.strip_prefix("--status-").unwrap_or("");
        rows.push(Row {
            variant,
            verb: get(&table, "verb").to_string(),
            glyph: get(&table, "icon").to_string(),
            token: token.to_string(),
            stem: stem.to_string(),
            hint: get(&table, "hint").to_string(),
        });
    }

    // An outcome in one registry and not the other renders with a verb and no rule, or the reverse.
    if let Some((_, drawn)) = found.iter().find(|(name, _)| *name == "check_outcome") {
        for row in drawn {
            if !advances.iter().any(|(outcome, _)| *outcome == row.variant) {
                return Err(format!("{OUTCOMES} has no [outcomes.{}]", row.variant));
            }
        }
        for (outcome, _) in &advances {
            if !drawn.iter().any(|row| row.variant == *outcome) {
                return Err(format!(
                    "{VERBS} has no [verbs.check_outcome.{outcome}]"
                ));
            }
        }
    }
    Ok(found)
}

/// Every act, in the registry's order. This validates and does not decide: `rules_actions` is the
/// gate on the registry itself, and what is checked here is that a row says something this emitter
/// can turn into Rust without guessing.
fn acts(text: &str) -> Result<Vec<Act>, String> {
    let mut acts = Vec::new();
    for (header, table) in tables(text, "actions.toml")? {
        let parts: Vec<&str> = header.split('.').collect();
        if parts[0] != "actions" || parts.len() != 2 {
            return Err(format!(
                "actions.toml — [{header}] is not an [actions.<id>] table"
            ));
        }
        let id = parts[1];
        let at = format!("actions.toml — [actions.{id}]");
        let kind = get(&table, "kind");
        let tier = get(&table, "tier");
        let verb = get(&table, "verb");
        let icon = get(&table, "icon");
        let absent = get(&table, "icon_absent");
        let unbuilt = get(&table, "unbuilt");

        if !ACTION_KINDS.contains(&kind) {
            return Err(format!("{at} — kind is \"{kind}\", not Action or Motion"));
        }
        if !ACTION_TIERS.contains(&tier) {
            return Err(format!("{at} — tier is \"{tier}\", not Global or Contextual"));
        }
        if verb.is_empty() {
            return Err(format!("{at} — no verb, and the palette draws the verb"));
        }
        if get(&table, "shortcut").is_empty() {
            return Err(format!("{at} — no shortcut, and the palette draws one per row"));
        }
        if get(&table, "scope").is_empty() {
            return Err(format!(
                "{at} — no scope, so no surface can decide whether to offer it"
            ));
        }
        // A Motion appears in no palette, so it carries neither a glyph nor a reason for having
        // none. An Action carries exactly one of the two.
        if kind == "Motion" && (!icon.is_empty() || !absent.is_empty()) {
            return Err(format!(
                "{at} — a Motion appears in no palette and carries no glyph column"
            ));
        }
        if !icon.is_empty() && !absent.is_empty() {
            return Err(format!(
                "{at} — a glyph and a reason for having none, and it is one or the other"
            ));
        }
        if kind == "Action" && icon.is_empty() && !ABSENCES.contains(&absent) {
            return Err(format!(
                "{at} — no glyph and icon_absent = \"{absent}\". It is \"undecided\" or \"by design\""
            ));
        }
        // `unbuilt` is an issue reference and nothing else: what is emitted is a number.
        let issue = match unbuilt {
            "" => None,
            _ => match unbuilt
                .strip_prefix('#')
                .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                .and_then(|n| n.parse::<u64>().ok())
            {
                Some(n) => Some(n),
                None => {
                    return Err(format!(
                        "{at} — unbuilt = \"{unbuilt}\", and it takes an issue reference"
                    ))
                }
            },
        };
        let flag = |key: &str| match table.get(key).map(String::as_str) {
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            other => Err(format!(
                "{at} — {key} is \"{}\", and it is true or false",
                other.unwrap_or("")
            )),
        };

        acts.push(Act {
            id: id.to_string(),
            kind: kind.to_string(),
            tier: tier.to_string(),
            verb: verb.to_string(),
            icon: icon.to_string(),
            absent: absent.to_string(),
            shortcut: get(&table, "shortcut").to_string(),
            scope: get(&table, "scope").to_string(),
            destructive: flag("destructive")?,
            confirms: flag("confirms")?,
            issue,
        });
    }
    if acts.is_empty() {
        return Err("actions.toml — no [actions.*] table, and Bridge draws them all".into());
    }
    Ok(acts)
}

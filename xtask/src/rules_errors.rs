//! One code, one failure — the collection `docs/contracts/error-contract.md`
//! says closes the code set.
//!
//! **The premise #345 was filed on is wrong in one detail.** It says no code is
//! declared in Rust yet and points at `crates/ipc/src/error.rs`. Seventeen are:
//! `crates/fleet/src/refusing.rs` declares fourteen and `crates/api` three,
//! each as a `const` beside the refusal that raises it. So there was nothing to
//! invent — the declaration form already exists in both languages, and this
//! reads it rather than proposing one.
//!
//! A Rust code is declared as `const NO_SUCH_JOB: &str = "fleet.no_such_job";`
//! and found by the literal's shape, because nothing types a code there — and a
//! dotted lowercase string is also what a file name looks like, which is why
//! [`FILE_EXTENSIONS`] exists. That is a guess, and it is the loud kind: a wrong
//! include shows up as a duplicate naming a file name, not as silence.
//!
//! **There is no Bridge half any more.** The Electron Bridge declared its own
//! codes under `bridge.`, found by a type annotation, and went with it. The
//! prefix stays reserved, so a Rust code that takes it still fails.
//!
//! **Nothing here is hardcoded to a path.** The scan walks the source root and
//! matches on the declaration, and losing every declaration fails on the empty
//! set rather than passing.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// Where a Rust code may be declared. `xtask` is excluded throughout: this
/// module's own tests hold declarations as fixture text, and a gate that
/// collides with itself is worse than no gate.
const RUST_ROOTS: &[&str] = &["crates"];

/// The namespace the Electron Bridge minted in, reserved: a Rust code that took
/// it would read as one of Bridge's.
const BRIDGE_PREFIX: &str = "bridge.";

/// Final segments that make a dotted lowercase literal a file name rather than
/// a code. Four live in `crates/` today — `fleet.json`, `armada.yml`,
/// `armada.db`, `mcp.json` — and every one of them is a `const` of the same
/// shape a code has. Nothing distinguishes the two but this.
const FILE_EXTENSIONS: &[&str] = &[
    "json", "yml", "yaml", "toml", "db", "lock", "md", "rs", "ts", "tsx", "js", "txt", "log",
    "sock", "sql", "sh", "css", "html", "ini", "env",
];

/// One code, where it is declared, and the one line above it.
pub struct Declaration {
    pub code: String,
    pub path: String,
    pub line: usize,
    pub meaning: Option<String>,
}

impl Declaration {
    /// `path:line`, the form every other rule in the gate names a site in.
    pub fn site(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }
}

/// Rule: two declarations of one code fail, naming both sites.
///
/// **The contract says the set is closed by collection and names a command
/// that did not exist.** Until this, the only thing keeping the codes distinct
/// was that they are declared beside the refusal that raises them and somebody
/// read it.
///
/// **An empty set fails.** It is non-empty today, so zero means the scan
/// stopped matching, which is the failure a rule keyed to a convention dies of
/// and never reports.
pub fn one_code_names_one_failure(root: &Path) -> Report {
    let mut report = Report::new("one code, one failure");
    let rust = collect(root);

    if rust.is_empty() {
        report.fail(
            "crates/ — no `const NAME: &str = \"ns.thing\";` matched anywhere. \
             Seventeen matched when this rule was written, so an empty set means \
             the declaration form moved and the scan went dark rather than green",
        );
    }

    collisions(&mut report, &rust, "Rust");
    borrowed_namespace(&mut report, &rust);
    report
}

/// A Rust code that took the Electron Bridge's prefix.
fn borrowed_namespace(report: &mut Report, rust: &[Declaration]) {
    for declared in rust {
        if declared.code.starts_with(BRIDGE_PREFIX) {
            report.fail(format!(
                "{} — `{}` is declared in Rust and takes Bridge's `{BRIDGE_PREFIX}` \
                 namespace, which is reserved",
                declared.site(),
                declared.code
            ));
        }
    }
}

/// Every code declared twice or more, named with every site that declares it.
///
/// One finding per code rather than one per site: the fault is the pair, and a
/// person reading two lines has to join them back together to see it.
fn collisions(report: &mut Report, declared: &[Declaration], half: &str) {
    let mut by_code: BTreeMap<&str, Vec<&Declaration>> = BTreeMap::new();
    for one in declared {
        by_code.entry(one.code.as_str()).or_default().push(one);
    }
    for (code, sites) in by_code {
        if sites.len() < 2 {
            continue;
        }
        let others = sites[1..]
            .iter()
            .map(|s| s.site())
            .collect::<Vec<_>>()
            .join(", ");
        report.fail(format!(
            "{} — the {half} code `{code}` is declared here and at {others}. \
             One code, one failure: a person quoting it out of a debug payload is \
             trusting that exactly one failure raises it, and nothing else on the \
             wire says which. Rename all but one",
            sites[0].site()
        ));
    }
}

/// Every Rust code, sorted by code then by site.
pub fn collect(root: &Path) -> Vec<Declaration> {
    let mut rust = Vec::new();
    for source_root in RUST_ROOTS {
        for path in files_with_ext(root, &root.join(source_root), &["rs"]) {
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            rust.extend(rust_declarations(&path, &text));
        }
    }
    rust.sort_by(|a, b| (&a.code, &a.path, a.line).cmp(&(&b.code, &b.path, b.line)));
    rust
}

/// Every Rust code declared in one file.
pub fn rust_declarations(path: &str, text: &str) -> Vec<Declaration> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        let Some(code) = rust_code_on(line) else {
            continue;
        };
        found.push(Declaration {
            code,
            path: path.to_string(),
            line: n + 1,
            meaning: meaning_above(&lines, n),
        });
    }
    found
}

/// The code a Rust `const` line declares, if the line declares one.
///
/// The visibility is stepped over rather than enumerated, so `pub`,
/// `pub(crate)` and a bare `const` all read the same. A commented-out
/// declaration is not one.
fn rust_code_on(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("//") || line.starts_with('*') {
        return None;
    }
    let (name, rest) = line.split_once("const ")?.1.split_once(':')?;
    if name.trim().is_empty()
        || !name
            .trim()
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return None;
    }
    let rest = rest.trim_start();
    let value = rest
        .strip_prefix("&str = ")
        .or_else(|| rest.strip_prefix("&'static str = "))?;
    let literal = value.trim().strip_suffix(';')?;
    let literal = unquote(literal)?;
    looks_like_a_code(literal).then(|| literal.to_string())
}

/// Whether a string literal is a code rather than a file name or a version.
///
/// Two or more segments of lowercase, each starting with a letter, and a final
/// segment that is not a file extension. Every part of that is load-bearing
/// against something already in `crates/`.
fn looks_like_a_code(literal: &str) -> bool {
    let segments: Vec<&str> = literal.split('.').collect();
    if segments.len() < 2 {
        return false;
    }
    for segment in &segments {
        let mut chars = segment.chars();
        if !chars.next().is_some_and(|c| c.is_ascii_lowercase()) {
            return false;
        }
        if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return false;
        }
    }
    !FILE_EXTENSIONS.contains(segments.last().unwrap())
}

/// The **first** line of the doc comment above a declaration — `///` in Rust,
/// `/** */` in TypeScript.
///
/// First and not last, because the last line of a five-line block is the end of
/// an argument and reads as a non-sequitur on its own. `None` where the
/// declaration carries no comment at all: two Rust codes sit under a block
/// written about the group, and this does not pretend they have their own.
fn meaning_above(lines: &[&str], at: usize) -> Option<String> {
    let end = at.checked_sub(1)?;
    let last = lines.get(end)?.trim();

    let (start, end) = if last.starts_with("///") {
        let mut start = end;
        while start > 0 && lines[start - 1].trim().starts_with("///") {
            start -= 1;
        }
        (start, end)
    } else if let Some(one) = last.strip_prefix("/**").and_then(|r| r.strip_suffix("*/")) {
        return (!one.trim().is_empty()).then(|| one.trim().to_string());
    } else if last == "*/" {
        (
            (0..end)
                .rev()
                .find(|i| lines[*i].trim().starts_with("/**"))?
                + 1,
            end,
        )
    } else {
        return None;
    };

    (start..=end)
        .map(|i| lines[i].trim().trim_start_matches(['/', '*']).trim())
        .find(|text| !text.is_empty())
        .map(str::to_string)
}

/// A string literal with its quotes taken off, single or double. `None` where
/// the value is not a plain literal — a computed code is not a declaration.
fn unquote(value: &str) -> Option<&str> {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|r| r.strip_suffix(quote))
        {
            return (!inner.contains(quote)).then_some(inner);
        }
    }
    None
}

#[cfg(test)]
mod tests;

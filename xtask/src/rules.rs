//! The six rules. Each returns what is missing, by name.

use std::fs;
use std::path::Path;

use crate::{crate_dirs, files_with_ext, Report};

/// Source extensions the line-length and literal rules read.
const SOURCE_EXTS: &[&str] = &["rs", "ts", "tsx"];

/// Where source lives. `docs/` is prose and is not gated.
const SOURCE_ROOTS: &[&str] = &["crates", "apps", "packages", "xtask"];

// ---------------------------------------------------------------- rule one

/// Where each milestone's acceptance test is named, one table row per claim.
const ACCEPTANCE_DOC: &str = "docs/practices/acceptance-tests.md";

/// Every milestone's acceptance test exists, and holds a test.
///
/// **That it passes is the `acceptance` Check's**, which the merge line runs
/// on every turn this rule could change the answer for. Running it here as
/// well compiled the suite twice a turn. `docs/practices/acceptance-tests.md`
/// says which owns which half.
///
/// **A file with no test in it is the falsehood a green run over nothing
/// tells**, so a named file must carry a test attribute, not merely exist.
pub fn acceptance_test_exists(root: &Path) -> Report {
    let mut report = Report::new("every milestone's acceptance test exists");

    if !root.join("crates/acceptance/Cargo.toml").is_file() {
        report.fail("crates/acceptance — the package holding the acceptance test");
        return report;
    }
    let tests = files_with_ext(root, &root.join("crates/acceptance/tests"), &["rs"]);
    if tests.is_empty() {
        report.fail("crates/acceptance/tests/ — no test in the acceptance package");
        return report;
    }

    let named = fs::read_to_string(root.join(ACCEPTANCE_DOC))
        .map(|doc| milestone_tests(&doc))
        .unwrap_or_default();
    if named.is_empty() {
        report.fail(format!(
            "{ACCEPTANCE_DOC} — the table naming each milestone's acceptance test"
        ));
    }
    for (milestone, rel) in named {
        match fs::read_to_string(root.join(&rel)) {
            Err(_) => report.fail(format!("{rel} — {milestone}'s acceptance test")),
            Ok(text) if !holds_a_test(&text) => report.fail(format!(
                "{rel} — {milestone}'s acceptance test holds no `#[test]`, \
                 and a file with none claims nothing"
            )),
            Ok(_) => {}
        }
    }
    report
}

/// `(milestone, path)` for each row of the doc's `| Milestone |` table whose
/// last cell names a file under `crates/acceptance/tests/`.
fn milestone_tests(doc: &str) -> Vec<(String, String)> {
    doc.lines()
        .skip_while(|line| !line.starts_with("| Milestone |"))
        .take_while(|line| line.starts_with('|'))
        .filter_map(|row| {
            let cells: Vec<&str> = row.trim_matches('|').split('|').map(str::trim).collect();
            let rel = cells.last()?.trim_matches('`');
            rel.starts_with("crates/acceptance/tests/")
                .then(|| (cells[0].to_string(), rel.to_string()))
        })
        .collect()
}

fn holds_a_test(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("#[test]") || t.starts_with("#[tokio::test")
    })
}

// ---------------------------------------------------------------- rule two

/// Every fixture the spec's assertions need exists.
///
/// Two groups, and the second was missing until 24 Aug 2026.
///
/// **The five failure modes** are Workflow's own table, in its order, and the
/// file names are the Fixture Specs' slugs.
///
/// **The three that make those five testable.** Three of the five assertions
/// describe something one stream cannot express, and the gate did not notice
/// because it only counted the failure modes:
///
/// - `thrashing` asserts the Judge is **not** invoked for a well-behaved Drone.
///   That needs a well-behaved stream to assert against.
/// - `silence` asserts a Drone answering a poke resumes and does **not**
///   escalate. Without it, a detector that escalates every quiet Drone passes.
/// - `claims-done-no-evidence` asserts a clarification round capped at two or
///   three, and escalation on exhausting it. A single terminated stream has no
///   cap in it.
const FAILURE_MODES: &[(&str, &str)] = &[
    ("silence", "Silence / Stalled"),
    (
        "claims-done-no-evidence",
        "Claims done, no evidence artifact",
    ),
    (
        "plain-text-bypass",
        "Claims done in plain text, bypasses structured report",
    ),
    ("thrashing", "Thrashing / off-rails"),
    ("evidence-gaming", "Evidence gaming"),
];

/// The control and the negative cases, each named with the assertion it serves.
const SUPPORTING: &[(&str, &str)] = &[
    (
        "happy-path",
        "the control — thrashing's negative assertion needs it",
    ),
    (
        "silence-poke-answered",
        "a poke answered, so quiet alone cannot escalate",
    ),
    (
        "clarification-exhausted",
        "the capped clarification loop, exhausted",
    ),
];

pub fn every_failure_mode_has_a_fixture(root: &Path) -> Report {
    let mut report = Report::new("every fixture the assertions need exists");
    let dir = root.join("crates/testkit/fixtures/ndjson");
    for (slug, why) in FAILURE_MODES.iter().chain(SUPPORTING) {
        if !dir.join(format!("{slug}.ndjson")).is_file() {
            report.fail(format!(
                "crates/testkit/fixtures/ndjson/{slug}.ndjson — {why}"
            ));
        }
    }
    report
}

// --------------------------------------------------------------- rule five

/// No `serde_json::from_*` outside `store` and `ipc`.
///
/// Untyped JSON is allowed exactly where bytes enter the process: the store
/// reading its own rows, and ipc reading the wire. Everywhere else a value
/// arrives already typed, and a `from_str` in the middle of the system means
/// something was serialised to get it there.
pub fn no_untyped_json_outside_store_and_ipc(root: &Path) -> Report {
    let mut report = Report::new("no serde_json::from_* outside store and ipc");
    let allowed = ["crates/store/", "crates/ipc/"];
    for source_root in SOURCE_ROOTS {
        for path in files_with_ext(root, &root.join(source_root), &["rs"]) {
            // The gate names the pattern it forbids, so it always matches
            // itself. `guard_write.py` carries the same exemption for the same
            // reason, and the two must not drift.
            if allowed.iter().any(|a| path.starts_with(a)) || path.starts_with("xtask/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            for (n, line) in text.lines().enumerate() {
                if line.contains("serde_json::from_") {
                    report.fail(format!("{path}:{} — serde_json::from_*", n + 1));
                }
            }
        }
    }
    report
}

// ---------------------------------------------------------------- rule six

/// No vendor literal outside `adapters`.
///
/// The adapter boundary is the only place that knows whose API it is talking
/// to. A vendor's name anywhere else — in a type, a string, a comment — is the
/// boundary having leaked, and it leaks in comments first.
///
/// **This list is the gate's own, not the plan's.** M0 step 7 names the rule
/// and not its vocabulary; these are the vendors Armada actually reaches for.
/// Adding one is a one-line change and should be made deliberately.
const VENDOR_LITERALS: &[&str] = &[
    "anthropic",
    "claude",
    "openai",
    "gpt-4",
    "gemini",
    "copilot",
    "github",
    "gitlab",
    "bitbucket",
    "docker",
    // Libraries, not vendors, and on the list for the same reason: naming one
    // outside the adapter layer means the boundary has already leaked. The
    // adapters contract has said "no `git2` or `gh` literal outside adapters"
    // since it was written, and nothing was checking it — found when a step
    // built a worktree behind the VCS trait and went looking for the rule that
    // was supposed to have stopped it going anywhere else.
    "git2",
];

/// Bridge's registry of unbuilt Fleet routes. Exempt from the rule above, by name.
const PENDING_ROUTES_FILE: &str = "packages/protocol/src/pending.ts";

/// The sign-in files, which may say `github`, and no other vendor.
///
/// The account a person signs in with is a GitHub account, and the approved
/// button, its mock and its walk name it so the owner can tell which one. The
/// owner chose this exception over neutral wording on 14 Oct 2026. The address
/// to sign in at still comes from Fleet when this is wired.
const SIGN_IN_FILES: &[&str] = &[
    "apps/desktop/src/renderer/src/mock/account.tsx",
    "apps/desktop/src/renderer/src/mock/scenarios/onboarding-home.ts",
    "apps/desktop/src/renderer/src/mock/walks/onboarding-home.ts",
    "apps/desktop/src/renderer/src/mock/walks/onboarding-work.ts",
    "packages/components/src/compositions/Account/Account.stories.tsx",
    "packages/components/src/compositions/Account/Account.tsx",
    "packages/components/src/compositions/Account/AccountScreen.tsx",
    "packages/components/src/compositions/Account/ForgeMark.tsx",
    "packages/components/src/compositions/TitleBar/TitleBar.stories.tsx",
    "packages/components/src/compositions/TitleBar/TitleMenu.tsx",
    "apps/store/src/enrol.ts",
];

/// The mock files that stand in for what Fleet sends about the forge's own name.
///
/// A `session.called` entry carries `name`, and Fleet's adapter says `GitHub` there, so the scenario that
/// plays the entries and the walk that points at the outpost it labels have to say it too. Product code
/// reads the name off the entry and says nothing.
const STANDS_IN_FOR_THE_FORGE: &[&str] = &[
    "apps/desktop/src/renderer/src/mock/scenarios/field-outposts.ts",
    "apps/desktop/src/renderer/src/mock/walks/field-outposts.ts",
];

/// `lower` with every identifier imported from `lucide-react` blanked out.
///
/// **Why the rule yields to these, and only these.** A glyph's component name
/// is its icon library's, not ours: `FolderGit2` is the drawing of a folder with
/// a branch, and a vendor literal inside it says nothing about whose API the
/// code talks to. The registry (`packages/icons/icons/`) already decides
/// which of those names may be imported. The exemption is the imported
/// identifiers and nothing else, so `git2` written anywhere else in the same
/// file still fails.
fn without_lucide_names(lower: &str, original: &str) -> String {
    let mut out = lower.to_string();
    for (at, _) in original.match_indices("from \"lucide-react\"") {
        let Some(start) = original[..at].rfind("import") else {
            continue;
        };
        let names = &original[start..at];
        let (Some(open), Some(close)) = (names.find('{'), names.rfind('}')) else {
            continue;
        };
        for name in names[open + 1..close].split(',') {
            let name = name
                .split(" as ")
                .next()
                .unwrap_or("")
                .trim()
                .to_lowercase();
            if !name.is_empty() && VENDOR_LITERALS.iter().any(|vendor| name.contains(vendor)) {
                out = out.replace(&name, &" ".repeat(name.len()));
            }
        }
    }
    out
}

pub fn no_vendor_literal_outside_adapters(root: &Path) -> Report {
    let mut report = Report::new("no vendor literal outside adapters");
    for source_root in SOURCE_ROOTS {
        for path in files_with_ext(root, &root.join(source_root), SOURCE_EXTS) {
            // The gate names vendors in order to forbid them, and the adapters
            // are where naming them is the job.
            if path.starts_with("crates/adapters/") || path.starts_with("xtask/") {
                continue;
            }
            // The one named file outside them: the routes Fleet does not serve
            // yet, each carrying the full link to the issue that builds it. The
            // link is Armada's own tracker, not a vendor the product talks to,
            // and the owner chose the whole URL over a short form on 29 Sep.
            if path == PENDING_ROUTES_FILE {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            let lower = without_lucide_names(&text.to_lowercase(), &text);
            let signs_in = SIGN_IN_FILES.contains(&path.as_str()) || STANDS_IN_FOR_THE_FORGE.contains(&path.as_str());
            for vendor in VENDOR_LITERALS {
                if signs_in && *vendor == "github" {
                    continue;
                }
                if lower.contains(vendor) {
                    let line = lower
                        .lines()
                        .position(|l| l.contains(vendor))
                        .map(|n| n + 1)
                        .unwrap_or(0);
                    report.fail(format!("{path}:{line} — the literal `{vendor}`"));
                }
            }
        }
    }
    report
}

// ------------------------------------------------------------- rule eight

/// The v1 harvest has an index, and the index knows every note.
///
/// v1 exists only as a tag now, so the harvest is the last time anything about
/// it gets written down while somebody still remembers why it mattered. An
/// unindexed note is one nobody finds, which makes it the same as one nobody
/// wrote.
///
/// The check runs both ways, like the source manifest: a note missing from the
/// index fails, and so does an index entry pointing at a note that is not there.
pub fn the_v1_harvest_has_an_index(root: &Path) -> Report {
    let mut report = Report::new("the v1 harvest has an index, and it knows every note");
    let dir = root.join("docs/v1-learnings");
    let index = dir.join("INDEX.md");
    let Ok(index_text) = fs::read_to_string(&index) else {
        report.fail("docs/v1-learnings/INDEX.md — the harvest index (M0 step 10 writes it)");
        return report;
    };

    for note in files_with_ext(root, &dir, &["md"]) {
        let name = note.rsplit('/').next().unwrap_or(&note);
        if name == "INDEX.md" {
            continue;
        }
        if !index_text.contains(name) {
            report.fail(format!(
                "{note} is a harvest note the index does not mention"
            ));
        }
    }

    for line in index_text.lines() {
        for token in
            line.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
        {
            if token.ends_with(".md") && token != "INDEX.md" && !dir.join(token).is_file() {
                report.fail(format!(
                    "the index names {token}, which is not in docs/v1-learnings/"
                ));
            }
        }
    }
    report
}

// --------------------------------------------------------------- rule ten

/// Everything logs through the envelope. No crate writes its own format.
///
/// The envelope only guarantees a join if every line has one. A `println!` in
/// the middle of `fleet` produces a line that no query reaches, no sink
/// redacts, and nothing can correlate to the Job that caused it — and it is the
/// easiest thing in the world to add while debugging and forget.
///
/// `armada` is exempt: it is the composition root and the CLI, and every one of
/// its verbs writes to stdout on purpose — a command answering a question is
/// not a component logging.
pub fn nothing_writes_its_own_log_format(root: &Path) -> Report {
    let mut report = Report::new("everything logs through the envelope");
    const MACROS: &[&str] = &["println!", "eprintln!", "print!", "eprint!"];

    for dir in crate_dirs(root) {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "armada" {
            continue;
        }
        for path in files_with_ext(root, &dir.join("src"), &["rs"]) {
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            for (n, line) in text.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for macro_name in MACROS {
                    if code.contains(macro_name) {
                        report.fail(format!(
                            "{path}:{} — `{macro_name}` writes a line the envelope does not carry",
                            n + 1
                        ));
                    }
                }
            }
        }
    }
    report
}

// Rule eleven, `the_tokens_generate_what_is_checked_in`, moved to
// `rules_tokens.rs` when a second token rule was written. Both read the token
// pipeline rather than Rust source, and the second one exists because the first
// cannot see whether what it checked in works in a browser.


#[cfg(test)]
mod tests;

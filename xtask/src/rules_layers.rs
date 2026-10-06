//! The one rule the layer split rests on: a package imports downward, never up
//! and never sideways.
//!
//! Bridge is being taken apart into layers — the wire, the components, the
//! shell, the screens, and the app that composes them — so that a screen can be
//! rendered, storied and tested without an Electron process. That only holds
//! while the dependency runs one way. A screen that reaches into `apps/desktop`
//! for a type is a screen that cannot leave it, and the split is a naming
//! convention rather than a structure.
//!
//! **The rule arrives before the packages do.** A rule written after the move
//! is a rule written after the violations, and the violations are the thing it
//! exists to prevent. `shell` and `screens` do not exist yet; a layer with no
//! package on it is not a fault, and the day one lands it is already governed.
//!
//! Two ways to import, and both are checked. A package name says which layer it
//! is on. A relative path that climbs out of its own package says nothing, so
//! it is refused outright — there is no legitimate reason for a file in one
//! package to reach another through `../../`, and it is the one form that would
//! slip past a rule reading names.
//!
//! **`@armada/bridge-api` (layer 4) sits above the screens and below the surfaces:** the
//! types and tiny helpers a surface and the app both build on, with no React.
//!
//! **Surfaces sit between that and the app.** Each directory under
//! `packages/surfaces/` is one package on layer 5, found by listing the
//! directory because the table is static. Two surfaces cannot import each other
//! (same layer). One exemption: a surface's `*.test.ts(x)` and `vitest.config.ts`
//! may import `@armada/desktop`, the mock harness that mounts the whole app.
//! Nothing else in a surface may.
//!
//! **No TS parser, and the gate keeps no dependencies.** This reads the two
//! shapes an import has: `from "…"` and `import("…")`.

// The second rule here is the same idea one directory down, and its argument
// is on [`nothing_in_the_main_process_reads_the_draft_schema`].

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// Where the drafts live, as a path spells them and as a specifier names them. The Jobs
/// surface took the schema out of the screens, so both places are watched.
const DRAFTS: &[(&str, &str)] = &[
    ("packages/screens/src/draft", "@armada/screens/src/draft"),
    ("packages/surfaces/jobs/src/draft", "@armada/jobs/draft"),
];
/// The files that could put the drafts behind a package name.
const DRAFT_INDEXES: &[&str] = &[
    "packages/screens/src/index.ts",
    "packages/surfaces/jobs/src/index.ts",
];

/// The layers, ground up. A package may import a package strictly below it.
///
/// **Strictly below, not below-or-equal.** Two packages on one layer importing
/// each other is a cycle waiting for a third, and the ground layer is the one
/// that has to import nothing at all — `@armada/protocol` is what every other
/// package depends on, so anything it reaches for is reached by all of them.
const LAYERS: &[(&str, &str)] = &[
    ("@armada/tokens", "packages/tokens"),
    ("@armada/brand", "packages/brand"),
    ("@armada/icons", "packages/icons"),
    ("@armada/protocol", "packages/protocol"),
    ("@armada/components", "packages/components"),
    ("@armada/shell", "packages/shell"),
    ("@armada/screens", "packages/screens"),
    ("@armada/bridge-api", "packages/bridge-api"),
    ("@armada/desktop", "apps/desktop"),
];

/// Where the surfaces live. Each directory under it is one package,
/// `@armada/<dir>`, and they all share one layer.
const SURFACES_DIR: &str = "packages/surfaces";

/// The layer of `apps/desktop`, which a surface's tests may reach up to.
const DESKTOP: &str = "@armada/desktop";

/// Which layer each package sits on. Everything in the first group is ground.
/// `surfaces` is the list of `@armada/<x>` names found under [`SURFACES_DIR`],
/// since the table above is static and the directory is not.
fn layer_of(name: &str, surfaces: &[String]) -> Option<usize> {
    match name {
        "@armada/tokens" | "@armada/brand" | "@armada/icons" | "@armada/protocol" => Some(0),
        "@armada/components" => Some(1),
        "@armada/shell" => Some(2),
        "@armada/screens" => Some(3),
        "@armada/bridge-api" => Some(4),
        "@armada/desktop" => Some(6),
        _ if surfaces.iter().any(|s| s == name) => Some(5),
        _ => None,
    }
}

/// The surfaces on disk, as `(@armada/<x>, packages/surfaces/<x>)`.
fn surfaces_on_disk(root: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(root.join(SURFACES_DIR)) else {
        return Vec::new();
    };
    let mut found: Vec<(String, String)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|dir| !dir.starts_with('.') && dir != "node_modules")
        .map(|dir| (format!("@armada/{dir}"), format!("{SURFACES_DIR}/{dir}")))
        .collect();
    found.sort();
    found
}

/// Whether a surface's file is a test or its test config, which may import the
/// mock harness in `@armada/desktop` even though that is a higher layer.
fn is_surface_test_file(path: &str, dir: &str) -> bool {
    path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
        || path == format!("{dir}/vitest.config.ts")
}

/// What is wrong with one import, if anything. `name` and `dir` are the
/// importing package; `surfaces` is every surface name on disk.
fn import_fault(
    path: &str,
    name: &str,
    dir: &str,
    spec: &str,
    surfaces: &[String],
) -> Option<String> {
    let mine = layer_of(name, surfaces)?;
    if let Some(theirs) = layer_of(package_of(spec), surfaces) {
        let exempt = surfaces.iter().any(|s| s == name)
            && package_of(spec) == DESKTOP
            && is_surface_test_file(path, dir);
        if theirs >= mine && !exempt {
            let them = package_of(spec);
            return Some(format!(
                "{path} imports `{them}`, which is on layer {theirs}, and {name} is on \
                 layer {mine}. A package imports one strictly below it — otherwise \
                 the layer it is in is a name rather than a boundary"
            ));
        }
        return None;
    }
    if escapes(path, spec, dir) {
        return Some(format!(
            "{path} reaches out of its own package with `{spec}`. A package is \
             reached by its name or not at all: a relative path that climbs out \
             states no layer and so can be checked against none"
        ));
    }
    None
}

pub fn every_package_imports_downward(root: &Path) -> Report {
    let mut report = Report::new("every package imports downward, and never out of itself");

    let surfaces = surfaces_on_disk(root);
    let surface_names: Vec<String> = surfaces.iter().map(|(n, _)| n.clone()).collect();

    let mut present: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, dir) in LAYERS
        .iter()
        .copied()
        .chain(surfaces.iter().map(|(n, d)| (n.as_str(), d.as_str())))
    {
        if root.join(dir).is_dir() {
            present.insert(name, dir);
        }
    }
    if present.is_empty() {
        report.fail("no package this rule governs is on disk. The layer list is wrong");
        return report;
    }

    for (name, dir) in &present {
        for path in files_with_ext(root, &root.join(dir), &["ts", "tsx", "mjs"]) {
            if path.contains("/node_modules/") || path.contains("/dist/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            for spec in specifiers(&text) {
                if let Some(fault) = import_fault(&path, name, dir, &spec, &surface_names) {
                    report.fail(fault);
                }
            }
        }
    }

    let absent: Vec<&str> = LAYERS
        .iter()
        .filter(|(name, _)| !present.contains_key(name))
        .map(|(name, _)| *name)
        .collect();
    if !absent.is_empty() {
        report.warn(format!(
            "governed and not built yet: {}. A layer with no package is not a fault — the rule \
             is here first so the day one lands it is already bound",
            absent.join(", ")
        ));
    }

    report
}

#[cfg(test)]
mod tests;

/// Nothing under `apps/desktop/src/main/` reads the draft schema.
///
/// **The drafts are shapes Fleet does not serve**, kept in
/// `packages/surfaces/jobs/src/draft/` (and the two files Studios reads, in
/// `packages/screens/src/draft/`) rather than `@armada/protocol` because a type
/// invented there is indistinguishable from one Fleet sends. `#1532`, decided
/// in `#1530`. The main process is the half that holds the connection, so a
/// draft type reaching it asks a daemon for a field nobody told it about.
///
/// **Two checks, and the second is what makes the first worth having.** A
/// re-export from either package's `index.ts` would put every draft behind
/// a package name the main process already imports, and the
/// first check would then see nothing at all.
pub fn nothing_in_the_main_process_reads_the_draft_schema(root: &Path) -> Report {
    let mut report = Report::new("nothing in the main process reads the draft schema");

    if !DRAFTS.iter().any(|(dir, _)| root.join(dir).is_dir()) {
        report.warn("no draft directory is on disk. Nothing to keep out of the main process");
        return report;
    }

    let main = root.join("apps/desktop/src/main");
    for path in files_with_ext(root, &main, &["ts", "tsx", "mjs"]) {
        if path.contains("/node_modules/") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        for spec in specifiers(&text) {
            if reaches_the_drafts(&spec) {
                report.fail(format!(
                    "{path} imports `{spec}`. A draft directory holds shapes Fleet does not \
                     serve, and the main process is what talks to Fleet — a draft type \
                     there is a field being asked of a daemon that was never told about \
                     it. Draw it in the renderer, or put it on the wire with its Rust DTO"
                ));
            }
        }
    }

    for index in DRAFT_INDEXES {
        // A package that is not on disk has no index to read.
        if !root.join(index).is_file() {
            continue;
        }
        match fs::read_to_string(root.join(index)) {
            Ok(text) => {
                for spec in specifiers(&text) {
                    if spec.starts_with("./draft") {
                        report.fail(format!(
                            "{index} exports `{spec}`. That puts every draft behind \
                             a package name the main process may import, and \
                             the check above would then see nothing to refuse. A draft is \
                             imported by its own path or not at all"
                        ));
                    }
                }
            }
            Err(_) => report.fail(format!("{index} would not read")),
        }
    }

    report
}

/// Whether a specifier resolves into the draft directory, by the package name
/// or by a relative path that spells it out.
fn reaches_the_drafts(spec: &str) -> bool {
    DRAFTS
        .iter()
        .any(|(dir, specifier)| spec.starts_with(specifier) || spec.contains(dir))
}

/// The package a specifier names, which is the first two segments of a scope.
fn package_of(spec: &str) -> &str {
    if !spec.starts_with('@') {
        return spec.split('/').next().unwrap_or(spec);
    }
    let mut parts = spec.splitn(3, '/');
    match (parts.next(), parts.next()) {
        (Some(scope), Some(name)) => &spec[..scope.len() + 1 + name.len()],
        _ => spec,
    }
}

/// Whether a relative specifier resolves outside the package it was written in.
///
/// Counted rather than resolved: the gate has no filesystem view of what a
/// bundler would pick, and `..` past the package root is the whole of what is
/// being refused.
fn escapes(path: &str, spec: &str, dir: &str) -> bool {
    if !spec.starts_with('.') {
        return false;
    }
    let depth = path.strip_prefix(dir).unwrap_or(path).matches('/').count();
    let up = spec.split('/').filter(|seg| *seg == "..").count();
    up >= depth
}

/// Every module specifier in a file: `from "…"`, and a dynamic `import("…")`.
fn specifiers(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for marker in [" from ", "import("] {
        let mut rest = text;
        while let Some(at) = rest.find(marker) {
            rest = &rest[at + marker.len()..];
            let rest_trimmed = rest.trim_start();
            let Some(quote) = rest_trimmed
                .chars()
                .next()
                .filter(|c| *c == '"' || *c == '\'')
            else {
                continue;
            };
            let after = &rest_trimmed[1..];
            let Some(close) = after.find(quote) else {
                continue;
            };
            found.push(after[..close].to_string());
        }
    }
    found
}

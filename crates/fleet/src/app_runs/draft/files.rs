//! What a repository's own files say about how its app runs: a `package.json` with a `dev` script,
//! and a Python app served by uvicorn. **Read, never run**: nothing here starts a process or
//! installs one, and a folder it cannot read says nothing.
//!
//! **No JSON parser, on purpose.** Bytes enter Fleet as JSON only through `store` and `ipc`, and a
//! `package.json` needs one string out of it: the `dev` script, read as the `"key": "value"` pairs
//! `scripts` always holds.

use std::path::{Path, PathBuf};

use super::{safe_name, DraftPort, DraftProcess, DraftSource};

/// Folders that hold what other people wrote or what a build wrote.
const SKIPPED: &[&str] = &["node_modules", "target", "dist", "build", "venv", "__pycache__", "vendor"];

/// Folders whose `package.json`s are libraries: their `dev` is a watcher, not an app.
const LIBRARIES: &[&str] = &["packages", "libs", "crates"];

/// The most folders looked at, so a very large tree costs a bounded read.
const MOST_FOLDERS: usize = 300;

/// How deep under the root a folder is looked at.
const DEEPEST: usize = 2;

/// One process for each folder whose files name a server.
pub(super) fn found(root: &Path) -> Vec<DraftProcess> {
    let folders = folders(root);
    let root_has_dev = dev_script_in(root).is_some();
    let mut out = Vec::new();
    for (relative, path) in &folders {
        let dir = (relative != ".").then(|| relative.clone());
        let below_a_library = relative.split('/').any(|part| LIBRARIES.contains(&part));
        // A root `dev` is how a monorepo starts everything, so its folders' own are not repeated.
        let js_wanted = (relative == "." || !root_has_dev) && !below_a_library;
        if js_wanted {
            if let Some(process) = javascript(root, path, dir.clone(), relative) {
                out.push(process);
            }
        }
        if let Some(process) = python(root, path, dir, relative) {
            out.push(process);
        }
    }
    out
}

/// The root, then the folders under it to [`DEEPEST`], by path.
fn folders(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = vec![(String::from("."), root.to_path_buf())];
    let mut next = 0;
    while next < out.len() && out.len() < MOST_FOLDERS {
        let (relative, path) = out[next].clone();
        next += 1;
        let depth = if relative == "." { 0 } else { relative.split('/').count() };
        if depth >= DEEPEST {
            continue;
        }
        let Ok(listing) = std::fs::read_dir(&path) else {
            continue;
        };
        let mut names: Vec<String> = listing
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| !name.starts_with('.') && !SKIPPED.contains(&name.as_str()))
            .collect();
        names.sort();
        for name in names {
            let below = match relative.as_str() {
                "." => name.clone(),
                parent => format!("{parent}/{name}"),
            };
            out.push((below, path.join(&name)));
        }
    }
    out
}

fn dev_script_in(folder: &Path) -> Option<String> {
    dev_script(&std::fs::read_to_string(folder.join("package.json")).ok()?)
}

/// The `dev` script of a `package.json`, where `scripts` has one that is a string.
fn dev_script(text: &str) -> Option<String> {
    let at = text.find("\"scripts\"")? + "\"scripts\"".len();
    let mut chars = text[at..].chars().peekable();
    // `:` then `{`, with only whitespace between.
    for wanted in [':', '{'] {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.next()? != wanted {
            return None;
        }
    }
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace() || *c == ',') {
            chars.next();
        }
        if chars.next()? != '"' {
            return None;
        }
        let key = string_rest(&mut chars)?;
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.next()? != ':' {
            return None;
        }
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.next()? != '"' {
            return None;
        }
        let value = string_rest(&mut chars)?;
        if key == "dev" {
            return Some(value);
        }
    }
}

/// A JSON string's text, its opening quote already taken.
fn string_rest(chars: &mut impl Iterator<Item = char>) -> Option<String> {
    let mut out = String::new();
    loop {
        match chars.next()? {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
}

/// Which program runs a package's scripts, from the lockfile beside it or at the root.
fn package_manager(root: &Path, folder: &Path) -> &'static str {
    for place in [folder, root] {
        for (file, manager) in [
            ("pnpm-lock.yaml", "pnpm"),
            ("yarn.lock", "yarn"),
            ("bun.lock", "bun"),
            ("bun.lockb", "bun"),
            ("package-lock.json", "npm"),
        ] {
            if place.join(file).is_file() {
                return manager;
            }
        }
    }
    "npm"
}

fn javascript(root: &Path, folder: &Path, dir: Option<String>, relative: &str) -> Option<DraftProcess> {
    let script = dev_script_in(folder)?;
    let manager = package_manager(root, folder);
    let name = safe_name(if relative == "." { "web" } else { relative });
    let first = script.split_whitespace().next().unwrap_or("");
    let (serve, port) = match (first, script.contains("--port")) {
        // A bundler that takes its port as a flag, which no environment variable moves.
        ("vite", false) => {
            let runner = match manager {
                "pnpm" => "pnpm exec",
                "yarn" => "yarn",
                "bun" => "bunx",
                _ => "npx",
            };
            (
                format!("{runner} {} --port ${{port.{name}}} --strictPort", script.trim()),
                Some(DraftPort { name: name.clone(), env: None }),
            )
        }
        ("vite", true) => (format!("{manager} run dev"), None),
        // Anything else reads `PORT`, as most dev servers do.
        _ => (
            match manager {
                "yarn" => String::from("yarn dev"),
                other => format!("{other} run dev"),
            },
            Some(DraftPort { name: name.clone(), env: Some(String::from("PORT")) }),
        ),
    };
    Some(DraftProcess { name, serve, dir, port, from: DraftSource::PackageJson })
}

/// The files of a folder that name Python's dependencies.
const REQUIREMENTS: &[&str] = &["requirements.txt", "requirements-dev.txt", "pyproject.toml", "Pipfile"];

/// Where a FastAPI app is usually made, relative to its folder.
const APP_FILES: &[&str] = &["main.py", "app.py", "app/main.py", "src/main.py", "src/app.py"];

fn python(root: &Path, folder: &Path, dir: Option<String>, relative: &str) -> Option<DraftProcess> {
    let wants_uvicorn = REQUIREMENTS.iter().any(|file| {
        std::fs::read_to_string(folder.join(file))
            .map(|text| {
                let text = text.to_lowercase();
                text.contains("uvicorn") || text.contains("fastapi")
            })
            .unwrap_or(false)
    });
    if !wants_uvicorn {
        return None;
    }
    let (module, variable) = APP_FILES.iter().find_map(|file| {
        let text = std::fs::read_to_string(folder.join(file)).ok()?;
        let variable = text.lines().find_map(app_variable)?;
        Some((file.trim_end_matches(".py").replace('/', "."), variable))
    })?;
    let name = safe_name(if relative == "." { "api" } else { relative });
    let python = interpreter(root, folder, relative);
    Some(DraftProcess {
        serve: format!("{python} -m uvicorn {module}:{variable} --reload --port ${{port.{name}}}"),
        port: Some(DraftPort { name: name.clone(), env: None }),
        name,
        dir,
        from: DraftSource::Python,
    })
}

/// `app` out of `app = FastAPI(...)`, or `app: FastAPI = FastAPI(...)`.
fn app_variable(line: &str) -> Option<String> {
    let (left, right) = line.split_once('=')?;
    if !right.trim_start().starts_with("FastAPI(") {
        return None;
    }
    let name = left.split(':').next()?.trim();
    (!name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then(|| name.to_string())
}

/// The virtual environment's Python where the folder or the root has one, as a path from the
/// folder the process starts in; else the machine's.
fn interpreter(root: &Path, folder: &Path, relative: &str) -> String {
    if folder.join(".venv/bin/python").is_file() {
        return String::from(".venv/bin/python");
    }
    if root.join(".venv/bin/python").is_file() {
        let up = match relative {
            "." => String::new(),
            below => "../".repeat(below.split('/').count()),
        };
        return format!("{up}.venv/bin/python");
    }
    String::from("python3")
}

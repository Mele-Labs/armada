//! The librarian's free drift part: a document linked to a feature that names a type, function or endpoint the code no
//! longer has. `docs/concepts/feature-catalog.md`, *Librarian*. **No model is asked.**
//!
//! A name has drifted where the code had it when the document was last changed and does not have it now. A name the
//! code never had is the document's own, and is no drift. Git is asked and no file is opened: the document is read at
//! `HEAD`, and each name is looked for with one `git grep` at the document's last commit and one at `HEAD`.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use store::{feature_link_kind, KeptDocDrift, Store, WriteError};

/// The most names checked of one document.
pub(crate) const MOST_NAMES: usize = 40;
/// The most documents checked in one run.
pub(crate) const MOST_DOCS: usize = 200;
/// A name shorter than this is too common to say anything.
const SHORTEST: usize = 4;
const METHODS: [&str; 4] = ["GET ", "POST ", "PUT ", "DELETE "];

/// The names a markdown document makes in inline code, once each, in the order it makes them, at most [`MOST_NAMES`].
/// Fenced blocks are left out.
pub(crate) fn names_in(markdown: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().filter(|c| matches!(c, '`' | '~'));
        let run = marker.map_or(0, |mark| trimmed.chars().take_while(|c| *c == mark).count());
        match (fence, marker) {
            (Some((mark, open)), Some(this)) if this == mark && run >= open && trimmed[run..].trim().is_empty() => fence = None,
            (Some(_), _) => {}
            (None, Some(mark)) if run >= 3 => fence = Some((mark, run)),
            (None, _) => {
                for name in spans(line).into_iter().filter_map(name_of) {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
        }
    }
    names.truncate(MOST_NAMES);
    names
}

/// The text between each pair of matching backtick runs on `line`.
fn spans(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at += 1;
            continue;
        }
        let open = at;
        while at < bytes.len() && bytes[at] == b'`' {
            at += 1;
        }
        let width = at - open;
        let mut look = at;
        while look < bytes.len() {
            if bytes[look] != b'`' {
                look += 1;
                continue;
            }
            let start = look;
            while look < bytes.len() && bytes[look] == b'`' {
                look += 1;
            }
            if look - start == width {
                spans.push(line[at..start].trim());
                at = look;
                break;
            }
        }
    }
    spans
}

/// The name a span makes, if it makes one.
fn name_of(span: &str) -> Option<String> {
    let name = endpoint_of(span).or_else(|| identifier_of(span))?;
    (name.chars().count() >= SHORTEST).then_some(name)
}

/// `/features/page`, from `GET /features/page?slug=a`. A file path with an extension is not an endpoint.
fn endpoint_of(span: &str) -> Option<String> {
    let rest = METHODS.iter().find_map(|method| span.strip_prefix(method)).unwrap_or(span).trim_start();
    let path = rest.split(['?', '#']).next()?;
    if !path.starts_with('/') || path.chars().any(|c| !(c.is_ascii_alphanumeric() || "/_-{}:.~".contains(c))) {
        return None;
    }
    if path.chars().filter(char::is_ascii_alphabetic).count() < 2 {
        return None;
    }
    let last = path.rsplit('/').next().unwrap_or_default();
    if last.rsplit_once('.').is_some_and(|(_, ending)| !ending.is_empty() && ending.len() <= 5) {
        return None;
    }
    Some(path.trim_end_matches('/').to_string())
}

fn is_identifier(word: &str) -> bool {
    word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && word.chars().next().is_some_and(|c| !c.is_ascii_digit())
}

/// The last segment of `name()`, `Type::name` or `module::Type`, or of a lone `snake_case` or `CamelCase` word.
fn identifier_of(span: &str) -> Option<String> {
    let inner = span.strip_suffix("()").unwrap_or(span);
    let segments: Vec<&str> = inner.split("::").collect();
    if !segments.iter().all(|segment| is_identifier(segment)) {
        return None;
    }
    let last = segments.last().copied()?;
    let named = inner.len() != span.len() || segments.len() > 1;
    let snake = last.contains('_') && last.chars().any(|c| c.is_ascii_alphabetic());
    let camel = last.chars().filter(char::is_ascii_uppercase).count() >= 2 && last.chars().any(|c| c.is_ascii_lowercase());
    (named || snake || camel).then(|| last.to_string())
}

/// What a check of one document found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Checked {
    /// The commit the document was last changed in.
    pub(crate) commit: String,
    /// The names it gives that the code had then and has not now.
    pub(crate) drifted: Vec<String>,
}

/// The documents linked to `project`'s features that are not addresses, newest read first, each once.
pub(crate) fn linked_docs(store: &Store, project: &str) -> Result<Vec<String>, WriteError> {
    let mut targets: Vec<String> = Vec::new();
    for link in store.project_links(project, feature_link_kind::DOC)? {
        if !link.target.contains("://") && !targets.contains(&link.target) {
            targets.push(link.target);
        }
    }
    Ok(targets)
}

/// `target` as a path inside the repository at `root`, or `None` for one that points elsewhere.
pub(super) fn inside(root: &Path, target: &str) -> Option<String> {
    let path = Path::new(target);
    let relative = if path.is_absolute() {
        match path.strip_prefix(root) {
            Ok(relative) => relative.to_path_buf(),
            Err(_) => path.strip_prefix(root.canonicalize().ok()?).ok()?.to_path_buf(),
        }
    } else {
        path.to_path_buf()
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in relative.components() {
        match part {
            Component::CurDir => {}
            Component::Normal(name) => parts.push(name.to_str()?),
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// What git printed for `args` in `root`, where it ran. `ok` says which exit codes are an answer.
pub(super) fn git(root: &Path, args: &[String], ok: &[i32]) -> Option<String> {
    let out = crate::files::git_in(root).args(args).output().ok()?;
    out.status.code().filter(|code| ok.contains(code))?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Which of `names` a file of the code at `rev` holds, whole words, outside markdown and `doc` itself. `None` where git
/// could not say.
fn present_at(root: &Path, rev: &str, doc: &str, names: &[String]) -> Option<BTreeSet<String>> {
    let mut args: Vec<String> = ["grep", "-h", "-o", "-I", "-F", "-w"].map(String::from).to_vec();
    for name in names {
        args.extend(["-e".to_string(), name.clone()]);
    }
    args.extend([rev.to_string(), "--".to_string(), ".".to_string(), ":!*.md".to_string(), format!(":(exclude,literal){doc}")]);
    // `git grep` answers 1 where nothing matched.
    Some(git(root, &args, &[0, 1])?.lines().map(str::to_string).collect())
}

/// `doc` read in the repository at `root`: `None` where it is not a file of it at `HEAD`.
fn checked_in(root: &Path, doc: &str) -> Option<Checked> {
    let text = git(root, &["show".to_string(), format!("HEAD:{doc}")], &[0])?;
    let commit = git(root, &["log".to_string(), "-1".to_string(), "--format=%H".to_string(), "--".to_string(), format!(":(literal){doc}")], &[0])?.trim().to_string();
    if commit.is_empty() {
        return None;
    }
    let names = names_in(&text);
    // Where git cannot say, nothing is flagged.
    let drifted = (|| {
        if names.is_empty() {
            return Some(Vec::new());
        }
        let then = present_at(root, &commit, doc, &names)?;
        let held: Vec<String> = names.into_iter().filter(|name| then.contains(name)).collect();
        if held.is_empty() {
            return Some(Vec::new());
        }
        let now = present_at(root, "HEAD", doc, &held)?;
        Some(held.into_iter().filter(|name| !now.contains(name)).collect())
    })();
    Some(Checked { commit, drifted: drifted.unwrap_or_default() })
}

/// Each of `targets`, at most [`MOST_DOCS`] that are files of a repository in `roots`, with what its check found. A target
/// that is not one is left out. **Blocking**: the caller runs it off the main thread.
pub(crate) fn check_docs(roots: &[PathBuf], targets: &[String]) -> Vec<(String, Option<Checked>)> {
    let mut found = Vec::new();
    let mut checked = 0;
    for target in targets {
        if checked >= MOST_DOCS {
            break;
        }
        let one = roots.iter().find_map(|root| checked_in(root, &inside(root, target)?));
        checked += usize::from(one.is_some());
        found.push((target.clone(), one));
    }
    found
}

/// Keep what `found` says for `project` at `at`: a document with no drifted name clears its row, and one that is no longer
/// a file of the repository too. How many documents name something the code no longer has.
pub(crate) fn keep_checked(store: &mut Store, project: &str, at: &str, found: Vec<(String, Option<Checked>)>) -> Result<usize, WriteError> {
    let mut flagged = 0;
    for (target, one) in found {
        let (doc_commit, names) = one.map(|one| (one.commit, one.drifted)).unwrap_or_default();
        flagged += usize::from(!names.is_empty());
        store.keep_doc_drift(&KeptDocDrift { project: project.to_string(), target, names, doc_commit, checked_at: at.to_string() })?;
    }
    Ok(flagged)
}

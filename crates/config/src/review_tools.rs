//! Where a review tool comes from, and which ones a repository has.
//! `docs/concepts/pull-review.md`, *Review tools*.
//!
//! **Three places, merged by id, a person's own winning**: the set Armada carries, then the
//! repository's own `.armada/review-tools/`, then Kit's `~/.armada/review-tools/`. The rule is
//! [`crate::Catalogue`]'s for workflows, and so is the other half of it: **a file that does not fit
//! is left out and named, wherever it came from**, and the others stand.
//!
//! A definition is one file, `<id>.md`: front matter naming `kind` (`tool` or `explanation`) and
//! `name`, then the prompt. **The id is the name lower cased with its words joined by a dash**, so
//! two places define the same thing by naming it the same. What shape an answer takes is not a
//! definition's to say: it is Fleet's, by kind and id.
//!
//! **Text in, not directories**, so a test drives it with no file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

use crate::error::{LoadError, Refusal};
use crate::yaml::{self, Table};

/// Where a carried definition says it was read from. Bracketed so it cannot be mistaken for a path.
pub const CARRIED_AT: &str = "<carried by Armada>";

/// The set Armada carries, in the order Bridge draws it.
const CARRIED: [(&str, &str); 8] = [
    ("risks.md", include_str!("../review-tools/risks.md")),
    ("bugs.md", include_str!("../review-tools/bugs.md")),
    (
        "api-breaking-changes.md",
        include_str!("../review-tools/api-breaking-changes.md"),
    ),
    ("how-it-works.md", include_str!("../review-tools/how-it-works.md")),
    ("user-impact.md", include_str!("../review-tools/user-impact.md")),
    ("how-to-test.md", include_str!("../review-tools/how-to-test.md")),
    ("why.md", include_str!("../review-tools/why.md")),
    ("walk.md", include_str!("../review-tools/walk.md")),
];

const KEYS: &[&str] = &["kind", "name"];
const KINDS: &[(&str, ToolKind)] = &[("tool", ToolKind::Tool), ("explanation", ToolKind::Explanation)];
const KIND_WORDS: &[&str] = &["tool", "explanation"];

/// The most characters a name or a prompt may have. A prompt is read by a model on every run, so a
/// file that is a book is a mistake worth naming.
const NAME_AT_MOST: usize = 80;
const PROMPT_AT_MOST: usize = 20_000;

/// Where a definition was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolSource {
    Armada,
    Kit,
    Repository,
}

impl ToolSource {
    pub fn as_wire(self) -> &'static str {
        match self {
            ToolSource::Armada => "armada",
            ToolSource::Kit => "kit",
            ToolSource::Repository => "repository",
        }
    }

    fn possessive(self) -> &'static str {
        match self {
            ToolSource::Armada => "Armada's",
            ToolSource::Kit => "Kit's",
            ToolSource::Repository => "the repository's",
        }
    }
}

/// What a definition is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Tool,
    Explanation,
}

impl ToolKind {
    pub fn as_wire(self) -> &'static str {
        match self {
            ToolKind::Tool => "tool",
            ToolKind::Explanation => "explanation",
        }
    }
}

/// One definition, as text, and the place it was read from. **No public way to say a definition is
/// Armada's**: [`carried`] is the only source of those.
#[derive(Debug, Clone)]
pub struct Written {
    source: ToolSource,
    path: PathBuf,
    text: String,
}

impl Written {
    pub fn in_kit(path: PathBuf, text: String) -> Written {
        Written {
            source: ToolSource::Kit,
            path,
            text,
        }
    }

    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written {
            source: ToolSource::Repository,
            path,
            text,
        }
    }
}

/// Every definition Armada carries.
pub fn carried() -> Vec<Written> {
    CARRIED
        .iter()
        .map(|(file, text)| Written {
            source: ToolSource::Armada,
            path: Path::new(CARRIED_AT).join(file),
            text: (*text).to_string(),
        })
        .collect()
}

/// One definition that fit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def {
    pub id: String,
    pub kind: ToolKind,
    pub name: String,
    pub prompt: String,
    pub source: ToolSource,
    /// The place a definition of the same id was replaced from, where this one replaced one.
    pub replaces: Option<ToolSource>,
    /// The repository, where this is Kit's and hides a repository definition of the same id that
    /// says something else. **Two copies that are the same hide nothing.**
    pub hides: Option<ToolSource>,
}

/// A file that did not fit, and the reason in a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOut {
    pub path: PathBuf,
    pub source: ToolSource,
    pub said: String,
}

/// What the three places come to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolved {
    /// Armada's first, in the order it carries them; a later place keeps the position of the
    /// definition it replaced, and a name nobody had is added at the end.
    pub defs: Vec<Def>,
    pub left_out: Vec<LeftOut>,
}

impl Resolved {
    pub fn get(&self, id: &str) -> Option<&Def> {
        self.defs.iter().find(|one| one.id == id)
    }
}

/// A name as the file that holds it is called: lower case, words joined by a dash.
pub fn id_of(name: &str) -> String {
    let mut id = String::new();
    let mut gap = false;
    for letter in name.trim().chars() {
        if letter.is_ascii_alphanumeric() {
            if gap && !id.is_empty() {
                id.push('-');
            }
            gap = false;
            id.push(letter.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    id
}

/// The text of a definition's file: what a save writes.
pub fn file_of(kind: ToolKind, name: &str, prompt: &str) -> String {
    let quoted = serde_yaml_ng::to_string(name).unwrap_or_else(|_| format!("\"{name}\""));
    format!(
        "---\nkind: {}\nname: {}\n---\n{}\n",
        kind.as_wire(),
        quoted.trim_end(),
        prompt.trim()
    )
}

/// The reasons a definition would not be kept, before anything is written: the checks a file is
/// held to. `Ok` is the id the file would be found by.
pub fn fit(kind: ToolKind, name: &str, prompt: &str) -> Result<String, String> {
    let path = Path::new("review-tool.md");
    parse(path, &file_of(kind, name, prompt)).map(|(_, _, id, _)| id)
}

type Parsed = (ToolKind, String, String, String);

/// Parse one file. Every fault of its front matter is reported, not the first.
fn parse(path: &Path, text: &str) -> Result<Parsed, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let opening = lines.next().unwrap_or("");
    if opening.trim_end() != "---" {
        return Err("it does not open with a `---` line holding its kind and name".to_string());
    }
    let (mut front, mut used, mut closed) = (String::new(), opening.len(), false);
    for line in lines {
        used += line.len();
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        front.push_str(line);
    }
    if !closed {
        return Err("its front matter has no closing `---` line".to_string());
    }
    let (front, prompt) = (front.as_str(), &text[used..]);
    let root: Value = serde_yaml_ng::from_str(front).map_err(|cause| {
        LoadError::NotYaml {
            path: path.to_path_buf(),
            cause,
        }
        .to_string()
    })?;
    let mut out: Vec<Refusal> = Vec::new();
    let mut table = Table::open("", &root, &mut out);
    let (kind, name) = match table.as_mut() {
        Some(table) => (
            table
                .required("kind", &mut out)
                .and_then(|v| yaml::word("kind", v, KINDS, KIND_WORDS, KIND_WORDS, &mut out)),
            table
                .required("name", &mut out)
                .and_then(|v| yaml::text("name", v, &mut out)),
        ),
        None => (None, None),
    };
    if let Some(table) = table {
        table.close(KEYS, &mut out);
    }
    if !out.is_empty() {
        return Err(LoadError::Refused {
            path: path.to_path_buf(),
            refusals: out,
        }
        .to_string());
    }
    let (Some(kind), Some(name)) = (kind, name) else {
        return Err("it names no kind or no name".to_string());
    };
    let name = name.trim().to_string();
    let id = id_of(&name);
    if id.is_empty() {
        return Err(format!("`{name}` has no letters or digits, so it makes no id"));
    }
    if name.chars().count() > NAME_AT_MOST {
        return Err(format!("its name is longer than {NAME_AT_MOST} characters"));
    }
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("it has no prompt after its front matter".to_string());
    }
    if prompt.chars().count() > PROMPT_AT_MOST {
        return Err(format!("its prompt is longer than {PROMPT_AT_MOST} characters"));
    }
    Ok((kind, name, id, prompt))
}

/// Merge the definitions in hand. **The order they arrive in decides nothing** and the source decides
/// everything. Two files in one place sharing an id are a fault in that place: both are left out.
pub fn resolve(written: impl IntoIterator<Item = Written>) -> Resolved {
    let mut left_out = Vec::new();
    let mut placed: BTreeMap<(ToolSource, String), Vec<(Parsed, PathBuf)>> = BTreeMap::new();
    let mut order: Vec<(ToolSource, String)> = Vec::new();
    for one in written {
        match parse(&one.path, &one.text) {
            Ok(parsed) => {
                let key = (one.source, parsed.2.clone());
                if !placed.contains_key(&key) {
                    order.push(key.clone());
                }
                placed.entry(key).or_default().push((parsed, one.path));
            }
            Err(said) => left_out.push(LeftOut {
                path: one.path.clone(),
                source: one.source,
                said: format!("{} was left out, because {said}", one.path.display()),
            }),
        }
    }
    let mut defs: Vec<Def> = Vec::new();
    // Armada's, then the repository's, then Kit's; inside a place, the order the files arrived in.
    for place in [ToolSource::Armada, ToolSource::Repository, ToolSource::Kit] {
        for key in order.iter().filter(|(source, _)| *source == place) {
            let Some(mut files) = placed.remove(key) else { continue };
            if files.len() > 1 {
                let paths: Vec<String> = files.iter().map(|(_, path)| path.display().to_string()).collect();
                for (_, path) in &files {
                    left_out.push(LeftOut {
                        path: path.clone(),
                        source: place,
                        said: format!(
                            "{} was left out, because {} define `{}` in {} place: {}",
                            path.display(),
                            files.len(),
                            key.1,
                            place.possessive(),
                            paths.join(", ")
                        ),
                    });
                }
                continue;
            }
            let ((kind, name, id, prompt), _) = files.remove(0);
            let def = Def {
                id: id.clone(),
                kind,
                name,
                prompt,
                source: place,
                replaces: None,
                hides: None,
            };
            match defs.iter().position(|held| held.id == id) {
                Some(at) => {
                    let replaced = &defs[at];
                    let differs = (replaced.kind, &replaced.name, &replaced.prompt) != (def.kind, &def.name, &def.prompt);
                    let hides = (place == ToolSource::Kit && replaced.source == ToolSource::Repository && differs)
                        .then_some(ToolSource::Repository);
                    defs[at] = Def {
                        replaces: Some(replaced.source),
                        hides,
                        ..def
                    };
                }
                None => defs.push(def),
            }
        }
    }
    Resolved { defs, left_out }
}

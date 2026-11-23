//! Where a judge type comes from, and which ones a person has. `docs/concepts/judge.md`, *Judge types*.
//!
//! **Four places, merged by id, the most specific winning**: the set Armada carries, then the repository's
//! own `.armada/judges/`, then Kit's `~/.armada/judges/` for every project, then Kit's
//! `~/.armada/judges/projects/<project>/` for one. The rule is [`crate::review_tools`]' and so is the other
//! half of it: **a file that does not fit is left out and named, wherever it came from**, and the others stand.
//!
//! A type is one file, `<id>.md`: front matter naming `name`, then the prompt. The id is the name lower cased
//! with its words joined by a dash. What shape an answer takes is not a type's to say: it is Fleet's.
//!
//! **Text in, not directories**, so a test drives it with no file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

use crate::error::{LoadError, Refusal};
use crate::review_tools::{named_and_worded, split_front};
use crate::yaml::{self, Table};

pub use crate::review_tools::{id_of, CARRIED_AT};

/// The type an ask with no `judge` reads as.
pub const ADVERSARIAL: &str = "adversarial";

/// The set Armada carries, in the order Bridge draws it.
const CARRIED: [(&str, &str); 5] = [
    ("adversarial.md", include_str!("../judges/adversarial.md")),
    ("code.md", include_str!("../judges/code.md")),
    ("design.md", include_str!("../judges/design.md")),
    ("system-design.md", include_str!("../judges/system-design.md")),
    ("api-review.md", include_str!("../judges/api-review.md")),
];

const KEYS: &[&str] = &["name"];

/// Where a type was read from, least specific first: a later place answers for an id over an earlier one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    Armada,
    Repository,
    /// `~/.armada/judges/`, every project.
    Kit,
    /// `~/.armada/judges/projects/<project>/`, one.
    KitProject,
}

impl Place {
    fn possessive(self) -> &'static str {
        match self {
            Place::Armada => "Armada's",
            Place::Repository => "the repository's",
            Place::Kit => "Kit's",
            Place::KitProject => "Kit's for the project",
        }
    }
}

/// One type, as text, and the place it was read from. **No public way to say a type is Armada's**:
/// [`carried`] is the only source of those.
#[derive(Debug, Clone)]
pub struct Written {
    place: Place,
    path: PathBuf,
    text: String,
}

impl Written {
    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written { place: Place::Repository, path, text }
    }

    pub fn in_kit(path: PathBuf, text: String) -> Written {
        Written { place: Place::Kit, path, text }
    }

    pub fn in_kit_project(path: PathBuf, text: String) -> Written {
        Written { place: Place::KitProject, path, text }
    }
}

/// Every type Armada carries.
pub fn carried() -> Vec<Written> {
    CARRIED
        .iter()
        .map(|(file, text)| Written {
            place: Place::Armada,
            path: Path::new(CARRIED_AT).join(file),
            text: (*text).to_string(),
        })
        .collect()
}

/// One type that fit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub place: Place,
    /// The place a type of the same id was replaced from, where this one replaced one.
    pub replaces: Option<Place>,
    /// The repository, where this is Kit's and hides a repository type of the same id that says something
    /// else. **Two copies that are the same hide nothing.**
    pub hides: Option<Place>,
}

/// A file that did not fit, and the reason in a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOut {
    pub path: PathBuf,
    pub place: Place,
    pub said: String,
}

/// What the places come to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolved {
    /// Armada's first, in the order it carries them; a later place keeps the position of the type it
    /// replaced, and a name nobody had is added at the end.
    pub defs: Vec<Def>,
    pub left_out: Vec<LeftOut>,
}

impl Resolved {
    pub fn get(&self, id: &str) -> Option<&Def> {
        self.defs.iter().find(|one| one.id == id)
    }
}

/// The text of a type's file: what a save writes.
pub fn file_of(name: &str, prompt: &str) -> String {
    let quoted = serde_yaml_ng::to_string(name).unwrap_or_else(|_| format!("\"{name}\""));
    format!("---\nname: {}\n---\n{}\n", quoted.trim_end(), prompt.trim())
}

/// The reasons a type would not be kept, before anything is written. `Ok` is the id the file would be
/// found by.
pub fn fit(name: &str, prompt: &str) -> Result<String, String> {
    parse(Path::new("judge-type.md"), &file_of(name, prompt)).map(|(_, id, _)| id)
}

type Parsed = (String, String, String);

/// Parse one file. Every fault of its front matter is reported, not the first.
fn parse(path: &Path, text: &str) -> Result<Parsed, String> {
    let (front, prompt) = split_front(text, "its name")?;
    let root: Value = serde_yaml_ng::from_str(&front)
        .map_err(|cause| LoadError::NotYaml { path: path.to_path_buf(), cause }.to_string())?;
    let mut out: Vec<Refusal> = Vec::new();
    let mut table = Table::open("", &root, &mut out);
    let name = table
        .as_mut()
        .and_then(|table| table.required("name", &mut out))
        .and_then(|v| yaml::text("name", v, &mut out));
    if let Some(table) = table {
        table.close(KEYS, &mut out);
    }
    if !out.is_empty() {
        return Err(LoadError::Refused { path: path.to_path_buf(), refusals: out }.to_string());
    }
    let Some(name) = name else {
        return Err("it names no name".to_string());
    };
    named_and_worded(&name, prompt)
}

/// Merge the types in hand. **The order they arrive in decides nothing** and the place decides everything.
/// Two files in one place sharing an id are a fault in that place: both are left out. Only one project's
/// files are ever in hand, so `KitProject` is that project's.
pub fn resolve(written: impl IntoIterator<Item = Written>) -> Resolved {
    let mut left_out = Vec::new();
    let mut placed: BTreeMap<(Place, String), Vec<(Parsed, PathBuf)>> = BTreeMap::new();
    let mut order: Vec<(Place, String)> = Vec::new();
    for one in written {
        match parse(&one.path, &one.text) {
            Ok(parsed) => {
                let key = (one.place, parsed.1.clone());
                if !placed.contains_key(&key) {
                    order.push(key.clone());
                }
                placed.entry(key).or_default().push((parsed, one.path));
            }
            Err(said) => left_out.push(LeftOut {
                path: one.path.clone(),
                place: one.place,
                said: format!("{} was left out, because {said}", one.path.display()),
            }),
        }
    }
    let mut defs: Vec<Def> = Vec::new();
    for place in [Place::Armada, Place::Repository, Place::Kit, Place::KitProject] {
        for key in order.iter().filter(|(held, _)| *held == place) {
            let Some(mut files) = placed.remove(key) else { continue };
            if files.len() > 1 {
                let paths: Vec<String> = files.iter().map(|(_, path)| path.display().to_string()).collect();
                for (_, path) in &files {
                    left_out.push(LeftOut {
                        path: path.clone(),
                        place,
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
            let ((name, id, prompt), _) = files.remove(0);
            let def = Def { id: id.clone(), name, prompt, place, replaces: None, hides: None };
            match defs.iter().position(|held| held.id == id) {
                Some(at) => {
                    let replaced = &defs[at];
                    let differs = (&replaced.name, &replaced.prompt) != (&def.name, &def.prompt);
                    // A person's copy hides the repository's it replaces, or one a nearer copy of theirs already hid.
                    let hides = match (replaced.place, replaced.hides) {
                        (Place::Repository, _) if place >= Place::Kit && differs => Some(Place::Repository),
                        (_, Some(hidden)) => Some(hidden),
                        _ => None,
                    };
                    defs[at] = Def { replaces: Some(replaced.place), hides, ..def };
                }
                None => defs.push(def),
            }
        }
    }
    Resolved { defs, left_out }
}

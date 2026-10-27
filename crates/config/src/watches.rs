//! Watch recipes: a command, how often to run it, and when to give up, kept where a Trigger is.
//! `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`, *Recipes*.
//!
//! Three places, the most specific winning: Armada's, the repository's `.armada/watches/` and this
//! machine's `~/Library/Application Support/Armada/machine/watches/`. Armada's name a provider, so
//! their files are the adapters' to carry and arrive here as [`Written::by_armada`]. Two files
//! naming one recipe are one recipe, and the specific copy replaces the other whole, as
//! [`crate::TriggerCatalogue`] does. Text in, not directories: reading the repository's files from
//! `main` only is the caller's.
//!
//! **The cadence floor and the give-up cap are not checked here.** They are settings a person
//! moves while Fleet runs, so a file that loads now stays loaded, and `start_watch` refuses.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::yaml::{self, Table};

const KEYS: &[&str] = &["name", "command", "every", "gives_up_after", "environment"];
const A_SPAN: &str = "a span such as `60s`, `2m` or `2h`";

/// Where a recipe was read from, least specific first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecipeSource {
    /// Compiled in, from the adapters.
    Armada,
    /// `.armada/watches/` in the repository, read from `main` only.
    Repository,
    /// This machine's own, which does not travel.
    Machine,
}

impl RecipeSource {
    pub fn as_wire(self) -> &'static str {
        match self {
            RecipeSource::Armada => "armada",
            RecipeSource::Repository => "repository",
            RecipeSource::Machine => "machine",
        }
    }
}

impl fmt::Display for RecipeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RecipeSource::Armada => "carried by Armada",
            RecipeSource::Repository => "from the repository",
            RecipeSource::Machine => "from this machine",
        })
    }
}

/// One recipe file as text, and the place it was read from.
#[derive(Debug, Clone)]
pub struct Written {
    source: RecipeSource,
    path: PathBuf,
    text: String,
}

impl Written {
    /// A recipe Armada carries, compiled in.
    pub fn by_armada(path: PathBuf, text: String) -> Written {
        Written { source: RecipeSource::Armada, path, text }
    }

    /// A file from the repository's `.armada/watches/`, as `main` holds it.
    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written { source: RecipeSource::Repository, path, text }
    }

    /// A file from `~/Library/Application Support/Armada/machine/watches/`.
    pub fn on_machine(path: PathBuf, text: String) -> Written {
        Written { source: RecipeSource::Machine, path, text }
    }

    pub fn source(&self) -> RecipeSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// One recipe, as a person wrote it. `command`'s `{name}` items are the values a start fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub name: String,
    pub command: Vec<String>,
    pub every_seconds: u32,
    /// `None` gives up after the machine's `timeouts.watchGiveUpMinutes`.
    pub gives_up_after_seconds: Option<u32>,
    /// A Requests environment whose variables, secrets included, the command is given.
    pub environment: Option<String>,
}

impl Recipe {
    /// The values a start must give, in the order the command names them.
    pub fn values(&self) -> Vec<&str> {
        let mut named: Vec<&str> = Vec::new();
        for slot in self.command.iter().filter_map(|item| core_model::value_slot(item)) {
            if !named.contains(&slot) {
                named.push(slot);
            }
        }
        named
    }
}

/// `recipe` as a file says it, which [`parse`] reads back whole. Each span is written in seconds.
/// **Built as a `Value`**, for the reason this crate reads one: no derive here.
pub fn written(recipe: &Recipe) -> String {
    let mut file = serde_yaml_ng::Mapping::new();
    let text = |value: &str| Value::String(value.to_string());
    file.insert(text("name"), text(&recipe.name));
    file.insert(text("command"), Value::Sequence(recipe.command.iter().map(|item| text(item)).collect()));
    file.insert(text("every"), text(&format!("{}s", recipe.every_seconds)));
    if let Some(seconds) = recipe.gives_up_after_seconds {
        file.insert(text("gives_up_after"), text(&format!("{seconds}s")));
    }
    if let Some(environment) = &recipe.environment {
        file.insert(text("environment"), text(environment));
    }
    serde_yaml_ng::to_string(&Value::Mapping(file)).unwrap_or_default()
}

/// Parse one recipe file, reporting every fault in it and not the first.
pub fn parse(path: &Path, text: &str) -> Result<Recipe, LoadError> {
    let root: Value = serde_yaml_ng::from_str(text).map_err(|cause| LoadError::NotYaml {
        path: path.to_path_buf(),
        cause,
    })?;
    let mut out = Vec::new();
    match read(&root, &mut out) {
        Some(recipe) if out.is_empty() => Ok(recipe),
        _ => Err(LoadError::Refused { path: path.to_path_buf(), refusals: out }),
    }
}

fn read(root: &Value, out: &mut Vec<Refusal>) -> Option<Recipe> {
    let mut table = Table::open("", root, out)?;
    let name = table.required("name", out).and_then(|v| yaml::text("name", v, out));
    let command = table.required("command", out).and_then(|v| command(v, out));
    let every = table.required("every", out).and_then(|v| span("every", v, out));
    let gives_up_after = table.optional("gives_up_after").map(|v| span("gives_up_after", v, out));
    let environment = table.optional("environment").map(|v| yaml::text("environment", v, out));
    table.close(KEYS, out);
    let gives_up_after_seconds = match gives_up_after {
        Some(read) => Some(read?),
        None => None,
    };
    let environment = match environment {
        Some(read) => Some(read?),
        None => None,
    };
    Some(Recipe {
        name: name?,
        command: command?,
        every_seconds: every?,
        gives_up_after_seconds,
        environment,
    })
}

/// A list of whole argument items. **Never a string**: a string would be split by somebody, and
/// a shell is the thing a watch never runs.
fn command(value: &Value, out: &mut Vec<Refusal>) -> Option<Vec<String>> {
    let items = yaml::list("command", value, out)?;
    let before = out.len();
    let read: Vec<Option<String>> = items.iter().map(|(key, item)| yaml::text(key, item, out)).collect();
    (out.len() == before).then(|| read.into_iter().flatten().collect())
}

fn span(key: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<u32> {
    let written = yaml::text(key, value, out)?;
    let seconds = core_model::span_seconds(&written);
    if seconds.is_none() {
        out.push(Refusal::new(key, Fault::WrongType { wanted: A_SPAN, found: "another word" }));
    }
    seconds
}

/// Why a file is not among the recipes.
#[derive(Debug)]
pub enum WhyLeftOut {
    /// It will not parse.
    Unparsed(LoadError),
    /// Another file in the same place names the same recipe. `also` is every file after the first.
    Duplicated { also: Vec<PathBuf> },
}

/// A file set aside, and why.
#[derive(Debug)]
pub struct LeftOut {
    source: RecipeSource,
    path: PathBuf,
    why: WhyLeftOut,
}

impl LeftOut {
    pub fn source(&self) -> RecipeSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn why(&self) -> &WhyLeftOut {
        &self.why
    }
}

impl fmt::Display for LeftOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a watch recipe {} was left out, because ", self.source)?;
        match &self.why {
            WhyLeftOut::Unparsed(why) => write!(f, "{why}"),
            WhyLeftOut::Duplicated { also } => {
                write!(f, "{}", self.path.display())?;
                for path in also {
                    write!(f, " and {}", path.display())?;
                }
                write!(f, " all name the same one")
            }
        }
    }
}

/// One recipe, the place it came from, and the copies it replaced, the most specific first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRecipe {
    pub recipe: Recipe,
    pub source: RecipeSource,
    pub path: PathBuf,
    pub replaced: Vec<(RecipeSource, PathBuf)>,
}

/// The recipes in force, one per name, and every file set aside on the way.
#[derive(Debug)]
pub struct Recipes {
    recipes: Vec<ResolvedRecipe>,
    left_out: Vec<LeftOut>,
}

impl Recipes {
    /// Parse every file and pick the most specific copy of each name. The order files arrive in
    /// decides nothing. Two in one place with one name are both left out; across places it is an
    /// override, and a place's copy that will not parse does not step down to a less specific one.
    pub fn of(written: impl IntoIterator<Item = Written>) -> Recipes {
        let mut placed: BTreeMap<(String, RecipeSource), Vec<(Recipe, PathBuf)>> = BTreeMap::new();
        let mut left_out = Vec::new();
        for one in written {
            match parse(&one.path, &one.text) {
                Ok(recipe) => placed.entry((recipe.name.clone(), one.source)).or_default().push((recipe, one.path)),
                Err(why) => left_out.push(LeftOut { source: one.source, path: one.path, why: WhyLeftOut::Unparsed(why) }),
            }
        }
        let mut held: BTreeMap<String, Vec<(RecipeSource, Recipe, PathBuf)>> = BTreeMap::new();
        for ((name, source), mut copies) in placed {
            if copies.len() > 1 {
                let also = copies.drain(1..).map(|(_, path)| path).collect();
                let path = copies.remove(0).1;
                left_out.push(LeftOut { source, path, why: WhyLeftOut::Duplicated { also } });
                continue;
            }
            let (recipe, path) = copies.remove(0);
            held.entry(name).or_default().push((source, recipe, path));
        }
        let recipes = held
            .into_values()
            .filter_map(|mut copies| {
                copies.sort_by_key(|(source, _, _)| *source);
                let (source, recipe, path) = copies.pop()?;
                let replaced = copies.into_iter().rev().map(|(source, _, path)| (source, path)).collect();
                Some(ResolvedRecipe { recipe, source, path, replaced })
            })
            .collect();
        Recipes { recipes, left_out }
    }

    /// One per name, in name order.
    pub fn recipes(&self) -> &[ResolvedRecipe] {
        &self.recipes
    }

    pub fn named(&self, name: &str) -> Option<&ResolvedRecipe> {
        self.recipes.iter().find(|one| one.recipe.name == name)
    }

    /// Kept with the reason, so an agent naming one is told why it is not there.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }
}

//! The features a note on a screen belongs to, as Fleet maps them (`docs/concepts/job-story.md`, An ask from outside a
//! Job). The files a note's element was drawn from, `source` and `definition`, go to `get_features_of_path`, each made
//! relative to the repository; where no file maps, the component's name is read against each feature's slug and title in
//! the library, which carries no paths of its own. Everything here but `of_path` and `resolve` is pure.

use std::path::Path;

use gpui::{App, Task};
use ipc::{FeatureJob, FeatureLibrary, FeatureOfPath, FeaturesOfPath, PathHow};
use serde_json::Value;

use crate::settings::wire::{request, Ask, Refused};

/// The most features one note carries, as many as the hooks hand a Job from one read.
pub const MOST: usize = 3;

/// Where a note says its element came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Place {
    /// Files from the repository root, `source` first and then `definition`.
    pub files: Vec<String>,
    /// Component names, the innermost first.
    pub components: Vec<String>,
}

/// A feature a note maps to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapped {
    pub project: String,
    pub slug: String,
    pub title: String,
    /// The Jobs on it now, which a note sent to the feature goes to.
    pub jobs: Vec<FeatureJob>,
}

/// A file made relative to the repository `root`, as Fleet reads a path. One outside the repository has none.
pub fn relative(file: &str, root: Option<&Path>) -> Option<String> {
    let file = file.trim();
    let path = Path::new(file);
    let inside = if path.is_absolute() {
        path.strip_prefix(root?).ok()?.to_string_lossy().into_owned()
    } else {
        file.strip_prefix("./").unwrap_or(file).to_string()
    };
    (!inside.is_empty()).then_some(inside)
}

/// What a note's file records of where its element came from. `source` and `definition` are what Electron's layer
/// stamps; a note the native layer wrote has neither, and a component only where it knew one, but `paths`, the files
/// that draw the screen it was made on (`annotate::drawn_by`).
pub fn place_of(raw: &Value, root: Option<&Path>) -> Place {
    let mut place = Place::default();
    let paths = raw["paths"].as_array().into_iter().flatten();
    for file in [&raw["source"]["file"], &raw["definition"]["file"]].into_iter().chain(paths) {
        if let Some(file) = file.as_str().and_then(|file| relative(file, root)) {
            if !place.files.contains(&file) {
                place.files.push(file);
            }
        }
    }
    let owners = raw["owners"].as_array().into_iter().flatten();
    for name in [&raw["component"], &raw["definition"]["component"]].into_iter().chain(owners.take(6)) {
        if let Some(name) = name.as_str().map(str::trim).filter(|name| !name.is_empty()) {
            if !place.components.iter().any(|one| one == name) {
                place.components.push(name.to_string());
            }
        }
    }
    place
}

/// Every feature the answers name, the first answer's first, each once.
pub fn merged(answers: &[FeaturesOfPath]) -> Vec<Mapped> {
    let mut found: Vec<Mapped> = Vec::new();
    for one in answers.iter().flat_map(|answer| &answer.features) {
        if !found.iter().any(|seen| seen.slug == one.slug && seen.project == one.project) {
            found.push(Mapped { project: one.project.clone(), slug: one.slug.clone(), title: one.title.clone(), jobs: one.jobs.clone() });
        }
    }
    found.truncate(MOST);
    found
}

/// Words of a name, lower case and split at a capital, a digit or anything that is not a letter: `JobThread` and
/// `job-thread` are the same two words.
fn words(name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut before_lower = false;
    for ch in name.chars() {
        if !ch.is_alphanumeric() {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            before_lower = false;
            continue;
        }
        if ch.is_uppercase() && before_lower && !word.is_empty() {
            out.push(std::mem::take(&mut word));
        }
        before_lower = ch.is_lowercase() || ch.is_numeric();
        word.extend(ch.to_lowercase());
    }
    if !word.is_empty() {
        out.push(word);
    }
    out.into_iter().map(|word| stem(&word)).collect()
}

/// A plural and its singular are one word.
fn stem(word: &str) -> String {
    match word.strip_suffix('s') {
        Some(rest) if rest.len() > 2 && !rest.ends_with('s') => rest.to_string(),
        _ => word.to_string(),
    }
}

/// What a component's name says about what it is, not about what it draws.
const SHAPES: &[&str] = &["view", "panel", "screen", "page", "tab", "card", "row", "item", "button", "sheet", "overlay", "layer"];

/// The features whose slug and title hold every word of a component's name, taken in the library's order. Only the first
/// name of `names`, innermost first, that matches anything counts.
pub fn by_component(library: &FeatureLibrary, names: &[String]) -> Vec<Mapped> {
    for name in names {
        let wanted: Vec<String> = words(name).into_iter().filter(|word| !SHAPES.contains(&word.as_str())).collect();
        if wanted.is_empty() {
            continue;
        }
        let found: Vec<Mapped> = library
            .areas
            .iter()
            .flat_map(|area| &area.features)
            .filter(|row| {
                let mut held = words(&row.slug);
                held.extend(words(&row.title));
                wanted.iter().all(|word| held.contains(word))
            })
            .map(|row| Mapped { project: library.project.clone(), slug: row.slug.clone(), title: row.title.clone(), jobs: row.jobs_now.clone() })
            .take(MOST)
            .collect();
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

/// `GET /features/of_path?path=&project=&manifest_id=`: the features a file belongs to.
/// `crates/ipc/operations/get_features_of_path.toml`.
pub fn of_path(port: u16, project: Option<&str>, path: &str, cx: &App) -> Task<Result<FeaturesOfPath, Refused>> {
    let mut route = format!("/features/of_path?path={}", crate::link::percent_encoded(path));
    if let Some(project) = project {
        route.push_str(&format!("&project={}", crate::link::percent_encoded(project)));
    }
    request(port, Ask::Get(route), "FeaturesOfPath", cx)
}

/// What a read of Fleet asks: its port, the project the title bar is scoped to, and the projects Fleet lists.
#[derive(Clone, Debug)]
pub struct Wanted {
    pub port: u16,
    pub project: Option<String>,
    pub projects: Vec<String>,
}

/// The features of a note's place, off the main thread: each file, and the component's name where no file mapped. A read
/// Fleet refused is a feature fewer, never a failure of the note.
pub async fn resolve(wanted: Wanted, place: Place, cx: &gpui::AsyncApp) -> Vec<Mapped> {
    let asks: Vec<_> = place.files.iter().map(|file| cx.update(|cx| of_path(wanted.port, wanted.project.as_deref(), file, cx))).collect();
    let mut answers = Vec::new();
    for ask in asks {
        if let Ok(answer) = ask.await {
            answers.push(answer);
        }
    }
    let found = merged(&answers);
    if !found.is_empty() || place.components.is_empty() {
        return found;
    }
    let asked: Vec<String> = wanted.project.clone().map_or_else(|| wanted.projects.clone(), |one| vec![one]);
    for project in asked {
        let read = cx.update(|cx| super::wire::library(wanted.port, &project, cx));
        if let Ok(library) = read.await {
            let found = by_component(&library, &place.components);
            if !found.is_empty() {
                return found;
            }
        }
    }
    Vec::new()
}

/// What the mock answers for a file: three prefixes, two of which are owned by two features, as the library has them.
pub fn mock_of_path(path: &str) -> FeaturesOfPath {
    const OWNED: &[(&str, &[&str])] = &[
        ("apps/desktop/src/renderer/src/dispatch/", &["start-a-job"]),
        ("apps/desktop/src/renderer/src/jobs/", &["job-list-and-live-status", "pause-resume-stop-snooze"]),
        ("crates/bridge-native/src/dispatch/", &["start-a-job"]),
    ];
    let library = super::mock::library();
    let features = OWNED
        .iter()
        .filter(|(prefix, _)| path.starts_with(prefix))
        .flat_map(|(prefix, slugs)| slugs.iter().map(move |slug| (*prefix, *slug)))
        .filter_map(|(prefix, slug)| {
            let row = library.areas.iter().flat_map(|area| &area.features).find(|row| row.slug == slug)?;
            Some(FeatureOfPath {
                project: library.project.clone(),
                slug: row.slug.clone(),
                title: row.title.clone(),
                area: row.area.clone(),
                how: PathHow::Prefix,
                prefix: Some(prefix.to_string()),
                jobs: row.jobs_now.clone(),
            })
        })
        .collect();
    FeaturesOfPath { path: path.to_string(), features }
}

/// The mock's reading of a place, as `resolve` reads Fleet's.
pub fn mock_resolve(place: &Place) -> Vec<Mapped> {
    let answers: Vec<FeaturesOfPath> = place.files.iter().map(|file| mock_of_path(file)).collect();
    let found = merged(&answers);
    if found.is_empty() {
        by_component(&super::mock::library(), &place.components)
    } else {
        found
    }
}

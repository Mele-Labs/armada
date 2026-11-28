//! Whose Operator hears his words where he named no project: the router in All. **No memory and no model.** Seven signals,
//! in order, and the first that names one project decides; where none does the words are kept and he is asked which.
//! `crates/ipc/operations/talk_to_operator.toml`.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::operator_talk::TalkToOperator;

use super::turning::turn_number;
use crate::daemon::Fleet;

/// The most projects an asking card offers.
pub(crate) const LIKELIEST: usize = 4;

/// A repository a project holds, as he names it and where its checkout is.
#[derive(Clone, Debug)]
pub(crate) struct RouteRepository {
    pub(crate) name: String,
    pub(crate) root: PathBuf,
}

/// One project as the router reads it.
#[derive(Clone, Debug)]
pub(crate) struct RouteProject {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) repositories: Vec<RouteRepository>,
    /// The slugs of its features.
    pub(crate) features: Vec<String>,
    /// Its own switch is on, so its Operator may hear him.
    pub(crate) on: bool,
    /// Its Operator's session runs now.
    pub(crate) running: bool,
}

/// Everything the router goes by.
pub(crate) struct RouteInput<'a> {
    pub(crate) talk: &'a TalkToOperator,
    /// The project of the card `about` names, where it names one Fleet holds.
    pub(crate) card_project: Option<String>,
    /// The project of the Job `looking_at` names.
    pub(crate) looking_at_project: Option<String>,
    pub(crate) projects: Vec<RouteProject>,
    /// The project he last said something to.
    pub(crate) last_talked: Option<String>,
}

/// Where his words go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Routed {
    To(String),
    /// None is sure: the likeliest projects, most likely first.
    Unsure(Vec<String>),
}

/// Whether `needle` stands in `haystack` as a whole word, ignoring case.
fn names_word(haystack: &str, needle: &str) -> bool {
    let (haystack, needle) = (haystack.to_lowercase(), needle.trim().to_lowercase());
    if needle.is_empty() {
        return false;
    }
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    haystack.match_indices(&needle).any(|(at, found)| {
        let before = haystack[..at].chars().next_back();
        let after = haystack[at + found.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// The `#slug` tokens of `words`, lowercased.
fn feature_tags(words: &str) -> Vec<String> {
    words
        .split('#')
        .skip(1)
        .map(|rest| rest.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect::<String>().to_lowercase())
        .filter(|slug| !slug.is_empty())
        .collect()
}

/// The words that read as a path: they hold a slash.
fn paths_in(words: &str) -> Vec<&str> {
    words.split_whitespace().map(|token| token.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ',' | ';' | ':' | '(' | ')' | '.'))).filter(|token| token.contains('/')).collect()
}

/// Whether a path in his words is in a checkout of `project`.
fn holds_path(project: &RouteProject, paths: &[&str], exists: &dyn Fn(&Path) -> bool) -> bool {
    paths.iter().any(|path| {
        let path = Path::new(path);
        project.repositories.iter().any(|repository| match path.is_absolute() {
            true => path.starts_with(&repository.root) && exists(path),
            false => exists(&repository.root.join(path)),
        })
    })
}

/// How many of the projects in `of` there are, and the one where there is exactly one.
fn only(of: &[usize], projects: &[RouteProject]) -> Option<String> {
    match of {
        [one] => Some(projects[*one].id.clone()),
        _ => None,
    }
}

/// Route `input`. `exists` says whether a path is a file or folder on this machine, so the router itself touches nothing.
pub(crate) fn route(input: &RouteInput<'_>, exists: &dyn Fn(&Path) -> bool) -> Routed {
    // 1. A card he talks about is its Operator's.
    if let Some(project) = input.card_project.clone() {
        return Routed::To(project);
    }
    // 2. The project of the screen he is on.
    if let Some(project) = input.talk.project.clone().filter(|project| !project.is_empty()) {
        return Routed::To(project);
    }
    // 3. The Job he has open.
    if let Some(project) = input.looking_at_project.clone() {
        return Routed::To(project);
    }
    let words = input.talk.words.as_str();
    let (tags, paths) = (feature_tags(words), paths_in(words));
    let (mut by_name, mut by_tag, mut by_path) = (Vec::new(), Vec::new(), Vec::new());
    // How many signals hit each project, which orders the ones to ask about.
    let mut hits = vec![0usize; input.projects.len()];
    for (at, project) in input.projects.iter().enumerate() {
        let named = names_word(words, &project.name) || project.repositories.iter().any(|repository| names_word(words, &repository.name));
        let tagged = project.features.iter().any(|slug| tags.contains(&slug.to_lowercase()));
        let pathed = holds_path(project, &paths, exists);
        for (hit, list) in [(named, &mut by_name), (tagged, &mut by_tag), (pathed, &mut by_path)] {
            if hit {
                list.push(at);
                hits[at] += 1;
            }
        }
    }
    // 4 to 6. One project named, one project's feature tagged, one project's checkout holding a path he gave.
    for found in [&by_name, &by_tag, &by_path] {
        if let Some(project) = only(found, &input.projects) {
            return Routed::To(project);
        }
    }
    // 7. Only one Operator runs.
    let running: Vec<usize> = (0..input.projects.len()).filter(|at| input.projects[*at].running).collect();
    if let Some(project) = only(&running, &input.projects) {
        return Routed::To(project);
    }
    let mut asked: Vec<usize> = (0..input.projects.len()).filter(|at| input.projects[*at].on).collect();
    // Most hits first, then the one he last talked to, then as they stand.
    asked.sort_by_key(|at| (std::cmp::Reverse(hits[*at]), input.last_talked.as_deref() != Some(input.projects[*at].id.as_str())));
    Routed::Unsure(asked.into_iter().take(LIKELIEST).map(|at| input.projects[at].id.clone()).collect())
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Each project of his own as the router reads it.
    async fn route_projects(&self) -> Vec<RouteProject> {
        let kept = self.store().lock().await.projects().map(|kept| kept.projects).unwrap_or_default();
        let held = self.projects_held();
        let served = self.repositories().hosted();
        let mut projects = Vec::new();
        for one in kept {
            let repositories = core_model::ScopeId::new(&one.id)
                .ok()
                .and_then(|scope| held.get(&scope).map(|project| project.repositories().into_iter().cloned().collect::<Vec<_>>()))
                .unwrap_or_default()
                .iter()
                .filter_map(|id| served.iter().find(|repository| repository.manifest().id() == id))
                .map(|repository| RouteRepository { name: self.board_repository(repository.manifest().id().as_str()).unwrap_or_default(), root: PathBuf::from(repository.root()) })
                .collect();
            let features = self.store().lock().await.features_in(&one.id).map(|kept| kept.into_iter().map(|feature| feature.slug).collect()).unwrap_or_default();
            projects.push(RouteProject {
                on: self.operator_settings_of(&one.id).await.on,
                running: self.operator_session_id(&one.id).await.is_some(),
                id: one.id,
                name: one.name,
                repositories,
                features,
            });
        }
        projects
    }

    /// Whose Operator hears `talk`, by the seven signals. Where none is sure, **the one project whose Operator is wanted** takes
    /// it while none runs, else `Routed::Unsure` names the likeliest.
    pub(crate) async fn talk_routed(&self, talk: &TalkToOperator) -> Routed {
        let card_project = match talk.about.as_deref().and_then(turn_number) {
            Some(n) => {
                let row = self.store().lock().await.operator_turn_item(n).ok().flatten().filter(|row| row.card.is_some());
                match row {
                    Some(row) => self.card_project(&row).await,
                    None => None,
                }
            }
            None => None,
        };
        let looking_at_project = match &talk.looking_at {
            Some(job) => self.project_of_job(job.as_str()).await,
            None => None,
        };
        let input = RouteInput { talk, card_project, looking_at_project, projects: self.route_projects().await, last_talked: self.operator_seat().last_said_project() };
        match route(&input, &|path| path.exists()) {
            Routed::Unsure(candidates) => match self.operator_projects_wanted().await.as_slice() {
                [project] => Routed::To(project.clone()),
                _ => Routed::Unsure(candidates),
            },
            sure => sure,
        }
    }
}

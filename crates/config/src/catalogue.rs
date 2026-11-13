//! Where a workflow comes from, and which one a repository runs.
//!
//! **Four sources, merged by `workflow_id`: yours first, then narrower.** A
//! repository runs, for each id, the first of: a project it belongs to, Kit's,
//! its own, and the set Armada carries. So a repository nobody set up
//! dispatches on the carried set, one file in a later place replaces a carried
//! definition by id (#425), a teammate's commit never replaces what a person
//! keeps in Kit, and what a person keeps for a project never loses to Kit's for
//! every project. Where Kit's or a project's hides a repository file that says
//! something else, the running file says so ([`WorkflowFile::hides`]).
//!
//! **Lifecycles and rules rank the other way** (`resolving`): scope first, then
//! whose. A workflow is a person's own way of working and a piece is a rule
//! about a place, and the owner chose each order for its own.
//!
//! **Which projects apply is the caller's**: it hands a repository only the
//! files of projects that hold it ([`Written::in_project`]), and a file of one
//! that does not is never read here.
//!
//! **Text in, not directories**, so an acceptance test drives it with no file.
//!
//! **A definition that does not fit is left out, wherever it came from.** One
//! that will not parse, will not resolve against this repository, or shares its
//! id with another file in the same place is set aside and named, and the next
//! place down answers for its id. The repository's own is held to the rule Kit's
//! and Armada's are, at start and on every re-read: a file somebody wrote badly
//! is not grounds for refusing the repository, and they are owed the reason
//! where they look for the workflow. One a more specific place replaced is never
//! resolved, so its Checks have no Job behind them.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use core_model::{ScopeId, WorkflowId, WorkflowSource};

use crate::error::{LoadError, ResolveError};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::roster::Roster;
use crate::workflow::WorkflowDef;

/// Where a carried definition says it was read from. **Not a file** — a name
/// for a refusal to cite, bracketed so it cannot be mistaken for a path
/// relative to a repository.
pub const CARRIED_AT: &str = "<carried by Armada>";

/// Files under `.armada/workflows/` that are this repository's own workflows,
/// for its contributors, and that Armada does not carry.
pub const REPOSITORY_ONLY: &[&str] = &["annotations.json", "doc-fix.json"];

/// The set Armada carries: this repository's own nine, compiled in.
///
/// **The files and not copies of them**, so an edit to one is an edit to what
/// every repository runs, and `tests/carried.rs` fails if a tenth is added
/// under `.armada/workflows/` without being listed here or in
/// [`REPOSITORY_ONLY`].
const CARRIED: [(&str, &str); 9] = [
    (
        "bug.json",
        include_str!("../../../.armada/workflows/bug.json"),
    ),
    (
        "create-workflow.json",
        include_str!("../../../.armada/workflows/create-workflow.json"),
    ),
    (
        "design-plan.json",
        include_str!("../../../.armada/workflows/design-plan.json"),
    ),
    (
        "feature.json",
        include_str!("../../../.armada/workflows/feature.json"),
    ),
    (
        "orchestrate.json",
        include_str!("../../../.armada/workflows/orchestrate.json"),
    ),
    (
        "prototype.json",
        include_str!("../../../.armada/workflows/prototype.json"),
    ),
    (
        "refactor.json",
        include_str!("../../../.armada/workflows/refactor.json"),
    ),
    (
        "revert.json",
        include_str!("../../../.armada/workflows/revert.json"),
    ),
    (
        "uat.json",
        include_str!("../../../.armada/workflows/uat.json"),
    ),
];

/// One definition, as text, and the place it was read from.
///
/// **No public way to say a definition is Armada's.** [`carried`] is the only
/// source of those, so a file on a machine cannot pass itself off as the set
/// the binary carries.
#[derive(Debug, Clone)]
pub struct Written {
    source: WorkflowSource,
    project: Option<ScopeId>,
    path: PathBuf,
    text: String,
}

impl Written {
    /// A definition read from Kit's Workflows.
    pub fn in_kit(path: PathBuf, text: String) -> Written {
        Written {
            source: WorkflowSource::Kit,
            project: None,
            path,
            text,
        }
    }

    /// A definition read from Kit's folder for one project. **Handed only for a
    /// project that holds the repository the catalogue is for.**
    pub fn in_project(path: PathBuf, text: String, project: ScopeId) -> Written {
        Written {
            source: WorkflowSource::Project,
            project: Some(project),
            path,
            text,
        }
    }

    /// A definition read from the repository's own `.armada/workflows/`.
    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written {
            source: WorkflowSource::Repository,
            project: None,
            path,
            text,
        }
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
    }

    /// The project a [`WorkflowSource::Project`] file is kept for.
    pub fn project(&self) -> Option<&ScopeId> {
        self.project.as_ref()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Every definition Armada carries, in file-name order.
pub fn carried() -> Vec<Written> {
    CARRIED
        .iter()
        .map(|(file, text)| Written {
            source: WorkflowSource::Armada,
            project: None,
            path: Path::new(CARRIED_AT).join(file),
            text: (*text).to_string(),
        })
        .collect()
}

/// Every definition in hand, grouped by id, and the ones already set aside.
#[derive(Debug)]
pub struct Catalogue {
    held: BTreeMap<WorkflowId, Vec<Held>>,
    left_out: Vec<LeftOut>,
}

/// One definition that parsed, with the place it came from.
#[derive(Debug)]
struct Held {
    def: WorkflowDef,
    source: WorkflowSource,
    project: Option<ScopeId>,
    text: String,
}

impl Catalogue {
    /// Parse every definition, keeping each beside the others for its id.
    ///
    /// **The order they arrive in decides nothing**, and the source decides
    /// everything. Two definitions sharing an id *and* a place are a fault in
    /// that place: both are left out, naming both. Across places, it is an
    /// override. **Projects are one place**: two of them keeping an id that a
    /// repository both belong to is a fault the same way, and names both.
    pub fn of(written: impl IntoIterator<Item = Written>, roster: &Roster) -> Catalogue {
        let mut placed: BTreeMap<(WorkflowSource, WorkflowId), Vec<(WorkflowDef, Option<ScopeId>, String)>> =
            BTreeMap::new();
        let mut left_out = Vec::new();
        for one in written {
            let def = match WorkflowDef::parse(&one.path, &one.text, roster) {
                Ok(def) => def,
                Err(why) => {
                    left_out.push(
                        LeftOut::of(None, one.source, one.path, WhyLeftOut::Unparsed(why))
                            .in_project(one.project),
                    );
                    continue;
                }
            };
            placed
                .entry((one.source, def.id().clone()))
                .or_default()
                .push((def, one.project, one.text));
        }

        let mut held: BTreeMap<WorkflowId, Vec<Held>> = BTreeMap::new();
        for ((source, id), mut defs) in placed {
            if defs.len() > 1 {
                let projects: Vec<(ScopeId, PathBuf)> = defs
                    .iter()
                    .filter_map(|(def, project, _)| {
                        project.clone().map(|project| (project, def.path().to_path_buf()))
                    })
                    .collect();
                let (first_project, first) = {
                    let (def, project, _) = &defs[0];
                    (project.clone(), def.path().to_path_buf())
                };
                let across = projects.iter().any(|(project, _)| Some(project) != first_project.as_ref());
                let why = if across {
                    WhyLeftOut::HeldByTwoProjects { files: projects }
                } else {
                    let also = defs
                        .drain(1..)
                        .map(|(def, _, _)| def.path().to_path_buf())
                        .collect();
                    WhyLeftOut::Duplicated { also }
                };
                let project = if across { None } else { first_project };
                left_out.push(LeftOut::of(Some(id), source, first, why).in_project(project));
                continue;
            }
            let (def, project, text) = defs.remove(0);
            held.entry(id).or_default().push(Held { def, source, project, text });
        }
        Catalogue { held, left_out }
    }

    /// Resolve the most specific definition for each id against the
    /// repository's Manifest, stepping down a place past any that will not.
    ///
    /// **Nothing here refuses the set.** A definition that names a Check the
    /// repository does not declare is left out with the reason, whoever wrote
    /// it, and the next place down answers for its id.
    pub fn resolve(self, manifest: &Manifest) -> ResolvedCatalogue {
        self.resolve_gated(manifest, &[manifest])
    }

    /// [`resolve`](Catalogue::resolve) over a gating set: the root and every
    /// workspace, in the order their Checks are expanded.
    pub fn resolve_gated(self, manifest: &Manifest, gating: &[&Manifest]) -> ResolvedCatalogue {
        let mut workflows: BTreeMap<WorkflowId, ResolvedWorkflow> = BTreeMap::new();
        let mut files: Vec<WorkflowFile> = Vec::new();
        let mut left_out = self.left_out;
        for (id, mut candidates) in self.held {
            candidates.sort_by(|a, b| b.source.cmp(&a.source));
            // The place that runs, and where its file sits in `files`.
            let mut winner: Option<(WorkflowSource, usize)> = None;
            for Held { def, source, project, text } in candidates {
                if let Some((by, at)) = winner {
                    if hidden(by, source, files[at].text(), &text) {
                        files[at].hides = Some(source);
                    }
                    files.push(WorkflowFile::of(&id, source, project, &def, text, Some(by)));
                    continue;
                }
                match ResolvedWorkflow::resolve_gated(&def, manifest, gating) {
                    Ok(resolved) => {
                        workflows.insert(id.clone(), resolved.read_from(source));
                        files.push(WorkflowFile::of(&id, source, project, &def, text, None));
                        winner = Some((source, files.len() - 1));
                    }
                    Err(why) => left_out.push(
                        LeftOut::of(
                            Some(id.clone()),
                            source,
                            def.path().to_path_buf(),
                            WhyLeftOut::Unresolved(why),
                        )
                        .in_project(project),
                    ),
                }
            }
        }
        // Which place answers for each left-out id, so the sentence can say
        // whose definition a person is running instead of their own.
        for left in &mut left_out {
            left.instead = left
                .id
                .as_ref()
                .and_then(|id| workflows.get(id))
                .map(ResolvedWorkflow::source);
        }
        ResolvedCatalogue {
            workflows,
            left_out,
            files,
        }
    }
}

/// Whether a person's own definition hides a repository's that says something
/// else. **Two copies that are the same hide nothing**, so they carry no mark.
fn hidden(by: WorkflowSource, under: WorkflowSource, running: &str, replaced: &str) -> bool {
    matches!(by, WorkflowSource::Kit | WorkflowSource::Project)
        && under == WorkflowSource::Repository
        && running.trim() != replaced.trim()
}

/// The workflows a repository runs, and every definition set aside on the way.
#[derive(Debug)]
pub struct ResolvedCatalogue {
    workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    left_out: Vec<LeftOut>,
    files: Vec<WorkflowFile>,
}

impl ResolvedCatalogue {
    /// One per id, each knowing its source.
    pub fn workflows(&self) -> &BTreeMap<WorkflowId, ResolvedWorkflow> {
        &self.workflows
    }

    /// **Kept, with the reason, so a person picking a workflow can be told why
    /// one they wrote is not there** — at start today, and later where they
    /// pick.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }

    /// **Every file that parsed and was not left out**, the one that runs and
    /// each one a more specific place replaced, with the text it was read from.
    /// What an editor opens, and what a list draws a dashed row for.
    pub fn files(&self) -> &[WorkflowFile] {
        &self.files
    }

    pub fn into_parts(
        self,
    ) -> (
        BTreeMap<WorkflowId, ResolvedWorkflow>,
        Vec<LeftOut>,
        Vec<WorkflowFile>,
    ) {
        (self.workflows, self.left_out, self.files)
    }
}

/// One definition file that was read, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowFile {
    id: WorkflowId,
    source: WorkflowSource,
    project: Option<ScopeId>,
    path: PathBuf,
    text: String,
    def: WorkflowDef,
    overridden_by: Option<WorkflowSource>,
    hides: Option<WorkflowSource>,
}

impl WorkflowFile {
    fn of(
        id: &WorkflowId,
        source: WorkflowSource,
        project: Option<ScopeId>,
        def: &WorkflowDef,
        text: String,
        overridden_by: Option<WorkflowSource>,
    ) -> WorkflowFile {
        WorkflowFile {
            id: id.clone(),
            source,
            project,
            path: def.path().to_path_buf(),
            text,
            def: def.clone(),
            overridden_by,
            hides: None,
        }
    }

    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
    }

    /// The project a [`WorkflowSource::Project`] file is kept for. `None` of any other.
    pub fn project(&self) -> Option<&ScopeId> {
        self.project.as_ref()
    }

    /// Where it was read from. A bracketed name, not a path, for a carried one.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The file as it was read, whole.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The file parsed, so a place that replaced this one can still be read step by step without
    /// parsing again. **Not resolved**: one a more specific place replaced never is.
    pub fn def(&self) -> &WorkflowDef {
        &self.def
    }

    /// The place that answers for this id instead. `None` where this one runs.
    pub fn overridden_by(&self) -> Option<WorkflowSource> {
        self.overridden_by
    }

    /// The place whose different definition of this id this one hides from the
    /// person: the repository's, where Kit's or a project's runs over it. `None` where the
    /// two are the same, or nothing is hidden.
    pub fn hides(&self) -> Option<WorkflowSource> {
        self.hides
    }
}

/// A definition from Kit or Armada this repository runs without.
#[derive(Debug)]
pub struct LeftOut {
    /// `None` where the file did not parse far enough to say.
    id: Option<WorkflowId>,
    source: WorkflowSource,
    /// The project the file is kept for, where one project's file was left out alone.
    project: Option<ScopeId>,
    path: PathBuf,
    why: WhyLeftOut,
    /// The place whose definition of the same id runs instead, if any does.
    instead: Option<WorkflowSource>,
}

/// Why a definition was left out.
#[derive(Debug)]
pub enum WhyLeftOut {
    /// It will not parse.
    Unparsed(LoadError),
    /// It parses, and names what this repository does not declare, or says
    /// something else about what it does.
    Unresolved(ResolveError),
    /// Another file in the same place declares the same id, and nothing picks
    /// between them. `also` is every file after the first.
    Duplicated { also: Vec<PathBuf> },
    /// Two or more projects that hold this repository each keep a file of the
    /// id, and nothing picks between them. Every file, with the project it is
    /// kept for.
    HeldByTwoProjects { files: Vec<(ScopeId, PathBuf)> },
}

impl LeftOut {
    fn of(
        id: Option<WorkflowId>,
        source: WorkflowSource,
        path: PathBuf,
        why: WhyLeftOut,
    ) -> LeftOut {
        LeftOut {
            id,
            source,
            project: None,
            path,
            why,
            instead: None,
        }
    }

    fn in_project(mut self, project: Option<ScopeId>) -> LeftOut {
        self.project = project;
        self
    }

    pub fn id(&self) -> Option<&WorkflowId> {
        self.id.as_ref()
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
    }

    /// The project the file is kept for. `None` where it is not a project's, or where two projects'
    /// files were left out together ([`WhyLeftOut::HeldByTwoProjects`] names them).
    pub fn project(&self) -> Option<&ScopeId> {
        self.project.as_ref()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn why(&self) -> &WhyLeftOut {
        &self.why
    }

    /// Whose definition of this id runs instead. `None` where none does, or
    /// where the file never said which id it was.
    pub fn instead(&self) -> Option<WorkflowSource> {
        self.instead
    }
}

/// A place as the owner of a definition: *Kit's `bug`*.
fn whose(source: WorkflowSource) -> &'static str {
    match source {
        WorkflowSource::Armada => "Armada's",
        WorkflowSource::Kit => "Kit's",
        WorkflowSource::Repository => "the repository's",
        WorkflowSource::Project => "a project's",
    }
}

impl fmt::Display for LeftOut {
    /// What Fleet prints at start: whose definition, why, and what runs instead
    /// — so a person who customised `bug` knows they are not running their own.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whose = match &self.project {
            Some(project) => format!("project `{}`'s", project.as_str()),
            None => whose(self.source).to_string(),
        };
        match &self.id {
            Some(id) => write!(f, "{whose} `{}` was left out, because ", id.as_str())?,
            None => write!(f, "a definition {} was left out, because ", self.source)?,
        }
        match &self.why {
            WhyLeftOut::Unparsed(why) => write!(f, "{why}")?,
            WhyLeftOut::Unresolved(why) => write!(f, "{why}")?,
            WhyLeftOut::Duplicated { also } => {
                write!(f, "{}", self.path.display())?;
                for path in also {
                    write!(f, " and {}", path.display())?;
                }
                write!(f, " all declare it")?;
            }
            WhyLeftOut::HeldByTwoProjects { files } => {
                for (at, (project, path)) in files.iter().enumerate() {
                    let joiner = match at {
                        0 => "",
                        last if last + 1 == files.len() => " and ",
                        _ => ", ",
                    };
                    write!(f, "{joiner}project `{}` ({})", project.as_str(), path.display())?;
                }
                write!(f, " each keep it and each hold this repository")?;
            }
        }
        match (&self.id, self.instead) {
            (Some(id), Some(instead)) => {
                write!(
                    f,
                    "; {} `{}` is used instead",
                    whose_capital(instead),
                    id.as_str()
                )
            }
            (Some(id), None) => write!(f, "; no `{}` runs here", id.as_str()),
            (None, _) => Ok(()),
        }
    }
}

/// [`whose`], at the start of a clause.
fn whose_capital(source: WorkflowSource) -> &'static str {
    match source {
        WorkflowSource::Repository => "The repository's",
        other => whose(other),
    }
}

/// Why one definition does not fit the repository it is for.
#[derive(Debug)]
pub enum Unfit {
    /// It will not parse, or names a model this machine does not offer.
    Unparsed(LoadError),
    /// It parses, and names what the repository does not declare, or says
    /// something else about what it does.
    Unresolved(ResolveError),
}

impl fmt::Display for Unfit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unfit::Unparsed(why) => write!(f, "{why}"),
            Unfit::Unresolved(why) => write!(f, "{why}"),
        }
    }
}

/// Whether one definition would be held for `manifest`, and under which id.
///
/// **The two steps a catalogue takes, for one file** — [`WorkflowDef::parse`]
/// against the roster, then [`ResolvedWorkflow::resolve`] against the
/// Manifest — so what is saved is judged by what would load it. Nothing is
/// merged: whether another place already holds the id is the caller's to say.
pub fn fit(
    path: &Path,
    text: &str,
    roster: &Roster,
    manifest: &Manifest,
) -> Result<WorkflowId, Unfit> {
    let def = WorkflowDef::parse(path, text, roster).map_err(Unfit::Unparsed)?;
    ResolvedWorkflow::resolve(&def, manifest).map_err(Unfit::Unresolved)?;
    Ok(def.id().clone())
}

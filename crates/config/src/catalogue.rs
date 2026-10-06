//! Where a workflow comes from, and which one a repository runs.
//!
//! **Three sources, merged by `workflow_id`, the most specific winning**: the
//! set Armada carries, then Kit's Workflows, then the repository's own. So a
//! repository nobody set up dispatches on the carried set, and one file in
//! either later place replaces a carried definition by id. #425.
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

use core_model::{WorkflowId, WorkflowSource};

use crate::error::{LoadError, ResolveError};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::roster::Roster;
use crate::workflow::WorkflowDef;

/// Where a carried definition says it was read from. **Not a file** — a name
/// for a refusal to cite, bracketed so it cannot be mistaken for a path
/// relative to a repository.
pub const CARRIED_AT: &str = "<carried by Armada>";

/// The set Armada carries: this repository's own eight, compiled in.
///
/// **The files and not copies of them**, so an edit to one is an edit to what
/// every repository runs, and `tests/carried.rs` fails if a ninth is added
/// under `.armada/workflows/` without being listed here.
const CARRIED: [(&str, &str); 8] = [
    (
        "bug.json",
        include_str!("../../../.armada/workflows/bug.json"),
    ),
    (
        "code-review.json",
        include_str!("../../../.armada/workflows/code-review.json"),
    ),
    (
        "design-plan.json",
        include_str!("../../../.armada/workflows/design-plan.json"),
    ),
    (
        "epic.json",
        include_str!("../../../.armada/workflows/epic.json"),
    ),
    (
        "feature.json",
        include_str!("../../../.armada/workflows/feature.json"),
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
];

/// One definition, as text, and the place it was read from.
///
/// **No public way to say a definition is Armada's.** [`carried`] is the only
/// source of those, so a file on a machine cannot pass itself off as the set
/// the binary carries.
#[derive(Debug, Clone)]
pub struct Written {
    source: WorkflowSource,
    path: PathBuf,
    text: String,
}

impl Written {
    /// A definition read from Kit's Workflows.
    pub fn in_kit(path: PathBuf, text: String) -> Written {
        Written {
            source: WorkflowSource::Kit,
            path,
            text,
        }
    }

    /// A definition read from the repository's own `.armada/workflows/`.
    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written {
            source: WorkflowSource::Repository,
            path,
            text,
        }
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
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
            path: Path::new(CARRIED_AT).join(file),
            text: (*text).to_string(),
        })
        .collect()
}

/// Every definition in hand, grouped by id, and the ones already set aside.
#[derive(Debug)]
pub struct Catalogue {
    held: BTreeMap<WorkflowId, Vec<(WorkflowDef, WorkflowSource, String)>>,
    left_out: Vec<LeftOut>,
}

impl Catalogue {
    /// Parse every definition, keeping each beside the others for its id.
    ///
    /// **The order they arrive in decides nothing**, and the source decides
    /// everything. Two definitions sharing an id *and* a place are a fault in
    /// that place: both are left out, naming both. Across places, it is an
    /// override.
    pub fn of(written: impl IntoIterator<Item = Written>, roster: &Roster) -> Catalogue {
        let mut placed: BTreeMap<(WorkflowSource, WorkflowId), Vec<(WorkflowDef, String)>> =
            BTreeMap::new();
        let mut left_out = Vec::new();
        for one in written {
            let def = match WorkflowDef::parse(&one.path, &one.text, roster) {
                Ok(def) => def,
                Err(why) => {
                    left_out.push(LeftOut::of(
                        None,
                        one.source,
                        one.path,
                        WhyLeftOut::Unparsed(why),
                    ));
                    continue;
                }
            };
            placed
                .entry((one.source, def.id().clone()))
                .or_default()
                .push((def, one.text));
        }

        let mut held: BTreeMap<WorkflowId, Vec<(WorkflowDef, WorkflowSource, String)>> =
            BTreeMap::new();
        for ((source, id), mut defs) in placed {
            if defs.len() > 1 {
                let also = defs
                    .drain(1..)
                    .map(|(def, _)| def.path().to_path_buf())
                    .collect();
                let first = defs.remove(0).0.path().to_path_buf();
                let why = WhyLeftOut::Duplicated { also };
                left_out.push(LeftOut::of(Some(id), source, first, why));
                continue;
            }
            let (def, text) = defs.remove(0);
            held.entry(id).or_default().push((def, source, text));
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
        let mut files = Vec::new();
        let mut left_out = self.left_out;
        for (id, mut candidates) in self.held {
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            let mut winner: Option<WorkflowSource> = None;
            for (def, source, text) in candidates {
                if let Some(by) = winner {
                    files.push(WorkflowFile::of(&id, source, &def, text, Some(by)));
                    continue;
                }
                match ResolvedWorkflow::resolve_gated(&def, manifest, gating) {
                    Ok(resolved) => {
                        workflows.insert(id.clone(), resolved.read_from(source));
                        files.push(WorkflowFile::of(&id, source, &def, text, None));
                        winner = Some(source);
                    }
                    Err(why) => left_out.push(LeftOut::of(
                        Some(id.clone()),
                        source,
                        def.path().to_path_buf(),
                        WhyLeftOut::Unresolved(why),
                    )),
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
    path: PathBuf,
    text: String,
    overridden_by: Option<WorkflowSource>,
}

impl WorkflowFile {
    fn of(
        id: &WorkflowId,
        source: WorkflowSource,
        def: &WorkflowDef,
        text: String,
        overridden_by: Option<WorkflowSource>,
    ) -> WorkflowFile {
        WorkflowFile {
            id: id.clone(),
            source,
            path: def.path().to_path_buf(),
            text,
            overridden_by,
        }
    }

    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
    }

    /// Where it was read from. A bracketed name, not a path, for a carried one.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The file as it was read, whole.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The place that answers for this id instead. `None` where this one runs.
    pub fn overridden_by(&self) -> Option<WorkflowSource> {
        self.overridden_by
    }
}

/// A definition from Kit or Armada this repository runs without.
#[derive(Debug)]
pub struct LeftOut {
    /// `None` where the file did not parse far enough to say.
    id: Option<WorkflowId>,
    source: WorkflowSource,
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
            path,
            why,
            instead: None,
        }
    }

    pub fn id(&self) -> Option<&WorkflowId> {
        self.id.as_ref()
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
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
    }
}

impl fmt::Display for LeftOut {
    /// What Fleet prints at start: whose definition, why, and what runs instead
    /// — so a person who customised `bug` knows they are not running their own.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whose = whose(self.source);
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

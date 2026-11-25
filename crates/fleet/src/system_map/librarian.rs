//! The parts the librarian adds to a project's map, on his press, and what he answers to them. **A stub without a model**:
//! the pass reads what detection did not follow (folders of source no part covers) and proposes each as a part, kept apart
//! until he answers. Keep makes it a detected part, Rework stores his note and the next press redoes it, Remove drops it and
//! it is not proposed again. `docs/practices/protocol/projects.md`, *The system map*.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AdditionAnswer, AnswerSystemAddition, Instant, SystemMap, SystemPartKind, WireError};
use store::{addition_state, KeptAddition, KeptFeature};

use super::detect::{humanize, slug};
use super::levels::Addition;
use super::model::Mapped;
use super::repo::extension_of;
use super::tables::known;
use crate::daemon::Fleet;

const UNKNOWN: &str = "fleet.system_addition_unknown";
const NOTE: &str = "fleet.system_addition_note";
const RUNNING: &str = "fleet.system_librarian_running";
/// A folder of source this big, that no part covers, is worth a part.
const WORTH_A_PART: usize = 8;
/// Folders that hold other folders' worth of code: a part is one level below them.
const HOLDERS: &[&str] = &["apps", "crates", "packages", "services", "libs", "cmd", "modules"];
const CODE: &[&str] = &["rs", "py", "js", "jsx", "ts", "tsx", "go", "java", "kt", "swift", "rb", "cs", "php"];

fn kind_of(word: &str) -> SystemPartKind {
    match word {
        "entry" => SystemPartKind::Entry,
        "store" => SystemPartKind::Store,
        "outside" => SystemPartKind::Outside,
        _ => SystemPartKind::Service,
    }
}

fn word_of(kind: SystemPartKind) -> &'static str {
    match kind {
        SystemPartKind::Entry => "entry",
        SystemPartKind::Store => "store",
        SystemPartKind::Outside => "outside",
        _ => "service",
    }
}

/// The folders of source no part covers, each as a part it could be: its id, name, kind, one line and the path.
fn uncovered(map: &Mapped) -> Vec<(String, String, SystemPartKind, String, String)> {
    let mut groups: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (file, _) in &map.files {
        if !CODE.contains(&extension_of(file)) || map.part_of(file).is_some() || map.piece_of(file).is_some() || super::repo::is_proof(file) {
            continue;
        }
        let mut segments = file.split('/');
        let first = segments.next().unwrap_or("");
        let Some(second) = segments.next() else { continue };
        let group = if HOLDERS.contains(&first) && file.matches('/').count() >= 2 { format!("{first}/{second}") } else { first.to_string() };
        if known().tooling(group.rsplit('/').next().unwrap_or(&group)) || matches!(group.as_str(), "docs" | "scripts" | ".github" | "tests") {
            continue;
        }
        let entry = groups.entry(group).or_default();
        entry.0 += 1;
        entry.1 += usize::from(matches!(extension_of(file), "jsx" | "tsx"));
    }
    groups
        .into_iter()
        .filter(|(_, (count, _))| *count >= WORTH_A_PART)
        .map(|(group, (count, screens))| {
            let kind = if screens * 2 > count { SystemPartKind::Entry } else { SystemPartKind::Service };
            let name = humanize(group.rsplit('/').next().unwrap_or(&group));
            let why = format!("Holds {count} source files, and no part found in the repository covers them.");
            (slug(&group), name, kind, why, format!("{group}/"))
        })
        .collect()
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
    /// What the levels are built from beyond the files: the catalog's features, the files Jobs filed under them, the parts
    /// the librarian added that he has not removed, and when it last mapped.
    #[allow(clippy::type_complexity)]
    pub(super) async fn system_ground(&self, project: &str, _maps: &[Arc<Mapped>]) -> Result<(Vec<KeptFeature>, Vec<(String, String)>, Vec<Addition>, Option<Instant>), Refusal> {
        let store = self.store().lock().await;
        let features = store.features_in(project).map_err(|why| self.ledger_fault(why))?;
        let filed = store.feature_filed_paths(project).map_err(|why| self.ledger_fault(why))?;
        let additions = store
            .system_additions(project)
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .filter(|one| one.state != addition_state::REMOVED)
            .map(|one| Addition {
                kept: one.state == addition_state::KEPT,
                kind: kind_of(&one.kind),
                id: one.id,
                name: one.name,
                summary: one.summary,
                why: one.why,
                paths: one.paths,
                manifest: one.manifest,
            })
            .collect();
        let at = store.system_librarian_at(project).map_err(|why| self.ledger_fault(why))?.map(Instant::carried);
        Ok((features, filed, additions, at))
    }

    /// `map_system_with_librarian`.
    pub(crate) async fn system_librarian_pressed(&self, project: String) -> Result<SystemMap, Refusal> {
        if self.librarian_running().now(&project).is_some() {
            return Err(Refusal::IllegalMove(WireError::raised(RUNNING, "the project's librarian is running; its map is added to when it ends", self.run_id())));
        }
        let maps = self.system_mapped(&project).await?;
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let held = store.system_additions(&project).map_err(|why| self.ledger_fault(why))?;
            for map in &maps {
                for (id, name, kind, why, path) in uncovered(map) {
                    let add = KeptAddition {
                        project: project.clone(),
                        id,
                        name,
                        kind: word_of(kind).to_string(),
                        summary: format!("{} in {}", humanize(word_of(kind)), path.trim_end_matches('/')),
                        why,
                        paths: vec![path],
                        manifest: map.manifest.clone(),
                        state: addition_state::PROPOSED.to_string(),
                        note: String::new(),
                        added_at: now.clone(),
                    };
                    store.propose_system_addition(&add).map_err(|why| self.ledger_fault(why))?;
                }
            }
            for redo in held.iter().filter(|one| one.state == addition_state::REWORK) {
                let why = format!("{} Redone from his note: {}", redo.why.split(" Redone from his note").next().unwrap_or(&redo.why), redo.note);
                store.redo_system_addition(&project, &redo.id, &redo.summary, &why, &redo.paths).map_err(|why| self.ledger_fault(why))?;
            }
            store.keep_system_librarian_at(&project, &now).map_err(|why| self.ledger_fault(why))?;
        }
        self.system_map_read(project, None, None).await
    }

    /// `answer_system_addition`.
    pub(crate) async fn system_addition_answered(&self, project: String, answer: AnswerSystemAddition) -> Result<SystemMap, Refusal> {
        self.project_held(&project).await?;
        let (state, note) = match &answer.answer {
            AdditionAnswer::Keep => (addition_state::KEPT, String::new()),
            AdditionAnswer::Remove => (addition_state::REMOVED, String::new()),
            AdditionAnswer::Rework { note } if note.trim().is_empty() => {
                return Err(Refusal::Unacceptable(WireError::raised(NOTE, "a rework carries his note, the words the librarian redoes the part from", self.run_id())));
            }
            AdditionAnswer::Rework { note } => (addition_state::REWORK, note.trim().to_string()),
        };
        let answered = self.store().lock().await.answer_system_addition(&project, &answer.addition, state, &note).map_err(|why| self.ledger_fault(why))?;
        if !answered {
            return Err(Refusal::NotFound(WireError::raised(UNKNOWN, format!("the project holds no addition `{}`", answer.addition), self.run_id())));
        }
        self.system_map_read(project, None, None).await
    }
}

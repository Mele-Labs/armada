//! A confirmed draft written down: into the repository's `armada.yml` where it has one, else into
//! the person's own copy in Kit. **Drafting and reading are in [`super::draft`]**; this only writes.
//!
//! **The writer is `config::amend`**, the one `run_memory/manifest.rs` uses for a learnt dev
//! server: a splice that leaves every comment where it was, and re-parses what it produced, so
//! nothing is written that would not load.
//!
//! **The repository's file is left changed and not committed.** It sits in the checkout the
//! person works in, and committing there is a decision about their branch that a press on a run
//! does not make. A Job's branch is another place; the caller names the tree.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{
    amend, CommandEdit, Edit, NewCommand, NewLink, NewPort, NewRun, NotAmended, PortEdit, RunEdit,
};

use super::draft::{safe_name, DraftProcess, RunDraft};
use super::unrunnable::Unrunnable;
use crate::daemon::Fleet;

/// Where a run went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SavedIn {
    /// The repository's own `armada.yml`, changed in its checkout and not committed.
    Repository(PathBuf),
    /// The person's copy in Kit, as the place their Kit is kept names it.
    Kit(String),
}

/// What a save did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SavedRun {
    pub(crate) name: String,
    pub(crate) saved_in: SavedIn,
    /// **Fleet holds the Manifest it started with**, and the servers and runs of it are read at
    /// boot, so a run Fleet does not yet declare starts only once it has read the file again.
    pub(crate) takes_effect_at_restart: bool,
}

/// The edits that declare `draft`: each process's port, then its server, then the run.
pub(crate) fn edits(draft: &RunDraft) -> Vec<Edit> {
    let mut edits = Vec::new();
    for process in &draft.processes {
        if let Some(port) = &process.port {
            edits.push(Edit::Port {
                name: port.name.clone(),
                edit: PortEdit::Add(NewPort { container: None, env: port.env.clone() }),
            });
        }
        let command = |edit| Edit::Command { name: process.name.clone(), edit };
        edits.push(command(CommandEdit::Add(NewCommand {
            run: None,
            destructive: false,
            serve: Some(process.serve.clone()),
            ready: None,
            links: process
                .port
                .iter()
                .map(|port| NewLink { url: format!("http://localhost:${{port.{}}}", port.name), name: None })
                .collect(),
        })));
        if process.dir.is_some() {
            edits.push(command(CommandEdit::Dir(process.dir.clone())));
        }
    }
    edits.push(Edit::Run {
        name: draft.name.clone(),
        edit: RunEdit::Add(NewRun {
            label: draft.label.clone(),
            commands: draft.processes.iter().map(|one| one.name.clone()).collect(),
        }),
    });
    edits
}

/// A draft a person confirmed that cannot be written as it stands, said before the file is read.
fn refused(draft: &RunDraft) -> Option<String> {
    let named = |name: &str| !name.is_empty() && safe_name(name) == name;
    if !named(&draft.name) {
        return Some(format!("`{}` is not a name for a run: letters, digits and `_`", draft.name));
    }
    if draft.processes.is_empty() {
        return Some(String::from("a run starts at least one server"));
    }
    draft.processes.iter().find_map(|one: &DraftProcess| {
        if !named(&one.name) {
            return Some(format!("`{}` is not a name for a server: letters, digits and `_`", one.name));
        }
        if one.serve.trim().is_empty() {
            return Some(format!("`{}` has nothing to serve", one.name));
        }
        None
    })
}

/// `text` with `draft` declared in it, or why it cannot be.
pub(crate) fn amended(path: &Path, text: &str, draft: &RunDraft, first: &[Edit]) -> Result<String, Unrunnable> {
    if let Some(why) = refused(draft) {
        return Err(Unrunnable::DraftRefused { why });
    }
    let all: Vec<Edit> = first.iter().cloned().chain(edits(draft)).collect();
    match amend(path, text, &all) {
        Ok(done) => Ok(done.text().to_string()),
        Err(why @ (NotAmended::Misnamed { .. } | NotAmended::Refused(_))) => {
            Err(Unrunnable::DraftRefused { why: why.to_string() })
        }
        Err(why) => Err(Unrunnable::NotSaved { why: why.to_string() }),
    }
}

/// Write `draft` into the `armada.yml` at `file`, over a temporary file renamed into place.
pub(crate) fn into_file(file: &Path, draft: &RunDraft) -> Result<(), Unrunnable> {
    let text = std::fs::read_to_string(file)
        .map_err(|why| Unrunnable::NotSaved { why: format!("{} could not be read: {why}", file.display()) })?;
    let written = amended(file, &text, draft, &[])?;
    let staging = file.with_extension("yml.writing");
    std::fs::write(&staging, written)
        .and_then(|()| std::fs::rename(&staging, file))
        .map_err(|why| Unrunnable::NotSaved { why: format!("{} could not be written: {why}", file.display()) })
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
    /// The run the repository's files and its Jobs' dev servers suggest, **where it declares
    /// none**. Saves nothing.
    pub(crate) async fn run_drafted(
        &self,
        manifest_id: Option<&ipc::ManifestId>,
    ) -> Result<Option<RunDraft>, Refusal> {
        let served = self.served_named(manifest_id)?;
        if !served.manifest().app_run_names().is_empty() {
            return Err(self.run_refusal(Unrunnable::AlreadySaved {
                runs: served.manifest().app_run_names().to_vec(),
            }));
        }
        let root = served.root().to_string();
        self.run_memory_started(&root).await?;
        let facts = self.store().lock().await.run_facts(&root).map_err(|why| self.run_fault(why))?;
        let tree = PathBuf::from(&root);
        Ok(tokio::task::spawn_blocking(move || super::draft::drafted(&tree, &facts)).await.unwrap_or(None))
    }

    /// Declare a confirmed `draft`: in the repository's `armada.yml` where the repository has one,
    /// else in the person's copy in Kit, which is then the whole Manifest of a repository with none.
    pub(crate) async fn run_saved(
        &self,
        manifest_id: Option<&ipc::ManifestId>,
        draft: RunDraft,
    ) -> Result<SavedRun, Refusal> {
        let served = self.served_named(manifest_id)?;
        let file = served.manifest().path().to_path_buf();
        let takes_effect_at_restart = served.manifest().app_run(&draft.name).is_none();
        let name = draft.name.clone();
        let saved_in = match file.is_file() {
            true => {
                let (at, draft) = (file.clone(), draft);
                tokio::task::spawn_blocking(move || into_file(&at, &draft))
                    .await
                    .unwrap_or_else(|why| Err(Unrunnable::NotSaved { why: why.to_string() }))
                    .map_err(|why| self.run_refusal(why))?;
                SavedIn::Repository(file)
            }
            false => self.run_saved_in_kit(served.root(), &file, &draft).await?,
        };
        Ok(SavedRun { name, saved_in, takes_effect_at_restart })
    }

    /// The person's copy for `root`, read from where their Kit keeps it, with `draft` declared in it.
    /// **A repository with no copy yet gets one made whole**: a version and an id are what let a file
    /// stand as the Manifest.
    async fn run_saved_in_kit(&self, root: &str, file: &Path, draft: &RunDraft) -> Result<SavedIn, Refusal> {
        let (label, read) = self.kit_manifest_read(root).await?;
        let (text, first) = match read {
            config::KitRead::Text(text) => (text, Vec::new()),
            config::KitRead::Absent => (
                String::new(),
                vec![Edit::Version(1), Edit::Id(self.mint().ulid().as_str().to_string())],
            ),
            config::KitRead::Unread(why) => {
                return Err(self.run_refusal(Unrunnable::NotSaved {
                    why: format!("{} could not be reached: {why}", label.display()),
                }))
            }
        };
        let written = amended(file, &text, draft, &first).map_err(|why| self.run_refusal(why))?;
        self.manifest_written(root, written).await?;
        Ok(SavedIn::Kit(label.display().to_string()))
    }
}

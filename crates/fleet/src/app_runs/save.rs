//! A confirmed draft written down, **into the person's own copy in Kit and nowhere else**.
//! **Drafting and reading are in [`super::draft`]**; this only writes.
//!
//! **The repository's `armada.yml` is never written here.** It sits in the checkout the person
//! works in, and a press on a run is no decision about their tree. Kit's copy lays over that file
//! by name, wherever the person keeps it (this Mac, a synced folder, the cloud), so the run is
//! there at once; getting it into the repository is the *Commit to repository* act on a Job's branch.
//!
//! **The writer is `config::splice`**, the splice `config::amend` is built on: it leaves every
//! comment where it was and does not need the document to be a Manifest by itself. What it
//! produced is then loaded as Kit's copy is, laid over the repository's file, before anything is
//! written, so nothing is saved that would not load.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{
    splice, CommandEdit, Edit, KitRead, Manifest, NewCommand, NewLink, NewPort, NewRun, NotAmended,
    PortEdit, RunEdit,
};

use super::draft::{safe_name, DraftProcess, RunDraft};
use super::unrunnable::Unrunnable;
use crate::daemon::Fleet;

/// Where a run went: the place the person's Kit is kept, and the file in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SavedIn {
    /// As the person chose it: `this_mac`, the synced folder, or the cloud.
    pub(crate) place: ipc::RunPlace,
    /// Kit's copy, as that place names it.
    pub(crate) file: String,
}

/// What a save did. **A saved run starts at once**: a run is read from the file when it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SavedRun {
    pub(crate) name: String,
    pub(crate) saved_in: SavedIn,
}

/// Where a saved run is written. **The one place that chooses**: Kit's copy for the repository,
/// at the place its Kit keeps it.
struct SaveTarget {
    place: ipc::RunPlace,
    label: PathBuf,
    /// What Kit's copy holds now, or `None` where there is none yet.
    text: Option<String>,
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

/// `text` with `draft` declared in it, or why it cannot be. `first` is what a new copy needs before
/// the draft: a version and an id, where Kit's copy is to be the whole Manifest.
pub(crate) fn spliced(text: &str, draft: &RunDraft, first: &[Edit]) -> Result<String, Unrunnable> {
    if let Some(why) = refused(draft) {
        return Err(Unrunnable::DraftRefused { why });
    }
    let all: Vec<Edit> = first.iter().cloned().chain(edits(draft)).collect();
    splice(text, &all).map_err(|why| match why {
        NotAmended::Misnamed { .. } | NotAmended::Refused(_) => Unrunnable::DraftRefused { why: why.to_string() },
        other => Unrunnable::NotSaved { why: other.to_string() },
    })
}

/// The text of Kit's copy once `draft` is in it: spliced, then loaded over the repository's file at
/// `file` as Kit's copy is, so what comes back is what the run will start from.
pub(crate) fn written(file: &Path, label: &Path, text: &str, draft: &RunDraft, first: &[Edit]) -> Result<String, Unrunnable> {
    let written = spliced(text, draft, first)?;
    loaded(file, label, &written, draft).map(|()| written)
}

/// Every name `draft` declares that the repository's own `armada.yml` already does. **Kit's copy
/// wins by name**, so saving one would change what a Command, a port or a run of the repository is.
fn already_declared(repository: &Manifest, draft: &RunDraft) -> Vec<String> {
    let ports = repository.port_names();
    let mut names: Vec<String> = Vec::new();
    for process in &draft.processes {
        let taken = repository.command(&process.name).is_some()
            || repository.server(&process.name).is_some()
            || process.port.as_ref().is_some_and(|port| ports.contains(&port.name));
        if taken {
            names.push(process.name.clone());
        }
    }
    if repository.app_run(&draft.name).is_some() {
        names.push(draft.name.clone());
    }
    names
}

/// `written` loaded as Kit's copy over the repository's file at `file` where there is one, else on
/// its own: the Manifest the run will start from, or why it will not load.
fn loaded(file: &Path, label: &Path, written: &str, draft: &RunDraft) -> Result<(), Unrunnable> {
    let refuse = |why: String| Unrunnable::DraftRefused { why };
    let manifest = match std::fs::read_to_string(file) {
        Ok(own) => {
            let repository = Manifest::parse(file, &own).map_err(|why| refuse(format!("{} does not load: {why}", file.display())))?;
            let clashing = already_declared(&repository, draft);
            if !clashing.is_empty() {
                let said = clashing.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>().join(", ");
                return Err(refuse(format!("{} already declares {said}", file.display())));
            }
            Manifest::with_kit(file, &own, label, written).map_err(|why| refuse(why.to_string()))?
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            Manifest::from_kit_alone(file, label, written).map_err(|why| refuse(why.to_string()))?
        }
        Err(why) => return Err(Unrunnable::NotSaved { why: format!("{} could not be read: {why}", file.display()) }),
    };
    if let Some(left_out) = manifest.from_kit().and_then(|kit| kit.left_out.clone()) {
        return Err(refuse(left_out));
    }
    match manifest.app_run(&draft.name) {
        Some(run) if run.servers().len() == draft.processes.len() => Ok(()),
        _ => Err(refuse(format!("`{}` is not a run in what was written", draft.name))),
    }
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
        let root = served.root().to_string();
        // A run saved since Fleet booted is one the repository declares.
        let declared = self.run_manifest(&served, &root).await?;
        if !declared.app_run_names().is_empty() {
            return Err(self.run_refusal(Unrunnable::AlreadySaved { runs: declared.app_run_names().to_vec() }));
        }
        self.run_memory_started(&root).await?;
        let facts = self.store().lock().await.run_facts(&root).map_err(|why| self.run_fault(why))?;
        let tree = PathBuf::from(&root);
        Ok(tokio::task::spawn_blocking(move || super::draft::drafted(&tree, &facts)).await.unwrap_or(None))
    }

    /// Declare a confirmed `draft` in the person's copy in Kit, at the place they keep it. **The
    /// copy is the whole Manifest of a repository with no `armada.yml`**, and only the names it
    /// changes of one that has. The run starts at once: it is read from the file when it starts.
    pub(crate) async fn run_saved(
        &self,
        manifest_id: Option<&ipc::ManifestId>,
        draft: RunDraft,
    ) -> Result<SavedRun, Refusal> {
        let served = self.served_named(manifest_id)?;
        let root = served.root().to_string();
        let file = served.manifest().path().to_path_buf();
        let target = self.run_save_target(&root).await?;
        let (text, first) = match target.text {
            Some(text) => (text, Vec::new()),
            None if file.is_file() => (String::new(), Vec::new()),
            None => (String::new(), vec![Edit::Version(1), Edit::Id(self.mint().ulid().as_str().to_string())]),
        };
        let name = draft.name.clone();
        let (label, at) = (target.label.clone(), file.clone());
        let kept = tokio::task::spawn_blocking(move || written(&at, &label, &text, &draft, &first))
            .await
            .unwrap_or_else(|why| Err(Unrunnable::NotSaved { why: why.to_string() }))
            .map_err(|why| self.run_refusal(why))?;
        self.manifest_written(&root, kept).await?;
        Ok(SavedRun { name, saved_in: SavedIn { place: target.place, file: target.label.display().to_string() } })
    }

    /// Where a run for the repository at `root` is saved, and what is there now. A place that cannot
    /// be reached refuses here, before anything is built to write.
    async fn run_save_target(&self, root: &str) -> Result<SaveTarget, Refusal> {
        let place = self.kit_place_of(root).await?;
        let (label, read) = self.kit_manifest_read(root).await?;
        let text = match read {
            KitRead::Text(text) => Some(text),
            KitRead::Absent => None,
            KitRead::Unread(why) => {
                return Err(self.run_refusal(Unrunnable::NotSaved {
                    why: format!("{} could not be reached: {why}", label.display()),
                }))
            }
        };
        Ok(SaveTarget { place, label, text })
    }
}

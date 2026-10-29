//! Bringing in what a person already has in their agent CLI: the folders it kept sessions for, how
//! each was run, and installing Fleet's mod into it. The reading and the install are `adapters`';
//! this is where a home and a request meet them.
//!
//! **The install is the only one of the three that changes anything, and it runs only for a
//! request**: a body that says `requested: false` is refused before the first command, and no other
//! operation reaches it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::agent_mod::{self, Asked, Installed, Step};
use adapters::session_folders::{self, FolderGit};
use adapters::session_runs::{self, Reading, RunKind};
use api::Refusal;
use core_model::{Component, Envelope, FieldValue, Level};
use ipc::{ImportFolder, ImportFolders, ImportGit, ImportRuns, Instant, ModInstallStep, ModInstalled, RunFact, RunFactKind, WireError};

use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;

/// `install_agent_mod` without a request. A 422.
pub(crate) const INSTALL_NOT_REQUESTED: &str = "fleet.install_not_requested";
/// A folder the sessions do not name. A 422.
pub(crate) const NOT_AN_IMPORT_FOLDER: &str = "fleet.not_an_import_folder";

/// How long one step of the install may take before it is stopped.
const STEP_WITHIN: Duration = Duration::from_secs(120);

fn at(millis: u64) -> Instant {
    Instant::carried(rfc3339_utc(millis as i64))
}

fn git(state: FolderGit) -> ImportGit {
    match state {
        FolderGit::Root => ImportGit::Root,
        FolderGit::Inside(root) => ImportGit::Inside { root },
        FolderGit::NotARepository => ImportGit::NotARepository,
        FolderGit::Unknown => ImportGit::Unknown,
    }
}

fn kind(of: RunKind) -> RunFactKind {
    match of {
        RunKind::Tests => RunFactKind::Tests,
        RunKind::Lint => RunFactKind::Lint,
        RunKind::Build => RunFactKind::Build,
        RunKind::DevServer => RunFactKind::DevServer,
        RunKind::Typecheck => RunFactKind::Typecheck,
        RunKind::Format => RunFactKind::Format,
    }
}

fn step(of: Step) -> ModInstallStep {
    match of {
        Step::Populate => ModInstallStep::Populate,
        Step::AddMarketplace => ModInstallStep::AddMarketplace,
        Step::Install => ModInstallStep::Install,
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
    fn sessions_dir(&self) -> PathBuf {
        Path::new(&self.host().home).join(session_folders::PROJECTS)
    }

    /// `list_import_folders`: read fresh, because a person who just worked in a folder expects to see it.
    pub(crate) async fn import_folders(&self) -> Result<ImportFolders, Refusal> {
        let projects = self.sessions_dir();
        let folders = tokio::task::spawn_blocking(move || session_folders::list_folders(&projects))
            .await
            .map_err(|why| self.import_fault(why.to_string()))?;
        Ok(ImportFolders {
            agent: session_folders::AGENT.to_string(),
            folders: folders
                .into_iter()
                .map(|folder| ImportFolder {
                    path: folder.path,
                    exists: folder.exists,
                    last_session_at: at(folder.newest_ms),
                    sessions: folder.sessions.min(u32::MAX as usize) as u32,
                    git: git(folder.git),
                })
                .collect(),
        })
    }

    /// `get_import_runs`: how one of those folders was run.
    pub(crate) async fn import_runs(&self, path: String) -> Result<ImportRuns, Refusal> {
        let projects = self.sessions_dir();
        let asked = path.clone();
        let read = tokio::task::spawn_blocking(move || {
            let folder = session_folders::list_folders(&projects).into_iter().find(|one| one.path == asked)?;
            let mut records: Vec<_> = folder.kept_in.iter().flat_map(|dir| session_folders::records_in(dir)).collect();
            records.sort_by(|a, b| b.1.cmp(&a.1));
            records.truncate(session_runs::MOST_SESSIONS);
            let mut reading = Reading::default();
            for (record, written) in &records {
                reading.feed_file(record, *written);
            }
            let root = folder.root().to_string();
            Some((session_runs::derive(&reading.records, &root), root))
        })
        .await
        .map_err(|why| self.import_fault(why.to_string()))?;
        let Some((facts, root)) = read else {
            return Err(Refusal::Unacceptable(WireError::raised(
                NOT_AN_IMPORT_FOLDER,
                format!("no session names the folder {path}"),
                self.run_id(),
            )));
        };
        Ok(ImportRuns {
            path,
            root,
            facts: facts
                .into_iter()
                .map(|fact| RunFact {
                    workspace: fact.workspace,
                    kind: kind(fact.kind),
                    command: fact.command,
                    port: fact.port,
                    succeeded: fact.succeeded,
                    last_ran: at(fact.last_ran),
                    source: session_runs::SOURCE.to_string(),
                })
                .collect(),
        })
    }

    /// `install_agent_mod`: refused unless the body says it was requested.
    pub(crate) async fn agent_mod_installed(&self, body: ipc::InstallAgentMod) -> Result<ModInstalled, Refusal> {
        let Some(asked) = Asked::when(body.requested) else {
            return Err(Refusal::Unacceptable(WireError::raised(
                INSTALL_NOT_REQUESTED,
                "the mod is installed only when a person asks for it".to_string(),
                self.run_id(),
            )));
        };
        let home = PathBuf::from(&self.host().home);
        let path = self.host().path.clone();
        let folder = crate::runtime::mod_dir(&self.host().home);
        let trees: Vec<PathBuf> = self.repositories().served().iter().map(|one| PathBuf::from(one.root())).collect();
        let done = tokio::task::spawn_blocking(move || {
            if let Err(said) = agent_mod::populated(&home, &folder, &trees, STEP_WITHIN) {
                return Installed::Failed { step: Step::Populate, said };
            }
            agent_mod::install(asked, agent_mod::PROGRAM.as_ref(), &path, &home, &folder, STEP_WITHIN)
        })
        .await
        .map_err(|why| self.import_fault(why.to_string()))?;
        Ok(match done {
            Installed::Done { said } => ModInstalled::Installed { said },
            Installed::Failed { step: stopped, said } => ModInstalled::Failed { step: step(stopped), said },
        })
    }

    /// Fleet's copy of the mod filled before a hosted process loads it with `--plugin-dir`. Nothing
    /// to do once it is; otherwise the script runs on a blocking thread. **A failure only means the
    /// process starts without the mod**, so it is logged and never fails the launch.
    pub(crate) async fn mod_filled_for_launch(&self) {
        if let Err(said) = self.mod_filled().await {
            let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "the mod's copy was not filled, so the job starts without it")
                .with_field("why", FieldValue::Str(said));
            crate::transcript::note_aloud(&envelope);
        }
    }

    /// [`Self::mod_filled_for_launch`] saying why it could not, for a caller that refuses on it.
    pub(crate) async fn mod_filled(&self) -> Result<(), String> {
        let home = PathBuf::from(&self.host().home);
        let folder = crate::runtime::mod_dir(&self.host().home);
        if agent_mod::is_populated(&folder) {
            return Ok(());
        }
        let trees: Vec<PathBuf> = self.repositories().served().iter().map(|one| PathBuf::from(one.root())).collect();
        tokio::task::spawn_blocking(move || agent_mod::populated(&home, &folder, &trees, STEP_WITHIN))
            .await
            .map_err(|why| why.to_string())?
    }

    fn import_fault(&self, why: String) -> Refusal {
        Refusal::Fault(WireError::raised("fleet.import_failed", why, self.run_id()))
    }
}

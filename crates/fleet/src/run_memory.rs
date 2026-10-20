//! How a repository runs, remembered from what its Jobs did. `docs/concepts/kit.md`, *What a
//! repository runs*.
//!
//! **No press and no agent's word.** The `Bash` calls of a hosted Job and what came back are read
//! off its stream (`adapters::session_runs`), kept for the session, and derived into facts as each
//! turn ends: the same rules as the import of past sessions, so the two merge under one key.
//! Fleet's working record is the store's `run_facts`; the place chosen for the repository when it
//! was added is where the facts are *written*:
//!
//! | place | written to |
//! |---|---|
//! | `this_mac` | `runs/<remote>.toml` under Kit's folder |
//! | `synced` | the same layout under the folder the person named |
//! | `repository` | `armada.yml` on the Job's own branch, committed, so it lands with the pull request |
//! | `cloud` | Armada's central store, or an `s3://`, `r2://` or `gs://` bucket: [`cloud`] |
//!
//! A repository marked Keep on this Mac takes `this_mac` only: [`kept_here`].
//!
//! **A failed guess is never kept** (a command whose every run failed is not a fact), **and a
//! dropped line never comes back**: it sits on a deny list the merge reads first.

pub(crate) mod cloud;
pub(crate) mod kept_here;
pub(crate) mod manifest;
pub(crate) mod places;

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, CommitTime, Delivery, Vcs, Worktree, WorkProduct};
use adapters::session_runs::{self, Record, RunKind};
use api::Refusal;
use ipc::{
    DropRunFact, Instant, RememberedRun, RunFact, RunFactKind, RunFactState, RunMemory, RunMemoryPlace, RunPlace,
    RunWorkspace, WireError,
};
use store::{KeptPlace, KeptRun};

use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;

/// A place that cannot be used as asked. A 422.
pub(crate) const PLACE_REFUSED: &str = "fleet.run_place_refused";
/// A cloud place with nothing to sign in with: no profile for the repository, or no credentials
/// for the bucket. A 422.
pub(crate) const CLOUD_UNSIGNED: &str = "fleet.run_cloud_unsigned";
/// A cloud place that did not answer. A 422.
pub(crate) const CLOUD_UNREACHABLE: &str = "fleet.run_cloud_unreachable";
/// A cloud place that answered no. A 422.
pub(crate) const CLOUD_REFUSED: &str = "fleet.run_cloud_refused";
/// A repository Fleet does not serve. A 404.
pub(crate) const NOT_SERVED: &str = "fleet.run_memory_repository_not_served";

/// The most records a session keeps, oldest dropped first.
const MOST_RECORDS: usize = 2000;

/// The commands a session ran and what came back, in memory.
#[derive(Default)]
pub(crate) struct RunLog {
    records: Vec<Record>,
}

impl RunLog {
    /// A stream line names no folder, so a call without one ran where the session runs.
    pub(crate) fn heard(&mut self, records: Vec<Record>, directory: &str) {
        for mut record in records {
            if let Record::Call { cwd, .. } = &mut record {
                if cwd.is_none() && !directory.is_empty() {
                    *cwd = Some(directory.to_string());
                }
            }
            self.records.push(record);
        }
        let over = self.records.len().saturating_sub(MOST_RECORDS);
        self.records.drain(..over);
    }
}

fn word(kind: RunKind) -> &'static str {
    match kind {
        RunKind::Tests => "tests",
        RunKind::Lint => "lint",
        RunKind::Build => "build",
        RunKind::DevServer => "dev_server",
        RunKind::Typecheck => "typecheck",
        RunKind::Format => "format",
    }
}

fn wire_word(kind: RunFactKind) -> &'static str {
    match kind {
        RunFactKind::Tests => "tests",
        RunFactKind::Lint => "lint",
        RunFactKind::Build => "build",
        RunFactKind::DevServer => "dev_server",
        RunFactKind::Typecheck => "typecheck",
        RunFactKind::Format => "format",
    }
}

fn wire_kind(word: &str) -> Option<RunFactKind> {
    Some(match word {
        "tests" => RunFactKind::Tests,
        "lint" => RunFactKind::Lint,
        "build" => RunFactKind::Build,
        "dev_server" => RunFactKind::DevServer,
        "typecheck" => RunFactKind::Typecheck,
        "format" => RunFactKind::Format,
        _ => return None,
    })
}

fn place_word(place: RunPlace) -> &'static str {
    match place {
        RunPlace::ThisMac => "this_mac",
        RunPlace::Repository => "repository",
        RunPlace::Synced => "synced",
        RunPlace::Cloud => "cloud",
    }
}

pub(crate) fn place_of(word: &str) -> RunPlace {
    match word {
        "repository" => RunPlace::Repository,
        "synced" => RunPlace::Synced,
        "cloud" => RunPlace::Cloud,
        _ => RunPlace::ThisMac,
    }
}

fn instant(millis: u64) -> Instant {
    Instant::carried(rfc3339_utc(millis as i64))
}

/// A Job's tree and branch, where `repository` writes.
pub(crate) struct JobTree {
    pub(crate) path: String,
    pub(crate) branch: String,
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
    /// What a session's stream said it ran.
    pub(crate) fn runs_heard(&self, runtime: &crate::session_host::Runtime, records: Vec<Record>) {
        let mut state = runtime.state();
        let directory = state.directory.clone();
        state.runs.heard(records, &directory);
    }

    /// A turn ended: derive what the session has run, merge it, and write it to the place.
    /// Quiet on every failure; the facts wait and the next turn tries again.
    pub(crate) async fn runs_learnt(&self, id: &str, runtime: &crate::session_host::Runtime) {
        let (records, directory) = {
            let state = runtime.state();
            (state.runs.records.clone(), state.directory.clone())
        };
        if records.is_empty() || directory.is_empty() {
            return;
        }
        let Ok((_, hosting)) = self.session_and_hosting(id).await else { return };
        let Ok(served) = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id))) else { return };
        let root = served.root().to_string();
        let learnt: Vec<KeptRun> = session_runs::derive(&records, &directory)
            .into_iter()
            .map(|fact| KeptRun {
                workspace: fact.workspace,
                kind: word(fact.kind).to_string(),
                command: fact.command,
                port: fact.port,
                succeeded: fact.succeeded,
                last_ran: fact.last_ran,
                source: session_runs::LIVE_SOURCE.to_string(),
                kept_in: None,
                why: None,
            })
            .collect();
        if self.run_memory_started(&root).await.is_err() {
            return;
        }
        if self.store().lock().await.merge_run_facts(&root, &learnt).is_err() {
            return;
        }
        let job = match (hosting.lease_slot, hosting.lease_branch) {
            (Some(slot), Some(branch)) => Some(JobTree { path: adapter_traits::slot_path(&root, slot), branch }),
            _ => None,
        };
        self.run_memory_written(&root, job).await;
    }

    /// The first time a repository is touched: the runs its past sessions show are merged in, and
    /// the place is `this_mac` until a person chooses another.
    pub(crate) async fn run_memory_started(&self, root: &str) -> Result<(), Refusal> {
        let started = self.store().lock().await.run_memory_place(root).map_err(|why| self.run_fault(why))?.is_some();
        if started {
            return Ok(());
        }
        if let Ok(imported) = self.import_runs(root.to_string()).await {
            let imported: Vec<KeptRun> = imported
                .facts
                .into_iter()
                .map(|fact| KeptRun {
                    workspace: fact.workspace,
                    kind: wire_word(fact.kind).to_string(),
                    command: fact.command,
                    port: fact.port,
                    succeeded: fact.succeeded,
                    last_ran: fact.last_ran.to_domain().epoch_millis().unwrap_or(0).max(0) as u64,
                    source: fact.source,
                    kept_in: None,
                    why: None,
                })
                .collect();
            self.store().lock().await.merge_run_facts(root, &imported).map_err(|why| self.run_fault(why))?;
        }
        self.store()
            .lock()
            .await
            .set_run_memory_place(root, place_word(RunPlace::ThisMac), None, None)
            .map_err(|why| self.run_fault(why))
    }

    /// Write what a repository remembers to its place, and say where each fact stands.
    pub(crate) async fn run_memory_written(&self, root: &str, job: Option<JobTree>) {
        let Ok(Some(KeptPlace { place, synced_folder: folder, cloud })) = self.store().lock().await.run_memory_place(root) else {
            return;
        };
        let place = place_of(&place);
        let mut standing: Vec<(KeptRun, Result<RunPlace, String>)> = Vec::new();
        match place {
            RunPlace::ThisMac | RunPlace::Synced => {
                let base = match (place, folder) {
                    (RunPlace::Synced, Some(folder)) if Path::new(&folder).is_dir() => Ok(PathBuf::from(folder)),
                    (RunPlace::Synced, _) => Err(String::from("the synced folder is not there")),
                    _ => Ok(PathBuf::from(&self.host().kit_home)),
                };
                let file = base.map(|base| places::file_for(&base, places::FOLDER, root));
                let held = match &file {
                    Ok(file) => places::read(file),
                    Err(why) => Err(why.clone()),
                };
                if let Ok(held) = &held {
                    let _ = self.store().lock().await.merge_run_facts(root, held);
                }
                let facts = self.store().lock().await.run_facts(root).unwrap_or_default();
                let wrote = match (file, held) {
                    (Ok(file), Ok(_)) => places::write(&file, root, &facts),
                    (Err(why), _) | (_, Err(why)) => Err(why),
                };
                standing.extend(facts.into_iter().map(|fact| (fact, wrote.clone().map(|()| place))));
            }
            RunPlace::Repository => {
                let facts: Vec<KeptRun> = self
                    .store()
                    .lock()
                    .await
                    .run_facts(root)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|fact| fact.kept_in.is_none())
                    .collect();
                match job {
                    None => standing.extend(
                        facts.into_iter().map(|fact| (fact, Err(String::from("waits for a Job's branch to write it on")))),
                    ),
                    Some(job) => standing.extend(self.run_memory_on_branch(job, facts).await),
                }
            }
            RunPlace::Cloud => {
                let synced = match cloud.as_deref().map(cloud::Target::kept) {
                    Some(Ok(target)) => self.cloud_synced(root, &target).await.map_err(|fault| fault.said().to_string()),
                    Some(Err(why)) => Err(why),
                    None => Err(String::from("no cloud address was chosen")),
                };
                let facts = self.store().lock().await.run_facts(root).unwrap_or_default();
                standing.extend(facts.into_iter().map(|fact| (fact, synced.clone().map(|()| place))));
            }
        }
        let mut store = self.store().lock().await;
        for (fact, outcome) in standing {
            let key = (fact.workspace.as_str(), fact.kind.as_str(), fact.command.as_str());
            let _ = match outcome {
                Ok(place) => store.stand_run_fact(root, key, Some(place_word(place)), None),
                Err(why) => store.stand_run_fact(root, key, None, Some(&why)),
            };
        }
    }

    /// Edit `armada.yml` in a Job's tree and commit those files on its branch.
    async fn run_memory_on_branch(
        &self,
        job: JobTree,
        facts: Vec<KeptRun>,
    ) -> Vec<(KeptRun, Result<RunPlace, String>)> {
        let vcs = std::sync::Arc::clone(self.vcs());
        let secs = self.now().epoch_millis().unwrap_or_default().div_euclid(1_000);
        let done = tokio::task::spawn_blocking(move || {
            let written = manifest::write(Path::new(&job.path), &facts);
            let committed = match written.changed.is_empty() {
                true => Ok(()),
                false => {
                    let paths: Vec<&str> = written.changed.iter().map(String::as_str).collect();
                    vcs.commit_paths(
                        &Worktree::at(job.path.clone(), job.branch.clone()),
                        &paths,
                        "Record how this repository runs, from what its Jobs ran",
                        CommitTime::seconds_since_epoch(secs),
                    )
                    .map(|_| ())
                    .map_err(|why| format!("armada.yml could not be committed: {why}"))
                }
            };
            written
                .standing
                .into_iter()
                .map(|(fact, outcome)| {
                    let outcome = match (&committed, outcome) {
                        (Err(why), Ok(())) => Err(why.clone()),
                        (_, Ok(())) => Ok(RunPlace::Repository),
                        (_, Err(why)) => Err(why),
                    };
                    (fact, outcome)
                })
                .collect::<Vec<_>>()
        })
        .await;
        done.unwrap_or_default()
    }

    /// The served repository a root names.
    pub(crate) fn run_repository(&self, repository: &str) -> Result<String, Refusal> {
        match self.repositories().served().iter().find(|one| one.root() == repository) {
            Some(served) => Ok(served.root().to_string()),
            None => Err(Refusal::NotFound(WireError::raised(
                NOT_SERVED,
                format!("Fleet does not serve {repository}"),
                self.run_id(),
            ))),
        }
    }

    pub(crate) fn run_fault(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised("fleet.run_memory_failed", why.to_string(), self.run_id()))
    }

    async fn run_memory_answer(&self, root: &str) -> Result<RunMemory, Refusal> {
        let (place, facts) = {
            let store = self.store().lock().await;
            let place = store.run_memory_place(root).map_err(|why| self.run_fault(why))?;
            (place, store.run_facts(root).map_err(|why| self.run_fault(why))?)
        };
        let mut workspaces: Vec<RunWorkspace> = Vec::new();
        for held in facts {
            let Some(kind) = wire_kind(&held.kind) else { continue };
            let remembered = RememberedRun {
                fact: RunFact {
                    workspace: held.workspace.clone(),
                    kind,
                    command: held.command,
                    port: held.port,
                    succeeded: held.succeeded,
                    last_ran: instant(held.last_ran),
                    source: held.source,
                },
                state: match held.kept_in {
                    Some(_) => RunFactState::Kept,
                    None => RunFactState::Waiting,
                },
                kept_in: held.kept_in.as_deref().map(place_of),
                why: held.why.or_else(|| held.kept_in.is_none().then(|| String::from("not written yet"))),
            };
            match workspaces.last_mut().filter(|one| one.workspace == held.workspace) {
                Some(one) => one.runs.push(remembered),
                None => workspaces.push(RunWorkspace { workspace: held.workspace, runs: vec![remembered] }),
            }
        }
        Ok(RunMemory {
            repository: root.to_string(),
            place: place.map_or(RunPlace::ThisMac, |kept| place_of(&kept.place)),
            workspaces,
        })
    }

    /// `get_run_memory`.
    pub(crate) async fn run_memory_read(&self, repository: String) -> Result<RunMemory, Refusal> {
        let root = self.run_repository(&repository)?;
        self.run_memory_started(&root).await?;
        // A read is not a reason to call the cloud: what it holds is brought level when a turn
        // ends, a place is chosen or a line is dropped.
        let in_cloud = self.store().lock().await.run_memory_place(&root).ok().flatten().is_some_and(|kept| place_of(&kept.place) == RunPlace::Cloud);
        if !in_cloud {
            self.run_memory_written(&root, None).await;
        }
        self.run_memory_answer(&root).await
    }

    /// `get_run_memory_place`.
    pub(crate) async fn run_memory_place_read(&self, repository: String) -> Result<RunMemoryPlace, Refusal> {
        let root = self.run_repository(&repository)?;
        let place = self.store().lock().await.run_memory_place(&root).map_err(|why| self.run_fault(why))?;
        Ok(match place {
            Some(kept) => RunMemoryPlace {
                repository: root,
                place: place_of(&kept.place),
                synced_folder: kept.synced_folder,
                cloud: kept.cloud.as_deref().and_then(|text| cloud::Target::kept(text).ok()).map(|target| target.wire()),
            },
            None => RunMemoryPlace { repository: root, place: RunPlace::ThisMac, synced_folder: None, cloud: None },
        })
    }

    /// `set_run_memory_place`.
    pub(crate) async fn run_memory_place_set(&self, chosen: RunMemoryPlace) -> Result<RunMemoryPlace, Refusal> {
        let root = self.run_repository(&chosen.repository)?;
        self.refused_while_kept_here(&root, chosen.place).await?;
        let refused = |code: &str, said: String| Refusal::Unacceptable(WireError::raised(code, said, self.run_id()));
        let mut cloud = None;
        let folder = match chosen.place {
            RunPlace::Cloud => {
                let Some(asked) = &chosen.cloud else {
                    return Err(refused(PLACE_REFUSED, String::from("a cloud place needs where: Armada's store or a bucket address")));
                };
                let target = cloud::Target::of(asked).map_err(|why| refused(PLACE_REFUSED, why))?;
                // The old place stays unless the new one answers now.
                self.cloud_read(&root, &target).await.map_err(|fault| refused(fault.code(), fault.said().to_string()))?;
                cloud = Some(target);
                None
            }
            RunPlace::Synced => match chosen.synced_folder.as_deref().map(str::trim).filter(|one| !one.is_empty()) {
                Some(folder) if Path::new(folder).is_dir() => Some(folder.to_string()),
                Some(folder) => return Err(refused(PLACE_REFUSED, format!("{folder} is not a folder"))),
                None => return Err(refused(PLACE_REFUSED, String::from("a synced place needs the folder they sync"))),
            },
            RunPlace::ThisMac | RunPlace::Repository => None,
        };
        self.run_memory_started(&root).await?;
        self.place_recorded(&root, chosen.place, folder.as_deref(), cloud.as_ref().map(cloud::Target::text).as_deref()).await?;
        self.run_memory_written(&root, None).await;
        Ok(RunMemoryPlace {
            repository: root,
            place: chosen.place,
            synced_folder: folder,
            cloud: cloud.as_ref().map(cloud::Target::wire),
        })
    }

    /// `drop_run_fact`.
    pub(crate) async fn run_fact_dropped(&self, drop: DropRunFact) -> Result<RunMemory, Refusal> {
        let root = self.run_repository(&drop.repository)?;
        self.run_memory_started(&root).await?;
        let at = self.now().as_str().to_string();
        self.store()
            .lock()
            .await
            .drop_run_fact(&root, (&drop.workspace, wire_word(drop.kind), &drop.command), &at)
            .map_err(|why| self.run_fault(why))?;
        // A place that keeps a file is written again without it.
        self.run_memory_written(&root, None).await;
        self.run_memory_answer(&root).await
    }
}

//! The librarian: Fleet work that keeps a project's catalog current. `docs/concepts/feature-catalog.md`, *Librarian*.
//!
//! **On demand until the owner says otherwise.** `features.librarianRuns` ships as `on_press`, and under it nothing here
//! starts a run: [`Fleet::librarians_due`] returns before it looks at a project. A press of Catch up is `run_librarian`.
//! `after_merges` and `nightly` are read only where the owner has set them, and a word that is neither reads as `on_press`.
//!
//! **This is the frame.** A run keeps its record (`librarian_runs`) and does the enabled parts that are built: a later group
//! adds its part to [`BUILT`] with the work it does. **A part that writes with a model spends from the night's share**
//! (`features.librarianModelShare`, in model calls for one project, a night being the machine's local midnight to midnight),
//! and where the share is spent the part is left out and the run says so.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Resolved};
use ipc::{Instant, LibrarianPart, LibrarianRan, LibrarianState, LibrarianStep, LibrarianStepMoved, LibrarianStepState, LibrarianStopping, RunLibrarian, StopLibrarian};

use super::doc_drift;
use super::librarian_gists;
use super::librarian_steps as steps;
use super::since_paragraph::Since;
use super::upkeep;
use store::{feature_link_kind, librarian_run_by, KeptLibrarianRun, Store, WriteError};

use crate::daemon::Fleet;

/// When the librarian runs without a press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Runs {
    /// Only on a press. What ships, and what any word Fleet cannot read means.
    OnPress,
    /// After a merge it has not read through.
    AfterMerges,
    /// Once in twenty-four hours.
    Nightly,
}

impl Runs {
    fn from_word(word: &str) -> Runs {
        match word {
            "after_merges" => Runs::AfterMerges,
            "nightly" => Runs::Nightly,
            _ => Runs::OnPress,
        }
    }

    fn by(self) -> &'static str {
        match self {
            Runs::OnPress => librarian_run_by::PRESS,
            Runs::AfterMerges => librarian_run_by::AFTER_MERGES,
            Runs::Nightly => librarian_run_by::NIGHTLY,
        }
    }
}

/// What `features.librarian*` say, read at one instant with the other live dials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Librarian {
    pub(crate) runs: Runs,
    /// The enabled parts, in the order they run.
    pub(crate) parts: Vec<LibrarianPart>,
    /// How many model calls a night may spend: `features.librarianModelShare`.
    pub(crate) model_share: u32,
}

impl Librarian {
    /// As it ships: on press, every part, twenty calls a night.
    pub(crate) fn shipped() -> Librarian {
        Librarian { runs: Runs::OnPress, parts: LibrarianPart::ALL.to_vec(), model_share: 20 }
    }

    pub(crate) fn overlaid_by(&self, settings: &Resolved) -> Librarian {
        Librarian {
            runs: settings.chosen(keys::LIBRARIAN_RUNS).map_or(self.runs, |word| Runs::from_word(&word)),
            parts: settings
                .chosen(keys::LIBRARIAN_PARTS)
                .map_or_else(|| self.parts.clone(), |names| LibrarianPart::ALL.into_iter().filter(|part| names.iter().any(|name| name == part.name())).collect()),
            model_share: settings.chosen(keys::LIBRARIAN_MODEL_SHARE).map_or(self.model_share, |n| u32::try_from(n).unwrap_or(0)),
        }
    }
}

/// The parts that have work behind them. Drift's is its free half: the model half runs only on a press of Check
/// (`check_docs`). Links and Retire share one free step, which reads features and linked docs against the code.
const BUILT: &[LibrarianPart] = &[LibrarianPart::Links, LibrarianPart::Drift, LibrarianPart::Retire, LibrarianPart::SinceYouLooked, LibrarianPart::Gists];

const A_DAY_MILLIS: i64 = 24 * 60 * 60 * 1000;

/// A press while the project's librarian is running.
const RUNNING: &str = "fleet.librarian_running";

/// Where `project`'s librarian stands: its last run, and the merges Fleet kept for its features since.
pub(crate) fn state_of(store: &Store, project: &str) -> Result<LibrarianState, WriteError> {
    let last = store.librarian_last_run(project)?;
    let mut merges = store.project_links(project, feature_link_kind::MERGE)?;
    // Oldest first, a pull request once however many features it touched.
    merges.reverse();
    let mut seen = std::collections::BTreeSet::new();
    let merged: Vec<(u64, String)> =
        merges.into_iter().filter_map(|link| link.target.parse::<u64>().ok().map(|number| (number, link.at))).filter(|(number, _)| seen.insert(*number)).collect();
    Ok(match last {
        None => LibrarianState { last_run: None, read_through: None, merged_since: merged.into_iter().map(|(number, _)| number).collect(), steps: Vec::new(), running: None },
        Some(run) => LibrarianState {
            last_run: Some(Instant::carried(run.ran_at)),
            read_through: Some(run.through_pr).filter(|number| *number > 0),
            merged_since: merged.into_iter().filter(|(_, at)| at.as_str() > run.through_at.as_str()).map(|(number, _)| number).collect(),
            steps: Vec::new(),
            running: None,
        },
    })
}

/// What the steps of `project` read, from the store.
fn counts_of(store: &Store, project: &str, docs: usize) -> Result<steps::Counts, WriteError> {
    let gaps = librarian_gists::lacking(store, project)?.len().min(librarian_gists::MOST_DOCS);
    Ok(steps::Counts { docs, features: upkeep::checked_count(store, project)?, gaps })
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
    /// `run_librarian`: the enabled parts for one project, now. **Whatever the setting says, a press runs.**
    pub(super) async fn librarian_pressed(&self, caller: Option<api::Caller>, run: RunLibrarian) -> Result<LibrarianRan, Refusal> {
        self.owner_or_operator_called(caller)?;
        self.librarian_ran(run.project.trim(), librarian_run_by::PRESS).await
    }

    /// One run for `project`, started by `by`, and its record. **Each step is told on the stream as it moves**, and a
    /// press of Stop skips the steps after the one running. One run of a project goes at a time.
    async fn librarian_ran(&self, project: &str, by: &str) -> Result<LibrarianRan, Refusal> {
        let Some(roots) = self.seed_roots(project) else {
            return Err(self.hosted_refusal(super::seeding::PROJECT, &format!("no project is named `{project}`")));
        };
        let at = self.now().as_str().to_string();
        let run = Instant::carried(at.clone());
        let Some(held) = self.librarian_running().begin(project, &run) else {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(RUNNING, format!("the librarian is already running for project `{project}`"), self.run_id())));
        };
        let set = self.librarian_set();
        let commit = match roots.first().cloned() {
            Some(root) => tokio::task::spawn_blocking(move || adapters::feature_commits::head(&root)).await.ok().flatten().unwrap_or_default(),
            None => String::new(),
        };
        let night = self.night_of(&at).await;
        let (state, last, spent, docs, counts) = {
            let store = self.store().lock().await;
            let state = state_of(&store, project).map_err(|why| self.ledger_fault(why))?;
            let last = store.librarian_last_run(project).map_err(|why| self.ledger_fault(why))?;
            let docs = doc_drift::linked_docs(&store, project).map_err(|why| self.ledger_fault(why))?;
            let counts = counts_of(&store, project, docs.len()).map_err(|why| self.ledger_fault(why))?;
            (state, last, super::doc_checking::night_spent(&store, project, &night).map_err(|why| self.ledger_fault(why))?, docs, counts)
        };
        let share = set.model_share;
        let steps = steps::steps_of(&set.parts, BUILT, &state, share, spent, counts);
        let tell = |step: &str, state: LibrarianStepState, said: String| {
            held.moved(step, state, &said);
            self.publish(ipc::Event::LibrarianStep(LibrarianStepMoved { project: project.to_string(), run: run.clone(), step: step.to_string(), state, said }));
        };
        let failed = |step: &str, why: WriteError| {
            tell(step, LibrarianStepState::Failed, "The store could not be read".to_string());
            self.ledger_fault(why)
        };
        let mut ran: Vec<LibrarianPart> = Vec::new();
        let mut skipped: Vec<LibrarianPart> = Vec::new();
        let mut paragraph = String::new();
        let mut calls: u32 = 0;
        let mut stopped = false;
        // Where the run read through: the run before's, until its own `read_through` step reads the newest merge.
        let mut through = last.as_ref().filter(|run| run.through_pr > 0).map(|run| (run.through_pr, run.through_at.clone()));
        let mut read_through = false;
        for step in &steps {
            stopped = stopped || held.stop_asked();
            if stopped {
                tell(&step.id, LibrarianStepState::Skipped, steps::STOPPED.to_string());
                continue;
            }
            tell(&step.id, LibrarianStepState::Running, String::new());
            match step.id.as_str() {
                steps::READ_MERGES => {
                    let since = last.as_ref().map_or("", |run| run.ran_at.as_str());
                    let sections = self.store().lock().await.sections_written_since(project, since).map_err(|why| failed(&step.id, why))?;
                    tell(&step.id, LibrarianStepState::Done, steps::merges_said(state.merged_since.len(), sections));
                }
                steps::READ_THROUGH => {
                    let newest = self.store().lock().await.project_links(project, feature_link_kind::MERGE).map_err(|why| failed(&step.id, why))?;
                    through = newest.first().and_then(|link| link.target.parse::<u64>().ok().map(|number| (number, link.at.clone())));
                    read_through = true;
                }
                steps::CHECK_DRIFT => {
                    let (inspected, checking) = (roots.clone(), docs.clone());
                    let found = tokio::task::spawn_blocking(move || doc_drift::check_docs(&inspected, &checking)).await.unwrap_or_default();
                    let flagged = doc_drift::keep_checked(&mut *self.store().lock().await, project, &at, found).map_err(|why| failed(&step.id, why))?;
                    ran.push(LibrarianPart::Drift);
                    tell(&step.id, LibrarianStepState::Done, steps::drift_said(flagged));
                }
                steps::CHECK_UPKEEP => {
                    let now = self.now().epoch_millis().unwrap_or(0);
                    let asked = upkeep::asked(&*self.store().lock().await, project, &docs, now).map_err(|why| failed(&step.id, why))?;
                    let (inspected, asking) = (roots.clone(), asked.clone());
                    let seen = tokio::task::spawn_blocking(move || upkeep::inspect(&inspected, &asking)).await.ok().flatten();
                    let Some(seen) = seen else {
                        tell(&step.id, LibrarianStepState::Skipped, steps::UPKEEP_UNREAD.to_string());
                        continue;
                    };
                    let counted = upkeep::keep(&mut *self.store().lock().await, project, &at, &asked, &seen).map_err(|why| failed(&step.id, why))?;
                    ran.extend(set.parts.iter().copied().filter(|part| matches!(part, LibrarianPart::Links | LibrarianPart::Retire)));
                    tell(&step.id, LibrarianStepState::Done, steps::upkeep_said(counted));
                }
                steps::WRITE_GISTS => {
                    let calls_left = share.saturating_sub(spent.saturating_add(calls));
                    let done = self.gist_docs(project, calls_left).await.map_err(|why| failed(&step.id, why))?;
                    calls += done.calls;
                    let used = spent + calls;
                    if done.found > 0 && done.calls == 0 {
                        skipped.push(LibrarianPart::Gists);
                        tell(&step.id, LibrarianStepState::Skipped, steps::share_spent_said(share));
                        continue;
                    }
                    if done.left > 0 {
                        skipped.push(LibrarianPart::Gists);
                    }
                    if done.calls > 0 && done.wrote == 0 {
                        tell(&step.id, LibrarianStepState::Failed, steps::gists_failed_said(used, share));
                        continue;
                    }
                    ran.push(LibrarianPart::Gists);
                    let said = match (done.found, done.lacking) {
                        (0, 0) => steps::GISTS_NONE.to_string(),
                        (0, _) => steps::GISTS_UNREAD.to_string(),
                        _ => steps::gists_said(done.wrote, used, share),
                    };
                    tell(&step.id, LibrarianStepState::Done, said);
                }
                name => {
                    let Some(part) = LibrarianPart::from_name(name) else { continue };
                    let since = self.store().lock().await.librarian_last_ran_part(project, name).map_err(|why| failed(&step.id, why))?;
                    let calls_left = share.saturating_sub(spent.saturating_add(calls));
                    if part == LibrarianPart::SinceYouLooked {
                        let (state, said) = match self.since_paragraph(project, since.as_deref(), calls_left).await {
                            Since::NoFacts => {
                                ran.push(part);
                                (LibrarianStepState::Skipped, steps::NOTHING_NEW.to_string())
                            }
                            Since::ShareSpent => {
                                skipped.push(part);
                                (LibrarianStepState::Skipped, steps::share_spent_said(share))
                            }
                            Since::Wrote(words) => {
                                calls += 1;
                                let said = steps::wrote_said(&words, spent + calls, share);
                                paragraph = words;
                                ran.push(part);
                                (LibrarianStepState::Done, said)
                            }
                            Since::Failed => {
                                calls += 1;
                                (LibrarianStepState::Failed, steps::failed_said(spent + calls, share))
                            }
                        };
                        tell(&step.id, state, said);
                    }
                }
            }
        }
        let kept = self.store().lock().await.keep_librarian_run(&KeptLibrarianRun {
            project: project.to_string(),
            ran_at: at.clone(),
            by: by.to_string(),
            through_pr: through.as_ref().map_or(0, |(number, _)| *number),
            through_at: through.as_ref().map(|(_, at)| at.clone()).unwrap_or_default(),
            at_commit: commit,
            parts: ran.iter().map(|part| part.name().to_string()).collect(),
            paragraph,
            night,
            model_calls: calls,
            skipped: skipped.iter().map(|part| part.name().to_string()).collect(),
            stopped,
        });
        if read_through {
            kept.map_err(|why| failed(steps::READ_THROUGH, why))?;
            tell(steps::READ_THROUGH, LibrarianStepState::Done, steps::through_said(through.as_ref().map(|(number, _)| *number)));
        } else {
            kept.map_err(|why| self.ledger_fault(why))?;
        }
        let read_through = through.filter(|_| read_through).map(|(number, _)| number);
        Ok(LibrarianRan { project: project.to_string(), ran_at: run, ran, read_through, skipped_for_share: skipped, stopped })
    }

    /// The steps a run of `project` would take now, for the listing. `night` is [`Fleet::librarian_night`]'s.
    pub(super) fn librarian_steps(&self, store: &Store, project: &str, state: &LibrarianState, night: &str) -> Result<Vec<LibrarianStep>, Refusal> {
        let set = self.librarian_set();
        let spent = super::doc_checking::night_spent(store, project, night).map_err(|why| self.ledger_fault(why))?;
        let docs = doc_drift::linked_docs(store, project).map_err(|why| self.ledger_fault(why))?;
        let counts = counts_of(store, project, docs.len()).map_err(|why| self.ledger_fault(why))?;
        Ok(steps::steps_of(&set.parts, BUILT, state, set.model_share, spent, counts))
    }

    /// The night it is now, which a night's share of model calls is counted by.
    pub(super) async fn librarian_night(&self) -> String {
        self.night_of(self.now().as_str()).await
    }

    /// `stop_librarian`: the project's running run stops before its next step.
    pub(super) fn librarian_stop_pressed(&self, caller: Option<api::Caller>, stop: StopLibrarian) -> Result<LibrarianStopping, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = stop.project.trim();
        if self.seed_roots(project).is_none() {
            return Err(self.hosted_refusal(super::seeding::PROJECT, &format!("no project is named `{project}`")));
        }
        Ok(LibrarianStopping { project: project.to_string(), stopping: self.librarian_running().stop(project) })
    }

    /// The night `at` falls in, as this machine's local date: `2026-11-18`. A night is local midnight to midnight.
    async fn night_of(&self, at: &str) -> String {
        let reading = crate::clock::local_reading(core_model::Timestamp::from_rfc3339(at.to_string()).epoch_millis().unwrap_or(0)).await;
        format!("{:04}-{:02}-{:02}", reading.year, reading.month, reading.day)
    }

    /// Once a tick: the projects whose librarian the setting says is due, run. **Under `on_press` this returns first, and
    /// nothing is read.**
    pub(crate) async fn librarians_due(&self) {
        let set = self.librarian_set();
        if set.runs == Runs::OnPress {
            return;
        }
        let projects: Vec<String> = self.projects_held().running().map(|one| one.id().as_str().to_string()).collect();
        for project in projects {
            let due = {
                let store = self.store().lock().await;
                if !store.project_has_features(&project).unwrap_or(false) {
                    continue;
                }
                let Ok(state) = state_of(&store, &project) else { continue };
                match set.runs {
                    Runs::OnPress => false,
                    Runs::AfterMerges => !state.merged_since.is_empty(),
                    Runs::Nightly => match &state.last_run {
                        None => true,
                        Some(at) => match (self.now().epoch_millis(), at.to_domain().epoch_millis()) {
                            (Some(now), Some(then)) => now - then >= A_DAY_MILLIS,
                            _ => false,
                        },
                    },
                }
            };
            if due {
                if let Err(refused) = self.librarian_ran(&project, set.runs.by()).await {
                    eprintln!("fleet: the librarian did not run for project {project}: {}", refused.error().message);
                }
            }
        }
    }
}

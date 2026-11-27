//! A note on a screen goes to its features' live Jobs, and the first to claim it takes it. Fleet routes by rule; which
//! Job's change a note is, the Jobs decide. `docs/concepts/feature-catalog.md`, *A note on a screen*.
//!
//! The paths that draw the screen map to features as any file does (`mapping`), and the Jobs on those features are the
//! ones `list_features` names. Unclaimed after [`UNCLAIMED_AFTER`], a note goes to the Job that last wrote one of the
//! paths, else to the first it was sent to. With no Job live on the features, Fleet starts one with his words.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ClaimNote, DispatchJob, DispatchThen, FeatureJob, NoteClaimed, NotesRouted, SendNotesToFeatures, SendSessionNotes, SessionId, StartSession, WireError};
use store::KeptNoteClaim;

use super::mapping::owns;
use super::reading::Loaded;
use crate::clock::elapsed;
use crate::daemon::Fleet;

/// How long a note sent to several Jobs waits for a claim before Fleet gives it to one. Long enough for a Job mid turn
/// to read it and answer; short enough that he does not wonder where it went. Chosen, not measured.
pub(crate) const UNCLAIMED_AFTER: Duration = Duration::from_secs(3 * 60);

/// The paths map to no feature: Bridge offers its picker instead. A 422.
const NO_FEATURE: &str = "fleet.notes_no_feature";
/// A note with no id cannot be claimed. A 422.
const UNNAMED: &str = "fleet.notes_unnamed";
const EMPTY: &str = "fleet.session_notes_empty";
/// A claim of a note Fleet did not send to several Jobs. A 404.
const UNKNOWN: &str = "fleet.note_unknown";
/// A claim from a Job the note was not sent to. A 422.
const NOT_OFFERED: &str = "fleet.note_not_offered";
/// A claim of a note another Job holds. A 409.
const TAKEN: &str = "fleet.note_taken";

/// How a Job came to hold a note Fleet gave it.
enum Given {
    LastWrote,
    FirstSent,
}

/// The most of his words a Job started for his notes is titled with, as Bridge titles one it starts.
const TITLE_CHARS: usize = 60;

/// A Job's title from his words: their first line that says anything, cut to [`TITLE_CHARS`].
fn titled(words: &str) -> String {
    let line = words.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
    line.chars().take(TITLE_CHARS).collect()
}

/// Names in a row: "a", "a and b", "a, b and c".
fn listed(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The line each Job reads under a note it was sent with other Jobs.
fn offer_line(note_id: &str, features: &[String], others: &[FeatureJob]) -> String {
    let on = listed(&features.iter().map(String::as_str).collect::<Vec<_>>());
    let sent = std::iter::once("you").chain(others.iter().map(|job| job.title.as_str())).collect::<Vec<_>>();
    format!("Fleet sent this note to the Jobs on {on}: {}. If its change is yours, call claim_note with note_id {note_id}. The first claim takes it.", listed(&sent))
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
    fn note_refusal(&self, code: &'static str, why: String) -> WireError {
        WireError::raised(code, why, self.run_id())
    }

    /// `send_notes_to_features`.
    pub(crate) async fn notes_to_features(self: &Arc<Self>, sent: SendNotesToFeatures) -> Result<NotesRouted, Refusal> {
        if sent.notes.is_empty() {
            return Err(Refusal::Unacceptable(self.note_refusal(EMPTY, "the notes sent hold no note: send at least one".into())));
        }
        if let Some(at) = sent.notes.iter().position(|note| note.note_id.as_deref().is_none_or(|id| !ipc::annotations::is_note_id(id))) {
            return Err(Refusal::Unacceptable(self.note_refusal(UNNAMED, format!("note {} has no note id, so no Job could claim it", at + 1))));
        }
        self.feature_writes_filed().await;
        let (project, features, jobs) = {
            let store = self.store().lock().await;
            let mut found: Option<(String, Vec<String>, Vec<FeatureJob>)> = None;
            for one in self.projects_asked(&store, None, sent.project.as_deref())? {
                let loaded = Loaded::read(&store, &one).map_err(|why| self.ledger_fault(why))?;
                let on = self.jobs_on_features(&store, &loaded.project)?;
                let (mut titles, mut ids, mut jobs): (Vec<String>, Vec<String>, Vec<FeatureJob>) = (Vec::new(), Vec::new(), Vec::new());
                for path in &sent.paths {
                    for owner in loaded.map.owners(Some(sent.manifest_id.as_str()), path.trim().trim_start_matches("./")) {
                        if !titles.contains(&owner.feature.title) {
                            titles.push(owner.feature.title.clone());
                            ids.push(owner.feature.id.clone());
                        }
                        for job in on.get(&owner.feature.id).into_iter().flatten() {
                            if !jobs.iter().any(|seen| seen.session_id == job.session_id) {
                                jobs.push(job.clone());
                            }
                        }
                    }
                }
                // The first Job on the features is the one that wrote under them first.
                let first_wrote = |job: &FeatureJob| {
                    let files = store.feature_files_of_job(job.session_id.as_str()).unwrap_or_default();
                    files.into_iter().filter(|file| ids.contains(&file.feature_id)).map(|file| file.first_at).min()
                };
                jobs.sort_by_cached_key(first_wrote);
                if !titles.is_empty() {
                    found = Some((loaded.project.clone(), titles, jobs));
                    break;
                }
            }
            found.ok_or_else(|| Refusal::Unacceptable(self.note_refusal(NO_FEATURE, format!("no feature owns {}", sent.paths.join(", ")))))?
        };
        let ids: Vec<String> = sent.notes.iter().filter_map(|note| note.note_id.clone()).collect();
        let at = self.now().as_str().to_string();
        if jobs.is_empty() {
            let start = StartSession {
                manifest_id: sent.manifest_id.clone(),
                title: Some(titled(&sent.words)),
                model: None,
                effort: None,
                mode: None,
                fork: None,
                root: None,
                request: Some(sent.words.clone()),
                after: None,
                held: false,
                project_rows: Vec::new(),
                project: Some(project),
                on_your_words: None,
                workflow: None,
            };
            let record = self.dispatch_started(DispatchJob { start, kind: None, then: vec![DispatchThen::Notes(sent.notes)] }).await?;
            let started = FeatureJob { session_id: record.id.clone(), title: record.title.clone().unwrap_or_else(|| titled(&sent.words)) };
            let mut store = self.store().lock().await;
            for id in &ids {
                let kept = KeptNoteClaim {
                    note_id: id.clone(),
                    manifest_id: sent.manifest_id.as_str().to_string(),
                    paths: sent.paths.clone(),
                    offered: vec![started.session_id.as_str().to_string()],
                    sent_at: at.clone(),
                    claimed_by: started.session_id.as_str().to_string(),
                    claimed_at: at.clone(),
                    given: true,
                };
                store.keep_note_offered(&kept).map_err(|why| self.ledger_fault(why))?;
            }
            return Ok(NotesRouted { features, offered: Vec::new(), started: Some(started) });
        }
        {
            let mut store = self.store().lock().await;
            for id in &ids {
                let kept = KeptNoteClaim {
                    note_id: id.clone(),
                    manifest_id: sent.manifest_id.as_str().to_string(),
                    paths: sent.paths.clone(),
                    offered: jobs.iter().map(|job| job.session_id.as_str().to_string()).collect(),
                    sent_at: at.clone(),
                    claimed_by: String::new(),
                    claimed_at: String::new(),
                    given: false,
                };
                store.keep_note_offered(&kept).map_err(|why| self.ledger_fault(why))?;
            }
        }
        for job in &jobs {
            let others: Vec<FeatureJob> = jobs.iter().filter(|other| other.session_id != job.session_id).cloned().collect();
            let mut notes = sent.notes.clone();
            for note in &mut notes {
                let id = note.note_id.clone().unwrap_or_default();
                note.said = format!("{}\n\n{}", note.said.trim_end(), offer_line(&id, &features, &others));
            }
            // One Job refusing its notes, ended since it was read, is no reason to keep them from the others.
            if let Err(why) = Arc::clone(self).notes_sent(SendSessionNotes { session_id: job.session_id.clone(), notes }).await {
                eprintln!("job {}: notes on {} were not sent: {}", job.session_id.as_str(), features.join(", "), why.error().message);
            }
        }
        Ok(NotesRouted { features, offered: jobs, started: None })
    }

    /// `claim_note`.
    pub(crate) async fn note_claimed(self: &Arc<Self>, caller: Option<api::Caller>, claim: ClaimNote) -> Result<NoteClaimed, Refusal> {
        let id = self.waiting_session(caller, claim.session_id.as_ref())?;
        let at = self.now().as_str().to_string();
        let (kept, took) = {
            let mut store = self.store().lock().await;
            let kept = store
                .note_claim(&claim.note_id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| Refusal::NotFound(self.note_refusal(UNKNOWN, format!("no note {} was sent to several jobs", claim.note_id))))?;
            if kept.claimed_by.is_empty() && !kept.offered.contains(&id) {
                return Err(Refusal::Unacceptable(self.note_refusal(NOT_OFFERED, format!("note {} was not sent to this job", claim.note_id))));
            }
            let took = kept.claimed_by.is_empty() && store.take_note(&claim.note_id, &id, &at, false).map_err(|why| self.ledger_fault(why))?;
            let kept = if took { kept } else { store.note_claim(&claim.note_id).map_err(|why| self.ledger_fault(why))?.unwrap_or(kept) };
            (kept, took)
        };
        let title_of = |store: &store::Store, job: &str| store.session(job).ok().flatten().and_then(|session| session.title).unwrap_or_else(|| job.to_string());
        if !took {
            let title = title_of(&*self.store().lock().await, &kept.claimed_by);
            if kept.claimed_by == id {
                return Ok(NoteClaimed { note_id: claim.note_id, session_id: SessionId::carried(&id), title, given: kept.given });
            }
            let how = if kept.given { "Fleet gave it to" } else { "it was claimed by" };
            return Err(Refusal::IllegalMove(self.note_refusal(TAKEN, format!("note {} is taken: {how} {title}", claim.note_id))));
        }
        let title = title_of(&*self.store().lock().await, &id);
        Ok(self.note_taken(&kept, &id, &title, None).await)
    }

    /// `job` holds the note now: its file names the Job, the others drop it, and Bridge is told.
    async fn note_taken(self: &Arc<Self>, kept: &KeptNoteClaim, job: &str, title: &str, given: Option<Given>) -> NoteClaimed {
        let others: Vec<String> = kept.offered.iter().filter(|other| *other != job).cloned().collect();
        self.note_moved_to(&kept.note_id, job, title, &others).await;
        let note = &kept.note_id;
        let dropped = match given {
            None => format!("{title} claimed note {note}, so it is theirs. Drop it."),
            Some(_) => format!("Nobody claimed note {note}, so Fleet gave it to {title}. Drop it."),
        };
        for other in &others {
            let _ = self.told_of_his_act(other, &dropped).await;
        }
        if let Some(why) = given {
            let why = match why {
                Given::LastWrote => "you last wrote the files of the screen it was left on",
                Given::FirstSent => "it went to you first",
            };
            let yours = format!("Nobody claimed note {note}, so it is yours: {why}. Make its change, then call mark_annotation_done with note_id {note}.");
            let _ = self.told_of_his_act(job, &yours).await;
        }
        let claimed = NoteClaimed { note_id: note.clone(), session_id: SessionId::carried(job), title: title.to_string(), given: kept.given };
        self.publish(ipc::Event::NoteClaimed(claimed.clone()));
        claimed
    }

    /// Each note no Job claimed within [`UNCLAIMED_AFTER`], given to one.
    pub(crate) async fn notes_given_away(self: &Arc<Self>) {
        self.notes_unclaimed_given(UNCLAIMED_AFTER).await;
    }

    /// Each note no Job claimed within `after`, given to the live Job that last wrote one of its paths, else to the first
    /// live one it was sent to. One none of them is live to take is forgotten, and stays in its file sent to nobody.
    pub(crate) async fn notes_unclaimed_given(self: &Arc<Self>, after: Duration) {
        let now = self.now();
        let waiting = match self.store().lock().await.notes_unclaimed() {
            Ok(waiting) => waiting,
            Err(_) => return,
        };
        let due: Vec<KeptNoteClaim> = waiting.into_iter().filter(|kept| elapsed(&core_model::Timestamp::from_rfc3339(&kept.sent_at), &now) >= after).collect();
        if due.is_empty() {
            return;
        }
        self.feature_writes_filed().await;
        let at = now.as_str().to_string();
        for mut kept in due {
            let picked = {
                let mut store = self.store().lock().await;
                let live: Vec<&String> = kept
                    .offered
                    .iter()
                    .filter(|job| store.session(job).ok().flatten().is_some_and(|session| session.state == store::SessionState::Live))
                    .collect();
                let wrote = |job: &str| {
                    let files = store.feature_files_of_job(job).unwrap_or_default();
                    files.into_iter().filter(|file| kept.paths.iter().any(|path| owns(path, &file.path))).map(|file| file.last_at).max()
                };
                let last = live.iter().filter_map(|job| Some((wrote(job)?, (*job).clone()))).max();
                let picked = match (last, live.first()) {
                    (Some((_, job)), _) => Some((job, Given::LastWrote)),
                    (None, Some(job)) => Some(((*job).clone(), Given::FirstSent)),
                    (None, None) => None,
                };
                match picked {
                    Some((job, why)) if store.take_note(&kept.note_id, &job, &at, true).unwrap_or(false) => {
                        let title = store.session(&job).ok().flatten().and_then(|session| session.title).unwrap_or_else(|| job.clone());
                        Some((job, title, why))
                    }
                    Some(_) => None,
                    None => {
                        let _ = store.drop_note_claim(&kept.note_id);
                        None
                    }
                }
            };
            if let Some((job, title, why)) = picked {
                kept.given = true;
                self.note_taken(&kept, &job, &title, Some(why)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_names_the_other_jobs_and_how_to_claim() {
        let job = |title: &str| FeatureJob { session_id: SessionId::carried(title), title: title.to_string() };
        let line = offer_line("mfz1-abc", &["Feature catalog".to_string()], &[job("Fix the rail"), job("Audit docs")]);
        assert_eq!(line, "Fleet sent this note to the Jobs on Feature catalog: you, Fix the rail and Audit docs. If its change is yours, call claim_note with note_id mfz1-abc. The first claim takes it.");
    }
}

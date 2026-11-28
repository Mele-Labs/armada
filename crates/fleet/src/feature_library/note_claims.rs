//! A note on a screen goes to its features' live Jobs, and the first to claim it takes it. Fleet routes by rule; which
//! Job's change a note is, the Jobs decide. `docs/concepts/feature-catalog.md`, *A note on a screen*.
//!
//! The features the screen showed, named by slug, and those the paths that draw it map to as any file does (`mapping`),
//! are the note's; the Jobs on them are the ones `list_features` names. One Job alone on them is given the note at once. Unclaimed after [`UNCLAIMED_AFTER`], a
//! note goes to the Job that last wrote one of the paths, else to the first it was sent to. With no Job live on the
//! features, or none of those it was sent to live when the wait is over, Fleet starts one with his words.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ClaimNote, DispatchJob, DispatchThen, FeatureJob, ManifestId, NoteClaimed, NotesRouted, SendNotesToFeatures, SendSessionNotes, SessionId, SessionNote, StartSession, WireError};
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

/// The claim row of each of `sent`'s notes, sent to `offered` at `at` and held by `holder` where Fleet gave it one.
fn kept_rows(sent: &SendNotesToFeatures, project: &str, offered: &[String], holder: Option<&str>, at: &str) -> Vec<KeptNoteClaim> {
    sent.notes
        .iter()
        .map(|note| KeptNoteClaim {
            note_id: note.note_id.clone().unwrap_or_default(),
            manifest_id: sent.manifest_id.as_str().to_string(),
            paths: sent.paths.clone(),
            offered: offered.to_vec(),
            sent_at: at.to_string(),
            claimed_by: holder.unwrap_or_default().to_string(),
            claimed_at: holder.map(|_| at.to_string()).unwrap_or_default(),
            given: holder.is_some(),
            project: project.to_string(),
            words: sent.words.clone(),
            note: ipc::encode(note).unwrap_or_default(),
        })
        .collect()
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
                // The features the screen showed, by slug, then those its files map to.
                let named = sent.features.iter().filter_map(|slug| loaded.feature_named(slug.trim()));
                let owning = sent.paths.iter().flat_map(|path| loaded.map.owners(Some(sent.manifest_id.as_str()), path.trim().trim_start_matches("./")).into_iter().map(|owner| owner.feature));
                for feature in named.chain(owning) {
                    if !ids.contains(&feature.id) {
                        titles.push(feature.title.clone());
                        ids.push(feature.id.clone());
                    }
                    for job in on.get(&feature.id).into_iter().flatten() {
                        if !jobs.iter().any(|seen| seen.session_id == job.session_id) {
                            jobs.push(job.clone());
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
        let at = self.now().as_str().to_string();
        let ids: Vec<String> = jobs.iter().map(|job| job.session_id.as_str().to_string()).collect();
        if let [only] = jobs.as_slice() {
            // One Job on the features: the note is its own, with nobody to claim it from.
            match Arc::clone(self).notes_sent(SendSessionNotes { session_id: only.session_id.clone(), notes: sent.notes.clone() }).await {
                Ok(_) => {
                    let mut store = self.store().lock().await;
                    for kept in kept_rows(&sent, &project, &ids, Some(only.session_id.as_str()), &at) {
                        store.keep_note_offered(&kept).map_err(|why| self.ledger_fault(why))?;
                    }
                    return Ok(NotesRouted { features, offered: jobs, started: None });
                }
                Err(why) => eprintln!("job {}: notes on {} were not sent, so a Job is started for them: {}", only.session_id.as_str(), features.join(", "), why.error().message),
            }
        }
        if jobs.len() < 2 {
            let started = self.started_for_notes(sent.manifest_id.clone(), &project, &sent.words, sent.notes.clone()).await?;
            let mut store = self.store().lock().await;
            for kept in kept_rows(&sent, &project, &[started.session_id.as_str().to_string()], Some(started.session_id.as_str()), &at) {
                store.keep_note_offered(&kept).map_err(|why| self.ledger_fault(why))?;
            }
            return Ok(NotesRouted { features, offered: Vec::new(), started: Some(started) });
        }
        {
            let mut store = self.store().lock().await;
            for kept in kept_rows(&sent, &project, &ids, None, &at) {
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

    /// A Job started with his words that holds `notes` from the start, titled with them and not held for his press.
    async fn started_for_notes(self: &Arc<Self>, manifest_id: ManifestId, project: &str, words: &str, notes: Vec<SessionNote>) -> Result<FeatureJob, Refusal> {
        let start = StartSession {
            manifest_id,
            title: Some(titled(words)),
            model: None,
            effort: None,
            mode: None,
            fork: None,
            root: None,
            request: Some(words.to_string()),
            after: None,
            held: false,
            project_rows: Vec::new(),
            project: Some(project.to_string()),
            on_your_words: None,
            workflow: None,
        };
        let record = self.dispatch_started(DispatchJob { start, kind: None, then: vec![DispatchThen::Notes(notes)] }).await?;
        Ok(FeatureJob { session_id: record.id.clone(), title: record.title.clone().unwrap_or_else(|| titled(words)) })
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
    /// live one it was sent to. Notes none of them is live to take start a Job, one for the notes sent together.
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
        let mut orphans: Vec<KeptNoteClaim> = Vec::new();
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
                        orphans.push(kept.clone());
                        None
                    }
                }
            };
            if let Some((job, title, why)) = picked {
                kept.given = true;
                self.note_taken(&kept, &job, &title, Some(why)).await;
            }
        }
        self.orphans_started(orphans, &at).await;
    }

    /// One Job started for each group of notes sent together whose Jobs have all ended. A start that fails leaves the
    /// notes unclaimed, to be tried on the next tick.
    async fn orphans_started(self: &Arc<Self>, mut orphans: Vec<KeptNoteClaim>, at: &str) {
        while let Some(first) = orphans.first().cloned() {
            let together = |kept: &KeptNoteClaim| kept.sent_at == first.sent_at && kept.offered == first.offered && kept.manifest_id == first.manifest_id;
            let (group, rest): (Vec<KeptNoteClaim>, Vec<KeptNoteClaim>) = orphans.into_iter().partition(together);
            orphans = rest;
            let notes: Vec<SessionNote> = group.iter().filter_map(|kept| ipc::decode::<SessionNote>("a note kept for its Jobs", kept.note.as_bytes()).ok()).collect();
            if notes.is_empty() {
                continue;
            }
            let started = match self.started_for_notes(ManifestId::carried(&first.manifest_id), &first.project, &first.words, notes).await {
                Ok(started) => started,
                Err(why) => {
                    eprintln!("notes {}: their Jobs have ended and no Job was started for them: {}", group.iter().map(|kept| kept.note_id.as_str()).collect::<Vec<_>>().join(", "), why.error().message);
                    continue;
                }
            };
            let job = started.session_id.as_str();
            for kept in &group {
                if !self.store().lock().await.take_note(&kept.note_id, job, at, true).unwrap_or(false) {
                    continue;
                }
                self.note_moved_to(&kept.note_id, job, &started.title, &kept.offered).await;
                self.publish(ipc::Event::NoteClaimed(NoteClaimed { note_id: kept.note_id.clone(), session_id: started.session_id.clone(), title: started.title.clone(), given: true }));
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

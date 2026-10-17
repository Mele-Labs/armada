//! Reading what a retro's record is assembled from: the routes' own answers
//! about the Job, its Drones' transcripts, its log and its notes, and the
//! owner's annotations made while it was open.
//!
//! **The routes' answers, by calling them.** `get_job`, `get_job_events`,
//! `get_evidence` and `list_job_drones` are what `scripts/job` reads over HTTP,
//! so the retro reads the Job the way a person investigating it does.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Queries, Refusal};
use core_model::JobId;
use ipc::{AnnotationFile, DroneId, LinkedAnnotation, RetroRecord, TranscriptRow};

use super::record::{assembled, Sources};
use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::transcript::transcript_of;

/// A Job's record, and what else a retro reads beside it.
pub(crate) struct Gathered {
    pub record: RetroRecord,
    pub annotations: Vec<LinkedAnnotation>,
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

    /// The record as it stands now. **Read again on every call**: nothing
    /// here is kept, so a read after the Job ended says what the record says.
    pub(crate) async fn retro_record(&self, job_id: &JobId) -> Result<Gathered, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let id: ipc::JobId = job_id.into();
        let detail = self.job_detail(id.clone()).await?;
        let history = Queries::get_job_events(self, id.clone()).await?;
        let (evidence, drone_ids) = {
            let store = self.store().lock().await;
            let recorded = store
                .step_evidence(job_id)
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let events = store
                .events_for(job_id)
                .map_err(|why| self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(why))))?;
            let drone_ids: Vec<DroneId> = events
                .iter()
                .filter_map(|event| match event.moved() {
                    store::Moved::Drone {
                        drone_id,
                        presence: core_model::DronePresence::Spawned,
                        ..
                    } => Some(DroneId::from(drone_id)),
                    _ => None,
                })
                .collect();
            (
                ipc::JobEvidence {
                    job_id: id,
                    steps: recorded.iter().map(crate::wire::submitted).collect(),
                },
                drone_ids,
            )
        };
        let served = self.served_by(&job).map_err(|why| self.refusal(why))?;
        let handle = job.handle();
        let transcripts = transcripts_of(served.records_root(), &handle, &drone_ids);
        let log = crate::journal::read_from(served.records_root(), &handle, 0).notes;
        let notes = self
            .store()
            .lock()
            .await
            .drone_notes_for(job_id)
            .map_err(|cause| {
                self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause)))
            })?;
        let record = assembled(&Sources {
            detail: &detail,
            history: &history,
            evidence: &evidence,
            transcripts: &transcripts,
            log: &log,
            notes: &notes,
        });
        Ok(Gathered {
            annotations: annotations_on(served.root(), job_id),
            record,
        })
    }
}

/// Every row of each Drone's transcript, one list per Drone, in the order the
/// Drones are named. **A line that will not decode is passed over**: it is a
/// row Armada did not write whole, and the rest of the file still reads.
pub(crate) fn transcripts_of(
    records_root: &str,
    handle: &str,
    drones: &[DroneId],
) -> Vec<Vec<TranscriptRow>> {
    drones
        .iter()
        .map(|drone| {
            let path = transcript_of(records_root, handle, &drone.to_domain());
            let Ok(file) = fs::File::open(path) else {
                return Vec::new();
            };
            BufReader::new(file)
                .lines()
                .map_while(Result::ok)
                .filter_map(|line| {
                    ipc::decode::<TranscriptRow>("a transcript row", line.as_bytes()).ok()
                })
                .collect()
        })
        .collect()
}

/// The owner's notes under `<root>/.armada/annotations/` that were left with
/// this Job's detail open, oldest first.
///
/// **Linked by the id the note carries, and by nothing else.** A note that
/// names no Job is linked to none: a time window would attach a note about the
/// Board to whichever Job happened to be running.
pub(crate) fn annotations_on(root: &str, job: &JobId) -> Vec<LinkedAnnotation> {
    let Ok(entries) = fs::read_dir(Path::new(root).join(".armada").join("annotations")) else {
        return Vec::new();
    };
    let mut linked: Vec<LinkedAnnotation> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|bytes| ipc::decode::<AnnotationFile>("an annotation", &bytes).ok())
        .filter(|note| note.open_job_id.as_deref() == Some(job.as_str()))
        .map(|note| LinkedAnnotation {
            id: note.id,
            at: ipc::Instant::carried(note.created_at),
            text: note.text,
            screen: note.screen,
            selector: note.selector,
        })
        .collect();
    linked.sort_by(|one, other| one.at.as_str().cmp(other.at.as_str()));
    linked
}

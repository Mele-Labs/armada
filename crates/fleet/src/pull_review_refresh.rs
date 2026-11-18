//! The Reviews lists kept, answered at once, and the read of the forge that follows them.
//! `docs/practices/protocol.md`, *The Reviews tab*.
//!
//! **Read when somebody looks, still never polled**: a read goes out only because `get_review_lists` was
//! asked. Where lists were kept they answer at once, `refreshing`, and the forge is read behind them on a task of
//! its own, one a project; what it finds is kept and told as `review_lists.changed`. Where it finds nothing, the
//! kept rows are left alone and the event says why.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{PullReviewLists, ReviewListsChanged, ReviewListsFailed, WireValue};
use store::review_lists_kept::KeptReviewLists;

use crate::daemon::Fleet;
use crate::pull_review_kept::{limited, Reading};

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
    /// A read that reached the forge: kept in memory and in the store, so a Fleet that starts has it.
    pub(crate) async fn keep_lists(&self, project: Option<&str>, lists: &PullReviewLists) {
        self.review_lists_kept().keep(project, lists);
        let Ok(text) = ipc::encode(lists) else { return };
        let kept = KeptReviewLists { project: project.unwrap_or_default().to_string(), fetched_at: lists.fetched_at.clone(), lists: text };
        // Unkept, the next Fleet reads the forge before it answers; this one answers from memory.
        if let Err(why) = self.store().lock().await.keep_review_lists(&kept) {
            eprintln!("the review lists were read and not kept: {why}");
        }
    }

    /// The lists last kept for a project: what this Fleet read, else what the store holds. **A row that
    /// does not decode, as an older build wrote it, is nothing kept**, never an error to the person.
    pub(crate) async fn kept_lists(&self, project: Option<&str>) -> Option<PullReviewLists> {
        if let Some(lists) = self.review_lists_kept().last(project) {
            return Some(lists);
        }
        let stored = self.store().lock().await.kept_review_lists(project.unwrap_or_default()).ok().flatten()?;
        let lists: PullReviewLists = ipc::decode("kept review lists", stored.lists.as_bytes()).ok()?;
        self.review_lists_kept().recalled(project, &lists);
        Some(lists)
    }

    /// `get_review_lists`: kept lists at once with the forge read behind them; none kept, the read itself.
    pub(crate) async fn review_lists_answered(self: Arc<Self>, project: Option<String>) -> Result<PullReviewLists, Refusal> {
        let key = project.as_deref();
        let began = std::time::Instant::now();
        let mut joined = false;
        loop {
            let now = self.clock().now().epoch_millis().unwrap_or(0);
            let hold = self.review_lists_kept().held(now);
            let Some(mut lists) = self.kept_lists(key).await else {
                if hold.is_some() {
                    return self.review_lists_read(key).await;
                }
                match self.review_lists_kept().begin(key) {
                    Reading::Began(_reading) => return self.review_lists_read(key).await,
                    Reading::Going(mut ended) => {
                        // Nothing to show yet, so wait on the read going rather than ask the forge twice.
                        let _ = ended.changed().await;
                        joined = true;
                        continue;
                    }
                }
            };
            // A project he has none of is still a 404, whatever was kept for it.
            if let Some(id) = key {
                self.project_repositories(id).await?;
            }
            let overlaid = self.overlay_lists(&mut lists).await;
            let answered = match hold {
                Some(hold) => limited(Some(lists), hold).map_err(|(until, said)| {
                    crate::pull_review_kept::rate_limited_refusal(|code, message| self.pr_refusal(code, message), until, said)
                })?,
                // Having waited on a read, these are as fresh as the forge gave them.
                None if joined => lists,
                None => {
                    lists.refreshing = true;
                    self.read_behind(project);
                    lists
                }
            };
            eprintln!("review_lists timing: kept lists answered in {}ms ({overlaid})", began.elapsed().as_millis());
            return Ok(answered);
        }
    }

    /// The local overlays of a read, applied again to lists kept, so what he has seen and his inbox are current.
    async fn overlay_lists(&self, lists: &mut PullReviewLists) -> String {
        // A review he ran since the read marks its row, on whichever list holds it.
        let served = self.named_repositories().await;
        let reviewed = self.reviewed_pulls(&served).await;
        for row in lists.lists_mut().into_iter().flatten() {
            row.reviewing = reviewed.iter().any(|(id, number)| *id == row.manifest_id && *number == row.number);
        }
        self.overlays_onto(&mut lists.lists_mut()).await
    }

    /// Read the forge for a project on a task of its own, unless that read is going already.
    fn read_behind(self: &Arc<Self>, project: Option<String>) {
        let Reading::Began(reading) = self.review_lists_kept().begin(project.as_deref()) else { return };
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            let read = fleet.review_lists_read(project.as_deref()).await;
            // Ended before it is told, so a Bridge that asks on the event finds no read going.
            drop(reading);
            let told = match read {
                Ok(lists) => ReviewListsChanged::read(project, lists),
                Err(refusal) => ReviewListsChanged::failed(project, failure_of(&refusal)),
            };
            fleet.publish(ipc::Event::ReviewListsChanged(told));
        });
    }
}

/// A refusal as the event says it: its code and sentence, and the time to try again where it names one.
fn failure_of(refusal: &Refusal) -> ReviewListsFailed {
    let error = refusal.error();
    let until = match error.fields.get("until") {
        Some(WireValue::Str(at)) => Some(ipc::Instant::carried(at.clone())),
        _ => None,
    };
    ReviewListsFailed { code: error.code.clone(), said: error.message.clone(), until }
}

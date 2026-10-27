//! `dispute_fleet`: a hosted job says Fleet's latest notice in it was wrong. The dispute is kept,
//! a review job is started or joined to settle it (`dispute_review`), and the job's thread gains a
//! `disputed` row naming both. Fleet reading the same out of the agent's replies is
//! `dispute_reading`, and ends here too.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DisputeFleet, DisputeSpotted, FleetDisputed, SessionRow, SessionVoice};
use store::KeptDispute;

use super::serving::FLEET_NAME;
use crate::daemon::Fleet;

const DISPUTE_REFUSED: &str = "fleet.dispute_refused";

/// Held while a dispute's review is settled, so two disputes of one notice at once start one job.
/// Process-wide, as `flight_controls`' lock is: one Fleet runs in a process.
static SETTLING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// What a dispute is about, before it is kept.
pub(crate) struct Disputing<'a> {
    pub session: &'a str,
    pub notice_row: &'a str,
    pub notice: &'a str,
    pub why: &'a str,
    pub spotted: DisputeSpotted,
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
    /// The job's latest Fleet notice, disputed. Refused for a job Fleet does not host, an ended
    /// one, an empty reason, and a job Fleet has told nothing.
    pub(crate) async fn fleet_disputed(
        self: &Arc<Self>,
        caller: Option<api::Caller>,
        dispute: DisputeFleet,
    ) -> Result<FleetDisputed, Refusal> {
        let id = self.waiting_session(caller, dispute.session_id.as_ref())?;
        let why = dispute.why.trim();
        if why.is_empty() {
            return Err(self.hosted_refusal(DISPUTE_REFUSED, "a dispute needs `why`, the reason Fleet's notice was wrong"));
        }
        let (session, _) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let rows = self.rows_of(&id).await?;
        let Some((notice_row, notice)) = latest_notice(&rows) else {
            return Err(self.hosted_refusal(
                DISPUTE_REFUSED,
                "Fleet has sent this job no notice, so there is nothing to dispute",
            ));
        };
        let disputing = Disputing { session: &id, notice_row, notice, why, spotted: DisputeSpotted::Called };
        self.dispute_kept(&disputing, &rows).await
    }

    /// Keep a dispute, settle its review, and write the `disputed` row to the job's thread.
    /// `rows` is the job's thread as it stood, which the review job is briefed with.
    pub(crate) async fn dispute_kept(
        self: &Arc<Self>,
        disputing: &Disputing<'_>,
        rows: &[SessionRow],
    ) -> Result<FleetDisputed, Refusal> {
        let dispute_id = self.mint().ulid().as_str().to_lowercase();
        let kept = KeptDispute {
            id: dispute_id.clone(),
            session_id: disputing.session.to_string(),
            notice_row: disputing.notice_row.to_string(),
            notice: disputing.notice.to_string(),
            why: disputing.why.to_string(),
            spotted: disputing.spotted.as_str().to_string(),
            at: self.now().as_str().to_string(),
            key: notice_key(disputing.notice),
            review: None,
        };
        self.store().lock().await.keep_dispute(&kept).map_err(|why| self.ledger_fault(why))?;
        // A reminder that learnt from the dispute is settled by it, and needs no review job.
        let learnt = self.claim_learnt(disputing, rows).await;
        let settled = {
            let _settling = SETTLING.lock().await;
            let settled = if learnt { Settled { review: None, joined: false } } else { self.review_settled(&kept, rows).await };
            if let Some(review) = &settled.review {
                self.store()
                    .lock()
                    .await
                    .keep_dispute_review(&dispute_id, review)
                    .map_err(|why| self.ledger_fault(why))?;
            }
            settled
        };
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.fleet_notice_read(disputing.session, disputing.notice_row, &now);
        self.row_put(
            disputing.session,
            SessionRow::Disputed {
                id: self.row_id(disputing.session),
                at: self.instant(),
                notice: disputing.notice_row.to_string(),
                why: disputing.why.to_string(),
                spotted: disputing.spotted,
                review: settled.review.clone(),
            },
        )
        .await;
        Ok(FleetDisputed { dispute_id, review_session_id: settled.review, joined: settled.joined })
    }
}

/// The review a dispute was settled by.
pub(crate) struct Settled {
    pub review: Option<String>,
    pub joined: bool,
}

/// Whether `from` is Fleet's own voice.
pub(crate) fn is_fleets(from: &SessionVoice) -> bool {
    matches!(from, SessionVoice::Session { id, .. } if *id == FLEET_NAME.to_lowercase())
}

/// The latest message in Fleet's voice: its row id and its words.
fn latest_notice(rows: &[SessionRow]) -> Option<(&str, &str)> {
    rows.iter().rev().find_map(|row| match row {
        SessionRow::Message { id, from, text, .. } if is_fleets(from) => Some((id.as_str(), text.as_str())),
        _ => None,
    })
}

/// What a second dispute of the same notice is matched by: its words with every number, and any
/// decimal point or thousands comma inside one, read as `#`. A rule's text differs between two
/// tellings only in its figures, so this is the same rule.
pub(crate) fn notice_key(text: &str) -> String {
    let chars: Vec<char> = text.trim().chars().collect();
    let mut key = String::with_capacity(chars.len());
    let mut at = 0;
    while at < chars.len() {
        if chars[at].is_ascii_digit() {
            while at < chars.len()
                && (chars[at].is_ascii_digit()
                    || (matches!(chars[at], '.' | ',') && chars.get(at + 1).is_some_and(char::is_ascii_digit)))
            {
                at += 1;
            }
            key.push('#');
        } else {
            key.push(chars[at]);
            at += 1;
        }
    }
    key
}

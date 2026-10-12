//! A Session's retro, drawn in its thread while it is written. Since 23.77.
//! `docs/concepts/retro.md`, *A Session's retro*.
//!
//! **A progress report and no record.** The `retro` row says which stage the writing is in, how
//! much of the answer has arrived and the items as they settle. Every put is best effort, and
//! the model call is the same whether anything is listening, so losing every frame keeps the
//! same retro. A terminal Session has no rows of Fleet's, so [`Live::none`] reports nothing and
//! its call is the unwatched one.

use std::time::Duration;

use adapter_traits::{AgentHarness, Ask, CallProgress, Delivery, Vcs, WorkProduct};
use ipc::{Instant, PartialRetroItem, RetroStage, SessionRow};
use tokio::time::Instant as Clock;

use super::streamed::settled_items;
use crate::daemon::Fleet;
use crate::explaining::Explaining;
use crate::judging::{said, watched, CallFailed};

/// The least between two replacements of the row for a count or an item moving. **A stage
/// change and the final state are never held to it.**
pub(crate) const GAP: Duration = Duration::from_millis(250);

/// What a row left `writing` by a Fleet that has gone is shown as.
pub(crate) const RESTARTED: &str = "Fleet restarted while it was written";

/// The row being kept, and when it was last sent.
pub(crate) struct Progress {
    id: String,
    at: Instant,
    stage: RetroStage,
    stage_since: Instant,
    characters: u64,
    /// The answer so far, joined. The items are read off it when a row is sent and not before.
    written: String,
    retro: Option<i64>,
    why: Option<String>,
    moved: bool,
    sent_at: Option<Clock>,
}

impl Progress {
    pub(crate) fn begin(id: String, at: Instant, stage: RetroStage) -> Progress {
        Progress {
            id,
            stage_since: at.clone(),
            at,
            stage,
            characters: 0,
            written: String::new(),
            retro: None,
            why: None,
            moved: false,
            sent_at: None,
        }
    }

    /// The row as it stands. Items are drawn only while the model is writing.
    pub(crate) fn row(&self) -> SessionRow {
        let items: Vec<PartialRetroItem> = match self.stage {
            RetroStage::Writing => settled_items(&self.written),
            _ => Vec::new(),
        };
        SessionRow::Retro {
            id: self.id.clone(),
            at: self.at.clone(),
            stage: self.stage,
            stage_since: self.stage_since.clone(),
            characters: self.characters,
            items,
            retro: self.retro,
            why: self.why.clone(),
        }
    }

    /// Take one frame of the call. Cheap: nothing is read until a row is sent.
    pub(crate) fn heard(&mut self, progress: CallProgress) {
        if let CallProgress::Answering { characters, text } = progress {
            self.characters += characters;
            self.written.push_str(&text);
            self.moved = true;
        }
    }

    /// Whether a replacement for movement may go now.
    pub(crate) fn due(&self, now: Clock) -> bool {
        self.moved
            && self
                .sent_at
                .is_none_or(|sent| now.saturating_duration_since(sent) >= GAP)
    }

    /// When the movement held back may go, if any is.
    pub(crate) fn next_due(&self) -> Option<Clock> {
        self.moved
            .then(|| self.sent_at.map_or_else(Clock::now, |sent| sent + GAP))
    }

    /// The row, sent at `now`.
    pub(crate) fn sent(&mut self, now: Clock) -> SessionRow {
        self.moved = false;
        self.sent_at = Some(now);
        self.row()
    }

    /// Move to a stage. `None` where it is the stage it is in.
    pub(crate) fn staged(
        &mut self,
        stage: RetroStage,
        since: Instant,
        now: Clock,
    ) -> Option<SessionRow> {
        if self.stage == stage {
            return None;
        }
        self.stage = stage;
        self.stage_since = since;
        Some(self.sent(now))
    }

    pub(crate) fn written(&mut self, retro: i64, since: Instant, now: Clock) -> SessionRow {
        self.retro = Some(retro);
        self.stage = RetroStage::Written;
        self.stage_since = since;
        self.sent(now)
    }

    pub(crate) fn failed(&mut self, why: String, since: Instant, now: Clock) -> SessionRow {
        self.why = Some(why);
        self.stage = RetroStage::Failed;
        self.stage_since = since;
        self.sent(now)
    }
}

/// A retro's row in a hosted Session's thread, or nothing for a Session without one.
pub(crate) struct Live {
    session: String,
    progress: Option<Progress>,
}

impl Live {
    pub(crate) fn none() -> Live {
        Live {
            session: String::new(),
            progress: None,
        }
    }

    pub(crate) fn on(session: &str, progress: Progress) -> Live {
        Live {
            session: session.to_string(),
            progress: Some(progress),
        }
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
    /// Put the row on a hosted Session's thread, at `stage`. **Called once a writing is
    /// accepted**, so a refused press adds none.
    pub(super) async fn retro_opened(&self, session: &str, stage: RetroStage) -> Live {
        let hosted = matches!(self.store().lock().await.hosting(session), Ok(Some(_)));
        if !hosted {
            return Live::none();
        }
        let id = format!("retro-{}", self.row_id(session));
        let progress = Progress::begin(id, self.instant(), stage);
        self.row_put(session, progress.row()).await;
        Live::on(session, progress)
    }

    pub(super) async fn retro_staged(&self, live: &mut Live, stage: RetroStage) {
        let Some(progress) = live.progress.as_mut() else {
            return;
        };
        if let Some(row) = progress.staged(stage, self.instant(), Clock::now()) {
            self.row_put(&live.session, row).await;
        }
    }

    /// The row's last word: kept, with its number, or not, with Fleet's.
    pub(super) async fn retro_closed<T>(
        &self,
        live: &mut Live,
        outcome: &Result<T, api::Refusal>,
        number: impl FnOnce(&T) -> i64,
    ) {
        let Some(progress) = live.progress.as_mut() else {
            return;
        };
        let row = match outcome {
            Ok(kept) => progress.written(number(kept), self.instant(), Clock::now()),
            Err(refused) => progress.failed(
                refused.error().message.clone(),
                self.instant(),
                Clock::now(),
            ),
        };
        self.row_put(&live.session, row).await;
    }

    /// The retro call. **Watched where the Session has a row**, and the unwatched call as
    /// before where it has none; the answer is the same either way.
    pub(crate) async fn retro_said(
        &self,
        live: &mut Live,
        explaining: &Explaining,
        ask: &Ask,
    ) -> Result<String, CallFailed> {
        let Some(progress) = live.progress.as_mut() else {
            return said(explaining.client.as_ref(), ask, explaining.budget).await;
        };
        let (tell, mut heard) = tokio::sync::mpsc::unbounded_channel();
        let telling = move |frame: CallProgress| {
            let _ = tell.send(frame);
        };
        let call = watched(
            explaining.client.as_ref(),
            ask,
            explaining.budget,
            &telling,
            std::future::pending::<()>(),
        );
        tokio::pin!(call);
        let outcome = loop {
            let wake = progress.next_due();
            tokio::select! {
                out = &mut call => break out,
                Some(frame) = heard.recv() => progress.heard(frame),
                () = async {
                    match wake {
                        Some(at) => tokio::time::sleep_until(at).await,
                        None => std::future::pending().await,
                    }
                } => {}
            }
            if progress.due(Clock::now()) {
                let row = progress.sent(Clock::now());
                self.row_put(&live.session, row).await;
            }
        };
        while let Ok(frame) = heard.try_recv() {
            progress.heard(frame);
        }
        outcome
    }

    /// A thread as it is read: a `retro` row still going with no writing in this Fleet is one
    /// a restart left, and is kept and shown `failed`.
    pub(crate) async fn retro_rows_settled(
        &self,
        session: &str,
        rows: Vec<SessionRow>,
    ) -> Vec<SessionRow> {
        let going =
            |row: &SessionRow| matches!(row, SessionRow::Retro { stage, .. } if !stage.over());
        if !rows.iter().any(going)
            || self
                .hosts()
                .of(session)
                .writing_retro
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return rows;
        }
        let since = self.instant();
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            match row {
                SessionRow::Retro {
                    id,
                    at,
                    stage,
                    characters,
                    ..
                } if !stage.over() => {
                    let gone = SessionRow::Retro {
                        id,
                        at,
                        stage: RetroStage::Failed,
                        stage_since: since.clone(),
                        characters,
                        items: Vec::new(),
                        retro: None,
                        why: Some(RESTARTED.to_string()),
                    };
                    self.row_put(session, gone.clone()).await;
                    out.push(gone);
                }
                other => out.push(other),
            }
        }
        out
    }
}

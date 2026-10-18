//! A person's answer to a retro item by starting a Session on it.
//! `docs/concepts/retro.md`, *Start Session*.
//!
//! `agreeing` and `session_agreeing` have the rules and these are they, for the
//! act that starts a hosted Session. **The brief
//! is sent at once**, by the road a person's first message takes, so the
//! process starts and the turn runs under the rules any message is under.
//!
//! **The first press wins, and the write is the claim.** The item moves off
//! `open` to `agreed` in one conditional update, so two presses start one
//! Session.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use core_model::{LandsIn, LessonState};
use ipc::{
    Lesson, LessonSessionStarted, ManifestId, SendSessionMessage, SessionId, StartSession, WireError,
};

use super::agreeing::LESSON_NAMES_NO_PLACE;
use super::session_agreeing::SESSION_NAMES_NO_REPOSITORY;
use super::serving::parsed_session;
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A `kit` item: Kit is no repository, so there is nothing to work on. A 422.
const NOTHING_TO_WORK_ON: &str = "fleet.lesson_session_nothing_to_work_on";
/// An item that stands answered with no Session started for it, so there is
/// none to hand back. A 409.
const NOT_OPEN: &str = "fleet.lesson_not_open";

/// Which item: the Session, its retro's number and the item's place.
enum Item {
    Session(String, i64, u32),
}

/// An item as it stands, with what the rules read off it.
struct Held {
    state: LessonState,
    line: store::RetroLine,
    lesson: Lesson,
    /// Why the brief was not sent, as kept.
    send_failed: Option<String>,
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
    /// Start a Session for an item and send it the brief.
    pub(crate) async fn started_from(
        self: &Arc<Self>,
        lesson_id: &str,
    ) -> Result<LessonSessionStarted, Refusal> {
        let (session, retro, ordinal) =
            parsed_session(lesson_id).ok_or_else(|| self.no_such_lesson(lesson_id))?;
        let item = Item::Session(session, retro, ordinal);
        let held = self.held_item(lesson_id, &item).await?;
        if held.state != LessonState::Open {
            return self.standing_session(held).await;
        }
        let Some(place) = held.line.lands_in else {
            return Err(Refusal::Unacceptable(WireError::raised(
                LESSON_NAMES_NO_PLACE,
                format!("{lesson_id} names no place its fix lands, so there is no repository to start a Job in"),
                self.run_id(),
            )));
        };
        // Settled before anything is claimed, so a refusal leaves the item open.
        let manifest = match place {
            LandsIn::Kit => {
                return Err(Refusal::Unacceptable(WireError::raised(
                    NOTHING_TO_WORK_ON,
                    format!(
                        "{lesson_id} is a change to Kit, which is no repository, so there is \
                         nothing to work on. Agree with it instead"
                    ),
                    self.run_id(),
                )));
            }
            LandsIn::Armada => self.armada_served(lesson_id)?,
            LandsIn::Manifest => self.item_repository(lesson_id, &item).await?,
        };
        if !self.claim_lesson_item(&item).await? {
            // Somebody pressed first. Their answer stands.
            let held = self.held_item(lesson_id, &item).await?;
            return self.standing_session(held).await;
        }
        let start = StartSession {
            manifest_id: manifest,
            title: held.line.title.clone(),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        };
        let started = match HostedSessions::start_session(self.as_ref(), start).await {
            Ok(record) => record.id.as_str().to_string(),
            Err(refused) => {
                // No Session came of it, so the item is as it was.
                self.reopen_lesson_item(&item).await;
                return Err(refused);
            }
        };
        // **Kept before it is sent**, so a refused send leaves the item agreed
        // with a Session that exists. A started Session cannot be taken back
        // cleanly (it has a row and may be opened), so the item is not reopened.
        self.keep_lesson_item_session(&item, &started).await?;
        let held = self.held_item(lesson_id, &item).await?;
        let brief = brief_of(&held.line, &source_of(&held.lesson));
        let sent = Arc::clone(self)
            .send_session_message(SendSessionMessage {
                session_id: SessionId::carried(started.clone()),
                text: brief.clone(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await;
        // **Not an error**: the Session exists and the item is agreed with it, so
        // the answer is 200 and `send_failed` says why nothing was sent.
        let send_failed = sent.err().map(|refused| refused.error().message.clone());
        if let Some(why) = &send_failed {
            self.keep_lesson_item_send_failed(&item, why).await?;
        }
        Ok(LessonSessionStarted { brief, session_id: started, lesson: held.lesson, send_failed })
    }

    async fn held_item(&self, lesson_id: &str, item: &Item) -> Result<Held, Refusal> {
        match item {
            Item::Session(session, retro, ordinal) => {
                let kept = self
                    .session_lesson_held(lesson_id, session, *retro, *ordinal)
                    .await?;
                let (state, line, send_failed) =
                    (kept.state, kept.line.clone(), kept.session_send_failed.clone());
                let lesson = self.session_lesson_standing(kept).await?;
                Ok(Held { state, line, lesson, send_failed })
            }
        }
    }

    /// The repository a `manifest` fix is worked in: the one its Session was
    /// worked in.
    async fn item_repository(&self, lesson_id: &str, item: &Item) -> Result<ManifestId, Refusal> {
        match item {
            Item::Session(session, _, _) => {
                let kept = self
                    .store()
                    .lock()
                    .await
                    .session(session)
                    .map_err(|why| self.ledger_fault(why))?;
                kept.and_then(|kept| kept.manifest_id)
                    .filter(|id| !id.is_empty())
                    .map(ManifestId::carried)
                    .ok_or_else(|| {
                        Refusal::Unacceptable(WireError::raised(
                            SESSION_NAMES_NO_REPOSITORY,
                            format!(
                                "{lesson_id} is a fix in a repository, and its job was \
                                 started in none"
                            ),
                            self.run_id(),
                        ))
                    })
            }
        }
    }

    /// Move the item `open` to `agreed`, and say whether this call did.
    async fn claim_lesson_item(&self, item: &Item) -> Result<bool, Refusal> {
        let mut store = self.store().lock().await;
        match item {
            Item::Session(_, retro, ordinal) => {
                store.answer_session_lesson(*retro, *ordinal, LessonState::Agreed)
            }
        }
        .map_err(|why| self.refusal(Adrift::Writing(why)))
    }

    async fn reopen_lesson_item(&self, item: &Item) {
        let mut store = self.store().lock().await;
        let _ = match item {
            Item::Session(_, retro, ordinal) => store.reopen_session_lesson(*retro, *ordinal),
        };
    }

    async fn keep_lesson_item_send_failed(&self, item: &Item, why: &str) -> Result<(), Refusal> {
        let mut store = self.store().lock().await;
        match item {
            Item::Session(_, retro, ordinal) => {
                store.keep_session_lesson_send_failed(*retro, *ordinal, why)
            }
        }
        .map_err(|why| self.refusal(Adrift::Writing(why)))
    }

    async fn keep_lesson_item_session(&self, item: &Item, started: &str) -> Result<(), Refusal> {
        let mut store = self.store().lock().await;
        match item {
            Item::Session(_, retro, ordinal) => {
                store.keep_session_lesson_session(*retro, *ordinal, started)
            }
        }
        .map_err(|why| self.refusal(Adrift::Writing(why)))
    }

    /// An item already answered. **Where a Session was started for it, that is
    /// the answer, and nothing starts**; any other answer has no Session to
    /// hand back.
    async fn standing_session(&self, held: Held) -> Result<LessonSessionStarted, Refusal> {
        let Some(session_id) = held.lesson.session_proposed.clone() else {
            return Err(Refusal::IllegalMove(WireError::raised(
                NOT_OPEN,
                format!(
                    "{} is {}, and no Job was started for it",
                    held.lesson.id,
                    held.state.as_wire()
                ),
                self.run_id(),
            )));
        };
        // **Never resent**: what was kept is read back, and nothing is sent.
        Ok(LessonSessionStarted {
            brief: brief_of(&held.line, &source_of(&held.lesson)),
            send_failed: held.send_failed,
            session_id,
            lesson: held.lesson,
        })
    }
}

/// Which Session the item came from, as the brief names it.
fn source_of(lesson: &Lesson) -> String {
    format!("Job {}", lesson.handle)
}

/// The first message an agent is sent for an item: the same parts
/// `request_of` hands the proposer, as words to the one who will do the work.
/// **Sent as it stands**, so it is short and plain.
pub(super) fn brief_of(line: &store::RetroLine, source: &str) -> String {
    let title = line.title.as_deref().unwrap_or(&line.said);
    let what = line.what.as_deref().unwrap_or(&line.said);
    let mut brief = format!("{title}\n\nThis came up in a retro. {what}");
    if let Some(fix) = &line.fix {
        brief.push_str(&format!("\n\nPlease change this: {fix}"));
    }
    brief.push_str(&format!("\n\nIt is from the retro of {source}."));
    brief
}

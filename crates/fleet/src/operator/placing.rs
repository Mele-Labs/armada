//! The Operator's order of Waiting to start, and the person's correction of it. A start ruled a
//! place takes it as it joins its repository's line, or at once where it waits there already;
//! **only while Dispatch is on**, so with it off, or the Operator off, the line is first come, first
//! served. A person's drag moves a Job whatever the Operator ruled, and is kept in its Log as a
//! correction, which the next call reads.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Actor;

use super::{Logged, Outcome, Place, Source};
use crate::daemon::Fleet;

const LINE_UNORDERED: &str = "fleet.line_unordered";
const LINE_NOT_WAITING: &str = "fleet.line_not_waiting";
const LINE_PLACE_UNKNOWN: &str = "fleet.line_place_unknown";

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
    /// The Operator ruled `id` a place: kept for when it joins the line, or taken now where it waits.
    pub(super) async fn operator_placed(&self, id: &str, place: Place) {
        // Kept before the line is read, so a Job joining in between still takes it as it joins.
        self.operator_seat().place(id, place);
        let Some(manifest) = self.hosts().slot_queue().line_holding(id) else { return };
        if let Some(place) = self.operator_seat().take_place(id) {
            self.line_reordered(&manifest, id, &place).await;
        }
    }

    /// The person moved `id` to `place` in Waiting to start: it moves, and the Log keeps it as a
    /// correction of the Operator's order. **Refused, with nothing moved**, while the Operator is off,
    /// where `id` is not waiting, and where `place` names a Job not waiting in the same line.
    pub(crate) async fn line_corrected(self: &Arc<Self>, id: &str, place: Place) -> Result<(), Refusal> {
        if !self.operator_on() {
            return Err(self.hosted_refusal(LINE_UNORDERED, "Waiting to start is first come, first served while the Operator is off"));
        }
        let Some(manifest) = self.hosts().slot_queue().line_holding(id) else {
            return Err(self.hosted_refusal(LINE_NOT_WAITING, &format!("job {id} is not waiting to start")));
        };
        if !self.line_reordered(&manifest, id, &place).await {
            return Err(self.hosted_refusal(LINE_PLACE_UNKNOWN, &format!("{place} is not a place in the line job {id} waits in")));
        }
        let subject = self.store().lock().await.session(id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| id.to_string());
        let mut row = Logged::about(id, &manifest, &subject, Outcome::Corrected, Actor::Human, self.now());
        row.because = format!("You moved it {place} in Waiting to start.");
        row.place = Some(place);
        self.operator_logged(row).await;
        let _ = self.published_hosted(id).await;
        Ok(())
    }

    /// Keep a row of the Operator's Log that no call made. Nothing while the Operator is off.
    pub(super) async fn operator_logged(&self, row: Logged) {
        if !self.operator_on() {
            return;
        }
        if let Err(why) = self.store().lock().await.add_operator_ruling(&row.kept()) {
            self.operator_warned("the Operator's Log was not kept", why);
        }
    }
}

impl Logged {
    /// A row about `session` after its ruling, made by no call: a correction, or what became of a hold.
    pub(crate) fn about(session: &str, manifest: &str, subject: &str, outcome: Outcome, by: Actor, at: core_model::Timestamp) -> Logged {
        let (arrival, source) = match outcome {
            Outcome::Corrected => (format!("line:{session}"), Source::Person),
            _ => (format!("hold:{session}"), Source::Fleet),
        };
        Logged {
            arrival,
            arrived: super::Arrived::Start,
            source,
            session_id: session.to_string(),
            manifest_id: manifest.to_string(),
            subject: subject.to_string(),
            outcome,
            other: None,
            shared: Vec::new(),
            because: String::new(),
            call: None,
            by,
            at,
            place: None,
            first: None,
            refused: Vec::new(),
        }
    }
}

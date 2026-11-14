//! `answer_items` and `dismiss_items`: his Approve all, Ask for changes and Dismiss all across every item that
//! asks for his approval. **Each item goes the way its own answer goes**, through the same settling, so a Job hears
//! exactly what one press would have told it. `docs/concepts/artifact.md`, *Several at once*.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    AnswerDocument, AnswerItems, AnswerSketch, AnswerWaiting, ApprovePlan, DismissItems, DocumentHow, ItemAnswer, ItemHow, ItemNote,
    ItemOutcome, ItemSettled, ItemsSettled, SendSessionMessage, SessionId, SketchHow, WaitingItem, WaitingSource, DOCUMENT_ITEM_PREFIX,
    DOCUMENT_REVISION, SKETCH_DRAWN,
};

use super::waiting::{APPROVE, ASK_FOR_CHANGES, SESSION_WAITING_UNNAMED};
use crate::daemon::Fleet;

const ITEMS_NOT_WAITING: &str = "fleet.items_not_waiting";
const ITEMS_NOT_APPROVABLE: &str = "fleet.items_not_approvable";
const ITEMS_SEND_BACK_EMPTY: &str = "fleet.items_send_back_empty";

/// What a Job is told for each item he passed on.
pub(crate) fn passed(title: &str) -> String {
    format!("He passed on \"{title}\" with no verdict.")
}

/// An item with an Approve: what Approve all and Dismiss all reach. Questions keep their own answers.
fn approvable(item: &WaitingItem) -> bool {
    match item.source {
        WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Plan | WaitingSource::Draft | WaitingSource::Walk => true,
        WaitingSource::Agent => item.approves,
        _ => false,
    }
}

/// His notes as a Job reads them, blanks left out. A note on words reads `On "<quote>": <text>`, and one with no
/// `on`, or a quote blank after trimming, reads as its text alone.
fn numbered(notes: &[ItemNote]) -> Vec<String> {
    notes
        .iter()
        .filter_map(|note| {
            let text = note.text.trim();
            if text.is_empty() {
                return None;
            }
            match note.on.as_ref().map(|on| on.quote.trim()).filter(|quote| !quote.is_empty()) {
                Some(quote) => Some(format!("On \"{quote}\": {text}")),
                None => Some(text.to_string()),
            }
        })
        .collect()
}

fn as_words(notes: &[String]) -> Option<String> {
    (!notes.is_empty()).then(|| notes.iter().enumerate().map(|(at, note)| format!("{}. {note}", at + 1)).collect::<Vec<_>>().join("\n"))
}

fn plan_version(item: &str) -> Option<u32> {
    item.strip_prefix("plan:").and_then(|version| version.parse().ok())
}

/// The title a Job is told an item by: a plan's call reads the same for every version, so it is named by its version.
fn title_of(item: &WaitingItem) -> String {
    match (item.source, plan_version(&item.id)) {
        (WaitingSource::Plan, Some(version)) => format!("plan v{version}"),
        _ => item.text.clone(),
    }
}

fn refused(refusal: &Refusal) -> ItemSettled {
    let error = refusal.error();
    ItemSettled::Refused { code: error.code.clone(), message: error.message.clone() }
}

fn refused_as(code: &str, message: String) -> ItemSettled {
    ItemSettled::Refused { code: code.to_string(), message }
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
    /// `answer_items`: each item in order, on its own. `refused` holds what the route refused already.
    pub(crate) async fn approvals_answered(self: Arc<Self>, said: AnswerItems, refused: Vec<Option<ItemSettled>>) -> ItemsSettled {
        let mut outcomes = Vec::with_capacity(said.items.len());
        for (at, one) in said.items.into_iter().enumerate() {
            let outcome = match refused.get(at).cloned().flatten() {
                Some(refused) => refused,
                None => Box::pin(Arc::clone(&self).item_answered(&one)).await,
            };
            outcomes.push(ItemOutcome { session_id: one.session_id, item: one.item, outcome });
        }
        ItemsSettled { outcomes }
    }

    async fn item_answered(self: Arc<Self>, one: &ItemAnswer) -> ItemSettled {
        let id = one.session_id.as_str().trim().to_string();
        let item = match self.approvable_item(&id, one.item.trim()).await {
            Ok(item) => item,
            Err(refusal) => return refusal,
        };
        let notes = numbered(&one.notes);
        let approve = one.how == ItemHow::Approve;
        if !approve && notes.is_empty() {
            return refused_as(ITEMS_SEND_BACK_EMPTY, format!("\"{}\" is sent back with a note saying what to change", item.text));
        }
        let words = as_words(&notes);
        let session_id = SessionId::carried(&id);
        let settled = match item.source {
            WaitingSource::Sketch => {
                let drawn = match self.sketch_held(&id, &item.id).await {
                    Ok((_, held)) => held.and_then(|one| one.detail.get(SKETCH_DRAWN).cloned()).unwrap_or_default(),
                    Err(refusal) => return refused(&refusal),
                };
                let how = if approve { SketchHow::Approved } else { SketchHow::SentBack };
                self.sketch_answered(AnswerSketch { session_id, sketch: item.id.clone(), drawn, how, words }).await
            }
            WaitingSource::Document => {
                let path = item.id.strip_prefix(DOCUMENT_ITEM_PREFIX).unwrap_or(&item.id).to_string();
                let revision = match self.document_held(&id, &path).await {
                    Ok((_, held)) => held.and_then(|one| one.detail.get(DOCUMENT_REVISION).cloned()).unwrap_or_default(),
                    Err(refusal) => return refused(&refusal),
                };
                let how = if approve { DocumentHow::Approved } else { DocumentHow::SentBack };
                self.document_answered(AnswerDocument { session_id, document: item.id.clone(), revision, how, words }).await
            }
            WaitingSource::Plan => match plan_version(&item.id) {
                Some(version) if approve && notes.is_empty() => self.plan_approved(ApprovePlan { session_id, version }).await,
                Some(version) => self.plan_noted(&id, version, approve, &notes).await,
                None => return refused_as(ITEMS_NOT_WAITING, format!("`{}` names no plan version", item.id)),
            },
            // A draft's Approve is its first option, and words alone send it back.
            WaitingSource::Draft => self.settle_verdict(waiting_answer(session_id, &item, approve.then_some(0), words), false).await,
            WaitingSource::Walk => self.settle_verdict(waiting_answer(session_id, &item, None, words), approve).await,
            _ => {
                let label = if approve { APPROVE } else { ASK_FOR_CHANGES };
                let choice = item.options.iter().position(|option| option.label == label).map(|at| at as u32);
                self.settle_verdict(waiting_answer(session_id, &item, choice, words), false).await
            }
        };
        match settled {
            Ok(_) => ItemSettled::Settled,
            Err(refusal) => refused(&refusal),
        }
    }

    /// `dismiss_items`: each Job is sent one message naming every item of it he passed on, then each is closed as his
    /// Dismiss closes it. Outcomes keep the order asked.
    pub(crate) async fn approvals_dismissed(self: Arc<Self>, said: DismissItems, refused: Vec<Option<ItemSettled>>) -> ItemsSettled {
        let mut outcomes: Vec<Option<ItemSettled>> = (0..said.items.len()).map(|at| refused.get(at).cloned().flatten()).collect();
        // Each Job once, in the order it is first named.
        let mut jobs: Vec<(String, Vec<usize>)> = Vec::new();
        for (at, one) in said.items.iter().enumerate() {
            if outcomes[at].is_some() {
                continue;
            }
            let id = one.session_id.as_str().trim().to_string();
            match jobs.iter_mut().find(|(job, _)| *job == id) {
                Some((_, ats)) => ats.push(at),
                None => jobs.push((id, vec![at])),
            }
        }
        for (id, ats) in jobs {
            let named: Vec<(usize, &str)> = ats.iter().map(|&at| (at, said.items[at].item.trim())).collect();
            for (at, outcome) in Box::pin(Arc::clone(&self).job_items_dismissed(&id, &named)).await {
                outcomes[at] = Some(outcome);
            }
        }
        let outcomes = said
            .items
            .into_iter()
            .zip(outcomes)
            .map(|(one, outcome)| ItemOutcome { session_id: one.session_id, item: one.item, outcome: outcome.unwrap_or(ItemSettled::Settled) })
            .collect();
        ItemsSettled { outcomes }
    }

    /// One Job's part of `dismiss_items`. The message goes first, so a send that fails leaves every item standing.
    async fn job_items_dismissed(self: Arc<Self>, id: &str, named: &[(usize, &str)]) -> Vec<(usize, ItemSettled)> {
        let mut outcomes = Vec::new();
        let terminal = match self.terminal_session(id).await {
            Ok(terminal) => terminal,
            Err(refusal) => return named.iter().map(|(at, _)| (*at, refused(&refusal))).collect(),
        };
        if terminal.is_some() {
            self.terminal_ask_standing(id).await;
        }
        // Each held, as one Dismiss holds it, from before the send until it is closed.
        let mut held = Vec::new();
        for &(at, named) in named {
            match self.approvable_item(id, named).await {
                Ok(item) => match self.hosts().settling(id, &item.id) {
                    Some(settling) => held.push((at, item, settling)),
                    None => outcomes.push((at, refused_as(ITEMS_NOT_WAITING, format!("`{named}` is being settled already")))),
                },
                Err(refusal) => outcomes.push((at, refusal)),
            }
        }
        if held.is_empty() {
            return outcomes;
        }
        let told = SendSessionMessage {
            session_id: SessionId::carried(id),
            text: held.iter().map(|(_, item, _)| passed(&title_of(item))).collect::<Vec<_>>().join("\n"),
            attachments: Vec::new(),
            mentions: Vec::new(),
            sketches: Vec::new(),
        };
        if let Err(refusal) = Arc::clone(&self).send_known(terminal, told).await {
            outcomes.extend(held.iter().map(|(at, ..)| (*at, refused(&refusal))));
            return outcomes;
        }
        for (at, item, _settling) in held {
            let outcome = match self.dismissal_closed(id, &item, None, false).await {
                Ok(_) => ItemSettled::Settled,
                Err(refusal) => refused(&refusal),
            };
            outcomes.push((at, outcome));
        }
        outcomes
    }

    /// The item `named` the Job waits on now, where it is one with an Approve.
    async fn approvable_item(&self, id: &str, named: &str) -> Result<WaitingItem, ItemSettled> {
        let not_waiting = || refused_as(ITEMS_NOT_WAITING, format!("this Job no longer waits on `{named}`"));
        let item = match self.waiting_now(id).await {
            Ok(items) => items.into_iter().find(|one| one.id == named).ok_or_else(not_waiting)?,
            Err(refusal) if refusal.error().code == SESSION_WAITING_UNNAMED => return Err(refused_as(ITEMS_NOT_WAITING, format!("no Job `{id}`"))),
            Err(refusal) => return Err(refused(&refusal)),
        };
        match approvable(&item) {
            true => Ok(item),
            false => Err(refused_as(ITEMS_NOT_APPROVABLE, format!("\"{}\" has no Approve: it is answered on its own", item.text))),
        }
    }
}

/// The `answer_waiting` an item's own Approve or Ask for changes would send.
fn waiting_answer(session_id: SessionId, item: &WaitingItem, choice: Option<u32>, words: Option<String>) -> AnswerWaiting {
    AnswerWaiting { session_id, item_id: item.id.clone(), choice, text: words, mode: None, fixed: None }
}

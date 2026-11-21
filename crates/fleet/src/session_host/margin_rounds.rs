//! Rounds of margin talk: each message Fleet sends a Job for his margin threads, kept with the row it wrote, and
//! the one line the Job's thread carries beside that row, for a screen to draw in its place. **The line is built from the kept round and the
//! threads it names**, never read off the message. `docs/concepts/document.md`, *Margin threads*.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    Instant, MarginAnchor, MarginMessage, MarginRoundKind, MarginState, MarginSuggestion, MarginThread, MarginVoice, SendSessionMessage,
    SessionId, SessionRow, SessionVoice, SuggestionDecided, SuggestionHow, ThreadMargin, ThreadMarginNote,
};
use store::{KeptMarginNote, KeptMarginRound, KeptMarginThread};

use crate::daemon::Fleet;
use crate::thread_keeping::Change;

/// The page a round is on, as the Job and the person name it.
pub(super) struct RoundPage {
    /// The page's file name, which its threads are kept by.
    pub name: String,
    pub title: String,
}

pub(super) fn state_word(state: MarginState) -> &'static str {
    match state {
        MarginState::Open => "open",
        MarginState::Answered => "answered",
        MarginState::Suggested => "suggested",
        MarginState::Held => "held",
        MarginState::Settled => "settled",
    }
}

pub(super) fn state_of(word: &str) -> MarginState {
    match word {
        "answered" => MarginState::Answered,
        "suggested" => MarginState::Suggested,
        "held" => MarginState::Held,
        "settled" => MarginState::Settled,
        _ => MarginState::Open,
    }
}

fn kind_word(kind: MarginRoundKind) -> &'static str {
    match kind {
        MarginRoundKind::Comments => "comments",
        MarginRoundKind::Change => "change",
        MarginRoundKind::Accepted => "accepted",
        MarginRoundKind::Rejected => "rejected",
        MarginRoundKind::Settled => "settled",
        MarginRoundKind::SentBack => "sent_back",
    }
}

fn kind_of(word: &str) -> Option<MarginRoundKind> {
    Some(match word {
        "comments" => MarginRoundKind::Comments,
        "change" => MarginRoundKind::Change,
        "accepted" => MarginRoundKind::Accepted,
        "rejected" => MarginRoundKind::Rejected,
        "settled" => MarginRoundKind::Settled,
        "sent_back" => MarginRoundKind::SentBack,
        _ => return None,
    })
}

/// A kept thread as the wire carries it.
pub(crate) fn thread_of(kept: &KeptMarginThread) -> MarginThread {
    MarginThread {
        id: kept.id.clone(),
        document: kept.document.clone(),
        anchor: MarginAnchor { quote: kept.quote.clone(), block: kept.block.clone(), section: kept.section.clone() },
        messages: kept
            .messages
            .iter()
            .map(|one| MarginMessage {
                by: if one.voice == "agent" { MarginVoice::Agent } else { MarginVoice::You },
                text: one.text.clone(),
                at: Instant::carried(&one.at),
                held: one.held,
            })
            .collect(),
        state: state_of(&kept.state),
        gone: kept.gone,
        suggestion: kept.suggestion.as_ref().map(|one| MarginSuggestion {
            struck: one.struck.clone(),
            added: one.added.clone(),
            at: Instant::carried(&one.at),
            decided: one.decided.as_deref().map(|how| SuggestionDecided {
                how: if how == "accepted" { SuggestionHow::Accepted } else { SuggestionHow::Rejected },
                at: Instant::carried(one.decided_at.as_deref().unwrap_or_default()),
                revision: one.decided_revision,
            }),
        }),
        outcome: kept.outcome.clone(),
    }
}

fn counted(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        _ => format!("{count} {many}"),
    }
}

/// The line of a round, and whether it waits on him: an answer or a suggested edit in it that no later round names.
fn summary_of(round: &KeptMarginRound, threads: &[&KeptMarginThread], newest: &dyn Fn(&str) -> bool) -> (String, bool) {
    let waits = threads.iter().any(|one| newest(&one.id) && matches!(state_of(&one.state), MarginState::Answered | MarginState::Suggested));
    let first = threads.first();
    let summary = match kind_of(&round.kind) {
        Some(MarginRoundKind::Comments) => {
            let answered = threads.iter().filter(|one| one.messages.iter().any(|said| said.voice == "agent")).count();
            let gone = threads.iter().filter(|one| one.gone).count();
            let mut said = counted(round.notes.len().max(threads.len()), "comment", "comments");
            if answered > 0 {
                said.push_str(&format!(", {answered} answered"));
            }
            if gone > 0 {
                said.push_str(&format!(", {} gone from the page", counted(gone, "passage", "passages")));
            }
            said
        }
        Some(MarginRoundKind::Change) => match first.and_then(|one| one.suggestion.as_ref()) {
            Some(suggestion) if suggestion.decided.as_deref() == Some("accepted") => String::from("Change asked, accepted"),
            Some(suggestion) if suggestion.decided.as_deref() == Some("rejected") => String::from("Change asked, rejected"),
            Some(_) => String::from("Change suggested"),
            None => String::from("Change asked"),
        },
        Some(MarginRoundKind::Accepted) => match first.and_then(|one| one.suggestion.as_ref()).and_then(|one| one.decided_revision) {
            Some(revision) => format!("Change accepted, revision {revision}"),
            None => String::from("Change accepted"),
        },
        Some(MarginRoundKind::Rejected) => String::from("Change rejected"),
        Some(MarginRoundKind::Settled) => match first.and_then(|one| one.outcome.as_deref()) {
            Some(outcome) => format!("Settled: {outcome}"),
            None => String::from("Settled"),
        },
        Some(MarginRoundKind::SentBack) => format!("Sent back with {}", counted(round.notes.len(), "note", "notes")),
        None => String::new(),
    };
    let waits = waits && matches!(kind_of(&round.kind), Some(MarginRoundKind::Comments | MarginRoundKind::Change));
    (summary, waits)
}

/// Every round of a Job as its thread draws it, oldest first.
pub(super) fn cards_of(rounds: &[KeptMarginRound], threads: &[KeptMarginThread]) -> Vec<ThreadMargin> {
    rounds
        .iter()
        .filter_map(|round| {
            let kind = kind_of(&round.kind)?;
            let named: Vec<&KeptMarginThread> = round.threads.iter().filter_map(|id| threads.iter().find(|one| &one.id == id)).collect();
            let newest = |thread: &str| !rounds.iter().any(|later| later.seq > round.seq && later.threads.iter().any(|one| one == thread));
            let (summary, waits_on_you) = summary_of(round, &named, &newest);
            Some(ThreadMargin {
                id: format!("margin:{}", round.seq),
                replaces: round.row.clone(),
                at: Instant::carried(&round.at),
                page: round.document.clone(),
                title: round.title.clone(),
                kind,
                summary,
                waits_on_you,
                threads: round.threads.clone(),
                notes: round
                    .notes
                    .iter()
                    .map(|one| ThreadMarginNote { thread: one.thread.clone(), section: one.section.clone(), passage: one.passage.clone(), note: one.note.clone() })
                    .collect(),
            })
        })
        .collect()
}

/// One round to send: what the thread keeps, what the agent reads, and what the line is built from.
pub(super) struct Round {
    pub kind: MarginRoundKind,
    /// His words, as the Job's thread keeps the message. **No path and no instruction.**
    pub said: String,
    /// What the agent reads in their place.
    pub told: String,
    pub threads: Vec<String>,
    pub notes: Vec<KeptMarginNote>,
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
    /// The Job's rounds of margin talk, as its thread draws them. None where the store will not read.
    pub(crate) async fn margin_cards_of(&self, id: &str) -> Vec<ThreadMargin> {
        let mut store = self.store().lock().await;
        let rounds = store.margin_rounds(id).unwrap_or_default();
        if rounds.is_empty() {
            return Vec::new();
        }
        let threads = store.margin_threads(id, None).unwrap_or_default();
        cards_of(&rounds, &threads)
    }

    /// The Job's threads or rounds moved: its thread, where a client holds it, is folded again with them.
    pub(crate) async fn margins_moved(self: &Arc<Self>, id: &str) {
        let cards = self.margin_cards_of(id).await;
        self.job_thread_changed(id, vec![Change::Margins(cards)]).await;
    }

    /// Send a round as one message, then keep it with the row it wrote. **The send goes first**, so one that fails
    /// keeps nothing.
    pub(super) async fn round_sent(self: &Arc<Self>, id: &str, page: &RoundPage, round: Round) -> Result<(), Refusal> {
        let terminal = self.terminal_session(id).await?;
        let sent = SendSessionMessage {
            session_id: SessionId::carried(id),
            text: round.said.clone(),
            attachments: Vec::new(),
            mentions: Vec::new(),
            sketches: Vec::new(),
        };
        let at = self.instant();
        Arc::clone(self).send_known_added(terminal, sent, None, Some(round.told.clone())).await?;
        self.round_kept(id, page, &at, round.kind, &round.said, round.threads, round.notes).await;
        Ok(())
    }

    /// Keep a round sent as `said` at `at`, with the row the send wrote where the thread holds it.
    pub(super) async fn round_kept(
        &self,
        id: &str,
        page: &RoundPage,
        at: &Instant,
        kind: MarginRoundKind,
        said: &str,
        threads: Vec<String>,
        notes: Vec<KeptMarginNote>,
    ) {
        let row = self.rows_of(id).await.ok().and_then(|rows| {
            rows.into_iter().rev().find_map(|row| match row {
                SessionRow::Message { id, from: SessionVoice::You, text, .. } if text == said => Some(id),
                _ => None,
            })
        });
        let round = KeptMarginRound {
            seq: 0,
            at: at.as_str().to_string(),
            document: page.name.clone(),
            title: page.title.clone(),
            kind: kind_word(kind).to_string(),
            row,
            threads,
            notes,
        };
        let _ = self.store().lock().await.keep_margin_round(id, &round);
    }
}

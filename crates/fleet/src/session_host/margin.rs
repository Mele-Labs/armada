//! His margin threads on a Job's page: a comment on a passage sent now or held, Send held, Make this change, Accept,
//! Reject and Settle. **An answer changes nothing on the page**, and only his Accept writes the agent's suggested
//! edit, as the page's next revision. `crates/ipc/operations/comment_margin.toml` and its neighbours are the
//! contract, `docs/concepts/document.md`, *Margin threads*, the rules.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ActOnMargin, CommentMargin, GetMarginThreads, MarginAct, MarginOn, MarginRoundKind, MarginState, MarginThreads, SendHeldMargin, SessionId,
    DOCUMENT_ITEM_PREFIX,
};
use store::{KeptMarginMessage, KeptMarginNote, KeptMarginThread};

use super::documents::{page_name, revisions_named, shown_page};
use super::margin_rounds::{state_of, state_word, thread_of, Round, RoundPage};
use super::page_words::{edit_applied, passage_in};
use crate::daemon::Fleet;

pub(super) const MARGIN_NO_PAGE: &str = "fleet.margin_no_page";
const MARGIN_NO_THREAD: &str = "fleet.margin_no_thread";
pub(super) const MARGIN_EMPTY: &str = "fleet.margin_empty";
pub(super) const MARGIN_SETTLED: &str = "fleet.margin_settled";
const MARGIN_NOTHING_HELD: &str = "fleet.margin_nothing_held";
const MARGIN_NO_SUGGESTION: &str = "fleet.margin_no_suggestion";
const MARGIN_BUSY: &str = "fleet.margin_busy";
const MARGIN_UNKEPT: &str = "fleet.margin_unkept";

/// A page of the Job's, as margin threads name it.
pub(super) struct MarginPage {
    /// The page's id: Fleet's copy.
    pub path: String,
    pub round: RoundPage,
    /// The file in a checkout it was last written at.
    pub written: Option<String>,
    /// The newest revision, where one is kept.
    pub newest: Option<std::path::PathBuf>,
    pub revisions: usize,
}

/// A thread's passage as a message names it: `Thread m1, under "<section>", on "<quote>"`.
pub(super) fn passage_named(thread: &KeptMarginThread) -> String {
    let under = thread.section.as_deref().map(|section| format!(", under \"{section}\"")).unwrap_or_default();
    format!("Thread {}{under}, on \"{}\"", thread.id, thread.quote)
}

/// What the agent reads for his comments: each passage with his words, and which act answers them.
fn comments_told(title: &str, path: &str, notes: &[(&KeptMarginThread, String)]) -> String {
    let mut lines = vec![format!("His margin comments on \"{title}\", the page at {}:", shown_page(path))];
    lines.extend(notes.iter().enumerate().map(|(at, (thread, note))| format!("{}. {}: {note}", at + 1, passage_named(thread))));
    lines.push(String::from(
        "Answer each with answer_thread, naming its thread. An answer changes nothing on the page, so do not write the page for these. \
         Call suggest_edit only when he asks for a change.",
    ));
    lines.join("\n")
}

/// His comments as the Job's thread keeps them: the passages and his words, no path.
fn comments_said(title: &str, notes: &[(&KeptMarginThread, String)]) -> String {
    let mut lines = vec![format!("Comments on {title}")];
    lines.extend(notes.iter().map(|(thread, note)| format!("On \"{}\": {note}", thread.quote)));
    lines.join("\n")
}

fn note_of(thread: &KeptMarginThread, note: &str) -> KeptMarginNote {
    KeptMarginNote { thread: Some(thread.id.clone()), section: thread.section.clone(), passage: Some(thread.quote.clone()), note: note.to_string() }
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
    /// `get_margin_threads`.
    pub(crate) async fn margin_threads_read(&self, said: GetMarginThreads) -> Result<MarginThreads, Refusal> {
        let id = said.session_id.as_str().trim().to_string();
        let page = self.margin_page(&id, &said.document).await?;
        self.margin_threads_of(&id, &page.round.name).await
    }

    /// `comment_margin`: a new thread on a passage, or his reply on one, sent as its own round or held.
    pub(crate) async fn margin_commented(self: Arc<Self>, said: CommentMargin) -> Result<MarginThreads, Refusal> {
        let id = said.session_id.as_str().trim().to_string();
        let text = said.text.trim().to_string();
        if text.is_empty() {
            return Err(self.hosted_refusal(MARGIN_EMPTY, "a comment says something"));
        }
        let page = self.margin_page(&id, &said.document).await?;
        let mut thread = match said.on {
            MarginOn::Passage { anchor } => {
                let quote = anchor.quote.trim().to_string();
                if quote.is_empty() {
                    return Err(self.hosted_refusal(MARGIN_EMPTY, "a comment is on words of the page: the quote is empty"));
                }
                let found = match &page.newest {
                    Some(newest) => passage_in(&read_page(newest).await.unwrap_or_default(), &quote),
                    None => None,
                };
                let count = self.store().lock().await.margin_thread_count(&id).map_err(|why| self.ledger_fault(why))?;
                KeptMarginThread {
                    id: format!("m{}", count + 1),
                    document: page.round.name.clone(),
                    section: found.clone().flatten().or(anchor.section),
                    quote,
                    block: anchor.block,
                    state: state_word(MarginState::Held).to_string(),
                    gone: found.is_none() && page.newest.is_some(),
                    suggestion: None,
                    outcome: None,
                    opened_at: self.instant().as_str().to_string(),
                    messages: Vec::new(),
                }
            }
            MarginOn::Thread { thread } => self.open_thread(&id, &thread).await?,
        };
        if thread.document != page.round.name {
            return Err(self.hosted_refusal(MARGIN_NO_THREAD, &format!("thread `{}` is on another page", thread.id)));
        }
        thread.messages.push(KeptMarginMessage { voice: "you".into(), text: text.clone(), at: self.instant().as_str().to_string(), held: said.hold });
        // A suggested edit still waits on his Accept or Reject, whatever he says beside it.
        let suggested = state_of(&thread.state) == MarginState::Suggested;
        if said.hold {
            if !suggested {
                thread.state = state_word(MarginState::Held).to_string();
            }
        } else {
            if !suggested {
                thread.state = state_word(MarginState::Open).to_string();
            }
            let notes = [(&thread, text.clone())];
            let round = Round {
                kind: MarginRoundKind::Comments,
                said: comments_said(&page.round.title, &notes),
                told: comments_told(&page.round.title, &page.path, &notes),
                threads: vec![thread.id.clone()],
                notes: vec![note_of(&thread, &text)],
            };
            self.round_sent(&id, &page.round, round).await?;
        }
        self.threads_kept(&id, &[thread]).await?;
        self.margins_moved(&id).await;
        let _ = self.published_hosted(&id).await;
        self.margin_threads_of(&id, &page.round.name).await
    }

    /// `send_held_margin`: every comment he holds on the page, in one message.
    pub(crate) async fn margin_held_sent(self: Arc<Self>, said: SendHeldMargin) -> Result<MarginThreads, Refusal> {
        let id = said.session_id.as_str().trim().to_string();
        let page = self.margin_page(&id, &said.document).await?;
        let threads = self.store().lock().await.margin_threads(&id, Some(&page.round.name)).map_err(|why| self.ledger_fault(why))?;
        let mut held: Vec<(KeptMarginThread, String)> = threads
            .into_iter()
            .filter_map(|thread| {
                let words: Vec<String> = thread.messages.iter().filter(|one| one.held).map(|one| one.text.clone()).collect();
                (!words.is_empty()).then(|| (thread, words.join("\n")))
            })
            .collect();
        if held.is_empty() {
            return Err(self.hosted_refusal(MARGIN_NOTHING_HELD, &format!("no comment is held on \"{}\"", page.round.title)));
        }
        let notes: Vec<(&KeptMarginThread, String)> = held.iter().map(|(thread, note)| (thread, note.clone())).collect();
        let round = Round {
            kind: MarginRoundKind::Comments,
            said: comments_said(&page.round.title, &notes),
            told: comments_told(&page.round.title, &page.path, &notes),
            threads: held.iter().map(|(thread, _)| thread.id.clone()).collect(),
            notes: held.iter().map(|(thread, note)| note_of(thread, note)).collect(),
        };
        self.round_sent(&id, &page.round, round).await?;
        for (thread, _) in &mut held {
            thread.messages.iter_mut().for_each(|one| one.held = false);
            if state_of(&thread.state) == MarginState::Held {
                thread.state = state_word(MarginState::Open).to_string();
            }
        }
        let held: Vec<KeptMarginThread> = held.into_iter().map(|(thread, _)| thread).collect();
        self.threads_kept(&id, &held).await?;
        self.margins_moved(&id).await;
        let _ = self.published_hosted(&id).await;
        self.margin_threads_of(&id, &page.round.name).await
    }

    /// `act_on_margin_thread`: Make this change, Accept, Reject or Settle.
    pub(crate) async fn margin_acted(self: Arc<Self>, said: ActOnMargin) -> Result<MarginThreads, Refusal> {
        let id = said.session_id.as_str().trim().to_string();
        let mut thread = self.open_thread(&id, &said.thread).await?;
        let page = self.margin_page(&id, &thread.document).await?;
        let title = page.round.title.clone();
        let one = vec![thread.id.clone()];
        let quote = thread.quote.clone();
        match said.act {
            MarginAct::MakeChange => {
                let told = format!(
                    "He asks for the change on {} of \"{title}\", the page at {}. Call suggest_edit with thread {}: struck, the words \
                     on the page to replace exactly as they read, and added, the words in their place. Do not write the page: his \
                     Accept writes it.",
                    passage_named(&thread),
                    shown_page(&page.path),
                    thread.id
                );
                let said = format!("Make this change on \"{quote}\" in {title}");
                self.round_sent(&id, &page.round, Round { kind: MarginRoundKind::Change, said, told, threads: one, notes: Vec::new() }).await?;
                thread.suggestion = None;
                thread.state = state_word(MarginState::Open).to_string();
            }
            MarginAct::Accept => return self.suggestion_accepted(&id, thread, page).await,
            MarginAct::Reject => {
                let mut suggestion = self.undecided(&thread)?;
                let told = format!("He rejected your edit on thread {} of \"{title}\". The page is as it was.", thread.id);
                let said = format!("Rejected the change on \"{quote}\" in {title}");
                self.round_sent(&id, &page.round, Round { kind: MarginRoundKind::Rejected, said, told, threads: one, notes: Vec::new() }).await?;
                suggestion.decided = Some(String::from("rejected"));
                suggestion.decided_at = Some(self.instant().as_str().to_string());
                thread.suggestion = Some(suggestion);
                thread.state = state_word(MarginState::Answered).to_string();
            }
            MarginAct::Settle { outcome } => {
                let outcome = outcome.map(|said| said.trim().to_string()).filter(|said| !said.is_empty());
                let add = match &outcome {
                    Some(outcome) => format!("Its outcome: {outcome}\nAdd it to the page's Decisions section and write the page again."),
                    None => String::from("Add its outcome, as the thread reached it, to the page's Decisions section and write the page again."),
                };
                let told = format!("He settled {} of \"{title}\", the page at {}.\n{add}", passage_named(&thread), shown_page(&page.path));
                let said = match &outcome {
                    Some(outcome) => format!("Settled \"{quote}\" in {title}\n{outcome}"),
                    None => format!("Settled \"{quote}\" in {title}"),
                };
                self.round_sent(&id, &page.round, Round { kind: MarginRoundKind::Settled, said, told, threads: one, notes: Vec::new() }).await?;
                thread.outcome = outcome;
                thread.state = state_word(MarginState::Settled).to_string();
            }
        }
        self.threads_kept(&id, &[thread]).await?;
        self.margins_moved(&id).await;
        let _ = self.published_hosted(&id).await;
        self.margin_threads_of(&id, &page.round.name).await
    }

    /// Accept: the edit applied to the newest revision, the Job told, and the page written and kept as its next
    /// revision **through the path his Edit takes**. Held as his Edit and his answer are, so they cannot cross.
    async fn suggestion_accepted(self: Arc<Self>, id: &str, mut thread: KeptMarginThread, page: MarginPage) -> Result<MarginThreads, Refusal> {
        let mut suggestion = self.undecided(&thread)?;
        let _settling = self
            .hosts()
            .settling(id, &format!("{DOCUMENT_ITEM_PREFIX}{}", page.path))
            .ok_or_else(|| self.hosted_refusal(MARGIN_BUSY, &format!("\"{}\" is being answered now", page.round.title)))?;
        let newest = page.newest.as_ref().ok_or_else(|| self.hosted_fault(MARGIN_UNKEPT, "the page's newest revision is not kept"))?;
        let html = read_page(newest).await.ok_or_else(|| self.hosted_fault(MARGIN_UNKEPT, "the page's newest revision could not be read"))?;
        let edited = edit_applied(&html, &thread.quote, &suggestion.struck, &suggestion.added)
            .map_err(|why| self.hosted_refusal(MARGIN_NO_SUGGESTION, &format!("the edit no longer fits the page: {why}")))?;
        let revision = page.revisions as u32 + 1;
        let title = &page.round.title;
        let told = format!(
            "He accepted your edit on thread {} of \"{title}\": \"{}\" now reads \"{}\". Revision {revision} holds it, at {}. Build on it.",
            thread.id,
            suggestion.struck,
            suggestion.added,
            shown_page(&page.path)
        );
        let said = format!("Accepted the change on \"{}\" in {title}", thread.quote);
        let round = Round { kind: MarginRoundKind::Accepted, said, told, threads: vec![thread.id.clone()], notes: Vec::new() };
        self.round_sent(id, &page.round, round).await?;
        // The passage reads as edited, so the revision kept next finds it.
        if thread.quote.contains(&suggestion.struck) {
            thread.quote = thread.quote.replacen(&suggestion.struck, &suggestion.added, 1);
        } else if suggestion.struck.contains(&thread.quote) {
            thread.quote = suggestion.added.clone();
        }
        suggestion.decided = Some(String::from("accepted"));
        suggestion.decided_at = Some(self.instant().as_str().to_string());
        suggestion.decided_revision = Some(revision);
        thread.suggestion = Some(suggestion);
        thread.state = state_word(MarginState::Answered).to_string();
        self.threads_kept(id, &[thread]).await?;
        let kept = self
            .page_rewritten(id, &page.path, page.written.clone(), &edited)
            .await
            .ok_or_else(|| self.hosted_fault(MARGIN_UNKEPT, "the job was told of his Accept, and the page could not be written"))?;
        let home = self.page_home(id, &page.path);
        let _ = self.page_kept(id, &home, page.written.as_deref(), &edited, &kept, false).await;
        self.margins_moved(id).await;
        let _ = self.published_hosted(id).await;
        self.margin_threads_of(id, &page.round.name).await
    }

    /// The suggestion on `thread` that waits on him.
    fn undecided(&self, thread: &KeptMarginThread) -> Result<store::KeptMarginSuggestion, Refusal> {
        thread
            .suggestion
            .clone()
            .filter(|one| one.decided.is_none())
            .ok_or_else(|| self.hosted_refusal(MARGIN_NO_SUGGESTION, &format!("thread `{}` holds no suggested edit to decide", thread.id)))
    }

    /// Thread `named` of the Job, where it is not settled.
    pub(super) async fn open_thread(&self, id: &str, named: &str) -> Result<KeptMarginThread, Refusal> {
        let named = named.trim();
        let thread = self.store().lock().await.margin_thread(id, named).map_err(|why| self.ledger_fault(why))?;
        let thread = thread.ok_or_else(|| self.hosted_refusal(MARGIN_NO_THREAD, &format!("this job has no margin thread `{named}`")))?;
        match state_of(&thread.state) {
            MarginState::Settled => Err(self.hosted_refusal(MARGIN_SETTLED, &format!("thread `{named}` is settled"))),
            _ => Ok(thread),
        }
    }

    pub(super) async fn threads_kept(&self, id: &str, threads: &[KeptMarginThread]) -> Result<(), Refusal> {
        self.store().lock().await.keep_margin_threads(id, threads).map_err(|why| self.ledger_fault(why))
    }

    /// The page `document` names, by its file name or its path.
    pub(super) async fn margin_page(&self, id: &str, document: &str) -> Result<MarginPage, Refusal> {
        let named = document.trim();
        let named = named.strip_prefix(DOCUMENT_ITEM_PREFIX).unwrap_or(named);
        let (_, held) = self.document_held(id, named).await?;
        let page = held.ok_or_else(|| self.hosted_refusal(MARGIN_NO_PAGE, &format!("this job holds no page `{named}`")))?;
        let mut revisions = revisions_named(&page.detail);
        let count = revisions.len();
        Ok(MarginPage {
            round: RoundPage {
                name: page_name(&page.target).to_string(),
                title: page.detail.get("title").cloned().unwrap_or_else(|| page_name(&page.target).to_string()),
            },
            written: self.written_at(id, &page),
            newest: revisions.pop().map(std::path::PathBuf::from),
            revisions: count,
            path: page.target,
        })
    }

    pub(super) async fn margin_threads_of(&self, id: &str, name: &str) -> Result<MarginThreads, Refusal> {
        let threads = self.store().lock().await.margin_threads(id, Some(name)).map_err(|why| self.ledger_fault(why))?;
        Ok(MarginThreads { session_id: SessionId::carried(id), document: name.to_string(), threads: threads.iter().map(thread_of).collect() })
    }
}

/// A kept revision's words, off the turn loop.
pub(super) async fn read_page(path: &std::path::Path) -> Option<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || std::fs::read_to_string(path)).await.ok().and_then(Result::ok)
}

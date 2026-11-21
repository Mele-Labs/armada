//! The agent's side of margin threads: `answer_thread` and `suggest_edit`, each on a thread he sent it, and each
//! thread found again on every revision a page keeps. **Neither act writes the page**: an answer stays in the
//! thread, and a suggested edit waits on his Accept. `crates/ipc/operations/answer_thread.toml` and
//! `suggest_edit.toml` are the contract.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AnswerThread, Instant, ItemNote, MarginRoundKind, MarginState, MarginThread, SuggestEdit};
use store::{KeptMarginMessage, KeptMarginNote, KeptMarginSuggestion, KeptMarginThread};

use super::documents::page_name;
use super::margin::{read_page, MARGIN_EMPTY};
use super::margin_rounds::{state_of, state_word, thread_of, RoundPage};
use super::page_words::{edit_applied, passage_in};
use crate::daemon::Fleet;

const MARGIN_UNNAMED: &str = "fleet.margin_unnamed";
const MARGIN_NOT_SENT: &str = "fleet.margin_not_sent";
const MARGIN_EDIT_UNFIT: &str = "fleet.margin_edit_unfit";

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
    /// `answer_thread`: the agent's words in the thread. The page is not touched.
    pub(crate) async fn thread_answered(self: Arc<Self>, caller: Option<api::Caller>, said: AnswerThread) -> Result<MarginThread, Refusal> {
        let id = self.margin_placed(caller, said.session_id.as_ref())?;
        let text = said.text.trim().to_string();
        if text.is_empty() {
            return Err(self.hosted_refusal(MARGIN_EMPTY, "an answer says something"));
        }
        let mut thread = self.sent_thread(&id, &said.thread).await?;
        thread.messages.push(KeptMarginMessage { voice: "agent".into(), text, at: self.instant().as_str().to_string(), held: false });
        if state_of(&thread.state) == MarginState::Open {
            thread.state = state_word(MarginState::Answered).to_string();
        }
        self.agent_kept(&id, thread).await
    }

    /// `suggest_edit`: the agent's edit on the thread's passage, checked against the newest revision, waiting on his
    /// Accept. A newer suggestion takes the place of one he has not decided.
    pub(crate) async fn edit_suggested(self: Arc<Self>, caller: Option<api::Caller>, said: SuggestEdit) -> Result<MarginThread, Refusal> {
        let id = self.margin_placed(caller, said.session_id.as_ref())?;
        let mut thread = self.sent_thread(&id, &said.thread).await?;
        let page = self.margin_page(&id, &thread.document).await?;
        let html = match &page.newest {
            Some(newest) => read_page(newest).await.unwrap_or_default(),
            None => String::new(),
        };
        edit_applied(&html, &thread.quote, &said.struck, &said.added).map_err(|why| self.hosted_refusal(MARGIN_EDIT_UNFIT, &why))?;
        thread.suggestion = Some(KeptMarginSuggestion {
            struck: said.struck,
            added: said.added,
            at: self.instant().as_str().to_string(),
            decided: None,
            decided_at: None,
            decided_revision: None,
        });
        thread.state = state_word(MarginState::Suggested).to_string();
        self.agent_kept(&id, thread).await
    }

    async fn agent_kept(self: &Arc<Self>, id: &str, thread: KeptMarginThread) -> Result<MarginThread, Refusal> {
        self.threads_kept(id, std::slice::from_ref(&thread)).await?;
        self.margins_moved(id).await;
        let _ = self.published_hosted(id).await;
        Ok(thread_of(&thread))
    }

    /// The Job the call is for: the one whose process holds the connection, else the one the body names.
    fn margin_placed(&self, caller: Option<api::Caller>, named: Option<&ipc::SessionId>) -> Result<String, Refusal> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(MARGIN_UNNAMED, "a job to answer the thread for"))
    }

    /// Thread `named`, where he has sent the agent a comment on it.
    async fn sent_thread(&self, id: &str, named: &str) -> Result<KeptMarginThread, Refusal> {
        let thread = self.open_thread(id, named).await?;
        match thread.messages.iter().any(|one| one.voice == "you" && !one.held) {
            true => Ok(thread),
            false => Err(self.hosted_refusal(MARGIN_NOT_SENT, &format!("thread `{}` holds no comment sent to you yet", thread.id))),
        }
    }

    /// Each thread on the page at `path` found again in `html`, its newest revision: where the quote stands it takes
    /// that section, and where it does not it keeps the quote and is `gone`. A thread is only kept where it moved.
    pub(super) async fn margins_found_again(&self, id: &str, path: &str, html: &str) {
        let name = page_name(path).to_string();
        let mut store = self.store().lock().await;
        let Ok(threads) = store.margin_threads(id, Some(&name)) else { return };
        let moved: Vec<KeptMarginThread> = threads
            .into_iter()
            .filter_map(|mut thread| {
                let found = passage_in(html, &thread.quote);
                let (gone, section) = match found {
                    Some(section) => (false, section.or_else(|| thread.section.clone())),
                    None => (true, thread.section.clone()),
                };
                if gone == thread.gone && section == thread.section {
                    return None;
                }
                thread.gone = gone;
                thread.section = section;
                Some(thread)
            })
            .collect();
        if !moved.is_empty() {
            let _ = store.keep_margin_threads(id, &moved);
        }
    }

    /// A page sent back with his notes is kept as a round, each note with its passage and the section that holds it,
    /// so the Job's thread draws it as the notes he wrote and not as the message the agent read.
    pub(super) async fn sent_back_kept(&self, id: &str, path: &str, at: &Instant, said: &str, words: &str, notes: &[ItemNote]) {
        let Ok(page) = self.margin_page(id, path).await else { return };
        let html = match &page.newest {
            Some(newest) => read_page(newest).await.unwrap_or_default(),
            None => String::new(),
        };
        let mut kept: Vec<KeptMarginNote> = notes
            .iter()
            .filter(|note| !note.text.trim().is_empty())
            .map(|note| {
                let passage = note.on.as_ref().map(|on| on.quote.trim().to_string()).filter(|quote| !quote.is_empty());
                let section = passage.as_deref().and_then(|quote| passage_in(&html, quote)).flatten();
                KeptMarginNote { thread: None, section, passage, note: note.text.trim().to_string() }
            })
            .collect();
        if kept.is_empty() {
            kept.push(KeptMarginNote { thread: None, section: None, passage: None, note: words.to_string() });
        }
        let round = RoundPage { name: page.round.name, title: page.round.title };
        self.round_kept(id, &round, at, MarginRoundKind::SentBack, said, Vec::new(), kept).await;
    }
}

impl<H, V, W> api::Margins for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn comment_margin(self: Arc<Self>, comment: ipc::CommentMargin) -> Result<ipc::MarginThreads, Refusal> {
        self.margin_commented(comment).await
    }

    async fn send_held_margin(self: Arc<Self>, send: ipc::SendHeldMargin) -> Result<ipc::MarginThreads, Refusal> {
        self.margin_held_sent(send).await
    }

    async fn act_on_margin_thread(self: Arc<Self>, act: ipc::ActOnMargin) -> Result<ipc::MarginThreads, Refusal> {
        self.margin_acted(act).await
    }

    async fn get_margin_threads(&self, get: ipc::GetMarginThreads) -> Result<ipc::MarginThreads, Refusal> {
        self.margin_threads_read(get).await
    }

    async fn answer_thread(self: Arc<Self>, caller: Option<api::Caller>, answer: AnswerThread) -> Result<MarginThread, Refusal> {
        self.thread_answered(caller, answer).await
    }

    async fn suggest_edit(self: Arc<Self>, caller: Option<api::Caller>, suggest: SuggestEdit) -> Result<MarginThread, Refusal> {
        self.edit_suggested(caller, suggest).await
    }
}

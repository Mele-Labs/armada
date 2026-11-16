//! Where an ask stands, read from what Fleet already sees. `docs/concepts/job-story.md`, *How an ask moves*.
//!
//! **Built** when a pull request of the Job building it merges after the ask took that Job, read off the Job's `pr` rows
//! the way a wait on a pull request reads them. **Handed** when the Job building it starts another Job or messages one,
//! and a small model says which of its open asks the words carry: nothing else in the words names an ask. The agent
//! is not asked, and nothing it says about its own work moves an ask.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{ask_state, AttachmentState, Holder, KeptAsk, KeptHandoff, KeptMerge};

use super::sorting::{ask_kind_word, cut};
use crate::daemon::Fleet;

const ANSWER: &str = "handed:";
/// The most of the words the model reads.
const WORDS_CHARS: usize = 2000;

/// What the model is asked.
pub(crate) fn handed_question(words: &str, asks: &[KeptAsk]) -> String {
    let listed: Vec<String> = asks
        .iter()
        .enumerate()
        .map(|(at, ask)| format!("- {}: [{}] {}", at + 1, ask_kind_word(super::reading::kind_from(&ask.kind).unwrap_or(ipc::AskKind::ChangeRequest)), ask.title))
        .collect();
    format!(
        "A coding agent sent this message to another coding job, which may be a brief for a job it started:\n\n{words}\n\n\
         The agent is working on these open asks from a person:\n{listed}\n\n\
         Which of those asks does the message hand to the other job to work on? An ask only mentioned, or kept by the sender, is not handed.\n\n\
         Answer in one line and nothing else:\n\
         {ANSWER} the numbers of the asks handed, separated by commas, or none",
        words = cut(words, WORDS_CHARS),
        listed = listed.join("\n"),
    )
}

/// The ids of the asks whose numbers the model named.
pub(crate) fn handed_read(reply: &str, asks: &[KeptAsk]) -> Vec<String> {
    let line = reply.lines().map(str::trim).find(|line| line.to_lowercase().starts_with(ANSWER));
    let Some(said) = line.map(|line| &line[ANSWER.len()..]) else { return Vec::new() };
    let mut found: Vec<String> = Vec::new();
    for part in said.split(',') {
        let id = super::sorting::number_in(part).and_then(|number| asks.get(number.checked_sub(1)?)).map(|ask| ask.ask_id.clone());
        if let Some(id) = id.filter(|id| !found.contains(id)) {
            found.push(id);
        }
    }
    found
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
    /// One pass of the notice loop: each ask a Job is building whose pull request merged since the Job took it is built.
    pub(crate) async fn asks_built_checked(self: &Arc<Self>) {
        let working = self.store().lock().await.asks_in_work().unwrap_or_default();
        for ask in working {
            let Some(job) = ask.building.clone() else { continue };
            let held = self.store().lock().await.attachments_of(&Holder::session(&job)).unwrap_or_default();
            let mut merged: Vec<_> = held
                .iter()
                .filter(|one| one.kind == "pr" && one.state != AttachmentState::GivenBack)
                .filter(|one| crate::pull_requesting::pull_merged(&one.detail, one.state == AttachmentState::Spent))
                .filter(|one| one.changed_at.as_str() >= ask.state_at.as_str())
                .collect();
            if merged.is_empty() {
                continue;
            }
            merged.sort_by(|a, b| a.changed_at.cmp(&b.changed_at));
            let now = self.now().as_str().to_string();
            {
                let mut store = self.store().lock().await;
                for pull in &merged {
                    let merge = KeptMerge { ask_id: ask.ask_id.clone(), pull_request: pull.target.clone(), merged_at: pull.changed_at.clone() };
                    let _ = store.keep_ask_merge(&merge);
                }
                let built = KeptAsk { state: ask_state::BUILT.to_string(), state_at: now, ..ask.clone() };
                if store.keep_ask(&built).is_err() {
                    continue;
                }
            }
            self.story_changed(&[&ask.session_id, &job]);
        }
    }

    /// A message from Job `from` to Job `to`, or the brief of a Job `from` started: read for the asks it hands, on a task of its
    /// own. **Read again, a message hands nothing twice**: an ask handed is the other Job's to build, so `from` no longer holds it.
    pub(crate) fn handing_begun(self: &Arc<Self>, from: &str, to: &str, words: &str) {
        let (fleet, from, to, words) = (Arc::clone(self), from.to_string(), to.to_string(), words.to_string());
        tokio::spawn(async move { fleet.asks_handed(&from, &to, &words).await });
    }

    /// `words` went from Job `from` to Job `to`: the open asks `from` builds that they carry are `to`'s now.
    /// **Run on a task of its own**, since it asks the model. Nothing is asked of a Job that builds nothing.
    pub(crate) async fn asks_handed(self: &Arc<Self>, from: &str, to: &str, words: &str) {
        let asks: Vec<KeptAsk> = self
            .store()
            .lock()
            .await
            .asks_of_session(from)
            .unwrap_or_default()
            .into_iter()
            .filter(|ask| ask.building.as_deref() == Some(from) && matches!(ask.state.as_str(), ask_state::BUILDING | ask_state::HANDED))
            .collect();
        if asks.is_empty() || from == to {
            return;
        }
        let reply = match self.story_asked(&handed_question(words, &asks)).await {
            Ok(reply) => reply,
            Err(why) => {
                self.session_warned(from, "what it handed to another job was not read", &why);
                return;
            }
        };
        let handed = handed_read(&reply, &asks);
        if handed.is_empty() {
            return;
        }
        let now = self.now().as_str().to_string();
        let mut told = vec![from.to_string(), to.to_string()];
        {
            let mut store = self.store().lock().await;
            for ask in asks.iter().filter(|ask| handed.contains(&ask.ask_id)) {
                let hand = KeptHandoff { ask_id: ask.ask_id.clone(), from_session: from.to_string(), to_session: to.to_string(), at: now.clone() };
                let moved = KeptAsk { state: ask_state::HANDED.to_string(), building: Some(to.to_string()), state_at: now.clone(), ..ask.clone() };
                if store.keep_ask_handoff(&hand).is_ok() && store.keep_ask(&moved).is_ok() {
                    told.push(ask.session_id.clone());
                }
            }
        }
        self.story_changed(&told.iter().map(String::as_str).collect::<Vec<_>>());
    }
}

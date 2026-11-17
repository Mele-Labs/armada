//! A question of his, linked to the reply of the Job's that answers it, and whether he read that answer.
//! `docs/concepts/job-story.md`, *A question and its answer*.
//!
//! **Read off the turn, like the sorting.** A turn's end marks the Job's open questions due (`questions_due_at`), and the
//! notice loop takes the due ones a few at a time and reads each on a task of its own. The reply read is the Job's last
//! message after the question. A failed or unclear read is tried again on the next passes, [`TRIES`] in all, and then
//! waits for the next turn's end; a reply read as not answering is kept, so it is not asked about twice.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Instant, JobQuestion, JobQuestions, MarkAnswerRead, JobAnswer, SessionId, SessionRow, SessionVoice, WireError};
use store::KeptQuestion;
use tokio::task::JoinHandle;

use super::sorting::cut;
use crate::daemon::Fleet;

/// How many due questions one pass of the loop takes.
const AT_ONCE: u32 = 3;
/// The most of a reply the model reads, and how much of that is its opening. The rest is its close, where an agent puts what it concludes.
pub(crate) const REPLY_CHARS: usize = 4000;
const REPLY_HEAD_CHARS: usize = 1000;
/// The most of his question the model reads.
const WORDS_CHARS: usize = 1500;
/// The longest excerpt of an answer kept.
pub(crate) const EXCERPT_CHARS: usize = 300;

pub(crate) const ANSWERS: &str = "answers:";
/// How many reads of one question, in a row, may come back failed or unclear before it waits for a turn's end.
pub(crate) const TRIES: u32 = 3;
/// The most of an unclear answer the warning carries.
const UNCLEAR_CHARS: usize = 200;

/// The failed or unclear reads of each question, by Job and row, since its last clear read or its Job's last turn end.
#[derive(Default)]
pub(crate) struct Tries(Mutex<HashMap<(String, String), u32>>);

impl Tries {
    /// One more failed or unclear read of the question: whether it may be tried again.
    pub(crate) fn failed(&self, session: &str, row: &str) -> bool {
        let mut held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tries = held.entry((session.to_string(), row.to_string())).or_insert(0);
        *tries += 1;
        *tries < TRIES
    }

    /// The question was read clearly, or its Job's turn ended: it starts over.
    pub(crate) fn cleared(&self, session: &str, row: Option<&str>) {
        let mut held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        held.retain(|(one, at), _| one != session || row.is_some_and(|row| row != at));
    }
}

const NO_SUCH_QUESTION: &str = "fleet.no_such_question";
const QUESTION_OPEN: &str = "fleet.question_open";

/// `reply` as the model reads it: whole where it is short, else its opening and its close.
pub(crate) fn reply_cut(reply: &str) -> String {
    let reply = reply.trim();
    let count = reply.chars().count();
    if count <= REPLY_CHARS {
        return reply.to_string();
    }
    let head: String = reply.chars().take(REPLY_HEAD_CHARS).collect();
    let tail: String = reply.chars().skip(count - (REPLY_CHARS - REPLY_HEAD_CHARS)).collect();
    format!("{}\n...\n{}", head.trim_end(), tail.trim_start())
}

/// What the model is asked about one question and one reply.
pub(crate) fn asked(words: &str, reply: &str) -> String {
    format!(
        "A person asked a coding agent a question. Read the agent's reply and say whether it answers the question.\n\
         A reply answers when it gives the person what he asked for, even in part. A reply that only says it will look, \
         or answers something else, does not.\n\n\
         The question:\n{words}\n\n\
         The agent's reply:\n{reply}\n\n\
         Answer in one line and nothing else:\n\
         {ANSWERS} yes or no",
        words = cut(words, WORDS_CHARS),
        reply = reply_cut(reply),
    )
}

/// The model's answer, read: whether the reply answers. `None` where it says neither, which leaves the question open.
///
/// **The label is not needed**: the cheapest model answers a bare `yes` more often than `answers: yes`, which left a
/// question he saw answered open for good. Markdown around either is read through. The labelled line wins, else the first.
pub(crate) fn read_of(said: &str) -> Option<bool> {
    let lines: Vec<String> = said
        .lines()
        .map(|line| line.replace(['*', '_', '`'], "").trim().trim_start_matches(['-', '>']).trim().to_lowercase())
        .filter(|line| !line.is_empty())
        .collect();
    let line = lines
        .iter()
        .find_map(|line| line.strip_prefix(ANSWERS).map(str::trim))
        .or_else(|| lines.first().map(String::as_str))?;
    match line.split(|c: char| !c.is_alphabetic()).find(|word| !word.is_empty())? {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

/// The first prose of `reply`, cut to [`EXCERPT_CHARS`]: fenced code, table rows, rules and headings left out.
pub(crate) fn excerpt_of(reply: &str) -> String {
    let mut fenced = false;
    let mut prose: Vec<&str> = Vec::new();
    for line in reply.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        let rule = line.len() >= 3 && line.chars().all(|c| matches!(c, '-' | '*' | '_' | '='));
        if fenced || line.is_empty() || rule || line.starts_with('|') || line.starts_with('#') {
            continue;
        }
        prose.push(line);
    }
    let joined = prose.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
    cut(&joined, EXCERPT_CHARS)
}

/// The Job's last message after the question's row: the reply its turn ended on. `None` where it has said nothing since.
fn reply_after(rows: &[SessionRow], question_row: &str) -> Option<(String, String)> {
    let from = rows.iter().position(|row| row.id() == question_row)?;
    rows[from + 1..].iter().rev().find_map(|row| match row {
        SessionRow::Message { id, from: SessionVoice::Agent, text, .. } if !text.trim().is_empty() => Some((id.clone(), text.clone())),
        _ => None,
    })
}

fn question_wire(kept: KeptQuestion) -> JobQuestion {
    let answer = match (kept.reply_row, kept.answered_at) {
        (Some(reply_row), Some(at)) => Some(JobAnswer {
            reply_row,
            excerpt: kept.excerpt.unwrap_or_default(),
            answered_at: Instant::carried(&at),
            read: kept.read_at.is_some(),
        }),
        _ => None,
    };
    JobQuestion { row_id: kept.row_id, words: kept.words, asked_at: Instant::carried(&kept.at), answer }
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
    /// The Job's turn ended: its open questions are due a read. **One quick write and nothing awaited on the model.**
    pub(crate) async fn questions_due_at(&self, session: &str) {
        self.hosts().question_tries().cleared(session, None);
        if let Err(why) = self.store().lock().await.questions_due(session) {
            self.session_warned(session, "its questions were not marked due a read", format!("{why:?}"));
        }
    }

    /// One pass of the notice loop: the due questions are read, each on its own task, and none is awaited.
    pub(crate) async fn questions_reading(self: &Arc<Self>) {
        drop(self.questions_taken_to_read().await);
    }

    /// The due questions, taken and each given a task. **Taken once**: a second pass finds none of them.
    pub(crate) async fn questions_taken_to_read(self: &Arc<Self>) -> Vec<JoinHandle<()>> {
        let Ok(taken) = self.store().lock().await.take_due_questions(AT_ONCE) else { return Vec::new() };
        taken
            .into_iter()
            .map(|question| {
                let fleet = Arc::clone(self);
                tokio::spawn(async move { fleet.question_read(question).await })
            })
            .collect()
    }

    async fn question_read(self: Arc<Self>, question: KeptQuestion) {
        let session = question.session_id.as_str();
        let Ok(rows) = self.rows_of(session).await else { return };
        let Some((reply_row, reply)) = reply_after(&rows, &question.row_id) else { return };
        if question.checked_row.as_deref() == Some(reply_row.as_str()) {
            return;
        }
        let answers = match self.story_asked(&asked(&question.words, &reply)).await {
            Ok(said) => {
                let read = read_of(&said);
                if read.is_none() {
                    self.session_warned(session, "whether a reply answers his question was unclear", cut(&said, UNCLEAR_CHARS));
                }
                read
            }
            Err(why) => {
                self.session_warned(session, "whether a reply answers his question was not read", &why);
                None
            }
        };
        let tries = self.hosts().question_tries();
        match answers {
            Some(_) => tries.cleared(session, Some(&question.row_id)),
            // Tried again on the next passes, so an idle Job, whose turn may not end again, is not left open.
            None if tries.failed(session, &question.row_id) => {
                if let Err(why) = self.store().lock().await.keep_question_due(session, &question.row_id) {
                    self.session_warned(session, "his question was not marked due again", format!("{why:?}"));
                }
            }
            None => {}
        }
        let kept = {
            let mut store = self.store().lock().await;
            match answers {
                Some(true) => store.keep_question_answered(session, &question.row_id, &reply_row, &excerpt_of(&reply), self.now().as_str()),
                Some(false) => store.keep_question_checked(session, &question.row_id, &reply_row).map(|_| false),
                None => Ok(false),
            }
        };
        match kept {
            Ok(true) => {
                self.story_changed(&[session]);
                let _ = self.published_hosted(session).await;
            }
            Ok(false) => {}
            Err(why) => self.session_warned(session, "the reply that answers his question was not kept", format!("{why:?}")),
        }
    }

    /// `list_job_questions`.
    pub(crate) async fn job_questions_listed(&self, session: &SessionId) -> Result<JobQuestions, Refusal> {
        let id = session.as_str();
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.story_not_found(super::NO_SUCH_SESSION, format!("no Job Fleet hosts is named {id}")));
        }
        let questions = store.questions_of(id).map_err(|why| self.ledger_fault(why))?.into_iter().map(question_wire).collect();
        Ok(JobQuestions { session_id: session.clone(), questions })
    }

    /// `mark_answer_read`: he read the answer. Marking one already read changes nothing and answers it as it stands.
    pub(crate) async fn answer_read_marked(&self, mark: MarkAnswerRead) -> Result<JobQuestion, Refusal> {
        let id = mark.session_id.as_str();
        let (marked, question) = {
            let mut store = self.store().lock().await;
            if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
                return Err(self.story_not_found(super::NO_SUCH_SESSION, format!("no Job Fleet hosts is named {id}")));
            }
            let Some(question) = store.question(id, &mark.row_id).map_err(|why| self.ledger_fault(why))? else {
                return Err(self.story_not_found(NO_SUCH_QUESTION, format!("job {id} holds no question of his named {}", mark.row_id)));
            };
            if question.reply_row.is_none() {
                return Err(Refusal::IllegalMove(WireError::raised(QUESTION_OPEN, "no reply of the Job's answers this question yet", self.run_id())));
            }
            let marked = store.keep_answer_read(id, &mark.row_id, self.now().as_str()).map_err(|why| self.ledger_fault(why))?;
            let question = store.question(id, &mark.row_id).map_err(|why| self.ledger_fault(why))?.unwrap_or(question);
            (marked, question)
        };
        if marked {
            self.story_changed(&[id]);
            let _ = self.published_hosted(id).await;
        }
        Ok(question_wire(question))
    }
}

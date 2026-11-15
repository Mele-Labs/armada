//! A side question put to a Job: one read-only turn on a thrown-away copy of its conversation, kept beside its
//! record and never in its thread. `docs/concepts/session.md`, *An aside*.
//!
//! **The Job is not asked and does not know.** The copy is cut at the Job's last finished turn, so a Job at
//! work is copied as it stood; the turn holds the three read tools and nothing else (`adapters`' argument list
//! is the confinement); and the copy is never written, so the Job's own transcript is the file it was. The
//! answer reaches the Job only where the owner tells it (`tell_aside`), as a message of his.

use std::num::NonZeroU8;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Ask, CallProgress, Delivery, Fork, Model, Reading, Vcs, WorkProduct};
use api::{Asides, Refusal};
use ipc::{Aside, AsideList, AsideState, AskAside, Event, SendSessionMessage, SessionId, TellAside, WireError};
use store::{aside_state, KeptAside, KeptHosting};

use super::serving::NO_SUCH_SESSION;
use crate::daemon::Fleet;
use crate::judging::watched;

/// A blank question. A 422.
const ASIDE_BLANK: &str = "fleet.aside_blank";
/// The Job holds no aside of that id. A 404.
const NO_SUCH_ASIDE: &str = "fleet.no_such_aside";
/// An aside told while it is still asking, or failed. A 409.
const ASIDE_NOT_ANSWERED: &str = "fleet.aside_not_answered";
/// An aside told twice. A 409.
const ASIDE_TOLD: &str = "fleet.aside_told";

/// Turns the copy may take, its reads and its answer together.
const TURNS: NonZeroU8 = NonZeroU8::new(8).expect("eight is not zero");

/// The most often the answer so far goes out: about five a second.
const BETWEEN_SENDS: Duration = Duration::from_millis(200);

/// What an aside that no turn of this Fleet holds says: Fleet ended while it was asking.
const DIED: &str = "Fleet restarted before this was answered";

/// What marks an aside going in this process, in the set an agent run's keys share.
fn going_key(aside_id: &str) -> String {
    format!("aside:{aside_id}")
}

fn stamped(at: &str) -> ipc::Instant {
    ipc::Instant::carried(at)
}

fn state_of(word: &str) -> AsideState {
    match word {
        aside_state::ANSWERED => AsideState::Answered,
        aside_state::FAILED => AsideState::Failed,
        _ => AsideState::Asking,
    }
}

/// The aside as the wire carries it.
fn wire(kept: &KeptAside) -> Aside {
    Aside {
        aside_id: kept.aside_id.clone(),
        session_id: SessionId::carried(&kept.session_id),
        question: kept.question.clone(),
        quote: kept.quote.clone(),
        answer: kept.answer.clone(),
        state: state_of(&kept.state),
        failed: kept.failed.clone(),
        asked_at: stamped(&kept.asked_at),
        answered_at: kept.answered_at.as_deref().map(stamped),
        told_at: kept.told_at.as_deref().map(stamped),
    }
}

/// What the copy is told. **One line says it is a side question**, answered read-only, and not to act.
fn brief(question: &str, quote: Option<&str>) -> String {
    let quoted = quote.map(|text| format!("The owner had selected this in the thread:\n\n{}\n\n", blockquote(text))).unwrap_or_default();
    format!(
        "This is a side question from the owner, put to a read-only copy of this conversation. Answer it from \
         what you know and, where it helps, by reading files with Read, Grep and Glob. Do not act: change \
         nothing, run nothing and do not carry on the work.\n\n{quoted}Question:\n\n{question}"
    )
}

fn blockquote(text: &str) -> String {
    text.trim().lines().map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n")
}

/// What the owner's message to the Job says: the aside, with the words it was asked in.
fn told_words(kept: &KeptAside) -> String {
    let quoted = kept.quote.as_deref().map(|text| format!("On this text from the thread:\n\n{}\n\n", blockquote(text))).unwrap_or_default();
    format!(
        "Aside, answered on a read-only copy of this Job:\n\n{quoted}{}\n\n{}",
        blockquote(&kept.question),
        kept.answer.trim()
    )
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
    /// `ask_aside`. **Everything that can refuse is read before anything is kept**, so a refusal leaves no aside.
    pub(crate) async fn aside_asked(self: Arc<Self>, ask: AskAside) -> Result<Aside, Refusal> {
        let id = ask.session_id.as_str().trim().to_string();
        let question = ask.question.trim().to_string();
        if question.is_empty() {
            return Err(self.hosted_refusal(ASIDE_BLANK, "a side question needs words"));
        }
        let quote = ask.quote.map(|text| text.trim().to_string()).filter(|text| !text.is_empty());
        let (session, hosting) = self.asided_job(&id).await?;
        let at = self.latest_fork_point(&id).await?;
        let cut = self.fork_cut(&id, &at).await?;
        let directory = match self.unset_up_directory(&id, &hosting).await {
            Some(root) => root,
            None => {
                let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
                Self::directory_of(&served, &hosting)
            }
        };
        let reviewing = self.reviewing().map_err(|why| self.hosted_fault("fleet.aside_no_agent", &format!("no agent could be asked: {why:?}")))?;
        let model = hosting.model.clone().filter(|model| !model.trim().is_empty()).unwrap_or_else(|| self.session_model());
        let model = Model::named(&model).map_err(|why| self.hosted_refusal(ASIDE_BLANK, &format!("the Job's model is not one: {why:?}")))?;
        let copy = super::rows::new_id();
        let refused = || self.hosted_refusal(super::fork_point::FORK_POINT_UNWRITTEN, "this conversation cannot be copied for a side question");
        let reading = Reading::checkout(&directory, TURNS).map_err(|_| refused())?;
        let reading = reading.forking(Fork::copy_of(&session.id, &cut, &copy).ok_or_else(refused)?);
        let ask = Ask::put(model, &brief(&question, quote.as_deref()), reviewing.environment.clone())
            .map(|ask| ask.reading(reading))
            .map_err(|why| self.hosted_refusal(ASIDE_BLANK, &format!("the question could not be put: {why:?}")))?;

        let kept = KeptAside {
            aside_id: self.mint().ulid().as_str().to_lowercase(),
            session_id: id.clone(),
            question,
            quote,
            answer: String::new(),
            state: aside_state::ASKING.to_string(),
            failed: None,
            asked_at: self.now().as_str().to_string(),
            answered_at: None,
            told_at: None,
        };
        self.store().lock().await.keep_aside(&kept).map_err(|why| self.ledger_fault(why))?;
        self.review_going().lock().await.insert(going_key(&kept.aside_id));
        let answer = wire(&kept);
        self.publish(Event::SessionAside(answer.clone()));
        tokio::spawn(async move { self.aside_driven(kept, ask, reviewing.client, reviewing.budget, copy).await });
        Ok(answer)
    }

    /// The turn, from the question to the kept answer. **Whatever happens the aside ends**, so none is left asking,
    /// and **whatever the copy wrote is removed**: it is told to write nothing, and this is the second lock.
    async fn aside_driven(
        self: Arc<Self>,
        mut kept: KeptAside,
        ask: Ask,
        client: Arc<dyn adapter_traits::ModelClient + Send + Sync>,
        budget: crate::judging::JudgeBudget,
        copy: String,
    ) {
        let sent: Arc<StdMutex<(String, Instant)>> = Arc::new(StdMutex::new((String::new(), Instant::now())));
        let telling = {
            let (fleet, sent, base) = (Arc::clone(&self), Arc::clone(&sent), kept.clone());
            move |progress: CallProgress| {
                let mut held = sent.lock().expect("not poisoned");
                match progress {
                    // A new request is a new turn, whose words replace the last one's: a turn that read a file
                    // first said "let me look" before its answer.
                    CallProgress::Requesting => held.0.clear(),
                    CallProgress::Answering { text, .. } => held.0.push_str(&text),
                    _ => return,
                }
                if held.1.elapsed() < BETWEEN_SENDS || held.0.is_empty() {
                    return;
                }
                held.1 = Instant::now();
                // Sent and not kept: the store holds the answer once it ends, and a late write of a part
                // could land after the whole.
                let mut so_far = base.clone();
                so_far.answer = held.0.clone();
                fleet.publish(Event::SessionAside(wire(&so_far)));
            }
        };
        let answered = watched(client.as_ref(), &ask, budget, &telling, std::future::pending::<()>()).await;
        match answered {
            Ok(said) => {
                kept.state = aside_state::ANSWERED.to_string();
                kept.answer = said.trim().to_string();
            }
            Err(why) => {
                kept.state = aside_state::FAILED.to_string();
                kept.failed = Some(why.in_a_line("aside"));
            }
        }
        kept.answered_at = Some(self.now().as_str().to_string());
        // The copy is written nowhere. Where one was written all the same it goes.
        let home = self.host().home.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Some(file) = adapters::terminal_thread::find(&home, &copy) {
                let _ = std::fs::remove_file(file);
            }
        })
        .await;
        let _ = self.store().lock().await.keep_aside(&kept);
        self.review_going().lock().await.remove(&going_key(&kept.aside_id));
        self.publish(Event::SessionAside(wire(&kept)));
    }

    /// `list_asides`. One that reads asking and that no turn of this process holds ended with the Fleet that ran it.
    pub(crate) async fn asides_listed(&self, session: &SessionId) -> Result<AsideList, Refusal> {
        let id = session.as_str().trim().to_string();
        self.asided_job(&id).await?;
        let kept = self.store().lock().await.asides_of(&id).map_err(|why| self.ledger_fault(why))?;
        let going = self.review_going().lock().await;
        let asides = kept
            .iter()
            .map(|one| match one.state == aside_state::ASKING && !going.contains(&going_key(&one.aside_id)) {
                true => wire(&self.aside_died(one.clone())),
                false => wire(one),
            })
            .collect();
        Ok(AsideList { session_id: SessionId::carried(&id), asides })
    }

    fn aside_died(&self, mut kept: KeptAside) -> KeptAside {
        kept.state = aside_state::FAILED.to_string();
        kept.failed = Some(DIED.to_string());
        kept
    }

    /// `tell_aside`: the answer goes to the Job as the owner's message, **once**, by the route a message of his takes.
    pub(crate) async fn aside_told(self: Arc<Self>, tell: TellAside) -> Result<Aside, Refusal> {
        let id = tell.session_id.as_str().trim().to_string();
        self.asided_job(&id).await?;
        let kept = self.store().lock().await.aside_of(&id, &tell.aside_id).map_err(|why| self.ledger_fault(why))?;
        let kept = kept.ok_or_else(|| Refusal::NotFound(WireError::raised(NO_SUCH_ASIDE, format!("no aside {} in this Job", tell.aside_id), self.run_id())))?;
        let refuse = |code: &'static str, why: &str| Refusal::IllegalMove(WireError::raised(code, why, self.run_id()));
        if kept.told_at.is_some() {
            return Err(refuse(ASIDE_TOLD, "this aside was told to the Job already"));
        }
        if kept.state != aside_state::ANSWERED {
            return Err(refuse(ASIDE_NOT_ANSWERED, "only an answered aside is told to the Job"));
        }
        let sent = SendSessionMessage {
            session_id: SessionId::carried(&id),
            text: told_words(&kept),
            attachments: Vec::new(),
            mentions: Vec::new(),
            sketches: Vec::new(),
        };
        let terminal = self.terminal_session(&id).await?;
        self.visit_kept(&id).await;
        Arc::clone(&self).mod_message_sent(terminal, sent, None).await?;
        let at = self.now().as_str().to_string();
        let held = {
            let mut store = self.store().lock().await;
            store.keep_aside_told(&id, &kept.aside_id, &at).map_err(|why| self.ledger_fault(why))?;
            store.aside_of(&id, &kept.aside_id).map_err(|why| self.ledger_fault(why))?
        };
        let told = wire(held.as_ref().unwrap_or(&kept));
        self.publish(Event::SessionAside(told.clone()));
        Ok(told)
    }

    /// The Job a side question is put to: one Fleet hosts, or it is unknown.
    async fn asided_job(&self, id: &str) -> Result<(store::KeptSession, KeptHosting), Refusal> {
        let store = self.store().lock().await;
        let found = store.session(id).and_then(|session| Ok((session, store.hosting(id)?))).map_err(|why| self.ledger_fault(why))?;
        match found {
            (Some(session), Some(hosting)) => Ok((session, hosting)),
            _ => Err(Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("no Job Fleet hosts is named {id}"), self.run_id()))),
        }
    }
}

impl<H, V, W> Asides for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn ask_aside(self: Arc<Self>, ask: AskAside) -> Result<Aside, Refusal> {
        self.aside_asked(ask).await
    }

    async fn list_asides(&self, session_id: SessionId) -> Result<AsideList, Refusal> {
        self.asides_listed(&session_id).await
    }

    async fn tell_aside(self: Arc<Self>, tell: TellAside) -> Result<Aside, Refusal> {
        self.aside_told(tell).await
    }
}

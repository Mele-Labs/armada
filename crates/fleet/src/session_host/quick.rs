//! `get_session_quick`: what a Job's quick bar is offered beyond his saved lines. `session_quick.rs` is the
//! question, the reading and the ranking; this makes the call and keeps what it cost.
//!
//! **Asked on `models.proposer`**, which ships as haiku, through the proposer's own client, budget and
//! environment: a one-shot call with no tools and no directory. The answers are kept by Job and agent
//! message, so one message is asked once, on the first read after it, and a failure is kept as an empty list
//! rather than asked again at the next read. **The lines he sent are read off every Job's thread at most once
//! a minute**, and only while `thread.look` has learning on.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Ask, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;
use ipc::{SessionQuick, SessionRow};
use tokio::sync::OnceCell;

use crate::daemon::Fleet;
use crate::judging::said;
use crate::session_quick::{agent_spoke_last, asks, learnt, question, read_asked, sent_in, QuickLook, Sent, VOICE_LINES};

/// The longest the model is waited on for one message's answers.
const ASK_WITHIN: Duration = Duration::from_secs(20);
/// The least time between two readings of every Job's thread for his sent lines.
const READ_EVERY: Duration = Duration::from_secs(60);

/// What Fleet keeps for the quick bar. Memory only.
pub(crate) struct Kept {
    /// By Job: the agent message answers were asked for, by its row id, and the answers. One message a Job.
    asked: Mutex<HashMap<String, (String, Arc<OnceCell<Vec<String>>>)>>,
    /// Every line he sent, in every Job, and when they were read. Held across a reading, so two reads make one.
    sent: tokio::sync::Mutex<Option<(Instant, Arc<Vec<Sent>>)>>,
    read_every: AtomicU64,
}

impl Default for Kept {
    fn default() -> Kept {
        Kept { asked: Mutex::default(), sent: tokio::sync::Mutex::new(None), read_every: AtomicU64::new(READ_EVERY.as_millis() as u64) }
    }
}

impl Kept {
    /// Where the answers to `row`, a Job's last agent message, are kept. A newer message replaces the older's.
    fn answers(&self, session: &str, row: &str) -> Arc<OnceCell<Vec<String>>> {
        let mut held = self.asked.lock().expect("held across no panic");
        match held.get(session) {
            Some((was, cell)) if was == row => Arc::clone(cell),
            _ => {
                let cell = Arc::new(OnceCell::new());
                held.insert(session.to_string(), (row.to_string(), Arc::clone(&cell)));
                cell
            }
        }
    }

    fn read_every(&self) -> Duration {
        Duration::from_millis(self.read_every.load(Ordering::Relaxed))
    }

    /// Whether his lines have been read off the threads since Fleet began.
    #[cfg(test)]
    pub(crate) fn sent_read(&self) -> bool {
        self.sent.try_lock().is_ok_and(|held| held.is_some())
    }

    /// A shorter wait than the shipped one, so a test does not sit through it.
    #[cfg(test)]
    pub(crate) fn read_sent_every(&self, every: Duration) {
        self.read_every.store(every.as_millis() as u64, Ordering::Relaxed);
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
    pub(crate) async fn session_quick(&self, id: &str) -> Result<SessionQuick, Refusal> {
        self.session_and_hosting(id).await?;
        let look = QuickLook::read(self.machine_settings().now().get(keys::THREAD_LOOK).as_ref());
        if look.hides(id) {
            return Ok(SessionQuick::default());
        }
        let rows = self.rows_of(id).await?;
        let asked = self.quick_asked(id, &rows).await;
        let learnt = match look.learn {
            false => Vec::new(),
            true => {
                let sent = self.quick_sent().await;
                let asking = agent_spoke_last(&rows).is_some_and(|(_, text)| asks(text));
                learnt(&sent, &look.left_out, asking, self.now().epoch_millis().unwrap_or_default())
            }
        };
        Ok(SessionQuick { asked, learnt })
    }

    /// The answers to the Job's last message where the agent spoke last. Asked once for that message.
    async fn quick_asked(&self, id: &str, rows: &[SessionRow]) -> Vec<String> {
        let Some((row, message)) = agent_spoke_last(rows) else { return Vec::new() };
        let cell = self.hosts().quick.answers(id, row);
        cell.get_or_init(|| async {
            match self.quick_written(message, rows).await {
                Ok(written) => written,
                Err(why) => {
                    self.session_warned(id, "its quick replies were not written", why);
                    Vec::new()
                }
            }
        })
        .await
        .clone()
    }

    async fn quick_written(&self, message: &str, rows: &[SessionRow]) -> Result<Vec<String>, String> {
        let proposing = self.proposing().map_err(|why| format!("{why:?}"))?;
        let mut his: Vec<String> = Vec::new();
        for one in sent_in(rows).into_iter().rev() {
            if his.len() < VOICE_LINES && !his.contains(&one.text) {
                his.push(one.text);
            }
        }
        let ask = Ask::put(proposing.model.clone(), &question(message, &his), proposing.environment.clone())
            .map_err(|_| String::from("there was nothing to ask"))?;
        let reply = tokio::time::timeout(ASK_WITHIN, said(proposing.client.as_ref(), &ask, proposing.budget))
            .await
            .map_err(|_| String::from("the model did not answer in time"))?
            .map_err(|why| why.to_string())?;
        Ok(read_asked(&reply))
    }

    /// Every line he sent in every Job, as the last reading had them if that was under a minute ago.
    async fn quick_sent(&self) -> Arc<Vec<Sent>> {
        let mut held = self.hosts().quick.sent.lock().await;
        if let Some((read, sent)) = held.as_ref() {
            if read.elapsed() < self.hosts().quick.read_every() {
                return Arc::clone(sent);
            }
        }
        let sent = Arc::new(self.sent_in_every_job().await);
        *held = Some((Instant::now(), Arc::clone(&sent)));
        sent
    }

    /// One Job's thread at a time, the store let go between them. A body that cannot be a message is not decoded.
    async fn sent_in_every_job(&self) -> Vec<Sent> {
        let Ok(hosted) = self.store().lock().await.hostings() else { return Vec::new() };
        let mut sent = Vec::new();
        for hosting in hosted {
            let Ok(bodies) = self.store().lock().await.session_rows(&hosting.session_id) else { continue };
            let rows: Vec<SessionRow> = bodies
                .iter()
                .filter(|body| body.contains("\"message\""))
                .filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok())
                .collect();
            sent.extend(sent_in(&rows));
            tokio::task::yield_now().await;
        }
        sent
    }
}

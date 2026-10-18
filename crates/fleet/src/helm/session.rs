//! Helm as a hosted Session: one per repository, made by the first message the dock sends and
//! ended by Start fresh. `docs/concepts/helm.md`.
//!
//! **It is a Session, so everything here is a Session's with one difference.** The send is
//! `send_known_ahead`, the thread is the Session's rows, the model and effort are `tune_session`'s,
//! the permission is the Auto gate (`session_host::asking`), the first write leases a slot and cuts
//! a branch (`session_host::gate`), and Start fresh is `close_session`. What is Helm's is the
//! lookup that finds the Session, the note appended to its system prompt, the line saying where the
//! person is, and the `sent` row that records where a routed thought went.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AskHelm, CloseSession, HelmDebugInfo, HelmSession, HelmThoughtRouted, Instant, ManifestId,
    SendSessionMessage, SessionId, SessionRecord, SessionRow, StartSession, ThoughtSent,
};
use std::sync::Arc;
use store::KeptHosting;

use super::composing::where_they_are;
use super::recording;
use super::{worded, Authority};
use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::session_host::{mode_of, MESSAGE_EMPTY};

/// An effort that is not `low`, `medium` or `high`. A 422.
const UNACCEPTABLE_EFFORT: &str = "fleet.unacceptable_effort";

/// What a Helm Session is titled. Its first message does not retitle it.
pub(crate) const TITLE: &str = "Helm";

/// A blank string counts as absent.
fn chosen(word: Option<String>) -> Option<String> {
    word.map(|word| word.trim().to_string())
        .filter(|word| !word.is_empty())
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
    /// The note appended to the system prompt of every process of this repository's Helm Session,
    /// in the words in force now. **Read at each start**, so an edit reaches the next process.
    pub(crate) fn helm_note_of(&self, served: &Served) -> String {
        worded(&self.prompts(), self.helm_authority(), served.manifest(), None)
            .as_str()
            .to_string()
    }

    /// The repository's live Helm Session, if it has one.
    pub(super) async fn live_helm(&self, manifest: &str) -> Result<Option<KeptHosting>, Refusal> {
        self.store()
            .lock()
            .await
            .live_helm_of(manifest)
            .map_err(|why| self.ledger_fault(why))
    }

    /// The id of the repository's live Helm Session, **made where there is none**. `model` and
    /// `effort` are read only by the call that makes it, and a Session that follows an ended one
    /// begins on what the last was tuned to. At most one is live: the lookup and the making are
    /// one step under a lock.
    pub(super) async fn helm_session_id(
        &self,
        served: &Served,
        model: Option<String>,
        effort: Option<String>,
    ) -> Result<String, Refusal> {
        let manifest = served.manifest().id().as_str().to_string();
        let _starting = self.hosts().helm_starting().lock().await;
        if let Some(live) = self.live_helm(&manifest).await? {
            return Ok(live.session_id);
        }
        let effort = chosen(effort);
        if let Some(word) = &effort {
            if crate::tuning::effort_of(word).is_none() {
                return Err(self.hosted_refusal(
                    UNACCEPTABLE_EFFORT,
                    &format!("`{word}` is not an effort. Use low, medium or high"),
                ));
            }
        }
        let last = self
            .store()
            .lock()
            .await
            .latest_helm_of(&manifest)
            .map_err(|why| self.ledger_fault(why))?;
        let made = self
            .started_hosted(
                StartSession {
                    manifest_id: ManifestId::carried(&manifest),
                    title: Some(String::from(TITLE)),
                    model: chosen(model).or(last.as_ref().and_then(|last| last.model.clone())),
                    effort: effort.or(last.as_ref().and_then(|last| last.effort.clone())),
                    mode: last.as_ref().map(|last| mode_of(&last.mode)),
                    root: None,
                    fork: None,
                    request: None,
                    after: None,
                },
                true,
            )
            .await?;
        Ok(made.id.as_str().to_string())
    }

    /// The rows of the repository's live Helm Session, for Fleet's own reads. `Err` says why there
    /// are none.
    pub(crate) async fn helm_thread_rows(&self, manifest: &str) -> Result<Vec<SessionRow>, String> {
        let live = self
            .live_helm(manifest)
            .await
            .map_err(|refusal| refusal.error().message.clone())?;
        match live {
            Some(live) => self
                .rows_of(&live.session_id)
                .await
                .map_err(|refusal| refusal.error().message.clone()),
            None => Err(String::from("this repository has no Helm Job")),
        }
    }

    /// `get_helm_session`.
    pub(super) async fn helm_session_read(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmSession, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let live = self.live_helm(served.manifest().id().as_str()).await?;
        let session = match live {
            Some(live) => Some(
                Arc::clone(&self)
                    .get_session(SessionId::carried(live.session_id))
                    .await?,
            ),
            None => None,
        };
        Ok(HelmSession {
            manifest_id: on_the_wire(&served),
            session,
        })
    }

    /// `ask_helm`: the message, sent as a Session's is, with the line saying where the person is
    /// ahead of their words in the turn. **A message with nothing in it makes no Session.**
    pub(super) async fn helm_asked(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<SessionRecord, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        if asked.text.trim().is_empty() && asked.attachments.is_empty() {
            return Err(self.hosted_refusal(
                MESSAGE_EMPTY,
                "a message needs words or something sent with it",
            ));
        }
        let ahead = asked.context.as_ref().map(where_they_are);
        let id = self
            .helm_session_id(&served, asked.model, asked.effort)
            .await?;
        let sent = SendSessionMessage {
            session_id: SessionId::carried(id),
            text: asked.text,
            attachments: asked.attachments,
            mentions: asked.mentions,
        };
        self.send_known_ahead(None, sent, ahead).await
    }

    /// `start_helm_fresh`: the live Session is closed, as `close_session` closes one.
    pub(super) async fn helm_started_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmSession, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        if let Some(live) = self.live_helm(served.manifest().id().as_str()).await? {
            self.close_session(CloseSession {
                session_id: SessionId::carried(live.session_id),
            })
            .await?;
        }
        Ok(HelmSession {
            manifest_id: on_the_wire(&served),
            session: None,
        })
    }

    /// `record_thought_sent`: where the words went, as a `sent` row in the Helm Session's thread
    /// and the audit event. **Never told to Helm's process**: the row is Fleet's record, and the
    /// agent hears only what is asked of it. A repository with no live Helm Session gets one made
    /// for the row, which starts no process.
    pub(crate) async fn thought_sent(
        &self,
        sent: ThoughtSent,
        manifest_id: Option<ManifestId>,
    ) -> Result<SessionRecord, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let id = self.helm_session_id(&served, None, None).await?;
        // Kept as a message's are, so a refused upload leaves no row.
        let kept = self.keep_uploads(&self.uploads_of(&id), &id, &sent.attachments)?;
        let at = Instant::from(&self.now());
        let (to, routed) = (sent.target.kind, sent.target.id.clone());
        self.row_put(
            &id,
            SessionRow::Sent {
                id: self.row_id(&id),
                at: at.clone(),
                text: sent.text,
                target: sent.target,
                files: kept.files,
                tags: sent.mentions,
            },
        )
        .await;
        self.events()
            .publish(ipc::Event::HelmThoughtRouted(HelmThoughtRouted {
                manifest_id: on_the_wire(&served),
                to,
                id: routed,
                at,
            }));
        self.published_hosted(&id).await
    }

    /// `get_helm_debug_info`. **Taken at a moment, and never a second read of the thread to follow**:
    /// everything here is read in one pass.
    pub(super) async fn helm_debugged(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmDebugInfo, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let authority: Authority = self.helm_authority();
        let live = self.live_helm(served.manifest().id().as_str()).await?;
        let (model, rows, polled, session) = match live {
            Some(live) => {
                let rows = self.rows_of(&live.session_id).await?;
                let polled = self.hosts().of(&live.session_id).state().polled.clone();
                (
                    live.model.unwrap_or_else(|| String::from("default")),
                    rows,
                    polled,
                    Some(live.session_id),
                )
            }
            None => (String::from("default"), Vec::new(), None, None),
        };
        let (thread, cut) = recording::thread(&rows);
        Ok(HelmDebugInfo {
            manifest_id: on_the_wire(&served),
            checkout: served.root().to_string(),
            authority: recording::authority_on_the_wire(authority),
            model,
            // The note the next process is started with, assembled the one way a start assembles
            // it and never a second wording of it.
            note: self.helm_note_of(&served),
            door: ipc::door::SERVER.to_string(),
            tools: recording::tools(authority),
            session,
            thread,
            cut,
            polled,
            run_id: self.run_id().as_str().to_string(),
            protocol_id: ipc::ProtocolId::current(),
            at: Instant::from(&self.now()),
        })
    }

    /// What a Helm Session's `get_events_since` was answered, kept for the record. **Told by the
    /// transport**, which has placed the caller as Helm.
    pub(super) async fn helm_poll_kept(&self, manifest_id: ManifestId, counted: ipc::EventsSince) {
        let Ok(served) = self.served_named(Some(&manifest_id)) else {
            return;
        };
        if let Ok(Some(live)) = self.live_helm(served.manifest().id().as_str()).await {
            self.hosts().of(&live.session_id).state().polled = Some(counted);
        }
    }
}

pub(super) fn on_the_wire(served: &Served) -> ManifestId {
    ManifestId::carried(served.manifest().id().as_str())
}

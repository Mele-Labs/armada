//! The Operator as one agent: a hosted session per project with Fleet's tools, which puts cards in front
//! of the person and which Fleet tells what it used to rule on. `crates/ipc/operations/raise_card.toml`.
//!
//! **It has no repository, no slot and no Manifest.** It starts in a folder of its own under the
//! Operator's home, as a Mod Job starts in the mods folder where Armada's own repository is not served,
//! so the door answers it without a Manifest and offers it the reads and acts that name their
//! repository themselves. **One runs for each project** while `features.operator` is on, the project's own switch is on and
//! none runs for it: it starts the moment the switch goes on or a send for it waits, and the tick starts one where that did
//! not. Either switch off ends it. A hand on at the context limit keeps `operator` and the project on the successor.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::task::JoinHandle;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::operator_page::OperatorFeedKind;
use ipc::{CloseSession, ManifestId, Notice, SendSessionMessage, SessionId, SessionTag, SessionUpload, SessionVoice};

use crate::daemon::Fleet;
use super::arriving::{Arrival, Arrived};
use crate::session_host::Marked;

/// What the Operator's session is called where a title is drawn.
pub(crate) const TITLE: &str = "Operator";

/// Its first turn. The rules are in its system prompt, so this only wakes it. **It watches from now**: what waited on him
/// before it came on stays on each Job's own Needs you, so a fresh session does not card the backlog.
const BEGIN: &str = "You are up. Your watch starts now: what already waits on him stays on each Job's own Needs you, \
so put no card up for it. Put a card in front of him only for something that arrives from here on and needs his decision.";

/// The refusal for an act only the Operator's session may make.
pub(crate) const NOT_THE_OPERATOR: &str = "fleet.not_the_operator";

/// Start or end the Operator's session the moment its switch moves or a send waits for it, not on the next tick.
/// Production starts it beside the other keepers, and a test starts it itself.
pub fn keep_the_operator_up<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        loop {
            fleet.operator_seat().wake.notified().await;
            fleet.operator_session_kept().await;
        }
    })
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
    /// Whether `id` is the Operator's own session, as its placed process says.
    pub(crate) fn session_is_the_operator(&self, id: &str) -> bool {
        self.hosts().of(id).state().operator
    }

    /// The folder the Operators' sessions run in, beside its journal. Each project's is under it, named for the project.
    pub(crate) fn operator_dir(&self) -> PathBuf {
        let journal = self.journal_dir();
        journal.parent().map(|home| home.join("session")).unwrap_or_else(|| journal.join("session"))
    }

    /// `project`'s live Operator session's id, where one runs.
    pub(crate) async fn operator_session_id(&self, project: &str) -> Option<String> {
        self.store().lock().await.live_operator_hosting_of(project).ok().flatten().map(|hosting| hosting.session_id)
    }

    /// Every live Operator session as its project and id, oldest first. A session of no project, which an earlier Fleet
    /// started, has the empty project.
    pub(crate) async fn operator_sessions(&self) -> Vec<(String, String)> {
        let hostings = self.store().lock().await.live_operator_hostings().unwrap_or_default();
        hostings.into_iter().map(|hosting| (hosting.project.unwrap_or_default(), hosting.session_id)).collect()
    }

    /// The project the Operator session `session_id` is for, where it is one.
    pub(crate) async fn operator_project_of(&self, session_id: &str) -> Option<String> {
        let hosting = self.store().lock().await.hosting(session_id).ok().flatten()?;
        hosting.project.filter(|_| hosting.operator)
    }

    /// Refuse a caller that is not the Operator's session, and name the session otherwise.
    pub(crate) fn operator_called(&self, caller: Option<api::Caller>) -> Result<String, Refusal> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .filter(|id| self.session_is_the_operator(id))
            .ok_or_else(|| self.hosted_refusal(NOT_THE_OPERATOR, "only the Operator's own session may call this"))
    }

    /// Refuse a call through the door that is not the Operator's session, and let through a call that arrived by no door.
    /// **The door is the only thing that places a caller** (`api::acting::asking`), so a call with none is the person's own
    /// from Bridge, and `api::callers::THE_PERSONS_OWN` has already refused it to a process held inside a Job.
    pub(crate) fn owner_or_operator_called(&self, caller: Option<api::Caller>) -> Result<(), Refusal> {
        match caller {
            None => Ok(()),
            Some(_) => self
                .operator_called(caller)
                .map(drop)
                .map_err(|_| self.hosted_refusal(NOT_THE_OPERATOR, "only Bridge or the Operator's own session may call this")),
        }
    }

    /// Once a tick, after each repository in no project has become one: start the session of every project whose Operator runs
    /// (the machine's switch on and the project's own) and has none, and end the session of every project whose does not. While
    /// the machine's switch is on, a picked card nothing settled in time comes back to the front.
    pub(crate) async fn operator_session_kept(self: &Arc<Self>) {
        let _one = self.operator_seat().session.lock().await;
        self.operator_loose_projects_made().await;
        let wanted = self.operator_projects_wanted().await;
        let live = self.operator_sessions().await;
        for project in &wanted {
            match live.iter().find(|(has, _)| has == project) {
                // A send kept while none ran, where another start won the race to tell it.
                Some((_, id)) => self.held_sends_delivered(project, id).await,
                None => match self.operator_session_started(project).await {
                    Ok(id) => self.held_sends_delivered(project, &id).await,
                    Err(why) => self.operator_warned("an Operator's session did not start", format!("{project}: {why:?}")),
                },
            }
        }
        for (_, id) in live.iter().filter(|(has, _)| !wanted.contains(has)) {
            if let Err(why) = self.closed_and_parked(CloseSession { session_id: ipc::SessionId::carried(id) }).await {
                self.operator_warned("an Operator's session did not end", format!("{id}: {why:?}"));
            }
        }
        if self.operator_on() {
            self.cards_stalled_returned().await;
        }
    }

    /// The projects whose Operator should run now: each of his own while the machine's switch is on and its own is.
    async fn operator_projects_wanted(&self) -> Vec<String> {
        if !self.operator_on() {
            return Vec::new();
        }
        let kept = self.store().lock().await.projects().map(|kept| kept.projects).unwrap_or_default();
        let mut wanted = Vec::new();
        for one in kept {
            if self.operator_settings_of(&one.id).await.on {
                wanted.push(one.id);
            }
        }
        wanted
    }

    /// Start `project`'s Operator session on its model and effort, and send its first turn.
    async fn operator_session_started(self: &Arc<Self>, project: &str) -> Result<String, Refusal> {
        let dir = self.operator_dir().join(project);
        std::fs::create_dir_all(&dir).map_err(|why| self.hosted_refusal("fleet.operator_session_folder", &format!("the Operator's folder could not be made: {why}")))?;
        let start = self.operator_start_of(project, Some(TITLE.to_string())).await;
        let id = self.started_hosted(start, Marked::Operator).await?.id.as_str().to_string();
        self.first_message_from(&id, SessionVoice::You, BEGIN).await?;
        Ok(id)
    }

    /// Fleet's word to `project`'s Operator session, where one runs. `false` where none does, or it could not
    /// be told, and the caller goes on as it did before the session.
    pub(crate) async fn operator_told(self: &Arc<Self>, project: &str, text: &str) -> bool {
        let Some(id) = self.operator_session_id(project).await else { return false };
        matches!(self.told_by_fleet(&id, text, Notice::Operator).await, Ok(true))
    }

    /// Tell the Operators whose project holds `arrival`'s repository of it, once each. **`false` where none runs, or none could
    /// be told**, and the caller goes on as it did with the Operator off.
    pub(crate) async fn arrival_told(self: &Arc<Self>, arrival: &Arrival) -> bool {
        self.arrival_handed(arrival).await != Handed::NotRunning
    }

    /// [`arrival_told`](Self::arrival_told), saying whether a session was told now or all had been before. **An Operator hears
    /// an arrival of a repository its project holds and of no other**; a repository two projects hold tells both, and says
    /// it is shared with the other.
    pub(crate) async fn arrival_handed(self: &Arc<Self>, arrival: &Arrival) -> Handed {
        let held = self.projects_held();
        // An arrival of a project alone, such as a pending feature, is that project's and no other's.
        let holding: Vec<(String, String)> = match arrival.project.as_str() {
            "" => held.holding(&ManifestId::carried(&arrival.manifest_id).to_domain()),
            alone => held.running().filter(|one| one.id().as_str() == alone).collect(),
        }
        .into_iter()
        .map(|one| (one.id().as_str().to_string(), one.name().as_str().to_string()))
        .collect();
        let mut handed = Handed::NotRunning;
        for (project, _) in &holding {
            if self.operator_session_id(project).await.is_none() {
                continue;
            }
            handed = handed.max(self.arrival_handed_to(project, &holding, arrival).await);
        }
        handed
    }

    /// `arrival`, told to `project`'s Operator once. `holding` is every project that holds the repository, as id and name.
    async fn arrival_handed_to(self: &Arc<Self>, project: &str, holding: &[(String, String)], arrival: &Arrival) -> Handed {
        let now = self.now();
        let key = format!("{project}:{}", arrival.id);
        {
            let mut store = self.store().lock().await;
            // What an earlier Fleet told its one Operator is told already.
            if store.linked_was_told("operator", TOLD, &arrival.id).unwrap_or(false) || !store.keep_linked_told("operator", TOLD, &key, &now).unwrap_or(false) {
                return Handed::Before;
            }
        }
        let repository = self.board_repository(&arrival.manifest_id).map(|name| format!(" in {name}")).unwrap_or_default();
        let others: Vec<&str> = holding.iter().filter(|(one, _)| one != project).map(|(_, name)| name.as_str()).collect();
        let shared = match others.is_empty() {
            true => String::new(),
            false => format!("\n\nThis repository is shared with {}: {} hears it too.", if others.len() == 1 { "the project" } else { "the projects" }, others.join(", ")),
        };
        let text = format!(
            "{}{repository}: {}\n\n{}{shared}\n\nSettle it with your own tools, or put a card in front of him where it is his to decide.",
            arrival.arrived.said(),
            arrival.subject,
            arrival.words
        );
        if !self.operator_told(project, &text).await {
            return Handed::NotRunning;
        }
        let kind = if matches!(arrival.arrived, Arrived::Conflict | Arrived::Held) { OperatorFeedKind::Asked } else { OperatorFeedKind::Told };
        let session = Some(arrival.session_id.as_str()).filter(|id| !id.is_empty());
        self.feed_kept(Some(project), kind, &arrival.subject, session, Some(arrival.arrived.said())).await;
        Handed::Now
    }

    /// Each send he made for `project` while its Operator ran no session, told to session `id` in the order he sent them. A
    /// send is kept with the project it was routed to, so no Operator is told another project's words. One the session was not
    /// told stays kept for the next start or tick.
    async fn held_sends_delivered(self: &Arc<Self>, project: &str, id: &str) {
        let held = match self.store().lock().await.operator_held_sends() {
            Ok(held) => held,
            Err(why) => return self.operator_warned("the sends kept for the Operator were not read", format!("{why:?}")),
        };
        for (n, kept) in held {
            let talk = match ipc::decode::<ipc::operator_talk::TalkToOperator>("TalkToOperator", kept.as_bytes()) {
                Ok(talk) => talk,
                Err(why) => {
                    self.operator_warned("a send kept for the Operator did not read", format!("{why}"));
                    let _ = self.store().lock().await.deliver_operator_held_send(n, self.now().as_str());
                    continue;
                }
            };
            if talk.project.as_deref() != Some(project) {
                continue;
            }
            if self.operator_session_id(project).await.as_deref() != Some(id) {
                return;
            }
            if let Err(why) = self.talk_delivered(project, talk).await {
                return self.operator_warned("a send kept for the Operator was not told to it", format!("{why:?}"));
            }
            if let Err(why) = self.store().lock().await.deliver_operator_held_send(n, self.now().as_str()) {
                self.operator_warned("a send told to the Operator was not marked told", format!("{why:?}"));
            }
        }
    }

    /// The person's words to `project`'s Operator session, as he wrote them, with the files and tags he sent. A refusal
    /// says it is not running.
    pub(crate) async fn operator_told_as_him(self: &Arc<Self>, project: &str, text: &str, attachments: Vec<SessionUpload>, mentions: Vec<SessionTag>) -> Result<(), Refusal> {
        let Some(id) = self.operator_session_id(project).await else {
            return Err(self.hosted_refusal(NOT_RUNNING, "the Operator's session is not running, so nothing here reaches it"));
        };
        let sent = SendSessionMessage { session_id: SessionId::carried(&id), text: text.to_string(), attachments, mentions, sketches: Vec::new() };
        self.message_from(sent, SessionVoice::You).await
    }
}

/// How the session came to hear of an arrival.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Handed {
    /// No session runs, or it could not be told.
    NotRunning,
    /// It had been told already.
    Before,
    /// It was told now.
    Now,
}

/// What `keep_linked_told` files an arrival told to the Operator's session under.
pub(super) const TOLD: &str = "told";

/// The Operator's session is not running.
pub(crate) const NOT_RUNNING: &str = "fleet.operator_session_not_running";

//! What Fleet saw happen, written as it happens and read back as a window —
//! `get_fleet_timeline` and `timeline.appended` in `crates/ipc/operations/`.
//! `docs/practices/protocol.md`, *The Fleet's timeline*, has the reasoning.
//!
//! ```text
//!  Broadcaster.tap ── events::Hearing ──┐   a Session row
//!  merge_lines::keep_reading ───────────┼─▶ Timeline::note ─▶ queue ─▶ writer ─▶ store
//!  the answering commands, the slot ────┘                                   └▶ timeline.appended
//! ```
//!
//! **A tap and not a subscription**, which drops its oldest when it falls behind. The tap runs
//! inside `publish`, so it only turns the event into a [`Fact`] and queues it; one writer takes
//! the store lock, so entries are numbered in the order they were queued. `at` is stamped by
//! whoever saw the thing happen, and the window is cut by it and never by the number.

mod events;
mod merge;
mod tools;

use std::sync::{Arc, Mutex};

use ipc::{TimelineAsk, TimelineKind, TimelineSubject, TimelineTone};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::clock::Clock;

pub(crate) use merge::Between;
pub(crate) use tools::{tool_of, Act, Doer, Seen};

/// What it happened to, before the wire spells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Subject {
    Session(String),
    Pull(u64, Option<String>),
    Main,
}

/// One thing that happened, as the code that saw it states it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fact {
    at: String,
    kind: TimelineKind,
    subject: Subject,
    title: Option<String>,
    tone: TimelineTone,
    says: String,
    detail: Option<String>,
    items: Vec<String>,
    service: Option<(String, String)>,
    manifest: Option<String>,
    live: bool,
    over: bool,
}

impl Fact {
    pub(crate) fn new(
        at: impl Into<String>,
        kind: TimelineKind,
        subject: Subject,
        tone: TimelineTone,
        says: impl Into<String>,
    ) -> Fact {
        Fact {
            at: at.into(),
            kind,
            subject,
            title: None,
            tone,
            says: says.into(),
            detail: None,
            items: Vec::new(),
            service: None,
            manifest: None,
            live: false,
            over: false,
        }
    }

    pub(crate) fn titled(mut self, title: Option<String>) -> Fact {
        self.title = title.filter(|title| !title.trim().is_empty());
        self
    }

    pub(crate) fn detail(mut self, detail: Option<String>) -> Fact {
        self.detail = detail.map(|detail| bare(&detail, DETAIL)).filter(|detail| !detail.is_empty());
        self
    }

    /// What a burst counted, newest first. **The caller has already cut each to a line** and the list to
    /// [`ipc::MOST_TIMELINE_ITEMS`], and the first is the `detail`.
    pub(crate) fn items(mut self, items: Vec<String>) -> Fact {
        self.items = items;
        self
    }

    /// The service a `session.called` entry went to, and what it is called; its line is the `detail`.
    pub(crate) fn calling(mut self, service: String, name: String) -> Fact {
        self.service = Some((service, name));
        self
    }

    pub(crate) fn in_repository(mut self, manifest: Option<String>) -> Fact {
        self.manifest = manifest;
        self
    }

    /// The subject is still working after this.
    pub(crate) fn live(mut self) -> Fact {
        self.live = true;
        self
    }

    /// The subject is over after this.
    pub(crate) fn over(mut self) -> Fact {
        self.over = true;
        self
    }

    #[cfg(test)]
    pub(crate) fn kind_name(&self) -> String {
        self.kind.as_wire()
    }

    #[cfg(test)]
    pub(crate) fn says(&self) -> &str {
        &self.says
    }

    #[cfg(test)]
    pub(crate) fn at(&self) -> &str {
        &self.at
    }

    #[cfg(test)]
    pub(crate) fn is_over(&self) -> bool {
        self.over
    }

    #[cfg(test)]
    pub(crate) fn detail_text(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn call_service(&self) -> Option<&str> {
        self.service.as_ref().map(|(service, _)| service.as_str())
    }

    #[cfg(test)]
    pub(crate) fn item_texts(&self) -> &[String] {
        &self.items
    }

    fn entry(self) -> store::TimelineEntry {
        let (subject_kind, subject_id, subject_url) = match self.subject {
            Subject::Session(id) => ("session", id, None),
            Subject::Pull(number, url) => ("pull_request", number.to_string(), url),
            Subject::Main => ("main", String::new(), None),
        };
        store::TimelineEntry {
            at: self.at,
            kind: self.kind.as_wire(),
            subject_kind: subject_kind.to_string(),
            subject_id,
            subject_url,
            title: self.title,
            repo: self.manifest,
            tone: self.tone.as_wire(),
            says: bare(&self.says, SAYS),
            detail: self.detail,
            items: self.items,
            service: self.service.as_ref().map(|(service, _)| service.clone()),
            service_name: self.service.map(|(_, name)| name),
            live: self.live,
            over: self.over,
        }
    }
}

/// How long a fact reads, in characters. A question or a command is cut here.
const SAYS: usize = 90;
const DETAIL: usize = 120;

/// One line: whitespace collapsed, cut at `most` characters with an ellipsis.
pub(crate) fn bare(text: &str, most: usize) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= most {
        return one;
    }
    let cut: String = one.chars().take(most.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

enum Msg {
    Kept(Box<store::TimelineEntry>),
    /// Answered once everything queued before it has been written.
    Settled(oneshot::Sender<()>),
}

/// The queue entries wait in, and the clock the ones with no instant of their own are stamped by.
///
/// **Minted with the Fleet**: nothing outside it holds one before the Fleet
/// does. Cheap to clone; every clone queues into the same writer.
#[derive(Clone)]
pub struct Timeline {
    queue: UnboundedSender<Msg>,
    /// Taken once, by [`keep_timeline`]. Until then entries wait, unwritten.
    unwritten: Arc<Mutex<Option<UnboundedReceiver<Msg>>>>,
    clock: Arc<dyn Clock>,
    /// Reads, edits and commands counted, and not yet entered.
    bursts: Arc<Mutex<tools::Bursts>>,
}

impl Timeline {
    pub(crate) fn new(clock: Arc<dyn Clock>) -> Timeline {
        let (queue, unwritten) = unbounded_channel();
        Timeline {
            queue,
            unwritten: Arc::new(Mutex::new(Some(unwritten))),
            clock,
            bursts: Arc::default(),
        }
    }

    /// Queue one fact. Never blocks and never fails: a timeline that cannot be
    /// written is not a reason for the act it describes to fail.
    pub(crate) fn note(&self, fact: Fact) {
        let _ = self.queue.send(Msg::Kept(Box::new(fact.entry())));
    }

    /// Fleet's clock now, for a fact nothing else dated.
    pub(crate) fn now(&self) -> String {
        self.clock.now().as_str().to_string()
    }

    /// Wait until everything queued so far is written. **For a test or a
    /// shutdown**; nothing on the way to a client waits on it.
    pub async fn settled(&self) {
        let (done, wait) = oneshot::channel();
        if self.queue.send(Msg::Settled(done)).is_ok() {
            let _ = wait.await;
        }
    }
}

/// Start entering what happens: tap the event stream, and write what is queued.
///
/// **Started before commands are let through**, so the first Job to start is
/// in it. Answers `None` where it was started already.
pub fn keep_timeline<H, V, W>(fleet: Arc<crate::Fleet<H, V, W>>) -> Option<JoinHandle<()>>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let timeline = fleet.timeline().clone();
    let mut unwritten = timeline.unwritten.lock().ok()?.take()?;
    let hearing = events::Hearing::new(timeline.clone());
    fleet.events().tap(move |event| hearing.hear(event));
    Some(tokio::spawn(async move {
        let mut tick = tokio::time::interval(tools::TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                msg = unwritten.recv() => match msg {
                    Some(Msg::Kept(entry)) => fleet.timeline_written(*entry).await,
                    Some(Msg::Settled(done)) => {
                        let _ = done.send(());
                    }
                    None => break,
                },
                _ = tick.tick() => timeline.flush_due(),
            }
        }
    }))
}

impl<H, V, W> crate::Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Keep one entry, then tell the clients it exists. **A store that will not
    /// take it loses the entry**, which is the whole of what a failed write to
    /// a record of what happened can honestly be.
    async fn timeline_written(&self, entry: store::TimelineEntry) {
        let kept = self.store().lock().await.append_timeline(&entry);
        match kept {
            Ok(seq) => {
                let road = road_stop(&entry);
                if let Some(event) = wire(&store::KeptTimeline { seq, entry }) {
                    self.publish(ipc::Event::TimelineAppended(ipc::TimelineAppended { event }));
                }
                if let Some(holders) = road {
                    self.road_moved(holders).await;
                }
            }
            Err(_) => {}
        }
    }

    /// A stop of a road was just entered, and a Session's build record reads its stops off the timeline, so
    /// the Sessions it is the road of are published again: the row published when the act happened could not
    /// have held an entry that was still queued.
    async fn road_moved(&self, holders: RoadOf) {
        let sessions = match holders {
            RoadOf::Session(id) => vec![id],
            RoadOf::Pull(number, manifest) => self
                .store()
                .lock()
                .await
                .attachments_at("pr", &number, Some(&manifest))
                .unwrap_or_default()
                .into_iter()
                .filter(|one| one.holder.kind == store::HolderKind::Session)
                .map(|one| one.holder.id)
                .collect(),
        };
        for id in sessions {
            let _ = self.published_hosted(&id).await;
        }
    }

    /// `get_fleet_timeline`: the newest entries inside the window, oldest first.
    pub(crate) async fn fleet_timeline(
        &self,
        ask: TimelineAsk,
    ) -> Result<ipc::FleetTimeline, api::Refusal> {
        let limit = ask
            .limit
            .unwrap_or(ipc::DEFAULT_TIMELINE_LIMIT)
            .clamp(1, ipc::MOST_TIMELINE_LIMIT);
        let window = self
            .store()
            .lock()
            .await
            .timeline_between(
                ask.since.as_ref().map(ipc::Instant::as_str),
                ask.until.as_ref().map(ipc::Instant::as_str),
                limit,
            )
            .map_err(|why| self.ledger_fault(why))?;
        Ok(ipc::FleetTimeline {
            events: window.events.iter().filter_map(wire).collect(),
            cut: window.cut,
        })
    }

    /// A person's answer to a Session, entered under the Session it settled.
    pub(crate) fn session_answered(&self, record: &ipc::SessionRecord, says: String) {
        self.timeline().note(
            Fact::new(
                self.timeline().now(),
                TimelineKind::SessionAnswered,
                Subject::Session(record.id.as_str().to_string()),
                TimelineTone::Run,
                says,
            )
            .titled(record.title.clone())
            .in_repository(record.manifest_id.as_ref().map(|id| id.as_str().to_string()))
            .live(),
        );
    }

    /// A Session's agent started another Session, or forked its own: `spawned` is the new one's title.
    pub(crate) fn session_spawned(
        &self,
        id: &str,
        (title, manifest): (Option<String>, Option<String>),
        spawned: &str,
        fork: bool,
    ) {
        let says = if fork { "Forked a job" } else { "Started a job" };
        self.timeline().note(
            Fact::new(self.timeline().now(), TimelineKind::SessionSpawned, Subject::Session(id.to_string()), TimelineTone::Run, says)
                .titled(title)
                .detail(Some(spawned.to_string()))
                .in_repository(manifest)
                .live(),
        );
    }

    /// A Session's turn ended, `began` to `ended`, both by Fleet's clock. **It is not live**: the turn is
    /// over, and whether the Session is moving, replied, done or stuck is what Fleet derives from the
    /// turn (`SessionRecord::standing`), never what an event's flag says. **Nothing is entered
    /// without a start**: a turn reattached after a restart has none, and a length guessed is not a
    /// fact.
    pub(crate) fn session_turn(
        &self,
        id: &str,
        (title, manifest): (Option<String>, Option<String>),
        began: Option<&str>,
        ended: &str,
    ) {
        self.timeline().close_bursts_of(&Doer::session(id, None, None));
        let took = |at: &str| core_model::Timestamp::from_rfc3339(at).epoch_millis();
        let (Some(began), Some(ended_ms)) = (began.and_then(took), took(ended)) else {
            return;
        };
        self.timeline().note(
            Fact::new(
                ended,
                TimelineKind::SessionTurn,
                Subject::Session(id.to_string()),
                TimelineTone::Idle,
                format!("Turn took {}", lasted((ended_ms - began).max(0))),
            )
            .titled(title)
            .in_repository(manifest),
        );
    }

    /// A Session was given a pool slot, or gave it back.
    pub(crate) fn session_slot(
        &self,
        id: &str,
        title: Option<String>,
        manifest: Option<String>,
        slot: u32,
        leased: bool,
    ) {
        self.worktrees_memory().announced();
        self.timeline().note(slot_fact(
            self.timeline().now(),
            Subject::Session(id.to_string()),
            title,
            manifest,
            slot,
            leased,
        ));
    }
}

fn slot_fact(
    at: String,
    subject: Subject,
    title: Option<String>,
    manifest: Option<String>,
    slot: u32,
    leased: bool,
) -> Fact {
    let (kind, says) = if leased {
        (TimelineKind::SlotLeased, format!("Slot {slot} leased"))
    } else {
        (TimelineKind::SlotReleased, format!("Slot {slot} given back"))
    };
    Fact::new(at, kind, subject, TimelineTone::Idle, says)
        .titled(title)
        .detail(Some(format!("slot-{slot}")))
        .in_repository(manifest)
}

/// A length of time in milliseconds, as a person reads one: `42s`, `2m 14s`, `1h 5m`.
pub(crate) fn lasted(millis: i64) -> String {
    let seconds = millis / 1000;
    match seconds {
        0 => "under 1s".to_string(),
        1..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m {}s", seconds / 60, seconds % 60),
        _ => format!("{}h {}m", seconds / 3600, seconds % 3600 / 60),
    }
}

/// What a person's answer to a Session's permission card reads as.
pub(crate) fn card_answer_says(answer: ipc::HelmCallAnswer) -> String {
    match answer {
        ipc::HelmCallAnswer::AllowOnce => "Answered: allow",
        ipc::HelmCallAnswer::AllowAndRemember => "Answered: always allow",
        ipc::HelmCallAnswer::Refuse => "Answered: refuse",
        ipc::HelmCallAnswer::AlwaysAllow => "Answered: always allow the folder",
        ipc::HelmCallAnswer::AlwaysBlock => "Answered: always block the folder",
        ipc::HelmCallAnswer::AllowForSession => "Answered: allow the folder for this job",
    }
    .to_string()
}

/// Whose road an entry is a stop of.
enum RoadOf {
    Session(String),
    /// A pull request's number and its repository.
    Pull(String, String),
}

/// The kinds a Session's build record reads its road from, and whose road each is.
fn road_stop(entry: &store::TimelineEntry) -> Option<RoadOf> {
    match (entry.kind.as_str(), entry.subject_kind.as_str()) {
        ("session.walked" | "session.committed" | "session.pushed", "session") => {
            Some(RoadOf::Session(entry.subject_id.clone()))
        }
        ("pr.opened" | "pr.checks" | "pr.queued" | "pr.merged", "pull_request") => {
            Some(RoadOf::Pull(entry.subject_id.clone(), entry.repo.clone()?))
        }
        _ => None,
    }
}

/// A kept entry as the wire spells it. `None` for a row this build cannot spell
/// — a kind a newer Fleet wrote — which is left out rather than guessed at.
pub(crate) fn wire(kept: &store::KeptTimeline) -> Option<ipc::TimelineEvent> {
    let entry = &kept.entry;
    let subject = match entry.subject_kind.as_str() {
        "session" => TimelineSubject::Session { id: ipc::SessionId::carried(&entry.subject_id) },
        "pull_request" => TimelineSubject::PullRequest {
            number: entry.subject_id.parse().ok()?,
            url: entry.subject_url.clone(),
        },
        "main" => TimelineSubject::Main,
        _ => return None,
    };
    Some(ipc::TimelineEvent {
        id: u64::try_from(kept.seq).ok()?,
        at: ipc::Instant::carried(&entry.at),
        kind: TimelineKind::from_wire(&entry.kind)?,
        subject,
        title: entry.title.clone(),
        tone: TimelineTone::from_wire(&entry.tone)?,
        says: entry.says.clone(),
        detail: entry.detail.clone(),
        items: entry.items.clone(),
        call: entry.service.as_ref().map(|service| ipc::ServiceCall {
            service: service.clone(),
            name: entry.service_name.clone().unwrap_or_else(|| service.clone()),
            line: entry.detail.clone().unwrap_or_default(),
        }),
        manifest_id: entry.repo.as_deref().map(ipc::ManifestId::carried),
        live: entry.live,
        over: entry.over,
    })
}

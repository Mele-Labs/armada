//! The claim, written before the code: **a Session's agent starts or forks a Session, and it waits
//! for the owner.** The new Session is listed at once with no process, held on its start card; the
//! agent's first message is the card's, off the thread, and reads as a waiting item of its own; Start
//! sends it, his edit of it where he made one, and settles the item; and the caller is placed by its
//! connection, never named. `docs/concepts/session.md`, *A Session an agent
//! started*. The stand-in process is `session_host`'s.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    CloseSession, ForkThisSession, HandToSession, LinkedEvent, ManifestId, PlanGroupInput, PlanTaskInput, RecordPlan,
    RenameSession, SendSessionMessage, SessionFact, SessionHeld, SessionId, SessionRecord, SessionReport, SessionRow, SessionState,
    SessionVoice, SetWaitingFor, StartAnotherSession, StartSession, WaitingActKind, WaitingInput, WaitingSource,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::peer::Placing;
use crate::tests::session_host::{eventually, Shared, StandIn};
use crate::tests::tmp::TempDir;

pub(super) type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const FIRST: &str = "Read crates/reader and say why it drops lines.";

pub(super) struct Rig {
    pub(super) fleet: Arc<Hosted>,
    pub(super) stand_in: Arc<StandIn>,
    pub(super) manifest: ManifestId,
    home: TempDir,
}

pub(super) fn rig(peers: Option<Arc<Placing>>) -> Rig {
    rigged(peers, None, None)
}

/// A rig whose proposer is `judge`, which the test keeps to count the calls it was put.
fn rig_reading(judge: &Arc<FakeJudge>) -> Rig {
    rigged(None, Some(Arc::clone(judge)), None)
}

/// A rig with the `rail` workflow and the rail watching. The proposer reads the first brief (the agent's
/// own) as no workflow and every later one as `rail`, as a Job an agent starts is read from its brief.
pub(super) fn rig_on_the_rail() -> Rig {
    rig_ruled(super::session_rail::workflow(), "rail")
}

fn rig_ruled(workflow: config::ResolvedWorkflow, id: &str) -> Rig {
    let judge = Arc::new(FakeJudge::answering_in_turn(&[
        "workflow: none\ntitle: The agent",
        &format!("workflow: {id}\ntitle: Why the reader drops lines"),
    ]));
    let rig = rigged(None, Some(judge), Some(workflow));
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    rig
}

pub(super) fn rigged(peers: Option<Arc<Placing>>, judge: Option<Arc<FakeJudge>>, workflow: Option<config::ResolvedWorkflow>) -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    if let Some(peers) = peers {
        fittings.peers = peers;
    }
    if let Some(judge) = judge {
        fittings.judge = judge;
    }
    if let Some(workflow) = workflow {
        fittings.starting().workflows = crate::tests::daemon::one(workflow);
    }
    let fleet = Fleet::assembled(fittings)
        .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), Duration::from_secs(600));
    let manifest = ManifestId::carried(fleet.repositories().first().expect("a served repository").manifest().id().as_str());
    Rig { fleet: Arc::new(fleet), stand_in, manifest, home }
}

pub(super) fn caller() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50999".parse().unwrap()))
}

pub(super) fn begins(rig: &Rig, title: &str, message: &str) -> StartAnotherSession {
    StartAnotherSession { manifest_id: rig.manifest.clone(), title: title.into(), message: message.into(), group: None, after: None }
}

/// A transcript line the agent CLI wrote: `kind` is `user` or `assistant`.
fn line(kind: &str, uuid: &str, text: &str) -> String {
    let origin = if kind == "user" { r#""origin":{"kind":"human"},"# } else { "" };
    format!(
        r#"{{"type":"{kind}","uuid":"{uuid}","timestamp":"2026-10-12T08:48:10.000Z",{origin}"message":{{"role":"x","content":[{{"type":"text","text":"{text}"}}]}}}}"#
    )
}

impl Rig {
    pub(super) async fn record(&self, id: &SessionId) -> SessionRecord {
        Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read").session
    }

    pub(super) async fn rows(&self, id: &SessionId) -> Vec<SessionRow> {
        Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read").rows
    }

    /// Send `text` and wait out the reading it begins, if it is the first message of a Session not yet read.
    pub(super) async fn send(&self, id: &SessionId, text: &str) {
        let runtime = self.fleet.hosts().of(id.as_str());
        let fresh = !runtime.state().proposal.asked;
        self.send_unsettled(id, text).await;
        if fresh {
            eventually(|| async { !runtime.state().proposal.holds_back() }).await;
        }
    }

    /// Start pressed on a held Job, with the box as he left it where `message` says.
    pub(super) async fn begin(&self, id: &SessionId, message: Option<&str>) -> SessionRecord {
        let begin = ipc::BeginSession {
            session_id: id.clone(),
            delivery: Some(ipc::DeliveryChoice::OpenPr),
            message: message.map(str::to_string),
        };
        Arc::clone(&self.fleet).begin_session(begin).await.expect("begun")
    }

    /// The first message the held Job's card shows, if it is held.
    pub(super) async fn card_message(&self, id: &SessionId) -> Option<String> {
        self.record(id).await.hosted.and_then(|hosted| hosted.start).and_then(|start| start.message)
    }

    pub(super) async fn send_unsettled(&self, id: &SessionId, text: &str) {
        Arc::clone(&self.fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: text.into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("taken");
    }

    /// A hosted session with its process up: the agent that makes the calls.
    pub(super) async fn an_agent(&self) -> SessionId {
        let id = self
            .fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: Some("The agent".into()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
            })
            .await
            .expect("started")
            .id;
        self.send(&id, "go on").await;
        self.stand_in.init(0);
        id
    }

    async fn listed(&self) -> Vec<SessionRecord> {
        Arc::clone(&self.fleet).list_sessions(None, None, None).await.expect("listed").sessions
    }

    /// The ids of the agent's messages in the session's thread, once there are `count`.
    async fn agent_rows(&self, id: &SessionId, count: usize) -> Vec<String> {
        for _ in 0..500 {
            let ids: Vec<String> = self
                .rows(id)
                .await
                .iter()
                .filter_map(|row| match row {
                    SessionRow::Message { id, from: SessionVoice::Agent, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect();
            if ids.len() == count {
                return ids;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the agent's {count} messages did not reach the thread within five seconds");
    }

    /// The transcript the CLI keeps for session `id`, from these lines.
    fn transcript(&self, id: &SessionId, lines: &[String]) {
        let dir = self.home.path().join(adapters::SESSIONS).join("-somewhere");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{}.jsonl", id.as_str())), lines.join("\n") + "\n").unwrap();
    }

    /// An agent with two turns of its own said and written, ready to be forked.
    pub(super) async fn an_agent_that_has_spoken(&self) -> (SessionId, Vec<String>) {
        let id = self.an_agent().await;
        self.stand_in.finishes(0, "Looking now.");
        self.send(&id, "and then").await;
        self.stand_in.finishes(0, "Fixed.");
        let agent = self.agent_rows(&id, 2).await;
        self.transcript(
            &id,
            &[
                line("user", "u0", "go on"),
                line("assistant", "a1", "Looking now."),
                line("user", "u1", "and then"),
                line("assistant", "a2", "Fixed."),
            ],
        );
        (id, agent)
    }
}

pub(super) fn kind<'a>(record: &'a SessionRecord, kind: &str) -> Option<&'a ipc::Attachment> {
    record.attachments.iter().find(|one| one.kind == kind)
}

fn code(refused: Result<SessionHeld, Refusal>) -> String {
    format!("{:?}", refused.expect_err("refused"))
}

#[tokio::test]
async fn a_started_session_is_listed_at_once_holding_the_message_and_starts_no_process() {
    let rig = rig(None);
    let agent = rig.an_agent().await;
    let processes = rig.stand_in.starts().len();

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    assert_eq!((held.title.as_str(), held.fork), ("Check the reader", false));

    let listed = rig.listed().await;
    let record = listed.iter().find(|one| one.id == held.session_id).expect("it is listed at once");
    assert_eq!(record.title.as_deref(), Some("Check the reader"));
    assert_eq!(record.state, SessionState::Live);
    assert!(!record.hosted.as_ref().expect("Bridge hosts it").running, "no process");
    assert_eq!(rig.stand_in.starts().len(), processes, "nothing was started");
    assert!(rig.rows(&held.session_id).await.is_empty(), "no turn, no row");

    assert_eq!(rig.card_message(&held.session_id).await.as_deref(), Some(FIRST), "held on its card with the message");
    assert!(kind(record, "unsent").is_none(), "no attachment stands for it");

    let [item] = &record.waiting_for[..] else { panic!("one item waits: {:?}", record.waiting_for) };
    assert_eq!(item.id, format!("unsent:{}", held.session_id.as_str()));
    assert_eq!((item.text.as_str(), item.source), ("First message", WaitingSource::FirstMessage));
    let act = item.act.as_ref().expect("an act of its own");
    assert_eq!((act.kind, act.target.as_str()), (WaitingActKind::Message, held.session_id.as_str()));
    assert!(item.options.is_empty(), "no reply is drawn for it");

    assert_eq!(kind(record, "started_by").expect("it names its starter").target, agent.as_str());
}

#[tokio::test]
async fn the_callers_thread_and_ledger_name_the_session_it_started() {
    let rig = rig(None);
    let agent = rig.an_agent().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");

    let rows = rig.rows(&agent).await;
    let started = rows
        .iter()
        .find_map(|row| match row {
            SessionRow::Started { session, fork, .. } => Some((session.clone(), *fork)),
            _ => None,
        })
        .expect("the thread says it");
    assert_eq!((started.0.id, started.0.title.as_str(), started.1), (held.session_id.clone(), "Check the reader", false));

    let record = rig.record(&agent).await;
    assert_eq!(kind(&record, "started_to").expect("the caller names it").target, held.session_id.as_str());
    assert_eq!(kind(&record, "started_to").unwrap().state, ipc::AttachmentState::Spent, "a fact that stays");
}

#[tokio::test]
async fn start_sends_his_edit_of_it_starts_the_first_turn_and_settles_the_item() {
    let rig = rig(None);
    rig.an_agent().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    let before = rig.stand_in.starts().len();

    // He may edit it first: what he sends is the box, not what the agent wrote.
    rig.begin(&held.session_id, Some("Read crates/reader and say why it drops blank lines.")).await;

    // The brief was read first, so the turn starts when the reading ends.
    eventually(|| async { rig.stand_in.starts().len() == before + 1 }).await;
    let record = rig.record(&held.session_id).await;
    assert!(record.hosted.as_ref().and_then(|hosted| hosted.start.as_ref()).is_none(), "begun");
    assert!(record.waiting_for.is_empty(), "nothing waits: {:?}", record.waiting_for);
    let sent: Vec<String> = rig
        .rows(&held.session_id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(sent, ["Read crates/reader and say why it drops blank lines."]);
}

#[tokio::test]
async fn ending_it_unsent_discards_the_message_and_the_item() {
    let rig = rig(None);
    rig.an_agent().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    let ended = Arc::clone(&rig.fleet)
        .close_session(CloseSession { session_id: held.session_id.clone() })
        .await
        .expect("closed");
    assert!(ended.waiting_for.is_empty());
    assert!(kind(&ended, "unsent").is_none());
    assert!(rig.stand_in.starts().len() == 1, "and it never ran");
}

#[tokio::test]
async fn a_fork_at_a_message_of_the_callers_own_live_session_waits_the_same_way() {
    let rig = rig(None);
    let (agent, rows) = rig.an_agent_that_has_spoken().await;
    let before = rig.stand_in.starts().len();

    let held = rig
        .fleet
        .fork_this_session(
            caller(),
            ForkThisSession { title: "The other way".into(), message: "Do it with a map.".into(), at: Some(rows[0].clone()) },
        )
        .await
        .expect("a live session forks itself at a message");
    assert!(held.fork);
    assert_eq!(rig.stand_in.starts().len(), before, "no process yet");

    let record = rig.record(&held.session_id).await;
    let from = kind(&record, "forked_from").expect("the fork links back");
    assert_eq!(from.target, agent.as_str());
    assert_eq!(from.detail.get("at").map(String::as_str), Some(rows[0].as_str()));
    assert!(kind(&record, "started_by").is_none(), "a fork's link is the fork's own");
    assert_eq!(rig.card_message(&held.session_id).await.as_deref(), Some("Do it with a map."), "held on its card");
    assert_eq!(record.waiting_for[0].source, WaitingSource::FirstMessage);
    assert!(kind(&rig.record(&agent).await, "forked_to").is_some());
    assert!(
        rig.rows(&agent).await.iter().any(|row| matches!(row, SessionRow::Started { fork: true, .. })),
        "the callout in the caller's thread"
    );

    rig.begin(&held.session_id, None).await;
    eventually(|| async { rig.stand_in.starts().len() == before + 1 }).await;
    let starts = rig.stand_in.starts();
    assert_eq!(starts[before].forking.as_deref(), Some(agent.as_str()));
    assert_eq!(starts[before].fork_cut.as_deref(), Some("a1"), "the copy ends at that line");
}

#[tokio::test]
async fn a_fork_with_no_message_ends_at_the_callers_latest_of_its_own() {
    let rig = rig(None);
    let (agent, rows) = rig.an_agent_that_has_spoken().await;
    let before = rig.stand_in.starts().len();
    let held = rig
        .fleet
        .fork_this_session(caller(), ForkThisSession { title: "Again".into(), message: "Go.".into(), at: None })
        .await
        .expect("forks");
    let from = kind(&rig.record(&held.session_id).await, "forked_from").cloned().expect("link");
    assert_eq!(from.detail.get("at").map(String::as_str), Some(rows[1].as_str()));
    rig.begin(&held.session_id, None).await;
    eventually(|| async { rig.stand_in.starts().len() == before + 1 }).await;
    assert_eq!(rig.stand_in.starts()[before].fork_cut.as_deref(), Some("a2"));
    assert_eq!(rig.stand_in.starts()[before].forking.as_deref(), Some(agent.as_str()));
}

#[tokio::test]
async fn a_message_the_conversation_does_not_hold_is_refused_and_nothing_is_written() {
    let rig = rig(None);
    let (agent, _) = rig.an_agent_that_has_spoken().await;
    let listed = rig.listed().await.len();
    let refused = code(
        rig.fleet
            .fork_this_session(caller(), ForkThisSession { title: "t".into(), message: "m".into(), at: Some("nowhere".into()) })
            .await,
    );
    assert!(refused.contains("fleet.fork_point_unwritten"), "{refused}");
    assert_eq!(rig.listed().await.len(), listed, "no session was left behind");
    assert!(kind(&rig.record(&agent).await, "forked_to").is_none());
}

#[tokio::test]
async fn a_call_that_places_no_hosted_session_is_refused_and_nothing_is_made() {
    // Nobody holds the connection: the call comes from no Session's process.
    let rig = rig(Some(Placing::nothing()));
    rig.an_agent().await;
    let listed = rig.listed().await.len();

    let refused = code(Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await);
    assert!(refused.contains("fleet.agent_start_unplaced"), "{refused}");
    let refused = code(
        rig.fleet
            .fork_this_session(caller(), ForkThisSession { title: "t".into(), message: "m".into(), at: None })
            .await,
    );
    assert!(refused.contains("fleet.agent_start_unplaced"), "{refused}");
    // And a call with no connection at all, which is what a person's request is.
    let refused = code(Arc::clone(&rig.fleet).start_another_session(None, begins(&rig, "Check the reader", FIRST)).await);
    assert!(refused.contains("fleet.agent_start_unplaced"), "{refused}");
    assert_eq!(rig.listed().await.len(), listed);
}

#[tokio::test]
async fn a_blank_title_or_message_is_refused() {
    let rig = rig(None);
    rig.an_agent().await;
    for (title, message) in [("  ", FIRST), ("Check", "  ")] {
        let refused = code(Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, title, message)).await);
        assert!(refused.contains("fleet.agent_start_empty"), "{refused}");
    }
}

// ---- Linked Sessions: a group of the plan handed to a Session the caller follows ----------------------

pub(super) fn two_groups(agent: &SessionId) -> RecordPlan {
    let group = |title: &str, tasks: &[&str]| PlanGroupInput {
        id: None,
        title: title.into(),
        agent: "rust-engineer".into(),
        paths: vec!["crates/store".into()],
        issue: None,
        gives: None,
        lands_in: None,
        own_job: false,
        tasks: tasks.iter().map(|one| PlanTaskInput { id: None, title: (*one).into() }).collect(),
    };
    RecordPlan {
        summary: "You get a plan.".into(),
        groups: vec![
            group("Name the migrations", &["Give each migration a name", "Record the name with the version"]),
            group("Apply them in order", &["Sort by name before applying"]),
        ],
        sketch: None,
        walk: None,
        session_id: Some(agent.clone()),
    }
}

pub(super) fn handing(rig: &Rig, group: &str) -> StartAnotherSession {
    StartAnotherSession { group: Some(group.into()), ..begins(rig, "Ignored", "Mind the order.") }
}

impl Rig {
    /// An agent whose plan holds two groups, with the Fleet telling parents about the Sessions they follow.
    pub(super) async fn a_parent_with_a_plan(&self) -> SessionId {
        crate::linked_following::keep_the_linked(Arc::clone(&self.fleet));
        let agent = self.an_agent().await;
        self.fleet.record_plan(None, two_groups(&agent), false).await.expect("recorded");
        agent
    }

    /// The `linked` rows of the thread, once there are `count`.
    pub(super) async fn linked_rows(&self, id: &SessionId, count: usize) -> Vec<(String, LinkedEvent)> {
        for _ in 0..500 {
            let rows: Vec<(String, LinkedEvent)> = self
                .rows(id)
                .await
                .iter()
                .filter_map(|row| match row {
                    SessionRow::Linked { session, event, .. } => Some((session.id.as_str().to_string(), *event)),
                    _ => None,
                })
                .collect();
            if rows.len() >= count {
                return rows;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the thread did not hold {count} linked rows within five seconds");
    }

    /// How many lines the process of the agent at index 0 was sent that contain `text`.
    pub(super) fn told(&self, text: &str) -> usize {
        self.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(text)).count()
    }

    pub(super) async fn report(&self, child: &SessionId, fact: SessionFact) {
        let harness = self.record(child).await.harness;
        self.fleet.report_session(SessionReport { harness, session_id: child.clone(), fact }).await.expect("reported");
    }
}

#[tokio::test]
async fn a_grouped_start_is_the_groups_own_and_both_ledgers_keep_the_group() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");
    assert_eq!(held.title, "Name the migrations", "the group's title, whatever the call said");

    let child = rig.record(&held.session_id).await;
    let unsent = rig.card_message(&held.session_id).await.expect("the brief is held");
    assert!(unsent.contains("- Give each migration a name\n- Record the name with the version"), "the tasks word for word: {unsent}");
    assert!(unsent.contains("Handoff from The agent"), "the parent's handoff: {unsent}");
    assert!(unsent.trim_end().ends_with("Mind the order."), "the caller's words last: {unsent}");

    let by = kind(&child, "started_by").expect("it names its parent");
    assert_eq!((by.target.as_str(), by.detail.get("group").map(String::as_str)), (agent.as_str(), Some("g1")));
    let parent = rig.record(&agent).await;
    let to = kind(&parent, "started_to").expect("the parent names it");
    assert_eq!((to.target.as_str(), to.detail.get("group").map(String::as_str)), (held.session_id.as_str(), Some("g1")));
    assert!(rig.rows(&agent).await.iter().any(|row| matches!(row, SessionRow::Started { session, .. } if session.id == held.session_id)));
}

#[tokio::test]
async fn a_group_is_handed_once_and_must_be_in_the_plan() {
    let rig = rig(None);
    rig.a_parent_with_a_plan().await;
    Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g2")).await.expect("held");

    let again = code(Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g2")).await);
    assert!(again.contains("fleet.group_handed"), "{again}");
    let missing = code(Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g9")).await);
    assert!(missing.contains("fleet.group_unknown"), "{missing}");
}

#[tokio::test]
async fn the_person_handing_a_group_starts_the_session_and_sends_the_brief() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;

    let held = Arc::clone(&rig.fleet)
        .hand_to_session(HandToSession { session_id: agent.clone(), group: "g1".into() })
        .await
        .expect("handed");
    assert_eq!(held.title, "Name the migrations");

    let child = rig.record(&held.session_id).await;
    assert_eq!(kind(&child, "started_by").unwrap().detail.get("group").map(String::as_str), Some("g1"));
    assert!(child.hosted.as_ref().and_then(|hosted| hosted.start.as_ref()).is_none(), "the press is the first send: not held");
    let rows = rig.rows(&held.session_id).await;
    let sent = rows.iter().find_map(|row| match row {
        SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
        _ => None,
    });
    assert!(sent.is_some_and(|text| text.contains("- Give each migration a name")), "the brief is the first message");
}

#[tokio::test]
async fn the_parent_is_not_told_when_a_linked_session_needs_the_owner_but_still_hears_it_end_and_merge() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");
    let child = held.session_id.as_str().to_string();

    let question = WaitingInput { id: "q1".into(), text: "Sort by name or by version?".into(), act: None, options: Vec::new(), steps: Vec::new() };
    rig.fleet.waiting_for(None, SetWaitingFor { items: vec![question], session_id: Some(held.session_id.clone()) }).await.expect("set");
    // Ended is told after the need, so by the time its row stands the need has been seen and passed over.
    let attached = SessionFact::Attached { attachment: ipc::AttachmentReport { kind: "pr".into(), target: "2412".into(), detail: Default::default() } };
    rig.report(&held.session_id, attached).await;
    rig.report(&held.session_id, SessionFact::Settled { attachment: ipc::AttachmentNamed { kind: "pr".into(), target: "2412".into() }, state: ipc::AttachmentState::Spent }).await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: held.session_id.clone() }).await.expect("closed");

    let rows = rig.linked_rows(&agent, 2).await;
    assert!(rows.contains(&(child.clone(), LinkedEvent::Merged)) && rows.contains(&(child.clone(), LinkedEvent::Done)), "{rows:?}");
    assert_eq!(rows.len(), 2, "no row for the need: {rows:?}");
    assert_eq!(rig.told("needs the owner"), 0, "the parent's agent is not woken with it");
    assert_eq!((rig.told("ended"), rig.told("pull request #2412 merged")), (1, 1));
}

#[tokio::test]
async fn the_parent_is_told_when_a_linked_session_ends() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: held.session_id.clone() }).await.expect("closed");

    assert_eq!(rig.linked_rows(&agent, 1).await, [(held.session_id.as_str().to_string(), LinkedEvent::Done)]);
    assert_eq!(rig.told("ended"), 1);
}

#[tokio::test]
async fn the_parent_is_told_once_when_a_linked_sessions_pull_request_merges() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");
    let attached = SessionFact::Attached { attachment: ipc::AttachmentReport { kind: "pr".into(), target: "2412".into(), detail: Default::default() } };
    rig.report(&held.session_id, attached).await;
    let merged = || SessionFact::Settled { attachment: ipc::AttachmentNamed { kind: "pr".into(), target: "2412".into() }, state: ipc::AttachmentState::Spent };
    rig.report(&held.session_id, merged()).await;

    assert_eq!(rig.linked_rows(&agent, 1).await, [(held.session_id.as_str().to_string(), LinkedEvent::Merged)]);
    // The same merge seen again is not news.
    rig.report(&held.session_id, merged()).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(rig.linked_rows(&agent, 1).await.len(), 1);
    assert_eq!(rig.told("pull request #2412 merged"), 1);
}

/// A `pr` row of `state` as the forge describes it, then settled spent, as the mod tells one.
async fn pull_spent(rig: &Rig, child: &SessionId, state: &str) {
    let detail = [("state".to_string(), state.to_string())].into();
    rig.report(child, SessionFact::Attached { attachment: ipc::AttachmentReport { kind: "pr".into(), target: "3042".into(), detail } }).await;
    rig.report(child, SessionFact::Settled { attachment: ipc::AttachmentNamed { kind: "pr".into(), target: "3042".into() }, state: ipc::AttachmentState::Spent }).await;
}

#[tokio::test]
async fn the_parent_is_not_told_of_a_spent_pull_request_the_forge_reads_open_and_is_told_once_when_it_merges() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");

    // `gh pr merge 3042 --disable-auto` read as a merge: spent, while the forge still says open.
    pull_spent(&rig, &held.session_id, "open").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(rig.told("pull request #3042 merged"), 0, "an open pull request is not merged");

    pull_spent(&rig, &held.session_id, "merged").await;
    assert_eq!(rig.linked_rows(&agent, 1).await, [(held.session_id.as_str().to_string(), LinkedEvent::Merged)]);
    pull_spent(&rig, &held.session_id, "merged").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(rig.told("pull request #3042 merged"), 1);
}

#[tokio::test]
async fn a_merge_kept_off_a_pull_request_the_forge_reads_open_is_forgotten_so_the_real_one_is_told() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");
    pull_spent(&rig, &held.session_id, "open").await;
    // What a Fleet before the fix kept for that row, and a restart that follows the linked again.
    let now = rig.fleet.now();
    rig.fleet.store().lock().await.keep_linked_told(held.session_id.as_str(), "merged", "3042", &now).expect("kept");
    crate::linked_following::keep_the_linked(Arc::clone(&rig.fleet));
    tokio::time::sleep(Duration::from_millis(200)).await;

    pull_spent(&rig, &held.session_id, "merged").await;
    assert_eq!(rig.linked_rows(&agent, 1).await, [(held.session_id.as_str().to_string(), LinkedEvent::Merged)]);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(rig.told("pull request #3042 merged"), 1, "told once, though two follow the linked");
}

const READING: &str = "workflow: none\ntitle: Why the reader drops lines\nsettings: model=another-model, effort=high";

#[tokio::test]
async fn a_session_an_agent_started_is_read_from_its_brief_before_the_person_sends() {
    let judge = Arc::new(FakeJudge::saying(READING).taking(Duration::from_millis(300)));
    let rig = rig_reading(&judge);
    let agent = rig.an_agent().await;
    eventually_read(&rig, &agent).await;
    let before = judge.asked().len();

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    assert!(rig.record(&held.session_id).await.proposal.is_some_and(|one| one.reading), "reading at once, unsent");
    eventually_read(&rig, &held.session_id).await;
    assert_eq!(judge.asked().len(), before + 1);
    assert!(judge.asked().last().is_some_and(|asked| asked.contains(FIRST)), "from the brief the agent gave");
    let record = rig.record(&held.session_id).await;
    assert_eq!(record.hosted.as_ref().expect("hosted").model.as_deref(), Some("another-model"));
    assert!(rig.card_message(&held.session_id).await.is_some(), "and the first message is still the person's to send");

    // Start sends it on the reading, and does not read it again.
    rig.begin(&held.session_id, None).await;
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert_eq!(judge.asked().len(), before + 1);
}

#[tokio::test]
async fn a_fork_an_agent_made_is_not_read() {
    let judge = Arc::new(FakeJudge::saying(READING));
    let rig = rig_reading(&judge);
    let (agent, _) = rig.an_agent_that_has_spoken().await;
    eventually_read(&rig, &agent).await;

    let held = Arc::clone(&rig.fleet)
        .fork_this_session(caller(), ForkThisSession { title: "Another way".into(), message: FIRST.into(), at: None })
        .await
        .expect("forked");
    rig.begin(&held.session_id, None).await;
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    // Its title alone is asked for (`a_fork_an_agent_titled_is_named_when_start_sends_it`), and nothing else is read.
    let record = rig.record(&held.session_id).await;
    assert!(record.proposal.is_none_or(|one| !one.reading), "a fork carries its source's conversation, and is not read");
    assert_eq!(record.hosted.expect("hosted").model, None, "its settings are not the proposer's");
}

const UNTITLED: &str = "workflow: none\nsettings: model=another-model, effort=high";

/// A rig over `judge`, an agent already read, and the Job that agent started, titled "Check the reader".
async fn an_agent_started(judge: &Arc<FakeJudge>) -> (Rig, SessionId) {
    let rig = rig_reading(judge);
    let agent = rig.an_agent().await;
    eventually_read(&rig, &agent).await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    (rig, held.session_id)
}

#[tokio::test]
async fn an_agents_title_is_replaced_by_the_proposers() {
    let (rig, id) = an_agent_started(&Arc::new(FakeJudge::saying(READING))).await;
    eventually_read(&rig, &id).await;
    let record = rig.record(&id).await;
    assert_eq!(record.title.as_deref(), Some("Why the reader drops lines"));
    assert!(record.proposal.is_some_and(|one| one.picked.contains(&ipc::ProposedField::Title)), "listed as Armada's pick");
}

#[tokio::test]
async fn an_agents_title_stays_when_the_answer_gives_none() {
    let (rig, id) = an_agent_started(&Arc::new(FakeJudge::saying(UNTITLED))).await;
    eventually_read(&rig, &id).await;
    let record = rig.record(&id).await;
    assert_eq!(record.title.as_deref(), Some("Check the reader"));
    assert_eq!(record.hosted.as_ref().expect("hosted").model.as_deref(), Some("another-model"), "the rest of the answer lands");
}

#[tokio::test]
async fn a_plan_groups_title_is_replaced_by_the_proposers() {
    let judge = Arc::new(FakeJudge::saying(READING));
    let rig = rig_reading(&judge);
    rig.a_parent_with_a_plan().await;

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");
    assert_eq!(held.title, "Name the migrations", "the group's title is what the card says");
    eventually_read(&rig, &held.session_id).await;
    assert_eq!(rig.record(&held.session_id).await.title.as_deref(), Some("Why the reader drops lines"));
}

#[tokio::test]
async fn a_rename_before_the_answer_keeps_his_title_over_an_agents_hint() {
    let judge = Arc::new(FakeJudge::saying(READING).taking(Duration::from_millis(300)));
    let (rig, id) = an_agent_started(&judge).await;
    assert!(rig.record(&id).await.proposal.is_some_and(|one| one.reading), "the answer is still out");

    Arc::clone(&rig.fleet).rename_session(RenameSession { session_id: id.clone(), title: "Mine".into() }).await.expect("renamed");
    eventually_read(&rig, &id).await;
    let record = rig.record(&id).await;
    assert_eq!(record.title.as_deref(), Some("Mine"));
    assert!(record.proposal.is_none_or(|one| !one.picked.contains(&ipc::ProposedField::Title)));
}

#[tokio::test]
async fn the_question_carries_an_agents_title_as_a_hint() {
    let judge = Arc::new(FakeJudge::saying(READING));
    let (rig, id) = an_agent_started(&judge).await;
    eventually_read(&rig, &id).await;
    let asked = judge.asked().pop().expect("the Job was read");
    let hint = asked.find("The agent that started this Job called it: Check the reader").expect("the hint");
    assert!(hint < asked.find("Answer with nothing but the block below").expect("the format"), "ahead of the format: {asked}");
}

#[tokio::test]
async fn a_fork_an_agent_titled_is_named_when_start_sends_it() {
    let judge = Arc::new(FakeJudge::saying("workflow: none\ntitle: Same fix on a map"));
    let rig = rig_reading(&judge);
    let (agent, _) = rig.an_agent_that_has_spoken().await;
    eventually_read(&rig, &agent).await;
    let before = judge.asked().len();

    let held = Arc::clone(&rig.fleet)
        .fork_this_session(caller(), ForkThisSession { title: "Another way".into(), message: FIRST.into(), at: None })
        .await
        .expect("forked");
    assert_eq!(rig.record(&held.session_id).await.title.as_deref(), Some("Another way"));
    assert_eq!(judge.asked().len(), before, "nothing is asked before Start");

    rig.begin(&held.session_id, None).await;
    eventually(|| async { rig.record(&held.session_id).await.title.as_deref() == Some("Same fix on a map") }).await;
    // Start also wakes the starter with a `started` row, and reading its reply is a call of its own.
    let asked: Vec<String> = judge.asked().into_iter().skip(before).filter(|one| one.contains(FIRST)).collect();
    assert_eq!(asked.len(), 1, "one call, from the words Start sent: {asked:?}");
    assert!(asked[0].contains("called it: Another way"), "with the agent's title as the hint: {}", asked[0]);
}

/// The reading of `id` has landed, if one began.
pub(super) async fn eventually_read(rig: &Rig, id: &SessionId) {
    for _ in 0..500 {
        if rig.record(id).await.proposal.is_none_or(|one| !one.reading) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the reading did not land within five seconds");
}

// ---- A Job on a workflow runs nothing, and is told nothing, until the person sends ----

impl Rig {
    /// The rail has entered a step on the timeline, and a moment more for anything it would send.
    pub(super) async fn rail_has_acted(&self) {
        eventually(|| async {
            self.fleet.timeline().settled().await;
            self.fleet
                .fleet_timeline(ipc::TimelineAsk::default())
                .await
                .expect("a timeline")
                .events
                .iter()
                .any(|one| one.kind.as_wire() == "session.step")
        })
        .await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    /// What was written to the process started `nth`, in the order it was.
    pub(super) fn sent_to(&self, nth: usize) -> Vec<String> {
        self.stand_in.process(nth).sent.lock().unwrap().clone()
    }
}

const STEP_WORDS: &str = "Record the plan before editing.";

#[tokio::test]
async fn a_session_an_agent_started_on_a_workflow_gets_no_turn_and_no_process_until_the_person_sends() {
    let rig = rig_on_the_rail();
    let agent = rig.an_agent().await;
    eventually_read(&rig, &agent).await;
    let processes = rig.stand_in.starts().len();

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    eventually_read(&rig, &held.session_id).await;
    assert_eq!(rig.record(&held.session_id).await.workflow.as_deref(), Some("rail"), "the brief put it on the workflow");
    rig.rail_has_acted().await;

    assert_eq!(rig.stand_in.starts().len(), processes, "the rail starts nothing for a Job that has not run");
    assert!(rig.rows(&held.session_id).await.is_empty(), "and tells it nothing");
    assert!(rig.card_message(&held.session_id).await.is_some(), "its first message is still the person's to send");
}

#[tokio::test]
async fn the_persons_send_of_the_draft_is_one_first_turn_with_the_workflow_the_step_and_the_message() {
    let rig = rig_on_the_rail();
    let agent = rig.an_agent().await;
    eventually_read(&rig, &agent).await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    eventually_read(&rig, &held.session_id).await;
    rig.rail_has_acted().await;

    rig.begin(&held.session_id, None).await;
    eventually(|| async { rig.stand_in.starts().len() == 2 && !rig.sent_to(1).is_empty() }).await;
    // The step went with the turn, so the rail has nothing to add once the process has run.
    rig.rail_has_acted().await;

    let sent = rig.sent_to(1);
    let [first] = &sent[..] else { panic!("one turn, not {}: {sent:?}", sent.len()) };
    let at = |part: &str| first.find(part).unwrap_or_else(|| panic!("{part} is not in the first turn: {first}"));
    assert!(at("follows the Rail workflow") < at(STEP_WORDS) && at(STEP_WORDS) < at(FIRST), "in that order: {first}");
    assert_eq!(first.matches(STEP_WORDS).count(), 1);
}

/// A Job an agent started on the `gate` workflow, whose first step asks how the work is delivered, begun
/// with the person's `message` and the landing `delivery`. Answers the sent turns once the rail has had its say.
async fn sent_on_a_step_that_asks_delivery(message: &str, delivery: ipc::DeliveryChoice) -> (Rig, SessionId, Vec<String>) {
    let rig = rig_ruled(super::delivery_gate::workflow(), "gate");
    let agent = rig.an_agent().await;
    eventually_read(&rig, &agent).await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    eventually_read(&rig, &held.session_id).await;

    let begin = ipc::BeginSession { session_id: held.session_id.clone(), delivery: Some(delivery), message: Some(message.to_string()) };
    Arc::clone(&rig.fleet).begin_session(begin).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 2 && !rig.sent_to(1).is_empty() }).await;
    rig.rail_has_acted().await;
    let sent = rig.sent_to(1);
    (rig, held.session_id, sent)
}

#[tokio::test]
async fn a_first_message_that_names_a_delivery_goes_with_the_steps_instructions_in_one_turn() {
    let message = "Read crates/reader and say why it drops lines, then keep it local.";
    let (rig, id, sent) = sent_on_a_step_that_asks_delivery(message, ipc::DeliveryChoice::KeepLocal).await;

    let [first] = &sent[..] else { panic!("one turn, not {}: {sent:?}", sent.len()) };
    assert!(first.contains(message) && first.contains("follows the Gate workflow"), "{first}");
    assert_eq!(first.matches(STEP_WORDS).count(), 1, "the instructions go with the first turn: {first}");
    assert!(!first.contains("choosing how this work is delivered"), "nothing is asked: {first}");
    assert!(
        rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Delivery { choice: Some(_), named: true, .. })),
        "and the message's answer is kept"
    );
}

#[tokio::test]
async fn a_first_message_that_names_no_delivery_goes_with_the_landing_chosen_on_the_card_and_the_instructions() {
    let (rig, id, sent) = sent_on_a_step_that_asks_delivery(FIRST, ipc::DeliveryChoice::OpenPr).await;

    let [first] = &sent[..] else { panic!("one turn, not {}: {sent:?}", sent.len()) };
    assert!(first.contains(FIRST) && first.contains("the owner chose"), "{first}");
    assert!(!first.contains("choosing how this work is delivered"), "nothing is asked: {first}");
    assert_eq!(first.matches(STEP_WORDS).count(), 1, "the instructions go with the first turn: {first}");
    assert!(
        rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Delivery { choice: Some(ipc::DeliveryChoice::OpenPr), named: false, .. })),
        "the card's answer is kept"
    );
}

// ---- An agent keeps a person's own armada.yml for its own repository only --------------------------------

/// A person's own copy wins over the repository's and can replace a Check, so the agent's door keeps
/// one only for the repository of the Job that calls. Bridge's call carries no connection and is not held to it.
#[tokio::test]
async fn an_agent_keeps_an_armada_yml_in_kit_for_its_own_repository_only() {
    let placed = rig(None);
    placed.an_agent().await;
    let mine = placed.fleet.repositories().first().expect("served").root().to_string();
    let saving = |repository: &str| ipc::SaveKitManifest { repository: repository.into(), text: "checks:\n  test:\n    run: make quick\n".into() };
    let said = |refused: Result<ipc::KitManifestSaved, Refusal>| match refused.expect_err("refused") {
        Refusal::Unacceptable(error) | Refusal::NotFound(error) => (error.code, error.message),
        other => panic!("neither a 422 nor a 404: {other:?}"),
    };

    let saved = placed.fleet.kit_manifest_saved(caller(), saving(&mine)).await.expect("its own repository");
    assert_eq!(saved.repository, mine);

    let (code, message) = said(placed.fleet.kit_manifest_saved(caller(), saving("/work/another")).await);
    assert_eq!(code, "fleet.kit_manifest_elsewhere");
    assert!(message.starts_with("Nothing was saved. This Job works in "), "{message}");
    assert!(message.contains("for that repository only, and /work/another is another one."), "{message}");
    assert!(message.ends_with(&format!("Send `repository` as {mine}.")), "says what to send: {message}");
    assert!(!message.replace(&mine, "").contains(['-', '\u{2014}']), "no dash in {message}");

    let (code, _) = said(placed.fleet.kit_manifest_saved(None, saving("/work/another")).await);
    assert_eq!(code, "fleet.run_memory_repository_not_served", "Bridge is not held to a Job's repository, only to a served one");

    let nobody = rig(Some(Placing::nothing()));
    nobody.an_agent().await;
    let (code, message) = said(nobody.fleet.kit_manifest_saved(caller(), saving(&mine)).await);
    assert_eq!(code, "fleet.kit_manifest_elsewhere");
    assert!(message.contains("this call comes from none"), "{message}");
}

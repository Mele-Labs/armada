//! What Fleet enters on the timeline, as it enters it: through the tap on the real stream, from the
//! merge lines' difference, and from the acts that leave no event.

use std::sync::Arc;

use ipc::{
    FleetTimeline, HubPullCi, HubQueue, HubQueueState, HubPullRequest, MainCiState, MainStanding,
    MergeLine, MergeLineHub, MergeLines, RunId, TimelineAsk, TimelineEvent, TimelineKind,
};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;
use crate::timeline::{keep_timeline, Between};

pub(super) type Fixture = crate::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// A Fleet whose timeline is being kept, and the router that serves it.
pub(super) fn kept(home: &TempDir) -> (Arc<Fixture>, axum::Router) {
    let fleet = Arc::new(a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"])));
    assert!(keep_timeline(Arc::clone(&fleet)).is_some(), "started once");
    assert!(keep_timeline(Arc::clone(&fleet)).is_none(), "and not twice");
    let events = fleet.events();
    let app = api::router(api::Served::sharing(Arc::clone(&fleet), RunId::carried("01RUN"), events));
    (fleet, app)
}

pub(super) async fn read(fleet: &Fixture) -> Vec<TimelineEvent> {
    fleet.timeline().settled().await;
    fleet.fleet_timeline(TimelineAsk::default()).await.expect("a timeline").events
}

pub(super) fn kinds(events: &[TimelineEvent]) -> Vec<String> {
    events.iter().map(|one| one.kind.as_wire()).collect()
}

pub(super) fn publish(fleet: &Fixture, json: &str) {
    let event: ipc::Event = ipc::decode("an event", json.as_bytes()).expect("an event");
    fleet.events().publish(event);
}

fn session(state: &str, waiting: &str, ended: &str) -> String {
    format!(
        r#"{{"kind":"session.changed","id":"s9","harness":"a-harness","origin":"bridge","manifest_id":"armada",
        "cwd":"/work","title":"Flaky store test","state":"{state}","started_at":"2026-10-12T09:00:00.000Z",
        "last_seen_at":"2026-10-12T09:40:00.000Z","usage":{{}},"attachments":[],{ended}
        "waiting_for":[{waiting}]}}"#
    )
}

fn ask(id: &str, source: &str) -> String {
    format!(
        r#"{{"id":"{id}","text":"Run git push?","since":"2026-10-12T09:20:00.000Z","source":"{source}"}}"#
    )
}

/// A Session is entered by what its row changed to: the first sighting is its start, a waiting item
/// that was not there is an ask, a shown walk is not, and live to ended is its end.
#[tokio::test]
async fn a_session_is_entered_by_what_its_row_changed_to() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    publish(&fleet, &session("live", "", ""));
    publish(&fleet, &session("live", &ask("perm:1", "permission"), ""));
    publish(&fleet, &session("live", &ask("perm:1", "permission"), ""));
    publish(&fleet, &session("live", &ask("walk:http://x", "walk"), ""));
    publish(
        &fleet,
        &session("ended", "", r#""ended_at":"2026-10-12T09:50:00.000Z","end_reason":"closed","#),
    );
    publish(
        &fleet,
        &session("ended", "", r#""ended_at":"2026-10-12T09:50:00.000Z","end_reason":"closed","#),
    );

    let events = read(&fleet).await;
    assert_eq!(kinds(&events), ["session.started", "session.asked", "session.ended"]);
    assert_eq!(events[0].at.as_str(), "2026-10-12T09:00:00.000Z", "the start the row states");
    assert_eq!(events[0].title.as_deref(), Some("Flaky store test"));
    assert_eq!(events[0].manifest_id.as_ref().map(|id| id.as_str()), Some("armada"));
    assert_eq!(events[1].at.as_str(), "2026-10-12T09:20:00.000Z", "when the item began to wait");
    assert_eq!(events[1].says, "Asked: Run git push?");
    assert_eq!(events[2].detail.as_deref(), Some("closed"));
    assert!(events[2].over);
}

/// A person's answer leaves no event of its own, so the acts that take it enter it.
#[tokio::test]
async fn an_answer_and_a_slot_are_entered_by_the_acts_that_make_them() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    let record: ipc::SessionRecord = match ipc::decode::<ipc::Event>("an event", session("live", "", "").as_bytes())
        .expect("an event")
    {
        ipc::Event::SessionChanged(record) => record,
        other => panic!("a session: {other:?}"),
    };
    fleet.session_answered(&record, crate::timeline::card_answer_says(ipc::HelmCallAnswer::AllowOnce));
    fleet.session_slot("s9", Some("Flaky store test".into()), Some("armada".into()), 3, true);
    fleet.session_slot("s9", None, None, 3, false);

    let events = read(&fleet).await;
    assert_eq!(kinds(&events), ["session.answered", "slot.leased", "slot.released"]);
    assert_eq!(events[0].says, "Answered: allow");
    assert_eq!(events[1].says, "Slot 3 leased");
    assert_eq!(events[1].detail.as_deref(), Some("slot-3"));
    assert_eq!(events[2].says, "Slot 3 given back");
    assert!(events[0].live);
}

/// A question or a command in a fact is one line and is cut.
#[test]
fn a_fact_reads_as_one_line() {
    let long = format!("line one\n   line  two {}", "x".repeat(200));
    let one = crate::timeline::bare(&long, 40);
    assert_eq!(one.chars().count(), 40);
    assert!(one.starts_with("line one line two x") && one.ends_with('…'));
    assert_eq!(crate::timeline::bare("short", 40), "short");
}

fn pull(number: u64, ci: Option<HubPullCi>, queue: Option<HubQueue>) -> HubPullRequest {
    HubPullRequest {
        number,
        title: "Gate policy on every run".to_string(),
        branch: "fleet/gate".to_string(),
        url: format!("https://forge.invalid/pull/{number}"),
        author: None,
        ci,
        queue,
        opened_at: None,
        updated_at: None,
        draft: false,
        mergeable: None,
        review: None,
        additions: None,
        deletions: None,
        owner: None,
        merge_by_hand: false,
        mine: true,
    }
}

fn main_is(state: MainCiState, read_at: &str) -> MainStanding {
    MainStanding {
        state,
        commit: "a".repeat(40),
        read_at: ipc::Instant::carried(read_at),
        red_since: None,
        failed: if state == MainCiState::Red {
            vec![ipc::MainFailedJob { name: "components_test".into(), check: None, log_url: None, tests: Vec::new() }]
        } else {
            Vec::new()
        },
        merge: None,
        red_commit: None,
        checking: Vec::new(),
    }
}

fn lines(main: Option<MainStanding>, open: Vec<HubPullRequest>, merged: Vec<ipc::HubMerged>) -> MergeLines {
    MergeLines {
        lines: vec![MergeLine {
            root: "/repo".to_string(),
            line: Vec::new(),
            off: Vec::new(),
            landed: Vec::new(),
            sent_back: Vec::new(),
            hub: Some(MergeLineHub { main, pull_requests: open, merged, fixing_session: None, fixing_elsewhere: None, checkout: None, forge_read_at: None, quiet_since: None }),
        }],
    }
}

fn merged(number: u64, at: &str) -> ipc::HubMerged {
    ipc::HubMerged {
        number,
        title: "Gate policy on every run".to_string(),
        branch: "fleet/gate".to_string(),
        url: format!("https://forge.invalid/pull/{number}"),
        author: None,
        merged_at: ipc::Instant::carried(at),
        commit: None,
        main_run: None,
    }
}

fn named(_: &str) -> Option<String> {
    Some("armada".to_string())
}

fn kinds_of(facts: &[crate::timeline::Fact]) -> Vec<String> {
    facts.iter().map(crate::timeline::Fact::kind_name).collect()
}

/// The first read is a baseline: what was already open, and already red, is not news.
#[test]
fn the_first_read_of_the_merge_lines_enters_nothing() {
    let mut between = Between::default();
    let first = lines(Some(main_is(MainCiState::Red, "2026-10-12T10:00:00.000Z")), vec![pull(7, None, None)], Vec::new());
    assert!(between.read(&first, "2026-10-12T10:00:02.000Z", &named).is_empty());
}

/// A pull request is entered when it opens, when its checks and its place in the queue change, and
/// when the forge lists it merged — dated by the forge, and only once.
#[test]
fn a_pull_request_is_entered_as_it_moves() {
    let mut between = Between::default();
    let at = |n: u32| format!("2026-10-12T10:00:{n:02}.000Z");
    between.read(&lines(None, Vec::new(), Vec::new()), &at(0), &named);

    let opened = between.read(&lines(None, vec![pull(7, None, None)], Vec::new()), &at(2), &named);
    assert_eq!(kinds_of(&opened), ["pr.opened"]);

    let running = between.read(&lines(None, vec![pull(7, Some(HubPullCi::Running), None)], Vec::new()), &at(4), &named);
    assert_eq!(kinds_of(&running), ["pr.checks"]);
    assert_eq!(running[0].says(), "Checks running");

    let queued = HubQueue { state: HubQueueState::InQueue, position: Some(1) };
    let both = lines(None, vec![pull(7, Some(HubPullCi::Passed), Some(queued))], Vec::new());
    let moved = between.read(&both, &at(6), &named);
    assert_eq!(kinds_of(&moved), ["pr.checks", "pr.queued"]);
    assert_eq!(moved[1].says(), "Queued, place 1");

    let same = between.read(&both, &at(8), &named);
    assert!(same.is_empty(), "nothing differs");

    let done = between.read(&lines(None, Vec::new(), vec![merged(7, "2026-10-12T10:00:09.500Z")]), &at(10), &named);
    assert_eq!(kinds_of(&done), ["pr.merged"]);
    assert_eq!(done[0].at(), "2026-10-12T10:00:09.500Z", "the forge's own instant");
    assert!(done[0].is_over());

    let later = between.read(&lines(None, Vec::new(), vec![merged(7, "2026-10-12T10:00:09.500Z")]), &at(12), &named);
    assert!(later.is_empty(), "a merge already listed is not new");
}

/// A pull request opened with auto-merge set and its checks already running, as a Session's usually
/// is, enters all three on the read that first sees it: neither changes again to be seen later.
#[test]
fn a_pull_request_first_seen_enters_its_checks_and_queue_with_it() {
    let mut between = Between::default();
    between.read(&lines(None, Vec::new(), Vec::new()), "2026-10-12T10:00:00.000Z", &named);
    let waiting = HubQueue { state: HubQueueState::WaitingForCi, position: None };
    let opened = between.read(
        &lines(None, vec![pull(7, Some(HubPullCi::Running), Some(waiting))], Vec::new()),
        "2026-10-12T10:00:02.000Z",
        &named,
    );
    assert_eq!(kinds_of(&opened), ["pr.opened", "pr.checks", "pr.queued"]);
    assert_eq!(opened[1].says(), "Checks running");
    assert_eq!(opened[2].says(), "Auto-merge waits on checks");
}

/// A hub that comes back with its merged list full is a read, not forty merges.
#[test]
fn a_merged_list_with_nothing_before_it_to_compare_is_not_news() {
    let mut between = Between::default();
    between.read(&lines(None, Vec::new(), Vec::new()), "2026-10-12T10:00:00.000Z", &named);
    let facts = between.read(
        &lines(None, Vec::new(), vec![merged(1, "2026-10-12T09:00:00.000Z"), merged(2, "2026-10-12T09:10:00.000Z")]),
        "2026-10-12T10:00:02.000Z",
        &named,
    );
    assert!(facts.is_empty());
}

/// Main going red is entered once, and its repair ends with green even through a commit still
/// running — and a main that was red when Fleet started still gets its green.
#[test]
fn main_is_entered_going_red_and_going_green() {
    let mut between = Between::default();
    let main = |state, at: &str| lines(Some(main_is(state, at)), Vec::new(), Vec::new());
    between.read(&main(MainCiState::Green, "2026-10-12T10:00:00.000Z"), "2026-10-12T10:00:02.000Z", &named);

    let red = between.read(&main(MainCiState::Red, "2026-10-12T10:05:00.000Z"), "2026-10-12T10:05:02.000Z", &named);
    assert_eq!(kinds_of(&red), ["main.red"]);
    assert_eq!(red[0].detail_text(), Some("components_test"));

    let again = between.read(&main(MainCiState::Red, "2026-10-12T10:06:00.000Z"), "2026-10-12T10:06:02.000Z", &named);
    assert!(again.is_empty(), "still red is not news");

    between.read(&main(MainCiState::Running, "2026-10-12T10:07:00.000Z"), "2026-10-12T10:07:02.000Z", &named);
    let green = between.read(&main(MainCiState::Green, "2026-10-12T10:08:00.000Z"), "2026-10-12T10:08:02.000Z", &named);
    assert_eq!(kinds_of(&green), ["main.green"]);

    let mut after_a_restart = Between::default();
    after_a_restart.read(&main(MainCiState::Red, "2026-10-12T11:00:00.000Z"), "2026-10-12T11:00:02.000Z", &named);
    let fixed = after_a_restart.read(&main(MainCiState::Green, "2026-10-12T11:05:00.000Z"), "2026-10-12T11:05:02.000Z", &named);
    assert_eq!(kinds_of(&fixed), ["main.green"]);
}

/// The window the route serves: a repository is whose it names, and a bound cuts.
#[tokio::test]
async fn the_window_is_cut_by_instant_and_limit() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    for minute in 0..4 {
        publish(
            &fleet,
            &format!(
                r#"{{"kind":"session.changed","id":"s{minute}","harness":"a-harness","origin":"bridge","manifest_id":"armada",
                "cwd":"/work","title":"A Session","state":"live","started_at":"2026-10-12T10:0{minute}:00.000Z",
                "last_seen_at":"2026-10-12T10:0{minute}:00.000Z","usage":{{}},"attachments":[],"waiting_for":[]}}"#
            ),
        );
    }
    fleet.timeline().settled().await;
    let ask = |since: Option<&str>, until: Option<&str>, limit: Option<u32>| TimelineAsk {
        since: since.map(ipc::Instant::carried),
        until: until.map(ipc::Instant::carried),
        limit,
    };
    let times = |window: &FleetTimeline| window.events.iter().map(|one| one.at.as_str()[14..16].to_string()).collect::<Vec<_>>();

    let middle = fleet
        .fleet_timeline(ask(Some("2026-10-12T10:01:00.000Z"), Some("2026-10-12T10:03:00.000Z"), None))
        .await
        .unwrap();
    assert_eq!(times(&middle), ["01", "02"]);
    assert!(!middle.cut);
    let newest = fleet.fleet_timeline(ask(None, None, Some(2))).await.unwrap();
    assert_eq!(times(&newest), ["02", "03"]);
    assert!(newest.cut);
    let none = fleet.fleet_timeline(ask(None, None, Some(0))).await.unwrap();
    assert_eq!(none.events.len(), 1, "a limit of nothing is raised to one");
    assert_eq!(TimelineKind::from_wire("session.started"), Some(newest.events[0].kind));
}

/// A hosted Session's turn is entered once, where it ends, and the fact is how long it took: the
/// tools and sentences between its start and its end enter nothing, and a second turn is a second
/// entry.
#[tokio::test]
async fn a_sessions_turn_is_entered_once_where_it_ends_with_its_length() {
    use crate::tests::planted::Held;
    use crate::tests::session_host::{eventually, Shared, StandIn};
    use adapter_traits::{CallDetail, DroneEvent, Speaker};
    use api::HostedSessions;

    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let clock = Arc::new(Held::started());
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.clock = clock.clone();
    let fleet = Arc::new(
        crate::Fleet::assembled(fittings)
            .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), std::time::Duration::from_secs(600)),
    );
    keep_timeline(Arc::clone(&fleet));
    let manifest = fleet.repositories().first().expect("a repository").manifest().id().as_str().to_string();
    let started = Arc::clone(&fleet)
        .start_session(ipc::StartSession {
            manifest_id: ipc::ManifestId::carried(&manifest),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
        })
        .await
        .expect("started");
    let say = |text: &str| ipc::SendSessionMessage {
        session_id: started.id.clone(),
        text: text.into(),
        attachments: Vec::new(),
        mentions: Vec::new(),
        aim: None,
    };
    let working = || async {
        let thread = Arc::clone(&fleet).get_session(started.id.clone()).await;
        thread.is_ok_and(|one| one.session.hosted.is_some_and(|hosted| hosted.turn == ipc::SessionTurn::Idle))
    };

    Arc::clone(&fleet).send_session_message(say("fix the login")).await.expect("taken");
    // The first message is read before its turn goes out.
    eventually(|| async { !fleet.hosts().of(started.id.as_str()).state().proposal.holds_back() }).await;
    stand_in.init(0);
    // Heard, not merely sent: the turn's start is taken when the process says it began.
    eventually(|| async { fleet.hosts().of(started.id.as_str()).state().turn_began.is_some() }).await;
    clock.on(134);
    stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Bash".into(), call: "c1".into(), detail: CallDetail::of("cargo test") },
            DroneEvent::Said { text: "reading".into(), by: Speaker::Drone },
            DroneEvent::Called { tool: "Read".into(), call: "c2".into(), detail: CallDetail::of("a.rs") },
        ],
    );
    stand_in.finishes(0, "done");
    eventually(|| async { working().await }).await;

    let events = read(&fleet).await;
    let turns: Vec<_> = events.iter().filter(|one| one.kind == TimelineKind::SessionTurn).collect();
    assert_eq!(turns.len(), 1, "one turn, however many rows it wrote: {:?}", kinds(&events));
    assert!(turns[0].says.starts_with("Turn took 2m "), "{}", turns[0].says);
    assert_eq!(turns[0].title.as_deref(), Some("fix the login"));
    assert_eq!(turns[0].manifest_id.as_ref().map(|id| id.as_str()), Some(manifest.as_str()));
    assert!(!turns[0].live && !turns[0].over, "a finished turn is never live");

    Arc::clone(&fleet).send_session_message(say("and the logout")).await.expect("taken");
    stand_in.init(0);
    stand_in.finishes(0, "done again");
    eventually(|| async {
        fleet.timeline().settled().await;
        read(&fleet).await.iter().filter(|one| one.kind == TimelineKind::SessionTurn).count() == 2
    })
    .await;
}

/// How a turn's length reads.
#[test]
fn a_length_reads_as_a_person_says_it() {
    use crate::timeline::lasted;
    assert_eq!(lasted(400), "under 1s");
    assert_eq!(lasted(42_000), "42s");
    assert_eq!(lasted(134_000), "2m 14s");
    assert_eq!(lasted(3_900_000), "1h 5m");
}

//! The Cockpit home's layout is kept whole across a restart, forgotten where a save carries none, and keeps a block
//! kind this build does not know.

use std::sync::Arc;
use std::time::Duration;

use api::{Next, OtherMacs, Refusal, Settings};
use ipc::{BoardBlock, BoardLayout, Event, HomeLayout, Instant, OtherMacHomeLayout, SaveHomeLayout};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

fn block(kind: &str, x: u8, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.to_string(), x, y: 0, w: 3, h: 2, view: view.to_string() }
}

fn layout(blocks: Vec<BoardBlock>) -> SaveHomeLayout {
    SaveHomeLayout { layout: Some(BoardLayout { blocks }) }
}

#[tokio::test]
async fn a_home_layout_saved_is_read_back_whole_and_survives_a_restart() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_home_layout().await.expect("reads"), HomeLayout::default(), "none saved reads none");

    let blocks = vec![block("needs", 0, "list"), block("run", 3, "table")];
    let answered = fleet.save_home_layout(layout(blocks.clone())).await.expect("saved");
    assert_eq!(answered.layout, Some(BoardLayout { blocks: blocks.clone() }));
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_home_layout().await.expect("reads").layout, Some(BoardLayout { blocks }));
}

#[tokio::test]
async fn a_save_with_no_layout_forgets_it() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    fleet.save_home_layout(layout(vec![block("needs", 0, "list")])).await.expect("saved");
    assert!(fleet.get_home_layout().await.expect("reads").layout.is_some());

    let answered = fleet.save_home_layout(SaveHomeLayout { layout: None }).await.expect("forgotten");
    assert_eq!(answered.layout, None);
    assert!(answered.saved_at.is_some(), "a forgetting keeps its stamp, so it can beat a layout elsewhere");
    assert_eq!(fleet.get_home_layout().await.expect("reads"), answered);
}

#[tokio::test]
async fn a_block_kind_and_view_this_build_does_not_know_survive_a_save() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let blocks = vec![block("sunrise", 0, "constellation"), block("needs", 3, "list")];
    fleet.save_home_layout(layout(blocks.clone())).await.expect("saved");
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_home_layout().await.expect("reads").layout.map(|one| one.blocks), Some(blocks));
}

#[tokio::test]
async fn a_blank_kind_or_view_is_refused_and_keeps_what_was_there() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let kept = vec![block("needs", 0, "list")];
    fleet.save_home_layout(layout(kept.clone())).await.expect("saved");

    for bad in [block("  ", 0, "list"), block("run", 0, "")] {
        match fleet.save_home_layout(layout(vec![bad])).await.expect_err("a blank word") {
            Refusal::Unacceptable(error) => assert_eq!(error.code, "fleet.unacceptable_home_layout"),
            other => panic!("not a 422: {other:?}"),
        }
    }
    assert_eq!(fleet.get_home_layout().await.expect("reads").layout.map(|one| one.blocks), Some(kept));
}

#[tokio::test]
async fn saving_the_home_layout_leaves_the_other_preferences_alone() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let theme = ipc::SavePreference { name: "theme".into(), value: false, text: Some("calm".into()) };
    fleet.save_preferences(theme).await.expect("theme saved");
    fleet.save_home_layout(layout(vec![block("needs", 0, "list")])).await.expect("saved");
    assert_eq!(fleet.get_preferences().await.expect("reads").theme, "calm");
}

const WORK: &str = "mac-work";

fn fleet_in(home: &TempDir) -> Arc<Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>> {
    Arc::new(Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[]))))
}

/// What the Gateway does: reads the other Mac's `share_home_layout` and hands it to this Fleet.
async fn handed_across<F>(from: &F, to: &F)
where
    F: OtherMacs,
{
    let home = from.share_home_layout().await.expect("shared");
    to.hold_other_mac_home_layout(OtherMacHomeLayout { mac: WORK.into(), home }).await.expect("held");
}

fn stamped(when: &str, blocks: Vec<BoardBlock>) -> HomeLayout {
    HomeLayout { layout: Some(BoardLayout { blocks }), saved_at: Some(Instant::carried(when)) }
}

async fn blocks_of(fleet: &impl Settings) -> Option<Vec<BoardBlock>> {
    fleet.get_home_layout().await.expect("reads").layout.map(|one| one.blocks)
}

#[tokio::test]
async fn a_save_on_one_fleet_reaches_a_second_and_survives_its_restart() {
    let (here, there) = (TempDir::new(), TempDir::new());
    let (mac, other) = (fleet_in(&here), fleet_in(&there));
    let blocks = vec![block("needs", 0, "list"), block("sunrise", 3, "constellation")];
    mac.save_home_layout(layout(blocks.clone())).await.expect("saved");
    assert_eq!(blocks_of(&*other).await, None);

    handed_across(&*mac, &*other).await;
    assert_eq!(blocks_of(&*other).await, Some(blocks.clone()));
    assert_eq!(other.get_home_layout().await.unwrap().saved_at, mac.get_home_layout().await.unwrap().saved_at, "its stamp comes with it");
    drop(other);
    assert_eq!(blocks_of(&*fleet_in(&there)).await, Some(blocks));
}

#[tokio::test]
async fn the_later_save_wins_on_both_macs_whichever_way_they_are_handed_across() {
    let (here, there) = (TempDir::new(), TempDir::new());
    let (mac, other) = (fleet_in(&here), fleet_in(&there));
    let early = vec![block("needs", 0, "list")];
    let late = vec![block("run", 0, "table")];
    mac.hold_other_mac_home_layout(OtherMacHomeLayout { mac: WORK.into(), home: stamped("2026-11-24T10:00:00.000Z", early.clone()) }).await.unwrap();
    other.hold_other_mac_home_layout(OtherMacHomeLayout { mac: WORK.into(), home: stamped("2026-11-24T10:00:05.000Z", late.clone()) }).await.unwrap();

    handed_across(&*mac, &*other).await;
    assert_eq!(blocks_of(&*other).await, Some(late.clone()), "the earlier save does not replace the later one");
    handed_across(&*other, &*mac).await;
    assert_eq!(blocks_of(&*mac).await, Some(late), "the later save replaces the earlier");

    // A forgetting later than a layout beats it too.
    let forgotten = HomeLayout { layout: None, saved_at: Some(Instant::carried("2026-11-24T10:00:09.000Z")) };
    mac.hold_other_mac_home_layout(OtherMacHomeLayout { mac: WORK.into(), home: forgotten }).await.unwrap();
    handed_across(&*mac, &*other).await;
    assert_eq!(blocks_of(&*other).await, None);
}

#[tokio::test]
async fn a_mac_that_was_offline_takes_the_latest_save_when_it_reconnects_and_keeps_a_later_own() {
    let (here, there) = (TempDir::new(), TempDir::new());
    let (mac, asleep) = (fleet_in(&here), fleet_in(&there));
    // Three saves while the other Mac is away: only the last is read when it comes back.
    for (second, kind) in [(1, "needs"), (2, "run"), (3, "plan")] {
        let home = stamped(&format!("2026-11-24T10:00:0{second}.000Z"), vec![block(kind, 0, "list")]);
        mac.hold_other_mac_home_layout(OtherMacHomeLayout { mac: WORK.into(), home }).await.unwrap();
    }
    handed_across(&*mac, &*asleep).await;
    assert_eq!(blocks_of(&*asleep).await, Some(vec![block("plan", 0, "list")]));

    // Its own save made later than what it hears is kept.
    let own = asleep.save_home_layout(layout(vec![block("mine", 0, "list")])).await.unwrap();
    assert!(own.saved_at.is_some());
    asleep.hold_other_mac_home_layout(OtherMacHomeLayout { mac: WORK.into(), home: stamped("2000-01-01T00:00:00.000Z", vec![block("old", 0, "list")]) }).await.unwrap();
    assert_eq!(blocks_of(&*asleep).await, Some(vec![block("mine", 0, "list")]));
}

#[tokio::test]
async fn a_save_tells_the_linked_macs_and_a_layout_taken_does_not() {
    let (here, there) = (TempDir::new(), TempDir::new());
    let (mac, other) = (fleet_in(&here), fleet_in(&there));
    let mut watching = mac.events().subscribe();
    let mut taking = other.events().subscribe();
    mac.save_home_layout(layout(vec![block("needs", 0, "list")])).await.unwrap();
    handed_across(&*mac, &*other).await;

    let told = |events: Vec<Event>| events.iter().any(|one| matches!(one, Event::HomeLayoutChanged(_)));
    assert!(told(sent(&mut watching).await), "the save is told, and crosses by `event_for_other_mac`");
    assert!(!told(sent(&mut taking).await), "a layout taken is not told on");
    let crossing = mac.event_for_other_mac(Event::HomeLayoutChanged(ipc::HomeLayoutChanged {})).await;
    assert!(crossing.is_some());
}

#[tokio::test]
async fn a_layout_another_mac_sends_with_a_blank_word_is_refused_and_nothing_is_written() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    let bad = stamped("2026-11-24T10:00:00.000Z", vec![block(" ", 0, "list")]);
    match fleet.hold_other_mac_home_layout(OtherMacHomeLayout { mac: WORK.into(), home: bad }).await.expect_err("refused") {
        Refusal::Unacceptable(error) => assert_eq!(error.code, "fleet.unacceptable_home_layout"),
        other => panic!("not a 422: {other:?}"),
    }
    assert_eq!(blocks_of(&*fleet).await, None);
}

/// Every event already sent. **Read after the call that sent them returns**, so nothing races the read.
async fn sent(watching: &mut api::Subscription) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(Some(next)) = tokio::time::timeout(Duration::from_millis(100), watching.next()).await {
        if let Next::Send(delivered) = next {
            events.push(delivered.event);
        }
    }
    events
}

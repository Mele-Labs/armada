//! The claim, written before the code: **every open ask of a Job carries its place and why; where the Job set no
//! order, what blocks the work comes first, then pages, then a build to try; a refresh comes before a prototype or a
//! walk that needs it; a Job's own order and whys win, the asks it left out following in Fleet's order; and an id
//! that is not an open ask is refused by name.** `crates/ipc/operations/order_waiting.toml`.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{Instant, OrderWaiting, SetWaitingFor, WaitingInput, WaitingItem, WaitingPlace, WaitingSource as Source};
use store::KeptWaitingPlace;

use super::page_actions::a_job;
use super::session_host::{rig_placing, Rig};
use crate::session_host::needs_first::{
    placed, NeedsBuild, WHY_BLOCKS, WHY_DONE, WHY_PAGE, WHY_PROTOTYPE_NEEDS_BUILD, WHY_TRY, WHY_WALK_NEEDS_BUILD,
};
use api::Sessions;

fn item(id: &str, source: Source) -> WaitingItem {
    WaitingItem {
        order: 0,
        why: String::new(),
        id: id.into(),
        text: format!("text of {id}"),
        since: Instant::carried("2026-11-25T10:00:00Z"),
        source,
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        approves: false,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
        artifact: None,
        goal_met: None,
    }
}

fn nothing_needs_the_build(_: &WaitingItem) -> Option<NeedsBuild> {
    None
}

/// Each ask's id, place and why, in the order served.
fn read(items: &[WaitingItem]) -> Vec<(&str, u32, &str)> {
    items.iter().map(|one| (one.id.as_str(), one.order, one.why.as_str())).collect()
}

fn said(places: &[(&str, &str)]) -> Vec<KeptWaitingPlace> {
    places.iter().map(|(item, why)| KeptWaitingPlace { item: (*item).into(), why: (*why).into() }).collect()
}

#[test]
fn with_no_order_set_what_blocks_the_work_comes_first_then_pages_then_a_build_to_try() {
    let items = vec![
        item("done:1", Source::Done),
        item("try:job", Source::ReadyToTry),
        item("document:lease.html", Source::Document),
        item("which-zone", Source::Agent),
        item("sketch:Bay flow", Source::Sketch),
        item("plan:2", Source::Plan),
        item("draft:4", Source::Draft),
    ];
    let served = placed(items, &[], &nothing_needs_the_build);
    assert_eq!(
        read(&served),
        [
            ("which-zone", 1, WHY_BLOCKS),
            ("sketch:Bay flow", 2, WHY_BLOCKS),
            ("plan:2", 3, WHY_BLOCKS),
            ("draft:4", 4, WHY_BLOCKS),
            ("document:lease.html", 5, WHY_PAGE),
            ("try:job", 6, WHY_TRY),
            ("done:1", 7, WHY_DONE),
        ]
    );
}

#[test]
fn a_refresh_comes_before_a_prototype_or_a_walk_that_needs_that_build() {
    let prototype = WaitingItem { artifact: Some("sketch:Ready card".into()), ..item("sketch:Ready card", Source::Sketch) };
    let needs = |one: &WaitingItem| match one.source {
        Source::Walk => Some(NeedsBuild::Walk),
        Source::Sketch if one.artifact.is_some() => Some(NeedsBuild::Prototype),
        _ => None,
    };
    let items = vec![item("which-zone", Source::Agent), prototype, item("walk:http://localhost:5173/", Source::Walk), item("try:job", Source::ReadyToTry)];
    let served = placed(items, &[], &needs);
    assert_eq!(
        read(&served),
        [
            ("try:job", 1, WHY_PROTOTYPE_NEEDS_BUILD),
            ("which-zone", 2, WHY_BLOCKS),
            ("sketch:Ready card", 3, WHY_BLOCKS),
            ("walk:http://localhost:5173/", 4, WHY_BLOCKS),
        ]
    );

    let walked = placed(vec![item("walk:http://localhost:5173/", Source::Walk), item("try:job", Source::ReadyToTry)], &[], &needs);
    assert_eq!(read(&walked)[0], ("try:job", 1, WHY_WALK_NEEDS_BUILD), "a walk needs the build too");

    let alone = placed(vec![item("which-zone", Source::Agent), item("try:job", Source::ReadyToTry)], &[], &needs);
    assert_eq!(read(&alone)[1], ("try:job", 2, WHY_TRY), "with nothing that needs it, a build to try waits its turn");
}

#[test]
fn the_jobs_own_order_and_whys_win_and_the_asks_it_left_out_follow_in_fleets_order() {
    let items = vec![
        item("try:job", Source::ReadyToTry),
        item("document:lease.html", Source::Document),
        item("which-zone", Source::Agent),
        item("sketch:Bay flow", Source::Sketch),
    ];
    let order = said(&[
        ("document:lease.html", "The zone question reads this page"),
        ("settled-long-ago", "Passed over"),
        ("which-zone", "The tests wait on the zone"),
    ]);
    let served = placed(items, &order, &nothing_needs_the_build);
    assert_eq!(
        read(&served),
        [
            ("document:lease.html", 1, "The zone question reads this page"),
            ("which-zone", 2, "The tests wait on the zone"),
            ("sketch:Bay flow", 3, WHY_BLOCKS),
            ("try:job", 4, WHY_TRY),
        ],
        "an id named whose ask is settled is passed over"
    );
}

fn ask(id: &str, text: &str) -> WaitingInput {
    WaitingInput { id: id.into(), text: text.into(), act: None, options: Vec::new(), approves: false, steps: Vec::new(), artifact: None }
}

/// A Job asking two things, in the order it stated them.
async fn a_job_asking(rig: &Rig) -> ipc::SessionId {
    let (id, _) = a_job(rig).await;
    let items = vec![ask("which-zone", "Which time zone do the tests run in?"), ask("which-port", "Which port does the server take?")];
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(id.clone()) }).await.expect("stated");
    id
}

fn place(id: &str, why: &str) -> WaitingPlace {
    WaitingPlace { id: id.into(), why: why.into() }
}

#[tokio::test]
async fn order_waiting_serves_the_jobs_order_and_whys_on_its_record() {
    let rig = rig_placing(None);
    let id = a_job_asking(&rig).await;
    let order = OrderWaiting { items: vec![place("which-port", "The server cannot start without it")], session_id: Some(id.clone()) };
    let record = Arc::clone(&rig.fleet).order_waiting(order).await.expect("ordered");
    assert_eq!(
        read(&record.waiting_for),
        [("which-port", 1, "The server cannot start without it"), ("which-zone", 2, WHY_BLOCKS)]
    );

    let cleared = Arc::clone(&rig.fleet).order_waiting(OrderWaiting { items: Vec::new(), session_id: Some(id) }).await.expect("cleared");
    assert_eq!(read(&cleared.waiting_for), [("which-zone", 1, WHY_BLOCKS), ("which-port", 2, WHY_BLOCKS)], "an empty list clears it");
}

#[tokio::test]
async fn order_waiting_refuses_an_id_that_is_not_an_open_ask_by_name_and_changes_nothing() {
    let rig = rig_placing(None);
    let id = a_job_asking(&rig).await;
    let order = OrderWaiting {
        items: vec![place("which-port", "First"), place("walk:http://nowhere", "Second")],
        session_id: Some(id.clone()),
    };
    let refused = Arc::clone(&rig.fleet).order_waiting(order).await.expect_err("refused");
    assert_eq!(refused.error().code, ipc::WAITING_ORDER_UNKNOWN);
    let message = &refused.error().message;
    assert!(message.contains("walk:http://nowhere") && message.contains("which-zone, which-port"), "{message}");

    let record = rig.fleet.waiting_now(id.as_str()).await.expect("read");
    assert_eq!(read(&record)[0], ("which-zone", 1, WHY_BLOCKS), "refused whole");

    let blank = OrderWaiting { items: vec![place("which-port", " ")], session_id: Some(id) };
    assert_eq!(Arc::clone(&rig.fleet).order_waiting(blank).await.expect_err("refused").error().code, ipc::WAITING_ORDER_BLANK);
}

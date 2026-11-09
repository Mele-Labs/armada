//! What a client that read a Job's thread is told when it changes: a tail from the first item that moved,
//! a mark where that is far back, and nothing where nothing did.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Refusal};
use ipc::{Event, SessionRow, ThreadItem};

use super::session_host::rig_placing;
use super::thread_view::{agent, ids, record, thread, tool, you};
use crate::thread_keeping::{Change, Held, HELD_MOST};

fn messages(count: u32) -> Vec<SessionRow> {
    (0..count).map(|n| agent(&format!("m{n:03}"), n % 50, "x")).collect()
}

async fn held_with(rows: Vec<SessionRow>) -> (Held, Vec<ThreadItem>) {
    let held = Held::default();
    held.begin("s1").await;
    let (_, items) = held.read_through("s1", thread(rows), Vec::new()).await;
    (held, items)
}

fn tails(events: &[Event]) -> Vec<(Option<&str>, Vec<&str>)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::JobThreadTail(tail) => Some((tail.from.as_deref(), ids(&tail.items))),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_row_appended_is_a_tail_of_the_new_item_and_names_no_item_held() {
    let (held, _) = held_with(messages(300)).await;
    let events = held.changed("s1", vec![Change::Row(agent("new", 1, "y"))]).await;
    assert_eq!(tails(&events), [(None, vec!["new"])]);
}

#[tokio::test]
async fn a_row_replaced_near_the_end_is_a_tail_from_that_item() {
    let (held, _) = held_with(messages(300)).await;
    let events = held.changed("s1", vec![Change::Row(agent("m295", 1, "changed"))]).await;
    assert_eq!(tails(&events), [(Some("m295"), vec!["m295", "m296", "m297", "m298", "m299"])]);
}

#[tokio::test]
async fn a_change_more_than_the_tail_cap_back_is_only_a_mark() {
    let (held, _) = held_with(messages(300)).await;
    let events = held.changed("s1", vec![Change::Row(agent("m050", 1, "changed"))]).await;
    assert!(tails(&events).is_empty(), "no tail of 250 items");
    let [Event::JobThreadChanged(mark)] = events.as_slice() else { panic!("one mark: {events:?}") };
    assert_eq!((mark.session_id.as_str(), mark.from.as_str()), ("s1", "m050"));
    // The 200th item from the end is still a tail; the 201st is the mark.
    let on_the_edge = held.changed("s1", vec![Change::Row(agent("m100", 1, "changed"))]).await;
    assert_eq!(tails(&on_the_edge).len(), 1, "exactly 200 items");
    let past_it = held.changed("s1", vec![Change::Row(agent("m099", 1, "changed"))]).await;
    assert!(matches!(past_it.as_slice(), [Event::JobThreadChanged(_)]));
}

#[tokio::test]
async fn a_change_to_the_head_alone_is_a_tail_with_no_from_and_no_items() {
    let (held, _) = held_with(vec![agent("a", 1, "x"), tool("t1", 2, "Bash ls")]).await;
    let running = record(r#""turn_since":"2026-11-03T10:00:00.000Z""#);
    let events = held.changed("s1", vec![Change::Record(Box::new(running))]).await;
    let [Event::JobThreadTail(tail)] = events.as_slice() else { panic!("one tail: {events:?}") };
    assert!(tail.from.is_none() && tail.items.is_empty());
    assert_eq!(tail.session_id.as_str(), "s1");
}

#[tokio::test]
async fn a_row_that_moves_nothing_publishes_nothing() {
    let (held, _) = held_with(messages(5)).await;
    assert!(held.changed("s1", vec![Change::Row(agent("m003", 3, "x"))]).await.is_empty());
    assert!(held.changed("s1", vec![Change::Record(Box::new(record("")))]).await.is_empty());
}

#[tokio::test]
async fn a_run_of_calls_that_grows_is_a_tail_from_the_run() {
    let (held, _) = held_with(vec![agent("a", 1, "x"), tool("t1", 2, "Bash a")]).await;
    let events = held.changed("s1", vec![Change::Row(tool("t2", 3, "Bash b"))]).await;
    assert_eq!(tails(&events), [(Some("t1"), vec!["t1"])]);
    let [Event::JobThreadTail(tail)] = events.as_slice() else { panic!("a tail") };
    let ThreadItem::Calls { rows, .. } = &tail.items[0] else { panic!("the run") };
    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn what_is_heard_while_a_read_is_made_is_in_the_read_and_not_told_twice() {
    let held = Held::default();
    held.begin("s1").await;
    assert!(held.changed("s1", vec![Change::Row(you("late", 9, "hi"))]).await.is_empty(), "buffered for the read");
    let (_, items) = held.read_through("s1", thread(vec![agent("a", 1, "x")]), Vec::new()).await;
    assert_eq!(ids(&items), ["a", "late"]);
    assert!(held.changed("s1", vec![Change::Row(you("late", 9, "hi"))]).await.is_empty());
}

#[tokio::test]
async fn only_the_jobs_read_most_recently_are_held_and_a_job_that_ended_is_let_go() {
    let held = Held::default();
    for n in 0..=HELD_MOST {
        held.begin(&format!("j{n}")).await;
    }
    assert!(!held.is_watched("j0"), "the oldest read went");
    assert!(held.is_watched(&format!("j{HELD_MOST}")));

    let (held, _) = held_with(messages(2)).await;
    assert!(held.is_watched("s1"));
    let mut ended = record("");
    ended.state = ipc::SessionState::Ended;
    held.changed("s1", vec![Change::Record(Box::new(ended))]).await;
    assert!(!held.is_watched("s1"));
}

#[tokio::test]
async fn a_read_with_a_before_no_item_holds_is_refused_and_a_read_follows_the_job() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let _keeping = crate::thread_keeping::keep_the_threads(Arc::clone(&rig.fleet));

    let refused = Arc::clone(&rig.fleet)
        .get_job_thread(ipc::JobThreadRead { session_id: id.clone(), before: Some("nowhere".into()), limit: None })
        .await;
    assert!(matches!(refused, Err(Refusal::Unacceptable(_))), "{refused:?}");

    let read = Arc::clone(&rig.fleet)
        .get_job_thread(ipc::JobThreadRead { session_id: id.clone(), before: None, limit: Some(5) })
        .await
        .expect("a read");
    assert_eq!(read.session_id, id);
    assert!(read.older.is_none());

    let mut watching = rig.fleet.events().subscribe();
    rig.fleet.row_put(id.as_str(), tool("fresh", 1, "Bash ls")).await;
    let mut told = None;
    while let Ok(Some(next)) = tokio::time::timeout(Duration::from_secs(5), watching.next()).await {
        if let Next::Send(delivered) = next {
            if let Event::JobThreadTail(tail) = delivered.event {
                told = Some(tail);
                break;
            }
        }
    }
    let tail = told.expect("a tail followed the row");
    assert_eq!(ids(&tail.items), ["fresh"]);
    assert_eq!(tail.session_id, id);
}

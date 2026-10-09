//! Reads side by side answer in order and never fan out past their width.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::concurrently;

#[test]
fn answers_come_back_in_the_order_of_the_items_whoever_finishes_first() {
    let items: Vec<u64> = (0..20).collect();
    let answers = concurrently(&items, |&n| {
        // The first items are the slowest, so a read that answered in the
        // order of finishing would put them last.
        std::thread::sleep(Duration::from_millis(20u64.saturating_sub(n)));
        n * 2
    });
    assert_eq!(answers, items.iter().map(|n| n * 2).collect::<Vec<_>>());
}

#[test]
fn no_more_run_at_once_than_the_width_and_more_than_one_does() {
    let (running, widest) = (AtomicUsize::new(0), AtomicUsize::new(0));
    concurrently(&[(); 30], |_| {
        let now = running.fetch_add(1, Ordering::SeqCst) + 1;
        widest.fetch_max(now, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(5));
        running.fetch_sub(1, Ordering::SeqCst);
    });
    let widest = widest.load(Ordering::SeqCst);
    assert!((2..=6).contains(&widest), "{widest} at once");
}

#[test]
fn nothing_and_one_item_are_read_without_a_thread() {
    assert!(concurrently(&[] as &[u8], |_| 1).is_empty());
    let here = std::thread::current().id();
    assert_eq!(concurrently(&[1u8], |_| std::thread::current().id()), [here]);
}

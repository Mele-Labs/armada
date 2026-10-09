//! A read of many independent checkouts, side by side.
//!
//! Each slot of the pool is read by its own git processes, and nothing one
//! answers depends on another, so reading them one after the other makes the
//! total the sum of every spawn.

use std::sync::atomic::{AtomicUsize, Ordering};

/// How many reads run at once. The pool has eight to fourteen slots; a wider
/// fan-out only queues more processes on a machine that is also building.
const WIDTH: usize = 6;

/// `read` over every item, at most [`WIDTH`] at a time, answers in the order
/// of `items`. Blocks the calling thread, so it belongs on a blocking one.
pub fn concurrently<T, R>(items: &[T], read: impl Fn(&T) -> R + Sync) -> Vec<R>
where
    T: Sync,
    R: Send,
{
    let width = items.len().min(WIDTH);
    if width < 2 {
        return items.iter().map(read).collect();
    }
    let next = AtomicUsize::new(0);
    let mut answers: Vec<Option<R>> = items.iter().map(|_| None).collect();
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..width)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(at) else { break };
                        done.push((at, read(item)));
                    }
                    done
                })
            })
            .collect();
        for worker in workers {
            let done = worker
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            for (at, answer) in done {
                answers[at] = Some(answer);
            }
        }
    });
    answers
        .into_iter()
        .map(|answer| answer.expect("every item was read"))
        .collect()
}

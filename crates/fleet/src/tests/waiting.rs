//! Waiting on what another task does: bounded, and failing by naming it.
//!
//! **A poll whose bound runs out must say so.** One that fell through instead
//! handed the failure to the next assertion, which named something else — a
//! scout that never read under load was reported as one that forgot what it
//! read. `#1494`.

use std::future::Future;
use std::time::Duration;

/// What `check` answers, looked for `looks` times `every` apart. **Panics
/// naming `never`** — what did not happen — when every look comes back empty.
///
/// Bounded by looks rather than a deadline, as the loops it replaced were: a
/// look under load takes longer, and the wait stretches with it.
pub(super) async fn until<T, F: Future<Output = Option<T>>>(
    never: &str,
    looks: u32,
    every: Duration,
    mut check: impl FnMut() -> F,
) -> T {
    for _ in 0..looks {
        if let Some(found) = check().await {
            return found;
        }
        tokio::time::sleep(every).await;
    }
    panic!("{never}, in {looks} looks {every:?} apart");
}

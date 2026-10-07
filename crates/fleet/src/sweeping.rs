//! The build-output sweep: every [`every`](keep_sweeping) Fleet trims the
//! `target/` of every checkout of each repository it serves, so a directory
//! nobody released or cleaned still cannot grow without bound. The rules are
//! `adapters::leasing::trim_target`'s, the same function a release calls.
//! **A piloted Job's checkout is left alone**: `held` names them each time.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use adapters::leasing::{sweep_repository, Trim, Trimmed};
use api::Queries;
use tokio::task::JoinHandle;

/// Sweep every served repository each `every`, and hand `said` one line for
/// each `target/` that was removed whole, since that one costs a cold rebuild.
pub fn keep_sweeping<D, F>(
    daemon: Arc<D>,
    every: Duration,
    trim: Trim,
    held: impl Fn() -> F + Send + 'static,
    said: impl Fn(&str) + Send + 'static,
) -> JoinHandle<()>
where
    D: Queries + Send + Sync + 'static,
    F: std::future::Future<Output = Vec<PathBuf>> + Send,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval_at(tokio::time::Instant::now() + every, every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let roots = crate::merge_lines::roots(daemon.as_ref()).await;
            let held = held().await;
            let Ok(removed) = tokio::task::spawn_blocking(move || {
                let mut removed: Vec<PathBuf> = Vec::new();
                for root in roots {
                    removed.extend(
                        sweep_repository(root.as_ref(), trim, SystemTime::now(), &held)
                            .into_iter()
                            .filter(|(_, done)| *done == Trimmed::Removed)
                            .map(|(target, _)| target),
                    );
                }
                removed
            })
            .await
            else {
                return;
            };
            for target in removed {
                said(&format!(
                    "{} was over its size ceiling and was removed",
                    target.display()
                ));
            }
        }
    })
}

//! When he last looked. `crates/ipc/operations/operator_in_front.toml`.
//!
//! **Fleet decides whether he was away**: the window coming to the front more than [`AWAY`] after the
//! last time it was in front is him coming back. That sets when he last looked, which the feed counts
//! from. The page's top line is no longer rewritten, so the answer is always that it is not.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

const DAY: i64 = 86_400_000;
/// How long the window must have been out of front for coming to it to be a return: a quarter of an hour.
const AWAY: i64 = 15 * 60 * 1000;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `operator_in_front`: Bridge's window came to the front. **Answers at once**, `false`: no top line is rewritten.
    pub(crate) async fn operator_in_front_now(self: &Arc<Self>) -> bool {
        if !self.operator_on() {
            return false;
        }
        let now = self.now();
        let millis = now.epoch_millis().unwrap_or(0);
        let mut page = self.store().lock().await.operator_page().unwrap_or_default();
        let seen = page.seen_at.take();
        let away = seen.as_deref().and_then(|at| core_model::Timestamp::from_rfc3339(at).epoch_millis()).is_none_or(|at| millis - at >= AWAY);
        page.seen_at = Some(now.as_str().to_string());
        if away {
            page.looked_at = Some(match seen {
                Some(seen) => seen,
                None => crate::clock::rfc3339_utc(crate::clock::local_day_began(millis - DAY).await),
            });
        }
        if let Err(why) = self.store().lock().await.keep_operator_page(&page) {
            self.operator_warned("when he last looked was not kept", why);
            return false;
        }
        if away {
            self.operator_page_changed();
        }
        false
    }
}

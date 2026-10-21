//! `set_notice_mute`: a person turns a Fleet notice off for a Job, a repository or an hour, or back on.
//! The mute is kept in the store, so a restart does not end it and an hour mute still ends at its time.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{NoticeScope, SetNoticeMute, WireError};

use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;

/// A mute that names nobody. A 422.
const MUTE_UNNAMED: &str = "fleet.notice_mute_unnamed";
const HOUR_MILLIS: i64 = 3_600_000;

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
    /// `set_notice_mute`. Answers with the body, as `set_kept_on_this_mac` does.
    pub(crate) async fn notice_mute_set(&self, asked: SetNoticeMute) -> Result<SetNoticeMute, Refusal> {
        let (scope, key, until) = match &asked.scope {
            NoticeScope::Job { session_id } => ("job", session_id, None),
            NoticeScope::Repo { manifest_id } => ("repo", manifest_id, None),
            NoticeScope::Hour { session_id } => {
                let now = self.now().epoch_millis().unwrap_or(0);
                ("job", session_id, Some(rfc3339_utc(now + HOUR_MILLIS)))
            }
        };
        if key.trim().is_empty() {
            return Err(Refusal::Unacceptable(WireError::raised(
                MUTE_UNNAMED,
                "a mute needs the Job or repository it is for".to_string(),
                self.run_id(),
            )));
        }
        let kind = asked.notice.word();
        let mut store = self.store().lock().await;
        let kept = if asked.muted {
            store.keep_mute(kind, scope, key, until.as_deref())
        } else {
            store.lift_mute(kind, scope, key)
        };
        kept.map_err(|why| Refusal::Fault(WireError::raised("fleet.notice_mute_failed", why.to_string(), self.run_id())))?;
        Ok(asked)
    }
}

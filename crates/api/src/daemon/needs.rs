//! `armada need`: what a checkout says it needs on a path, kept on the session
//! ledger. Since 23.46. `docs/capabilities/needs.md`.
//!
//! **Its own surface**, for [`Sessions`](super::Sessions)' reason: whole on its
//! own, and a handler that takes only this cannot reach for a Job.

use std::future::Future;

use ipc::{ManifestId, NeedAnswer, NeedCall, NeedList};

use crate::daemon::Refusal;

pub trait Needs: Send + Sync + 'static {
    /// `act_on_need` — declare, say what was taken, or give back.
    /// [`Refusal::Unacceptable`] for a blank branch or path, a declaration with
    /// no `what`, a `took` with no `value`, and a `took` on a need the holder
    /// never declared.
    fn act_on_need(
        &self,
        call: NeedCall,
    ) -> impl Future<Output = Result<NeedAnswer, Refusal>> + Send;

    /// `list_needs` — every standing need of one repository, by path and in
    /// order. `manifest_id` absent is the first served.
    fn list_needs(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<NeedList, Refusal>> + Send;
}

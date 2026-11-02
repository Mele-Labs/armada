//! A process's opening line kept, so the debug report can say what each Job came up with.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, HarnessReading, Vcs, WorkProduct};
use store::KeptInit;

use crate::daemon::Fleet;

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
    /// Keep what the process said it came up with. **A store that will not take it loses the
    /// report's line and nothing else**: the Job goes on as it did.
    pub(super) async fn init_kept(self: &Arc<Self>, id: &str, reading: &HarnessReading) {
        let kept = KeptInit {
            harness_version: reading.version.clone(),
            mod_loaded: reading.mod_loaded,
            door: reading.door.clone(),
            task_tools: reading.task_tools,
            tools: reading.tools.clone(),
            skills: reading.skills.clone(),
        };
        let at = self.now();
        let _ = self.store().lock().await.keep_init(id, &kept, at.as_str());
    }
}

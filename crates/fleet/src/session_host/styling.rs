//! `get_style_sources` and `get_kept_styles`: the stylesheets a `mock` node in a sketch is drawn
//! with. **Only the seam is here**: both refuse until Fleet finds and keeps styles. The wire, the
//! route and the door are done, so the next change fills these two methods and nothing else.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{KeptStyles, SessionId, StyleSources, WireError};

use crate::daemon::Fleet;

const STYLE_SOURCES_NOT_BUILT: &str = "fleet.style_sources_not_built";
const NO_SUCH_KEPT_STYLES: &str = "fleet.no_such_kept_styles";

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
    /// The stylesheets nearest `path`, or the Job's own work where it is none.
    pub(crate) async fn style_sources_read(
        &self,
        _caller: Option<api::Caller>,
        _session_id: Option<SessionId>,
        _path: Option<String>,
    ) -> Result<StyleSources, Refusal> {
        Err(Refusal::Unacceptable(WireError::raised(
            STYLE_SOURCES_NOT_BUILT,
            "Fleet does not find stylesheets yet, so a mock is drawn with its own css alone",
            self.run_id(),
        )))
    }

    /// The style set kept under `hash`.
    pub(crate) async fn kept_styles_read(&self, hash: &str) -> Result<KeptStyles, Refusal> {
        Err(Refusal::NotFound(WireError::raised(
            NO_SUCH_KEPT_STYLES,
            format!("Fleet holds no kept styles under {hash}."),
            self.run_id(),
        )))
    }
}

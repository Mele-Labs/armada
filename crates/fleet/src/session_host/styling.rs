//! `get_style_sources`, `get_kept_styles`, and the style set a `mock` node in a sketch is drawn
//! with. `.armada/documents/hi-fi-sketches.html`, *How Fleet finds the styles*.
//!
//! **Fleet finds the stylesheets nearest the code a Job works on, and keeps a copy by content hash**
//! in its attachments folder, so a sketch keeps the look it was drawn with. The modules below are
//! the walk (`walk`), the tokens a file sets (`tokens`), making a sheet usable (`sheet`), keeping a
//! set (`keeping`), which set a mock gets (`choosing`), and the Job those are stood under (`applying`).

pub(crate) mod applying;
pub(crate) mod choosing;
pub(crate) mod keeping;
pub(crate) mod sheet;
pub(crate) mod tokens;
pub(crate) mod walk;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{KeptStyles, SessionId, StyleSources, WireError};

use crate::daemon::Fleet;

const STYLE_SOURCES_UNNAMED: &str = "fleet.style_sources_unnamed";
const STYLE_SOURCES_OUTSIDE: &str = "fleet.style_sources_outside";
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
        caller: Option<api::Caller>,
        session_id: Option<SessionId>,
        path: Option<String>,
    ) -> Result<StyleSources, Refusal> {
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                self.hosted_refusal(STYLE_SOURCES_UNNAMED, "the stylesheets near a job's work need a job, and this call places none")
            })?;
        let look = self.job_look(&id).await?;
        let named = path.map(|path| path.trim().to_string()).filter(|path| !path.is_empty());
        let found = tokio::task::spawn_blocking(move || applying::sources_for(&look, named.as_deref()))
            .await
            .map_err(|why| self.hosted_fault(STYLE_SOURCES_OUTSIDE, &format!("the walk did not finish: {why}")))?;
        found.map_err(|why| self.hosted_refusal(STYLE_SOURCES_OUTSIDE, &why))
    }

    /// The style set kept under `hash`.
    pub(crate) async fn kept_styles_read(&self, hash: &str) -> Result<KeptStyles, Refusal> {
        let folder = keeping::folder(&self.host().attachments_dir);
        let asked = hash.to_string();
        let read = tokio::task::spawn_blocking(move || keeping::read(&folder, &asked)).await.ok().flatten();
        read.ok_or_else(|| {
            Refusal::NotFound(WireError::raised(
                NO_SUCH_KEPT_STYLES,
                format!("Fleet holds no kept styles under {hash}."),
                self.run_id(),
            ))
        })
    }
}

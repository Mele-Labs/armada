//! `show_sketch`: a session draws the person a diagram, which is kept on its ledger.
//! `docs/concepts/session.md`, *Drawing a sketch*.
//!
//! **An artifact of form `sketch`, keyed by its title.** The ledger row is upserted on kind and
//! target, so a second call with the same title replaces the first where it stands, and a different
//! title is a second sketch. The scene is checked before anything is written: Bridge draws one that
//! fails as an empty frame and says nothing, so this is where the agent hears of it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionRecord, SessionReport, ShowSketch};

use crate::daemon::Fleet;

const SKETCH_REFUSED: &str = "fleet.sketch_refused";
const SKETCH_UNNAMED: &str = "fleet.sketch_unnamed";

/// What a sketch's ledger row is keyed by, apart from a file or a page an artifact may be.
const TARGET_PREFIX: &str = "sketch:";

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
    pub(crate) async fn sketch_shown(
        &self,
        caller: Option<api::Caller>,
        show: ShowSketch,
    ) -> Result<SessionRecord, Refusal> {
        let sketch = show
            .checked()
            .map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?;
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| show.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                self.hosted_refusal(
                    SKETCH_UNNAMED,
                    "a sketch needs a session to draw it for, and this call places none",
                )
            })?;
        let harness = {
            let store = self.store().lock().await;
            store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SKETCH_UNNAMED, "a session Fleet knows"))?
                .harness
        };
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(&id),
            fact: SessionFact::Attached {
                attachment: AttachmentReport {
                    kind: String::from("artifact"),
                    target: format!("{TARGET_PREFIX}{}", sketch.title),
                    detail: [
                        ("form".to_string(), "sketch".to_string()),
                        ("by".to_string(), "agent".to_string()),
                        ("title".to_string(), sketch.title),
                        ("scene".to_string(), sketch.scene),
                    ]
                    .into(),
                },
            },
        })
        .await
    }
}

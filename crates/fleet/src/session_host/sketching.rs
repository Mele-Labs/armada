//! `show_sketch`: a session draws the person a diagram, which is kept on its ledger.
//! `docs/concepts/session.md`, *Drawing a sketch*.
//!
//! **An artifact of form `sketch`, keyed by its title.** The ledger row is upserted on kind and
//! target, so a second call with the same title replaces the first where it stands, and a different
//! title is a second sketch. The scene is checked before anything is written: Bridge draws one that
//! fails as an empty frame and says nothing, so this is where the agent hears of it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionRecord, SessionReport, ShowSketch, Sketch};

use crate::daemon::Fleet;

const SKETCH_REFUSED: &str = "fleet.sketch_refused";
const SKETCH_UNNAMED: &str = "fleet.sketch_unnamed";
const SKETCH_AGAINST_UNKNOWN: &str = "fleet.sketch_against_unknown";

/// What a sketch's ledger row is keyed by, apart from a file or a page an artifact may be.
pub(crate) const TARGET_PREFIX: &str = "sketch:";

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
        let against = show
            .against_title()
            .map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?;
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| show.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                self.hosted_refusal(
                    SKETCH_UNNAMED,
                    "a sketch needs a job to draw it for, and this call places none",
                )
            })?;
        let (harness, held) = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SKETCH_UNNAMED, "a job Fleet knows"))?;
            let held = match against {
                Some(_) => self.ledger_row(&store, &session)?.attachments,
                None => Vec::new(),
            };
            (session.harness, held)
        };
        let sketch = match against {
            None => sketch,
            Some(title) => {
                let target = format!("{TARGET_PREFIX}{title}");
                let is_sketch = |one: &&ipc::Attachment| {
                    one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("sketch")
                };
                let earlier = held.iter().filter(is_sketch).find(|one| one.target == target);
                let Some(earlier) = earlier.and_then(|one| one.detail.get("scene")) else {
                    let titles: Vec<&str> = held
                        .iter()
                        .filter(is_sketch)
                        .filter_map(|one| one.detail.get("title").map(String::as_str))
                        .take(20)
                        .collect();
                    let held = match titles.is_empty() {
                        true => String::from("none"),
                        false => titles.join(", "),
                    };
                    return Err(self.hosted_refusal(
                        SKETCH_AGAINST_UNKNOWN,
                        &format!(
                            "this job holds no sketch titled `{title}` to read this one against. \
                             Sketches it holds: {held}. Draw that one first, or leave `against` out"
                        ),
                    ));
                };
                sketch
                    .read_against(&title, earlier)
                    .map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?
            }
        };
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(&id),
            fact: SessionFact::Attached { attachment: sketch_attachment(sketch) },
        })
        .await
    }
}

/// A sketch as the ledger keeps it: the artifact a title replaces where it stands. A plan's sketch is
/// kept the same way, so the scene is on the ledger once and the plan names it.
pub(crate) fn sketch_attachment(sketch: Sketch) -> AttachmentReport {
    AttachmentReport {
        kind: String::from("artifact"),
        target: format!("{TARGET_PREFIX}{}", sketch.title),
        detail: [
            ("form".to_string(), "sketch".to_string()),
            ("by".to_string(), "agent".to_string()),
            ("title".to_string(), sketch.title),
            ("scene".to_string(), sketch.scene),
        ]
        .into_iter()
        .chain(sketch.against.map(|kept| ("against".to_string(), kept)))
        .collect(),
    }
}

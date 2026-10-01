//! A Picture a person pastes onto a Studio: the image alone, kept the way a
//! Note's frame is. Decided with the owner, 28 Sep 2026.
//!
//! **The same keeping as `capture_studio_note`'s**, through
//! [`Fleet::frame_kept`]: the staged PNG is copied into the Studio's own
//! directory under the node's id, refused over the cap and refused where it
//! will not read. So a Picture is served by `get_studio_frame` and goes with
//! its node exactly as a captured Note's frame does.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Redirector, Refusal};
use core_model::{StudioEdgeId, StudioNode, StudioNodeContent, StudioNodeId, StudioNodeKind};
use ipc::{ManifestId, StagedFrame, StudioPosition};

use crate::daemon::Fleet;
use crate::studios::author;

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
    /// Keep the staged frame, then write the Picture naming it.
    ///
    /// **Who may add one is asked first, and the Studio is held next**, so a
    /// refused add leaves no file behind. The node's id is minted before the
    /// frame is kept because the file is named for it. A write the store then
    /// refuses — a `produced_by` naming no node of this Studio — takes the
    /// file back out, since nothing would name it.
    pub(crate) async fn picture_added(
        &self,
        studio_id: ipc::StudioId,
        staged: StagedFrame,
        position: StudioPosition,
        produced_by: Option<ipc::StudioNodeId>,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.addable_by(StudioNodeKind::Picture, by)?;
        let id = studio_id.to_domain();
        {
            let store = self.store().lock().await;
            self.studio_held(&store, &id, within.as_ref())?;
        }
        let node_id = StudioNodeId::carried(self.mint().ulid());
        let frame = self.frame_kept(&id, &node_id, staged)?;
        let kept = self.studio_frames(&id).join(&frame.filename);
        let at = self.now();
        let node = StudioNode::added(
            node_id,
            StudioNodeContent::Picture { frame },
            position.to_domain(),
            at.clone(),
            author(by),
        );
        let produced_by =
            produced_by.map(|from| (from.to_domain(), StudioEdgeId::carried(self.mint().ulid())));
        let written = self
            .written(&studio_id, within, |store, id| {
                let produced_by = produced_by
                    .as_ref()
                    .map(|(from, edge)| (from, edge.clone()));
                store.add_studio_node(id, &node, produced_by, &at)
            })
            .await;
        if written.is_err() {
            let _ = std::fs::remove_file(kept);
        }
        written
    }
}

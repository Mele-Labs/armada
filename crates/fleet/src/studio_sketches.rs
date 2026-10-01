//! A Sketch: the dispatch composer's pad, kept on a Studio. Decided with the
//! owner, 1 Oct 2026.
//!
//! **Its pictures are kept the way a Picture's is**, through
//! [`Fleet::frame_kept_as`]: each staged PNG is copied into the Studio's own
//! directory, under the cap, named by Fleet — `<node>-<minted>.png`, so no two
//! pictures and no redraw ever share a name. A picture arriving with nothing
//! staged keeps the frame the node already holds under its id, read off the
//! record, which is what stops a request naming a file of its choosing.
//!
//! **The record goes first and the files follow it**, as removing a node does:
//! a frame the new drawing no longer names is deleted only once it is written,
//! and a write Fleet refuses takes back only the frames it had just kept.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Redirector, Refusal};
use core_model::{
    CaptureFrame, Drawing, SketchDrawing, StudioEdgeId, StudioId, StudioNode, StudioNodeContent,
    StudioNodeId, StudioNodeKind,
};
use ipc::{EditStudioSketch, ManifestId, SketchDrawn, StagedFrame, StudioPosition};

use crate::daemon::Fleet;
use crate::promoting::NOT_A_SKETCH;
use crate::studios::{author, NODE_BLANK};

/// A picture written with nothing staged, under an id the Sketch keeps no
/// picture under. A 422.
const PICTURE_NOT_KEPT: &str = "fleet.studio_sketch_picture_not_kept";

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
    /// A Sketch placed by hand, with what was drawn on its pad.
    ///
    /// **Who may add one and whether anything is drawn are asked first, and
    /// the Studio is held next**, so a refused add leaves no file behind.
    pub(crate) async fn sketch_added(
        &self,
        studio_id: ipc::StudioId,
        drawn: SketchDrawn,
        position: StudioPosition,
        produced_by: Option<ipc::StudioNodeId>,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.addable_by(StudioNodeKind::Sketch, by)?;
        let drawn = self.something_drawn(drawn.to_domain())?;
        let id = studio_id.to_domain();
        {
            let store = self.store().lock().await;
            self.studio_held(&store, &id, within.as_ref())?;
        }
        let node_id = StudioNodeId::carried(self.mint().ulid());
        let (drawing, fresh) = self.drawing_kept(&id, &node_id, drawn, None)?;
        let at = self.now();
        let node = StudioNode::added(
            node_id,
            StudioNodeContent::Sketch { drawing },
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
            gone(fresh);
        }
        written
    }

    /// `edit_studio_sketch`: the whole drawing a person left on the pad.
    pub(crate) async fn sketch_redrawn(
        &self,
        studio_id: ipc::StudioId,
        edit: EditStudioSketch,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let drawn = self.something_drawn(edit.drawing.to_domain())?;
        let id = studio_id.to_domain();
        let node_id = edit.node_id.to_domain();
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &id, within.as_ref())?
        };
        let node = self.node_on(&graph, &node_id)?;
        let before: Vec<String> = node
            .content()
            .frames()
            .into_iter()
            .map(|frame| frame.filename.clone())
            .collect();
        // Asked of the kind before anything is kept, so a redraw of a Note
        // leaves no file behind.
        let StudioNodeContent::Sketch { drawing: held } = node.content() else {
            return Err(self.studio_unacceptable(
                NOT_A_SKETCH,
                format!("a {} node is not drawn on", node.kind().as_wire()),
            ));
        };
        let (drawing, fresh) = self.drawing_kept(&id, &node_id, drawn, Some(held))?;
        let after: Vec<String> = drawing
            .pictures()
            .iter()
            .map(|one| one.frame.filename.clone())
            .collect();
        let redrawn = match node.redrawn(drawing) {
            Ok(redrawn) => redrawn,
            Err(fault) => {
                gone(fresh);
                return Err(self.not_rewritable(NOT_A_SKETCH, fault));
            }
        };
        let at = self.now();
        let written = self
            .written(&studio_id, within, |store, id| {
                store.keep_rewritten(id, &redrawn, &at)
            })
            .await;
        match &written {
            Err(_) => gone(fresh),
            Ok(_) => gone(
                before
                    .iter()
                    .filter(|filename| !after.contains(filename))
                    .filter_map(|filename| self.kept_frame(&id, filename))
                    .collect(),
            ),
        }
        written
    }

    /// The drawing, refused where nothing is on it: a Sketch with nothing
    /// drawn is a blank node, and taking one off the board is a delete.
    fn something_drawn(
        &self,
        drawn: Drawing<Option<StagedFrame>>,
    ) -> Result<Drawing<Option<StagedFrame>>, Refusal> {
        if drawn.is_empty() {
            return Err(self.studio_unacceptable(
                NODE_BLANK,
                String::from("a sketch node's `drawing` cannot be blank"),
            ));
        }
        Ok(drawn)
    }

    /// Every picture's frame, kept: a staged one copied in under a name Fleet
    /// mints, one with nothing staged read off what `held` already keeps
    /// under its id. Answers the drawing and the files it just wrote, which a
    /// refusal here has already taken back.
    fn drawing_kept(
        &self,
        studio_id: &StudioId,
        node_id: &StudioNodeId,
        drawn: Drawing<Option<StagedFrame>>,
        held: Option<&SketchDrawing>,
    ) -> Result<(SketchDrawing, Vec<PathBuf>), Refusal> {
        let mut fresh = Vec::new();
        let kept = drawn.kept(|picture| -> Result<CaptureFrame, Refusal> {
            match &picture.frame {
                Some(staged) => {
                    let filename =
                        format!("{}-{}.png", node_id.as_str(), self.mint().ulid().as_str());
                    let frame = self.frame_kept_as(studio_id, filename, staged.clone())?;
                    fresh.extend(self.kept_frame(studio_id, &frame.filename));
                    Ok(frame)
                }
                None => held
                    .and_then(|drawing| drawing.picture(&picture.id))
                    .map(|one| one.frame.clone())
                    .ok_or_else(|| {
                        self.studio_unacceptable(
                            PICTURE_NOT_KEPT,
                            format!(
                                "picture `{}` carries no staged file, and this sketch keeps no \
                                 picture by that id",
                                picture.id
                            ),
                        )
                    }),
            }
        });
        match kept {
            Ok(drawing) => Ok((drawing, fresh)),
            Err(refusal) => {
                gone(fresh);
                Err(refusal)
            }
        }
    }
}

/// Files nothing names any more. One that will not go is not worth failing a
/// write that already happened, or one already refused.
fn gone(paths: Vec<PathBuf>) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

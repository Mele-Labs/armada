//! A Sketch's drawing on the wire, read and written. Since 20.0: a Studio's
//! Sketch is the dispatch composer's pad, decided with the owner 1 Oct 2026.
//!
//! **Two shapes, for the reason a Picture has two.** Read, a picture names the
//! frame Fleet kept for it. Written, a picture carries the file Bridge's main
//! staged, or nothing — which names the picture of that id the node already
//! keeps. **No write names a kept frame**: a body carrying `frame` on a picture
//! does not decode, so a client never chooses a file `get_studio_frame` opens.
//!
//! **Checked on decode, by the domain's own rule**, so a join to nothing, an id
//! on two parts or a one-point stroke is refused before Fleet sees the request.

use serde::{Deserialize, Serialize};

use crate::capturing::{CaptureFrame, StagedFrame};

/// One box: where a person put it, and the words in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchBox {
    pub id: String,
    pub x: i64,
    pub y: i64,
    pub body: String,
}

/// Two parts joined, boxes or pictures, in the direction drawn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchJoin {
    pub id: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchPoint {
    pub x: i64,
    pub y: i64,
}

/// A line drawn by hand, as its points.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchStroke {
    pub id: String,
    pub points: Vec<SketchPoint>,
}

/// A picture on a Sketch as a read answers it: the frame Fleet kept, fetched
/// from `get_studio_frame` by its node and `?picture=` its id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchPicture {
    pub id: String,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub frame: CaptureFrame,
}

/// A picture on a Sketch as a write carries it. `staged` is a new image
/// Bridge's main wrote; absent keeps the image the node already holds under
/// this `id`, and is refused where it holds none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchPictureDrawn {
    pub id: String,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged: Option<StagedFrame>,
}

/// The four parts, unchecked, as they arrive.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Parts<P> {
    boxes: Vec<SketchBox>,
    joins: Vec<SketchJoin>,
    strokes: Vec<SketchStroke>,
    pictures: Vec<P>,
}

/// A Sketch's drawing, as `get_studio` reads it. **Only decoding and the
/// domain make one**, so every one held is a drawing the domain accepts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Parts<SketchPicture>")]
pub struct SketchDrawing {
    boxes: Vec<SketchBox>,
    joins: Vec<SketchJoin>,
    strokes: Vec<SketchStroke>,
    pictures: Vec<SketchPicture>,
}

/// A Sketch's drawing, as `add_studio_node` and `edit_studio_sketch` carry it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Parts<SketchPictureDrawn>")]
pub struct SketchDrawn {
    boxes: Vec<SketchBox>,
    joins: Vec<SketchJoin>,
    strokes: Vec<SketchStroke>,
    pictures: Vec<SketchPictureDrawn>,
}

fn boxes_of(boxes: &[SketchBox]) -> Vec<core_model::SketchBox> {
    boxes
        .iter()
        .map(|one| core_model::SketchBox {
            id: one.id.clone(),
            x: one.x,
            y: one.y,
            body: one.body.clone(),
        })
        .collect()
}

fn joins_of(joins: &[SketchJoin]) -> Vec<core_model::SketchJoin> {
    joins
        .iter()
        .map(|one| core_model::SketchJoin {
            id: one.id.clone(),
            from: one.from.clone(),
            to: one.to.clone(),
        })
        .collect()
}

fn strokes_of(strokes: &[SketchStroke]) -> Vec<core_model::SketchStroke> {
    strokes
        .iter()
        .map(|one| core_model::SketchStroke {
            id: one.id.clone(),
            points: one
                .points
                .iter()
                .map(|point| core_model::SketchPoint {
                    x: point.x,
                    y: point.y,
                })
                .collect(),
        })
        .collect()
}

fn on_the_wire<F, P>(
    drawing: &core_model::Drawing<F>,
    picture: impl Fn(&core_model::SketchPicture<F>) -> P,
) -> (Vec<SketchBox>, Vec<SketchJoin>, Vec<SketchStroke>, Vec<P>) {
    (
        drawing
            .boxes()
            .iter()
            .map(|one| SketchBox {
                id: one.id.clone(),
                x: one.x,
                y: one.y,
                body: one.body.clone(),
            })
            .collect(),
        drawing
            .joins()
            .iter()
            .map(|one| SketchJoin {
                id: one.id.clone(),
                from: one.from.clone(),
                to: one.to.clone(),
            })
            .collect(),
        drawing
            .strokes()
            .iter()
            .map(|one| SketchStroke {
                id: one.id.clone(),
                points: one
                    .points
                    .iter()
                    .map(|point| SketchPoint {
                        x: point.x,
                        y: point.y,
                    })
                    .collect(),
            })
            .collect(),
        drawing.pictures().iter().map(picture).collect(),
    )
}

impl SketchDrawing {
    /// As the domain holds it. **Infallible**, because the wire one was
    /// checked by the same rule on its way in.
    pub fn to_domain(&self) -> core_model::SketchDrawing {
        core_model::SketchDrawing::drawn(
            boxes_of(&self.boxes),
            joins_of(&self.joins),
            strokes_of(&self.strokes),
            self.pictures
                .iter()
                .map(|one| core_model::SketchPicture {
                    id: one.id.clone(),
                    x: one.x,
                    y: one.y,
                    width: one.width,
                    height: one.height,
                    frame: core_model::CaptureFrame {
                        filename: one.frame.filename.clone(),
                        byte_size: one.frame.byte_size,
                        width: one.frame.width,
                        height: one.frame.height,
                    },
                })
                .collect(),
        )
        .expect("a wire drawing was checked by the domain's rule when it decoded")
    }

    pub fn pictures(&self) -> &[SketchPicture] {
        &self.pictures
    }
}

impl From<&core_model::SketchDrawing> for SketchDrawing {
    fn from(drawing: &core_model::SketchDrawing) -> SketchDrawing {
        let (boxes, joins, strokes, pictures) = on_the_wire(drawing, |one| SketchPicture {
            id: one.id.clone(),
            x: one.x,
            y: one.y,
            width: one.width,
            height: one.height,
            frame: CaptureFrame {
                filename: one.frame.filename.clone(),
                byte_size: one.frame.byte_size,
                width: one.frame.width,
                height: one.frame.height,
            },
        });
        SketchDrawing {
            boxes,
            joins,
            strokes,
            pictures,
        }
    }
}

impl SketchDrawn {
    /// As the domain holds it, each picture carrying its staged file or
    /// nothing. Infallible, for [`SketchDrawing::to_domain`]'s reason.
    pub fn to_domain(&self) -> core_model::Drawing<Option<StagedFrame>> {
        unchecked(&self.boxes, &self.joins, &self.strokes, &self.pictures)
            .expect("a wire drawing was checked by the domain's rule when it decoded")
    }
}

impl SketchDrawn {
    /// The drawing as a read answers it once every picture names the frame
    /// `frame` says was kept for it. Its parts are the ones already checked,
    /// so it is the same drawing.
    pub fn kept_as(&self, frame: impl Fn(&SketchPictureDrawn) -> CaptureFrame) -> SketchDrawing {
        SketchDrawing {
            boxes: self.boxes.clone(),
            joins: self.joins.clone(),
            strokes: self.strokes.clone(),
            pictures: self
                .pictures
                .iter()
                .map(|one| SketchPicture {
                    id: one.id.clone(),
                    x: one.x,
                    y: one.y,
                    width: one.width,
                    height: one.height,
                    frame: frame(one),
                })
                .collect(),
        }
    }
}

impl From<&core_model::Drawing<Option<StagedFrame>>> for SketchDrawn {
    fn from(drawing: &core_model::Drawing<Option<StagedFrame>>) -> SketchDrawn {
        let (boxes, joins, strokes, pictures) = on_the_wire(drawing, |one| SketchPictureDrawn {
            id: one.id.clone(),
            x: one.x,
            y: one.y,
            width: one.width,
            height: one.height,
            staged: one.frame.clone(),
        });
        SketchDrawn {
            boxes,
            joins,
            strokes,
            pictures,
        }
    }
}

fn unchecked(
    boxes: &[SketchBox],
    joins: &[SketchJoin],
    strokes: &[SketchStroke],
    pictures: &[SketchPictureDrawn],
) -> Result<core_model::Drawing<Option<StagedFrame>>, core_model::SketchMalformed> {
    core_model::Drawing::drawn(
        boxes_of(boxes),
        joins_of(joins),
        strokes_of(strokes),
        pictures
            .iter()
            .map(|one| core_model::SketchPicture {
                id: one.id.clone(),
                x: one.x,
                y: one.y,
                width: one.width,
                height: one.height,
                frame: one.staged.clone(),
            })
            .collect(),
    )
}

impl TryFrom<Parts<SketchPicture>> for SketchDrawing {
    type Error = String;

    fn try_from(parts: Parts<SketchPicture>) -> Result<SketchDrawing, String> {
        let drawing = SketchDrawing {
            boxes: parts.boxes,
            joins: parts.joins,
            strokes: parts.strokes,
            pictures: parts.pictures,
        };
        let placed: Vec<SketchPictureDrawn> = drawing
            .pictures
            .iter()
            .map(|one| SketchPictureDrawn {
                id: one.id.clone(),
                x: one.x,
                y: one.y,
                width: one.width,
                height: one.height,
                staged: None,
            })
            .collect();
        unchecked(&drawing.boxes, &drawing.joins, &drawing.strokes, &placed)
            .map_err(|why| why.to_string())?;
        Ok(drawing)
    }
}

impl TryFrom<Parts<SketchPictureDrawn>> for SketchDrawn {
    type Error = String;

    fn try_from(parts: Parts<SketchPictureDrawn>) -> Result<SketchDrawn, String> {
        unchecked(&parts.boxes, &parts.joins, &parts.strokes, &parts.pictures)
            .map_err(|why| why.to_string())?;
        Ok(SketchDrawn {
            boxes: parts.boxes,
            joins: parts.joins,
            strokes: parts.strokes,
            pictures: parts.pictures,
        })
    }
}

/// `edit_studio_sketch`: the drawing a person left on a Sketch's pad.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditStudioSketch {
    pub node_id: crate::StudioNodeId,
    pub drawing: SketchDrawn,
}

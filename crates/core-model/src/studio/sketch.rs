//! A Sketch: the drawing the dispatch composer's pad makes — boxes, the joins
//! between them, lines drawn by hand and pictures pasted on. Decided with the
//! owner, 1 Oct 2026: a Studio's Sketch and the pad are one drawing.
//!
//! **Generic over what a picture carries.** On a Studio it is the frame Fleet
//! kept; on its way in it is whatever the wire hands over before Fleet keeps
//! one. Both are checked by one constructor, so a drawing Fleet stores and one
//! a request carries are refused for the same things.
//!
//! **Whole pad units.** The pad rounds every place it reports, so a coordinate
//! here is an integer and the node stays `Eq`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::CaptureFrame;

/// One box: where a person put it, and the words in it. **No size**: the pad
/// draws a box at one width and grows it down the page with its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SketchBox {
    pub id: String,
    pub x: i64,
    pub y: i64,
    pub body: String,
}

/// Two things joined, boxes or pictures, in the direction a person drew it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SketchJoin {
    pub id: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SketchPoint {
    pub x: i64,
    pub y: i64,
}

/// A line drawn by hand, kept as its points so it can be redrawn and undone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SketchStroke {
    pub id: String,
    pub points: Vec<SketchPoint>,
}

/// A picture pasted onto the drawing, at the place and size it is drawn at.
/// `frame` is the image: kept by Fleet on a Studio, not yet kept on the wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SketchPicture<F> {
    pub id: String,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub frame: F,
}

/// What a drawing is refused for. Each names the part at fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SketchMalformed {
    /// A box, join, stroke or picture with no id.
    BlankId,
    /// One id on two parts. A join names a box or a picture by id, so two
    /// holding one would leave it naming either.
    IdTwice {
        id: String,
    },
    /// A join whose end names no box or picture on the drawing.
    JoinToNothing {
        join: String,
        end: String,
    },
    JoinToItself {
        join: String,
    },
    /// A stroke of fewer than two points, which draws nothing.
    StrokeTooShort {
        stroke: String,
    },
    /// A picture drawn at no size.
    PictureWithNoSize {
        picture: String,
    },
}

impl fmt::Display for SketchMalformed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SketchMalformed::BlankId => out.write_str("a part of the drawing has a blank id"),
            SketchMalformed::IdTwice { id } => {
                write!(out, "`{id}` names two parts of the drawing")
            }
            SketchMalformed::JoinToNothing { join, end } => {
                write!(
                    out,
                    "join `{join}` ends at `{end}`, which is on no box or picture"
                )
            }
            SketchMalformed::JoinToItself { join } => {
                write!(out, "join `{join}` joins a part to itself")
            }
            SketchMalformed::StrokeTooShort { stroke } => {
                write!(out, "stroke `{stroke}` has fewer than two points")
            }
            SketchMalformed::PictureWithNoSize { picture } => {
                write!(out, "picture `{picture}` is drawn at no size")
            }
        }
    }
}

/// The drawing, checked. **Only [`Drawing::drawn`] makes one**, so every
/// drawing held anywhere has joins that reach something and ids that name one
/// part each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawing<F> {
    boxes: Vec<SketchBox>,
    joins: Vec<SketchJoin>,
    strokes: Vec<SketchStroke>,
    pictures: Vec<SketchPicture<F>>,
}

/// A drawing as a Studio keeps it: every picture a frame Fleet kept.
pub type SketchDrawing = Drawing<CaptureFrame>;

impl<F> Drawing<F> {
    pub fn drawn(
        boxes: Vec<SketchBox>,
        joins: Vec<SketchJoin>,
        strokes: Vec<SketchStroke>,
        pictures: Vec<SketchPicture<F>>,
    ) -> Result<Drawing<F>, SketchMalformed> {
        let mut seen: Vec<&str> = Vec::new();
        let ids = boxes
            .iter()
            .map(|one| one.id.as_str())
            .chain(pictures.iter().map(|one| one.id.as_str()))
            .chain(joins.iter().map(|one| one.id.as_str()))
            .chain(strokes.iter().map(|one| one.id.as_str()));
        for id in ids {
            if id.trim().is_empty() {
                return Err(SketchMalformed::BlankId);
            }
            if seen.contains(&id) {
                return Err(SketchMalformed::IdTwice {
                    id: String::from(id),
                });
            }
            seen.push(id);
        }
        let joinable = |id: &str| {
            boxes.iter().any(|one| one.id == id) || pictures.iter().any(|one| one.id == id)
        };
        for join in &joins {
            if join.from == join.to {
                return Err(SketchMalformed::JoinToItself {
                    join: join.id.clone(),
                });
            }
            for end in [&join.from, &join.to] {
                if !joinable(end) {
                    return Err(SketchMalformed::JoinToNothing {
                        join: join.id.clone(),
                        end: end.clone(),
                    });
                }
            }
        }
        if let Some(stroke) = strokes.iter().find(|one| one.points.len() < 2) {
            return Err(SketchMalformed::StrokeTooShort {
                stroke: stroke.id.clone(),
            });
        }
        if let Some(picture) = pictures
            .iter()
            .find(|one| one.width <= 0 || one.height <= 0)
        {
            return Err(SketchMalformed::PictureWithNoSize {
                picture: picture.id.clone(),
            });
        }
        Ok(Drawing {
            boxes,
            joins,
            strokes,
            pictures,
        })
    }

    pub fn boxes(&self) -> &[SketchBox] {
        &self.boxes
    }
    pub fn joins(&self) -> &[SketchJoin] {
        &self.joins
    }
    pub fn strokes(&self) -> &[SketchStroke] {
        &self.strokes
    }
    pub fn pictures(&self) -> &[SketchPicture<F>] {
        &self.pictures
    }

    /// Nothing drawn. A join alone cannot be, since it needs its two ends.
    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty() && self.strokes.is_empty() && self.pictures.is_empty()
    }

    /// The same drawing with each picture's frame replaced by `keep`'s answer,
    /// stopping at the first refusal. Ids and places are untouched, so it is
    /// still the drawing that was checked.
    pub fn kept<G, E>(
        self,
        mut keep: impl FnMut(&SketchPicture<F>) -> Result<G, E>,
    ) -> Result<Drawing<G>, E> {
        let mut pictures = Vec::with_capacity(self.pictures.len());
        for picture in &self.pictures {
            let frame = keep(picture)?;
            pictures.push(SketchPicture {
                id: picture.id.clone(),
                x: picture.x,
                y: picture.y,
                width: picture.width,
                height: picture.height,
                frame,
            });
        }
        Ok(Drawing {
            boxes: self.boxes,
            joins: self.joins,
            strokes: self.strokes,
            pictures,
        })
    }
}

impl SketchDrawing {
    /// A drawing of one box holding `body` — what a Sketch written as text
    /// before 1 Oct 2026 became.
    pub fn one_box(body: String) -> SketchDrawing {
        Drawing {
            boxes: alloc::vec![SketchBox {
                id: String::from("b1"),
                x: 0,
                y: 0,
                body,
            }],
            joins: Vec::new(),
            strokes: Vec::new(),
            pictures: Vec::new(),
        }
    }

    /// The picture `id` names, and its frame.
    pub fn picture(&self, id: &str) -> Option<&SketchPicture<CaptureFrame>> {
        self.pictures.iter().find(|one| one.id == id)
    }
}

//! What `add_studio_node` names: a node's content as a read answers it, except
//! a Picture, which arrives as the file Bridge's main staged (since 19.3), and
//! a Sketch, whose pictures arrive the same way (since 20.0).
//!
//! **A request never names a kept frame.** A kept frame is a file name Fleet
//! chose under the Studio's own directory, and `get_studio_frame` opens what a
//! node names — so a client that could write one would choose what Fleet
//! opens. A body with `"kind": "picture"` decodes as the staged shape alone,
//! with no other field beside it, and [`AddedContent`] holds neither a Picture
//! nor a Note whose capture names a frame, so no write reaches Fleet carrying
//! one. A captured Note's frame arrives staged, through `capture_studio_note`.

use serde::de::Error as _;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::capturing::StagedFrame;
use crate::studio::StudioNodeContent;
use crate::studio_sketch::SketchDrawn;

/// The two tags decoded apart from the rest.
const PICTURE: &str = "picture";
const SKETCH: &str = "sketch";

/// A node's content as `add_studio_node` carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudioNodeAdded {
    /// `{"kind":"picture","staged":{…}}`. Fleet copies the staged PNG into the
    /// Studio's keeping and writes the Picture naming the file it kept.
    Picture { staged: StagedFrame },
    /// `{"kind":"sketch","drawing":{…}}`, each picture carrying its staged
    /// file. Fleet keeps each and writes the Sketch naming what it kept.
    Sketch { drawing: SketchDrawn },
    /// Any other kind, in the shape `get_studio` reads it back in.
    Content(AddedContent),
}

/// A node's content that names no kept frame: not a Picture, and not a Note
/// whose capture carries `frame`. **The field is private**, and both ways in —
/// the decoder and [`StudioNodeAdded`]'s `TryFrom` — refuse either.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddedContent(StudioNodeContent);

impl AddedContent {
    /// The content, as the domain holds it.
    pub fn to_domain(&self) -> core_model::StudioNodeContent {
        self.0.to_domain()
    }
}

impl From<AddedContent> for StudioNodeContent {
    fn from(added: AddedContent) -> StudioNodeContent {
        added.0
    }
}

impl TryFrom<StudioNodeContent> for StudioNodeAdded {
    /// Content naming a kept frame, handed back: the frame is Fleet's to name.
    type Error = StudioNodeContent;

    fn try_from(content: StudioNodeContent) -> Result<StudioNodeAdded, StudioNodeContent> {
        match names_a_kept_frame(&content) {
            Some(_) => Err(content),
            None => Ok(StudioNodeAdded::Content(AddedContent(content))),
        }
    }
}

/// Which field names a kept frame, on content that does — the one question
/// both ways into [`AddedContent`] ask.
fn names_a_kept_frame(content: &StudioNodeContent) -> Option<&'static str> {
    match content {
        StudioNodeContent::Picture { .. } => Some("frame"),
        StudioNodeContent::Sketch { drawing } if !drawing.pictures().is_empty() => {
            Some("drawing.pictures.frame")
        }
        StudioNodeContent::Note {
            capture: Some(capture),
            ..
        } if capture.frame.is_some() => Some("capture.frame"),
        _ => None,
    }
}

/// A Sketch as written: its drawing, and nothing beside it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SketchWritten {
    #[serde(rename = "kind")]
    _kind: serde::de::IgnoredAny,
    drawing: SketchDrawn,
}

/// The staged shape, and nothing beside it — a `frame` too is refused.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PictureStaged {
    #[serde(rename = "kind")]
    _kind: serde::de::IgnoredAny,
    staged: StagedFrame,
}

impl<'de> Deserialize<'de> for StudioNodeAdded {
    /// **Read by its tag first**, so a Picture is never tried as the read
    /// shape: a body naming `frame` and no `staged` is refused here rather
    /// than decoded as a frame a client claimed.
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let body = Map::<String, Value>::deserialize(input)?;
        if body.get("kind").and_then(Value::as_str) == Some(PICTURE) {
            let picture = PictureStaged::deserialize(Value::Object(body)).map_err(|why| {
                D::Error::custom(format!(
                    "a picture is added as the file Bridge staged, `staged`, and nothing else: \
                     {why}"
                ))
            })?;
            return Ok(StudioNodeAdded::Picture {
                staged: picture.staged,
            });
        }
        if body.get("kind").and_then(Value::as_str) == Some(SKETCH) {
            let sketch = SketchWritten::deserialize(Value::Object(body)).map_err(|why| {
                D::Error::custom(format!(
                    "a sketch is added as its drawing, each picture its staged file: {why}"
                ))
            })?;
            return Ok(StudioNodeAdded::Sketch {
                drawing: sketch.drawing,
            });
        }
        let content =
            StudioNodeContent::deserialize(Value::Object(body)).map_err(D::Error::custom)?;
        if let Some(field) = names_a_kept_frame(&content) {
            return Err(D::Error::custom(format!(
                "`{field}` is a file Fleet kept and names itself, so no request may carry one: a \
                 captured Note's frame is staged, through `capture_note`"
            )));
        }
        Ok(StudioNodeAdded::Content(AddedContent(content)))
    }
}

impl Serialize for StudioNodeAdded {
    fn serialize<S: Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        match self {
            StudioNodeAdded::Picture { staged } => {
                let mut map = out.serialize_map(Some(2))?;
                map.serialize_entry("kind", PICTURE)?;
                map.serialize_entry("staged", staged)?;
                map.end()
            }
            StudioNodeAdded::Sketch { drawing } => {
                let mut map = out.serialize_map(Some(2))?;
                map.serialize_entry("kind", SKETCH)?;
                map.serialize_entry("drawing", drawing)?;
                map.end()
            }
            StudioNodeAdded::Content(AddedContent(content)) => content.serialize(out),
        }
    }
}

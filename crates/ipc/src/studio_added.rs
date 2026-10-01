//! What `add_studio_node` names: a node's content as a read answers it, except
//! a Picture, which arrives as the file Bridge's main staged. Since 19.2.
//!
//! **A request never names a kept frame.** A kept frame is a file name Fleet
//! chose under the Studio's own directory, and `get_studio_frame` opens what a
//! node names — so a client that could write one would choose what Fleet
//! opens. A body with `"kind": "picture"` decodes as the staged shape alone,
//! with no other field beside it, and [`AddedContent`] cannot hold a Picture,
//! so no write reaches Fleet carrying one.

use serde::de::Error as _;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::capturing::StagedFrame;
use crate::studio::StudioNodeContent;

/// The one tag decoded apart from the rest.
const PICTURE: &str = "picture";

/// A node's content as `add_studio_node` carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudioNodeAdded {
    /// `{"kind":"picture","staged":{…}}`. Fleet copies the staged PNG into the
    /// Studio's keeping and writes the Picture naming the file it kept.
    Picture { staged: StagedFrame },
    /// Any other kind, in the shape `get_studio` reads it back in.
    Content(AddedContent),
}

/// A node's content that is not a Picture. **The field is private**, and both
/// ways in — the decoder and [`StudioNodeAdded`]'s `TryFrom` — refuse a
/// Picture.
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
    /// A Picture as read, handed back: its frame is Fleet's to name.
    type Error = StudioNodeContent;

    fn try_from(content: StudioNodeContent) -> Result<StudioNodeAdded, StudioNodeContent> {
        match content {
            StudioNodeContent::Picture { .. } => Err(content),
            content => Ok(StudioNodeAdded::Content(AddedContent(content))),
        }
    }
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
        StudioNodeContent::deserialize(Value::Object(body))
            .map(|content| StudioNodeAdded::Content(AddedContent(content)))
            .map_err(D::Error::custom)
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
            StudioNodeAdded::Content(AddedContent(content)) => content.serialize(out),
        }
    }
}

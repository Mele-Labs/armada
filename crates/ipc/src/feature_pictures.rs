//! The pictures of what a feature looks like in the running app: the act a Job attaches one with, his "Not this feature",
//! and the row a page carries. `docs/concepts/feature-catalog.md`, *Pictures*.

use serde::{Deserialize, Serialize};

use crate::{ids::Instant, SessionId};

/// Who took a picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PictureBy {
    /// A Job, of a screen it built: `add_feature_picture`.
    Job,
    /// The librarian, on a run.
    Librarian,
}

/// One picture of a feature. Its bytes are `get_feature_picture`'s.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturePicture {
    pub id: String,
    /// The page's address as it was taken. Absent for a screen that has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The screen's name where it has no address, as a native walk is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<String>,
    /// The commit the app was built from. Absent where it could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub taken_at: Instant,
    /// In pixels.
    pub width: u32,
    pub height: u32,
    /// The file's size.
    pub bytes: u64,
    pub by: PictureBy,
    /// The Job that took it. Absent for the librarian's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
}

/// `add_feature_picture`: this Job keeps a picture of a screen under a feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddFeaturePicture {
    /// The feature's slug.
    pub feature: String,
    /// The PNG on this Mac, absolute: what `browser_screenshot` answered, or a native walk's `--shots` picture. Fleet copies it.
    pub path: String,
    /// The page's address. One of `url` and `screen` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The screen's name where it has no address: a native walk's name, as `--list-walks` names it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<String>,
    /// The project's id. **Read only where the Job's repository is in more than one project and the slug is in each.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `add_feature_picture` kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturePictureAdded {
    pub feature: String,
    pub picture: FeaturePicture,
}

/// `remove_feature_picture`: his "Not this feature" on a picture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveFeaturePicture {
    /// A `FeaturePicture.id`.
    pub picture: String,
    /// What he said is wrong with it, kept with the removal and as a rule of the catalog. Absent where he said nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// What `remove_feature_picture` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturePictureRemoved {
    /// The slug of the feature it was under.
    pub feature: String,
    pub picture: String,
    /// The feature's cover now. Absent where it has no picture left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    /// The catalog rule his `why` became, by id. Absent where he gave none, or the picture was removed before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

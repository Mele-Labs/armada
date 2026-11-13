//! The picture a Job last showed, which the native done card leads with.
//! `docs/practices/protocol/how-he-works-with-jobs-and-a-question-offered-as-pictures.md`.

use serde::{Deserialize, Serialize};

/// `get_lead_picture`: what the Job last showed that is a picture, newest first by when it was shown.
/// **Fleet names it and Bridge draws what it says**: a sketch by its title, a window by its address, an
/// image by the address `GET /sessions/shown` serves it at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LeadPicture {
    /// A `show_sketch`: the title the Job's record holds its scene under.
    Sketch { title: String },
    /// A `show_window`: the page's address.
    Window { url: String },
    /// A `show_file` of an image: `/sessions/shown?session_id=&path=`.
    Image { path: String },
}

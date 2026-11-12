//! A file of a Job, read for a client that cannot read the agent's disk: `get_repository_file`.
//!
//! **Fleet reads, the client draws.** Electron's file panel reads the disk itself; the native Bridge and
//! Pocket do not, so Fleet reads the file by the same rules and hands back the text or the picture. A
//! file it will not read is a refusal, never an answer.

use serde::{Deserialize, Serialize};

/// What `get_repository_file` answers: the file as text, or as a picture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepositoryFile {
    /// Strict UTF-8 with no NUL in its first 4096 bytes. `type` is `text/markdown` for `.md` and
    /// `.markdown`, `text/plain` for the rest.
    Text {
        text: String,
        #[serde(rename = "type")]
        media_type: String,
    },
    /// A picture, whatever its bytes: `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp` or `.svg`. `bytes` is
    /// base64, and `type` its media type.
    Picture {
        bytes: String,
        #[serde(rename = "type")]
        media_type: String,
    },
}

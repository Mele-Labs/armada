//! The local language model, as the one thing Fleet asks of it: a vector for each text.
//!
//! **One model, loaded once, for every reader.** A reader is a small head trained on these vectors,
//! so it is only valid for the model it was trained on: [`Embedder::model`] names that model, and a
//! head carries the name it expects.

use alloc::string::String;
use alloc::vec::Vec;

/// A text embedding model: texts in, one unit-length vector each out, in order. Blocking and
/// CPU-bound, so a caller on an async runtime runs it on a thread of its own.
pub trait Embedder: Send + Sync {
    /// The model and its pinned revision, as `<name>@<revision>`.
    fn model(&self) -> &str;

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded>;
}

/// Why nothing came back. The model's own words, for the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotEmbedded(pub String);

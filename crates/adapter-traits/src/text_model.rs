//! The local language model, as the two things Fleet asks of it: a vector for each text, and a
//! fine-tuned reading of each text.
//!
//! **One model, loaded once, for every reader.** A reader built on the vectors is valid only for the
//! model it was trained on, so [`TextModel::model`] names it.

use alloc::string::String;
use alloc::vec::Vec;

/// What a fine-tuned head of the model reads a text for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextReading {
    /// Whether an agent's reply asks the person to answer, decide or do something.
    ReplyAsks,
    /// Whether a Job's reply to a tell from Fleet turns it down.
    TellRefused,
}

/// A text model: texts in, one unit-length vector or one probability each out, in order. Blocking
/// and CPU-bound, so a caller on an async runtime runs it on a thread of its own.
pub trait TextModel: Send + Sync {
    /// The model and its pinned revision, as `<name>@<revision>`.
    fn model(&self) -> &str;

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded>;

    /// The probability that each text is what `reading` looks for.
    fn read(&self, reading: TextReading, texts: &[&str]) -> Result<Vec<f32>, NotEmbedded>;
}

/// Why nothing came back. The model's own words, for the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotEmbedded(pub String);

//! What `start_lesson_session` answers. Since 23.76.
//! `docs/concepts/retro.md`, *Start Session*.

use serde::{Deserialize, Serialize};

use crate::Lesson;

/// A Session started for a retro item, and the first message Fleet sent it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonSessionStarted {
    /// The item as it now stands: `agreed`, with `session_proposed` set.
    pub lesson: Lesson,
    /// The Session that was started. It holds no process until a message is
    /// sent.
    pub session_id: String,
    /// The first message, written for the agent who will do the work, **as it
    /// was sent**. The Session runs on it at once.
    pub brief: String,
}

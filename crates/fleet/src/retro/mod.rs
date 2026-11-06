//! A Session's retro: once it has run, what got in the way while it ran, and
//! whose way. `docs/concepts/retro.md` is the concept.
//!
//! [`record`] holds the helpers a record is cited with; [`session_record`]
//! assembles it and does no I/O; [`session_writing`] asks a model once per
//! Session, off the turn; [`serving`] answers `list_lessons`; [`agreeing`] and
//! [`starting`] answer a person's disagree and start. **Nothing here moves a
//! Session or reaches an agent**, and the one thing a retro does, a person's
//! pressing Start, is to start a Session on the item.

pub mod agreeing;
pub mod asking;
pub mod live;
pub mod record;
pub mod reviewing;
pub mod serving;
pub mod session_agreeing;
pub mod session_files;
pub mod session_question;
pub mod session_record;
pub mod session_time;
pub mod session_writing;
pub mod starting;
pub mod timing_read;
pub mod streamed;
pub mod writing;

pub use agreeing::ARMADAS_OWN;
pub use asking::ASK;
pub use reviewing::REVIEW;
pub use session_question::SESSION_QUESTION;
pub use session_record::has_news;
pub use writing::{LANDS_IN, TEXTS};

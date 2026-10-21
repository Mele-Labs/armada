//! A Session's retro: once it has run, what got in the way while it ran, and
//! whose way. `docs/concepts/retro.md` is the concept.
//!
//! [`record`] holds the helpers a record is cited with; [`session_record`]
//! assembles it and does no I/O; [`session_writing`] asks a model once per
//! Session, off the turn; [`serving`] answers `list_lessons`; [`agreeing`] and
//! [`starting`] answer a person's disagree and start. **Nothing here moves a
//! Session or reaches an agent**, and the one thing a retro does, a person's
//! pressing Start, is to start a Session on the item.

mod agreeing;
mod asking;
pub(crate) mod live;
pub(crate) mod record;
mod reviewing;
mod serving;
mod session_agreeing;
mod session_files;
mod session_question;
mod session_record;
mod session_writing;
mod starting;
pub(crate) mod streamed;
mod writing;

pub(crate) use agreeing::ARMADAS_OWN;
pub(crate) use asking::ASK;
pub(crate) use reviewing::REVIEW;
pub(crate) use session_question::SESSION_QUESTION;
pub(crate) use writing::{LANDS_IN, TEXTS};

//! Helm: a hosted Session per repository, over the agent door.
//!
//! What the Helm Session is told and what it may do are here so the host and the door (`#941`) read
//! one answer. The Session itself is `session_host`'s, and `session` is the part that is Helm's:
//! the lookup, the first message that makes one, Start fresh and the `sent` row.

mod admitting;
mod asking;
mod brief;
mod composing;
mod conversation;
mod deciding;
mod permitting;
mod reach;
mod reading;
mod recording;
pub(crate) mod routing;
mod serving;
mod session;
mod thread;

pub use asking::{answering, blocked_folder, question_unanswered, restarted, unanswered, Asks, HelmAskHold, NotAnswerable, Said};
pub use brief::{brief, Brief, Voice};
pub(crate) use brief::{shipped, worded, VOICE};
pub use conversation::Conversations;
pub use deciding::{
    because, because_in_a_session, because_in_a_session_owning, guarded_in_a_session, pages_opened, Because, Folders,
    Guarded, Own,
};
pub use permitting::SHIPPED_ASK_HOLD;
pub use reach::{may, Authority};
pub use thread::what_was_said;

#[cfg(test)]
mod tests;

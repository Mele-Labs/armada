//! Helm: a hosted Session per repository, over the agent door.
//!
//! What the Helm Session is told and what it may do are here so the host and the door (`#941`) read
//! one answer. The Session itself is `session_host`'s, and `session` is the part that is Helm's:
//! the lookup, the first message that makes one, Start fresh and the `sent` row. The host that
//! carries a message per process, behind one interface, [`Hosting`], now carries a Job's Commander
//! (`commander*.rs`) and nothing else; it carried Helm's replies when this was `#939`'s.

mod admitting;
mod asking;
mod brief;
pub(crate) mod commander_brief;
mod commander;
mod commander_card;
mod composing;
mod conversation;
mod deciding;
mod hosting;
mod permitting;
mod reach;
mod recording;
mod rows;
pub(crate) mod routing;
mod serving;
mod session;
mod thread;
pub(crate) mod unanswered;

pub use asking::{answering, unanswered, Asks, HelmAskHold, NotAnswerable, Said};
pub use brief::{brief, Brief, Voice};
pub(crate) use brief::{shipped, worded, VOICE};
pub use conversation::{Commanded, ConversationKey, Conversations, Input};
pub use deciding::{because, because_in_a_session, because_in_a_session_owning, pages_opened, Because, Own};
#[cfg(test)]
pub(crate) use hosting::REPLY_BUDGET;
pub use hosting::{Carried, Carry, Carrying, Commander, Heard, Hosting, ProcessHost, Seat};
pub use permitting::SHIPPED_ASK_HOLD;
pub use reach::{may, Authority};
pub use thread::{what_was_said, THREAD};

#[cfg(test)]
mod tests;

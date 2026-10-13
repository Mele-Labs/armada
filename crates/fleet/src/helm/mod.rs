//! Helm: the conversation Fleet hosts for one repository, over the agent door.
//!
//! What a session is told and what it may do are here so the host and the door
//! (`#941`) read one answer. The conversation itself — one per repository,
//! resumed by id for each message — is `#939`'s, and its host sits behind one
//! interface, [`Hosting`], so a host elsewhere is a switch.

mod admitting;
mod asking;
mod brief;
pub(crate) mod commander_brief;
mod commander;
mod commander_card;
mod conversation;
mod deciding;
mod hosting;
mod permitting;
mod reach;
mod recording;
mod rows;
pub(crate) mod routing;
mod serving;
mod thread;
pub(crate) mod unanswered;

pub use asking::{answering, unanswered, Asks, HelmAskHold, NotAnswerable, Said};
pub use brief::{brief, Brief, Voice};
pub(crate) use brief::{shipped, worded, VOICE};
pub use conversation::{Commanded, ConversationKey, Conversations, Input};
pub use deciding::{because, because_in_a_session, pages_opened, Because};
#[cfg(test)]
pub(crate) use hosting::REPLY_BUDGET;
pub use hosting::{Carried, Carry, Carrying, Commander, Heard, Hosting, ProcessHost, Seat};
pub use permitting::SHIPPED_ASK_HOLD;
pub use reach::{may, Authority};
pub use thread::{what_was_said, THREAD};

#[cfg(test)]
mod tests;

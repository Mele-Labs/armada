//! The Operator's page: Your turn, the queue he works through, beside the digest Since you last looked
//! (`docs/concepts/operator.md`, "Its page is a queue the person works through"). It draws what
//! `operator_page` serves and derives nothing; `docs/practices/protocol/the-operators-page-fleet-keeps.md`
//! is the wire.
//!
//! | Piece | File |
//! |---|---|
//! | An item's kind, chip and glyph, and the head's pills | `face.rs` |
//! | The page's state, keys and presses, each answered at once | `page.rs` |
//! | The page drawn, and the pane from a Job | `view.rs` |
//! | The chat Ask opens | `chat.rs` |
//! | The box at the page's foot, whose send is a dispatch | `composer.rs` |
//! | The ideas the Operator kept for later | `parked.rs` |
//! | Another Mac waiting for his Confirm to pair, as a card atop Your turn | `pairing.rs` |
//! | The six routes, live or mock | `wire.rs` |
//! | The walk's Fleet and its scenes | `scenes.rs` |
//! | The screen that puts the page beside the Jobs screen | `screen.rs` |
//!
//! **A new item raises a toast in the shared shell, which this module does not draw.** The page emits
//! [`page::Arrived`] with the items that joined Your turn since it last read; subscribe on [`live`].
//!
//! **Switch on and Forget wait on Fleet's `operator.page_changed`**, which it sends when
//! `features.operator` changes and when a rule or hold is removed. Each draws its change at once and
//! reads nothing itself; the walk's mock sends no event, so there each reads the page in its place.

mod chat;
pub mod composer;
pub mod face;
pub mod page;
mod pairing;
mod parked;
pub mod scenes;
pub mod screen;
pub(crate) mod view;
mod wire;

pub use screen::{live, OperatorScreen};

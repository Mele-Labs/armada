//! The Operator's page: one card in front, the box under it with a Discuss tag, Waiting behind, and the feed Since you
//! last looked (`docs/concepts/operator.md`). The Operator is one long-running session that writes the cards; the page
//! draws what `operator_page` serves and derives nothing. `docs/practices/protocol/the-operators-page-fleet-keeps.md`
//! is the wire.
//!
//! | Piece | File |
//! |---|---|
//! | A card's kind chip, a feed entry's glyph, the head's pills and the words a pick is said in | `face.rs` |
//! | The page's state, keys and presses, each answered at once | `page.rs` |
//! | The page drawn: the head, the main column, the feed beside it | `view.rs` |
//! | The card in front, its options and footer, and the card folded to a line; a Job's Needs you draws the same card | `card.rs` |
//! | Waiting's list of the cards behind the front one | `waiting.rs` |
//! | The talk under a folded card, in the Operator's thread, with the cards it raises | `talk.rs` |
//! | The feed Since you last looked | `feed.rs` |
//! | The box under the card, whose send is a dispatch or, tagged, a talk about a card | `composer.rs` |
//! | The ideas the Operator kept for later | `parked.rs` |
//! | Another Mac waiting for his Confirm to pair, as a card atop the page | `pairing.rs` |
//! | The routes, live or mock | `wire.rs` |
//! | The walk's Fleet and its scenes | `scenes.rs` |
//! | The screen that puts the page beside the Jobs screen | `screen.rs` |
//!
//! **A new card raises a toast in the shared shell, which this module does not draw.** The page emits
//! [`page::Arrived`] with the cards that joined the queue since it last read; subscribe on [`live`].
//!
//! **Switch on waits on Fleet's `operator.page_changed`**, which it sends when `features.operator` changes. It draws
//! its change at once and reads nothing itself; the walk's mock sends no event, so there it reads the page in its place.

pub(crate) mod card;
pub mod composer;
pub mod face;
mod feed;
pub mod page;
mod pairing;
mod parked;
pub mod scenes;
pub mod screen;
mod talk;
pub(crate) mod view;
mod waiting;
mod wire;

pub use screen::{live, OperatorScreen};

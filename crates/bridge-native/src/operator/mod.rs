//! The Operator's page: one card in front, the box under it with a Discuss tag, Next up and the feed Since you
//! last looked (`docs/concepts/operator.md`). The Operator is one long-running session that writes the cards; the page
//! draws what `operator_page` serves and derives nothing. `docs/practices/protocol/the-operators-page-fleet-keeps.md`
//! is the wire.
//!
//! | Piece | File |
//! |---|---|
//! | A card's kind chip, a feed entry's glyph, the head's pills and the words a pick is said in | `face.rs` |
//! | The page's state, keys and presses, each answered at once | `page.rs` |
//! | The page drawn: the main panel (head, card, strip, talk and box) and Next up and the feed in a panel beside it | `view.rs` |
//! | The card in front, its options and footer, a card that did not settle, a picked card read only with its steps, and the card folded to a line; a Job's Needs you draws the same card | `card.rs` |
//! | Next up: the cards behind the front one, then the kept ideas, folded in the right column | `next_up.rs` |
//! | A kept idea brought to the front, drawn as a card with Start it and Remove | `idea.rs` |
//! | The talk under the card, the Operator's own thread with its About rows and the cards it raises, and the new-card strip | `talk.rs` |
//! | The feed Since you last looked, under Working: the cards the Operator is working on | `feed.rs` |
//! | The box under the card, whose send is a dispatch or, tagged, a talk about a card | `composer.rs` |
//! | The ideas the Operator kept for later: what Next up and the idea card read | `parked.rs` |
//! | Another Mac waiting for his Confirm to pair, as a card atop the page | `pairing.rs` |
//! | The routes, live or mock | `wire.rs` |
//! | The walk's Fleet and its scenes, with a card working and one that comes back | `scenes.rs` |
//! | The status card where nothing is in front, or the Operator is off or stopped: its state, the last act and its one press | `status.rs` |
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
mod idea;
mod next_up;
pub mod page;
mod pairing;
mod parked;
pub mod scenes;
pub mod screen;
pub mod status;
mod talk;
pub(crate) mod view;
mod wire;

pub use screen::{live, Front, OperatorScreen};
pub use status::session_of;

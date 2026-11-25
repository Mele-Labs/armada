//! The dispatch sheet and the strip a send leaves (`classic-final:apps/desktop/src/renderer/src/dispatch-panel/`).
//!
//! | Piece | File |
//! |---|---|
//! | What the sheet draws, as data a walk's mock fills | `face.rs` |
//! | The sheet's state, keys and acts | `sheet.rs` |
//! | The sheet drawn | `view.rs` |
//! | A sentence written into the box: its kinds, its state and keys, and its drawing | `story_kinds.rs`, `story.rs`, `story_view.rs` |
//! | The mock scenario and its scenes | `scenes.rs` |
//! | The sheet's face and its asks on a live Fleet | `live.rs` |
//! | The screen that mounts the sheet, the strip and the annotation layer over the Jobs screen | `screen.rs` |

pub mod face;
pub(crate) mod live;
pub mod scenes;
pub mod screen;
pub mod sheet;
pub mod story;
pub mod story_kinds;
mod story_view;
mod view;

pub use screen::{start_queued, DispatchScreen};

//! The dispatch sheet and the strip a send leaves (`apps/desktop/src/renderer/src/dispatch-panel/`).
//!
//! | Piece | File |
//! |---|---|
//! | What the sheet draws, as data a walk's mock fills | `face.rs` |
//! | The sheet's state, keys and acts | `sheet.rs` |
//! | The sheet drawn | `view.rs` |
//! | The mock scenario and its scenes | `scenes.rs` |
//! | The sheet's face and its asks on a live Fleet | `live.rs` |
//! | The screen that mounts the sheet, the strip and the annotation layer over the Jobs screen | `screen.rs` |

pub mod face;
pub(crate) mod live;
pub mod scenes;
pub mod screen;
pub mod sheet;
mod view;

pub use screen::{start_queued, DispatchScreen};

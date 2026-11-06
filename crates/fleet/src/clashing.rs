//! Telling two agents that their changes would conflict, from what they wrote. `docs/concepts/fleet.md`,
//! *Write clashes*.
//!
//! A write is seen from a hosted Session's gate and a terminal Session's mod, and goes into the table (`index`). A second holder on a file in the same repository
//! schedules a check, and git decides (`adapters::clash`): the same file edited in two places that
//! merge clean says nothing. **Nobody is paused or refused.**
//!
//! The checks ride the notice loop's tick (`notice_loop`), not a loop of their own. The table is
//! memory: a Fleet that restarts rebuilds it from the next writes.

pub mod checking;
pub mod file_change;
pub mod file_read;
pub mod index;
pub mod serving;
pub mod standing;
pub mod tap;
pub mod telling;

pub use checking::{base_of, changed_since_base};
pub use index::{Opposed, Side, Who, Writes};
pub use tap::repository_relative;
pub use telling::listed;

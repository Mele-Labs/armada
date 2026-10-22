//! Telling two agents that their changes would conflict, from what they wrote. `docs/concepts/fleet.md`,
//! *Write clashes*.
//!
//! A write is seen from a hosted Session's gate and a terminal Session's mod, and goes into the table (`index`). A second holder on a file in the same repository
//! schedules a check, and git decides (`adapters::clash`): the same file edited in two places that
//! merge clean says nothing. **Nobody is paused or refused.**
//!
//! The checks ride the notice loop's tick (`notice_loop`), not a loop of their own. The table is
//! memory: a Fleet that restarts rebuilds it from the next writes.

mod checking;
mod file_change;
mod file_read;
mod index;
mod serving;
mod standing;
mod tap;
mod telling;

pub(crate) use index::{Opposed, Side, Who, Writes};
pub(crate) use tap::{inside_worktree, repository_relative};
pub(crate) use telling::listed;

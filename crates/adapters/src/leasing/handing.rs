//! Handing a held slot from one agent session to another, as it stands: nothing
//! is committed, released or reset. `release_session_holds` with a session to
//! hand to is the only caller. `docs/concepts/session.md`, *Releasing what a
//! Session holds*.

use super::{Holder, ParkRefused, Pool, Record};

impl Pool {
    /// Write slot `number`'s lease as `to`'s, **only while `from` is who holds it**, named as
    /// [`Holder::said`] names it. The new holder is a Job of `to`'s id, the way a hosted session
    /// holds its own, since a session's process is not something Fleet can name for it.
    pub fn hand(&self, number: usize, from: &str, to: &str) -> Result<(), ParkRefused> {
        let slot = self.path_of(number);
        let Some(lock) = self.locked(number).map_err(ParkRefused::Vcs)? else {
            return Err(ParkRefused::Busy);
        };
        let path = self.record_path(number);
        let Some(mut record) = Record::read(&path) else {
            return Err(ParkRefused::NotLeased(slot));
        };
        if record.holder.said() != from {
            return Err(ParkRefused::HeldByAnother(record.holder.said()));
        }
        record.holder = Holder::hosted(to);
        // A Job's record names no session: the holder already does.
        record.session = None;
        record.write(&path).map_err(ParkRefused::Vcs)?;
        drop(lock);
        Ok(())
    }
}

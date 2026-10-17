//! What a run holds of the machine: a place in the line, the memory and disk
//! for it. The line itself is `places`.

use std::path::Path;
use std::sync::Arc;

use super::{Ask, Asking, ChecksAtOnce, Place, Places};
use crate::headroom::{Bytes, Headroom, Machine, Reading, Spare};

/// What a run asks before each command it starts: one or more of the
/// machine's places, and the memory and disk for it. #284, #1063, #1102.
///
/// **The first on the machine always starts**, so a short machine slows Checks
/// and never stops them. The headroom is taken when the run begins.
#[derive(Clone)]
pub struct Room {
    places: Places,
    asking: Asking,
    machine: Arc<dyn Machine>,
    headroom: Headroom,
}

/// A machine that never answers, so it never holds a Check back.
struct Unread;

impl Machine for Unread {
    fn read(&self) -> Option<Reading> {
        None
    }

    fn disk_free_at(&self, _path: &Path) -> Option<Bytes> {
        None
    }
}

impl Room {
    /// A gate's room with places of its own, sharing the machine with nothing.
    pub fn of(at_once: ChecksAtOnce, machine: Arc<dyn Machine>, headroom: Headroom) -> Room {
        Room::sharing(
            &Places::of(at_once),
            Asking::Gate,
            machine,
            headroom,
        )
    }

    /// A room in `places`, asking as `asking`.
    pub(crate) fn sharing(
        places: &Places,
        asking: Asking,
        machine: Arc<dyn Machine>,
        headroom: Headroom,
    ) -> Room {
        Room {
            places: places.clone(),
            asking,
            machine,
            headroom,
        }
    }

    /// Bounded by `at_once` and nothing else: the machine is never read. For a
    /// caller with no machine to ask, such as a test or the acceptance bench.
    pub fn ignoring_the_machine(at_once: ChecksAtOnce) -> Room {
        Room::of(
            at_once,
            Arc::new(Unread),
            Headroom::of(Spare::percent(0), Bytes::gibibytes(0)),
        )
    }

    /// Join the line for a place.
    pub(crate) fn ask(&self) -> Ask {
        self.places.ask(self.asking, 1)
    }

    /// The place `ask` waits for. `waiting` is told how many places are held
    /// each time it waits. **Safe to drop and call again.**
    pub(crate) async fn granted(&self, ask: &mut Ask, waiting: impl FnMut(usize)) -> Place {
        ask.granted(&self.machine, self.headroom, waiting).await
    }

    /// Wait in line and hold a place, for a heavy run of any kind.
    pub async fn place(&self) -> Place {
        let mut ask = self.ask();
        self.granted(&mut ask, |_| {}).await
    }
}

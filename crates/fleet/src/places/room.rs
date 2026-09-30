//! What a run holds of the machine: a place in the line, the memory and disk
//! for it, and how wide one command may run. The line itself is `places`.

use std::path::Path;
use std::sync::Arc;

use checks_runner::CheckWidth;

use super::{Ask, Asking, ChecksAtOnce, Place, Places};
use crate::headroom::{Bytes, Headroom, Machine, Reading, Spare};
use crate::ordering::Past;

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
    /// How long each Check took before, which decides which starts first.
    past: Past,
    /// How wide one command in this batch may run.
    ///
    /// **Here rather than threaded beside it**, because it is the same fact
    /// this type already carries: what of the machine a batch may take. The
    /// places bound how many run at once; this bounds what one of them spawns
    /// once it is running. A caller holding one already holds the other. #1444.
    width: CheckWidth,
}

impl Room {
    /// A gate's room with places of its own, sharing the machine with nothing.
    pub fn of(at_once: ChecksAtOnce, machine: Arc<dyn Machine>, headroom: Headroom) -> Room {
        Room::sharing(
            &Places::of(at_once),
            Asking::Gate,
            machine,
            headroom,
            // Sharing with nothing, so nothing divides it.
            CheckWidth::read(1),
        )
    }

    /// A room in `places`, asking as `asking`.
    pub(crate) fn sharing(
        places: &Places,
        asking: Asking,
        machine: Arc<dyn Machine>,
        headroom: Headroom,
        width: CheckWidth,
    ) -> Room {
        Room {
            places: places.clone(),
            asking,
            machine,
            headroom,
            past: Past::default(),
            width,
        }
    }

    /// The same room, starting the Checks this repository has timed fastest
    /// first. #1062.
    pub(crate) fn knowing(self, past: Past) -> Room {
        Room { past, ..self }
    }

    /// How wide one command in this batch may run, before any Check's own
    /// declaration lowers it. #1444.
    pub(crate) fn width(&self) -> CheckWidth {
        self.width
    }

    /// The same room, at a width a test can name. [`Room::knowing`]'s shape.
    #[cfg(test)]
    pub(crate) fn wide(self, width: CheckWidth) -> Room {
        Room { width, ..self }
    }

    pub(crate) fn past(&self) -> &Past {
        &self.past
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

    /// `env` for what a place here starts: where the place holds the machine's
    /// slots, a Check inside it runs under them. `checks_runner::HELD_ENV`.
    pub(crate) fn handing_down(&self, env: &[(String, String)]) -> Vec<(String, String)> {
        let mut env = env.to_vec();
        if self.places.0.slots.is_some() {
            env.push((checks_runner::HELD_ENV.to_string(), String::from("1")));
        }
        env
    }

    /// Join the line for a place.
    pub(crate) fn ask(&self) -> Ask {
        self.places.ask(self.asking, 1)
    }

    /// Join the line for a Check's own `places`. #1102.
    pub(crate) fn ask_for(&self, places: std::num::NonZeroU32) -> Ask {
        self.places.ask(self.asking, places.get() as usize)
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

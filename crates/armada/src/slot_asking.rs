//! Telling Fleet that `armada worktree lease`, run by a hosted Session, found every slot held, so
//! Fleet can ask the person whether it waits or a slot is added. `ipc::slot_asking`.
//!
//! **Best effort and silent when there is nobody to ask.** No `ARMADA_SESSION`, no Fleet, or a
//! Fleet that does not host that Session: the lease waits as it always did. Nothing here ever
//! makes it stop waiting, because the lease itself is what takes the slot, whoever made it.

use std::time::{Duration, Instant};

use fleet::runtime;
use ipc::{AskForSlot, AskForSlotAct, SessionId, SlotAskStanding, SlotAsked};

use crate::loopback::Loopback;

const ROUTE: &str = "/sessions/slot_wanted";

/// How often Fleet is told the lease is still waiting. Fleet forgets one it has not heard from for
/// ten seconds.
const EVERY: Duration = Duration::from_secs(1);

/// A lease that is waiting, and what its Session's owner has said about it.
pub struct Asking {
    fleet: Loopback,
    session: SessionId,
    last: Option<Instant>,
    standing: Option<SlotAskStanding>,
}

impl Asking {
    /// The Fleet running on this machine, for the Session whose process this is.
    pub fn for_session(session: &str) -> Option<Asking> {
        let at = runtime::machine_path().ok()?;
        let port = crate::mcp::listening(runtime::read(&at), &at).ok()?;
        Some(Asking::at(port, session))
    }

    pub fn at(port: u16, session: &str) -> Asking {
        Asking {
            fleet: Loopback::at(port),
            session: SessionId::carried(session),
            last: None,
            standing: None,
        }
    }

    /// Tell Fleet the lease is still waiting, about once a second, and say once each time what the
    /// owner has chosen.
    pub fn look(&mut self) {
        if self.last.is_some_and(|last| last.elapsed() < EVERY) {
            return;
        }
        self.last = Some(Instant::now());
        let Some(asked) = self.say(AskForSlotAct::Ask) else {
            return;
        };
        if self.standing == Some(asked.standing) {
            return;
        }
        self.standing = Some(asked.standing);
        match asked.standing {
            SlotAskStanding::Asking => {
                eprintln!("every slot is held: asked the owner whether to wait for one or add one")
            }
            SlotAskStanding::Waiting => eprintln!("the owner chose to wait for a slot"),
            SlotAskStanding::Added => {
                eprintln!("the owner chose to add a slot: one was made and is held for this lease")
            }
            SlotAskStanding::Unasked => {}
        }
    }

    fn say(&self, act: AskForSlotAct) -> Option<SlotAsked> {
        let call = AskForSlot { act, session_id: self.session.clone() };
        let body = ipc::encode(&call).ok()?;
        let answer = self.fleet.post(ROUTE, body.as_bytes()).ok()?;
        (answer.status == 200)
            .then(|| ipc::decode("a slot asked answer", &answer.body).ok())
            .flatten()
    }
}

/// The lease ended, leased or not: the call comes off the owner's list and a slot held for it is let go.
impl Drop for Asking {
    fn drop(&mut self) {
        if self.last.is_some() {
            self.say(AskForSlotAct::Done);
        }
    }
}

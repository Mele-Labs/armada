//! What a Job on your words may do, as one predicate over the operations the agent door offers,
//! and the door's answer about who is holding a connection.
//!
//! **A denylist of acts, not an allowlist.** `#73` drew the line as an allowlist inside the
//! `agent_access` column the door shares with every agent on the machine; Nick's later ruling
//! (`#1150`) reversed the default: such a Job may call any command it is offered, when a person
//! asks it to, except what is named in [`RESERVED`]. The column still keeps it and a Drone apart:
//! `agent_access = "No"` rows stay invisible to every agent regardless of this list, so this
//! predicate only ever sees what the door already offered.
//!
//! **Placed by the connection, never by what the caller says.** A call is a Job on your words'
//! where `crate::peer::held_within` finds its socket inside the process of a hosted Job started
//! with the flag (`Fleet::session_placed`). The reach is [`may`] under `Fleet::action_authority`.
//! Any other hosted Job keeps a hosted Job's door.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Admitting, Caller, WordsReach};
use ipc::door::Reachable;

use crate::daemon::Fleet;

/// How far this machine lets a Job on your words go: the Machine setting *Jobs on your words can
/// act*, resolved. `Fleet::action_authority` (`crate::daemon::seams`) is
/// the one place the setting becomes one of these two; `#943`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authority {
    /// The setting's shipped value: every read, and every act not in
    /// [`RESERVED`].
    Acting,
    /// Every read, and no act.
    ReadOnly,
}

/// The acts that stay a person's, independent of what the machine's authority
/// setting allows. **None now.** `#1150` named `undo_run` the sole exception to
/// "any command it is offered, on request", and it went with the Job runs it
/// reversed; the list stays so the next act that must stay a person's has
/// somewhere to be named.
pub(crate) const RESERVED: &[&str] = &[];

/// The acts on a Studio a Job on your words may take **without being asked**: adding a node
/// that starts proposed, proposing an edge, and naming an untitled Studio.
///
/// **Not a second predicate beside [`may`].** The door answers a call the same
/// whether a person asked for it or not, so nothing there could read this. Every other act on a
/// Studio falls under the rule that an act waits for an ask. Each is still a command `may`
/// refuses under [`Authority::ReadOnly`]. Read by the test that pins the list to the inventory.
#[cfg(test)]
pub(crate) const UNASKED: &[&str] = &["add_studio_node", "propose_studio_edge", "rename_studio"];

/// Whether a Job on your words, under `authority`, may call one operation the door offers.
///
/// Every query is a read and every read is allowed. A command is allowed where
/// the machine lets such a Job act at all and the operation is not [`RESERVED`].
pub fn may(authority: Authority, offered: &Reachable) -> bool {
    // **Not an act, so no authority decides it.** `ask_the_person` is the
    // channel a person answers a call on, and a Fleet that withheld it under
    // `ReadOnly` would leave every uncovered call refused with nobody asked —
    // which is the state `#1389` exists to end. It grants nothing: what comes
    // back is the person's own answer.
    if offered.operation == ipc::door::ASKS_A_PERSON {
        return true;
    }
    match offered.kind {
        "query" => true,
        "command" => authority == Authority::Acting && !RESERVED.contains(&offered.operation),
        _ => false,
    }
}

impl<H, V, W> Admitting for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn words_reach_at(&self, caller: Caller) -> Option<WordsReach> {
        let (_, on_your_words) = self.session_placed(&caller)?;
        if !on_your_words {
            return Some(WordsReach::of_a_hosted_session());
        }
        let authority = self.action_authority();
        Some(WordsReach::deciding(
            |row| may(authority, row),
            otherwise(authority),
        ))
    }

    fn always_loads(&self) -> Option<&'static str> {
        self.harness().always_loads_tool()
    }
}

/// Why a call outside the reach is refused, said after its name.
fn otherwise(authority: Authority) -> &'static str {
    match authority {
        Authority::Acting => "that act is left to the person",
        Authority::ReadOnly => "this machine is set so that Jobs on your words only read",
    }
}

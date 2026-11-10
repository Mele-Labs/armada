//! Whether Compact is offered on a Job, off the record the ledger built. **A port of Bridge's
//! `offeredOf`** in `cockpit/deck/session-tiles.ts` for its Compact tile, with `mute` for the dead
//! states: the order of the checks is the rule. The words for each reason stay with the client.

use ipc::{CompactHow, CompactOff, CompactOffer, SessionRecord, SessionState};

/// The command a Job lists for it.
const COMPACT: &str = "compact";

pub(crate) fn compact_of(record: &SessionRecord) -> CompactOffer {
    if record.state == SessionState::Ended {
        return off(CompactOff::Ended);
    }
    match &record.hosted {
        // The agent Fleet hosts: sent as a message, and offered once the agent has listed `compact`.
        Some(hosted) if hosted.commands.iter().any(|name| name == COMPACT) => {
            CompactOffer::Offered { how: CompactHow::Message }
        }
        Some(_) => off(CompactOff::AgentNotListed),
        // A terminal Job: typed as a command, and only to a mod that is listening and listed it.
        None => match &record.terminal {
            Some(terminal) if terminal.listening => {
                if terminal.commands.iter().any(|one| one.name == COMPACT) {
                    CompactOffer::Offered { how: CompactHow::Terminal }
                } else {
                    off(CompactOff::TerminalNotListed)
                }
            }
            _ => off(CompactOff::NotListening),
        },
    }
}

fn off(why: CompactOff) -> CompactOffer {
    CompactOffer::Off { why }
}

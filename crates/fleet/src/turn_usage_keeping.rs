//! Where a Session's turn usage meets the store: `turn_measured` kept, and `get_session_usage`
//! read back, under the lock the caller holds. `docs/practices/protocol.md`, *Usage a turn at a time*.

use ipc::{BashUse, SessionTurnUsage, TurnUsage};
use adapters::turn_lines::{Part, TurnRead};
use store::{KeptBackground, KeptOpenCall, KeptReading, KeptToolResult, KeptTurnSpent, KeptTurnTimes, KeptTurnUsage, Store, WriteError};

/// A turn as the store keeps it.
pub(crate) fn kept_form(turn: &TurnUsage) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: turn.message_id.clone(),
        at: turn.at.as_str().to_string(),
        model: turn.model.clone(),
        input_tokens: turn.input_tokens,
        cache_creation_tokens: turn.cache_creation_tokens,
        cache_read_tokens: turn.cache_read_tokens,
        output_tokens: turn.output_tokens,
        tools: turn.tools.clone(),
        subagent: turn.subagent,
        bash_use: turn.bash_use.map(|one| one.label().to_string()),
    }
}

/// What a read learnt of when its turns ran, as the store keeps it.
pub(crate) fn reading_kept(read: &TurnRead) -> KeptReading {
    KeptReading {
        times: read
            .times
            .iter()
            .map(|one| KeptTurnTimes {
                message_id: one.message_id.clone(),
                model_began: one.model_began.clone(),
                tools_began: one.tools_began.clone(),
                tools_ended: one.tools_ended.clone(),
            })
            .collect(),
        late: read.late.iter().map(|one| KeptToolResult { tool_use_id: one.tool_use_id.clone(), at: one.at.clone() }).collect(),
        open: read
            .open
            .iter()
            .map(|one| KeptOpenCall { tool_use_id: one.tool_use_id.clone(), message_id: one.message_id.clone() })
            .collect(),
        input_at: read.input_at.clone(),
        spent: read
            .spent
            .iter()
            .map(|one| KeptTurnSpent {
                message_id: one.message_id.clone(),
                cache_creation_1h_tokens: one.cache_creation_1h_tokens,
                shares: one.shares.as_ref().map(|shares| (named(&shares.read), named(&shares.write))),
            })
            .collect(),
        background: read
            .background
            .iter()
            .map(|one| KeptBackground {
                id: one.id.clone(),
                at: one.at.clone(),
                model: one.model.clone(),
                cache_read_tokens: one.cache_read_tokens,
                output_tokens: one.output_tokens,
            })
            .collect(),
        carried: read.carry.kept(),
    }
}

fn named(shares: &[(Part, f64)]) -> Vec<(String, f64)> {
    shares.iter().map(|(part, share)| (part.label().to_string(), *share)).collect()
}

/// One turn, kept. Whether the store changed.
pub(crate) fn kept(store: &mut Store, id: &str, turn: &TurnUsage) -> Result<bool, WriteError> {
    store.keep_turn_usage(id, &kept_form(turn))
}

/// The session's turns in the window, oldest first.
pub(crate) fn window(
    store: &Store,
    id: &str,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<SessionTurnUsage, WriteError> {
    let turns = store
        .turn_usage(id, since, until)?
        .into_iter()
        .map(|one| TurnUsage {
            message_id: one.message_id,
            at: ipc::Instant::carried(one.at),
            model: one.model,
            input_tokens: one.input_tokens,
            cache_creation_tokens: one.cache_creation_tokens,
            cache_read_tokens: one.cache_read_tokens,
            output_tokens: one.output_tokens,
            tools: one.tools,
            subagent: one.subagent,
            bash_use: one.bash_use.as_deref().and_then(BashUse::from_label),
        })
        .collect();
    Ok(SessionTurnUsage { turns })
}

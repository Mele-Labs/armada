//! Where a Session's turn usage meets the store: `turn_measured` kept, and `get_session_usage`
//! read back, under the lock the caller holds. `docs/practices/protocol.md`, *Usage a turn at a time*.

use ipc::{BashUse, SessionTurnUsage, TurnUsage};
use store::{KeptTurnUsage, Store, WriteError};

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

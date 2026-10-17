//! Where a Session's Loadout meets the store: the mod's `equipped` and `used` kept, and the record's
//! summary and `get_session_loadout` read back, under the lock the caller holds.
//! `docs/practices/protocol.md`, *A Session's Loadout*.

use ipc::{LoadoutItems, LoadoutSummary, LoadoutTurn, LoadoutUse, SessionLoadout};
use store::{Store, WriteError};

/// How many of a session's turns keep their uses.
pub(crate) const TURNS: usize = 50;

/// The latest list, kept. Whether it differs from the one held.
pub(crate) fn equipped(store: &mut Store, id: &str, loadout: &LoadoutItems) -> Result<bool, WriteError> {
    match ipc::encode(loadout) {
        Ok(kept) => store.keep_session_loadout(id, &kept),
        Err(_) => Ok(false),
    }
}

/// One use, kept in its turn. Whether it was new: the same use again in its turn is not.
pub(crate) fn used(store: &mut Store, id: &str, used: &LoadoutUse) -> Result<bool, WriteError> {
    match ipc::encode(used) {
        Ok(kept) => store.keep_session_use(id, used.turn_started_at.as_str(), &kept, TURNS),
        Err(_) => Ok(false),
    }
}

/// What the record's summary is read from.
pub(crate) struct LoadoutReads {
    items: Option<String>,
    latest: Option<(String, Vec<String>)>,
}

pub(crate) fn summary_read(store: &Store, id: &str) -> Result<LoadoutReads, WriteError> {
    let items = store.session_loadout(id)?;
    let latest = if items.is_some() { store.session_latest_uses(id)? } else { None };
    Ok(LoadoutReads { items, latest })
}

/// The summary, absent where no list was ever reported. A kept row that no longer reads is left out.
pub(crate) fn summary(reads: LoadoutReads) -> Option<LoadoutSummary> {
    let items: LoadoutItems = ipc::decode("a Loadout", reads.items?.as_bytes()).ok()?;
    let latest_turn = reads.latest.map(|(started_at, uses)| LoadoutTurn {
        started_at: ipc::Instant::carried(started_at),
        used: uses.iter().filter_map(|one| ipc::decode("a Loadout use", one.as_bytes()).ok()).collect(),
    });
    Some(LoadoutSummary::of(&items.items, latest_turn))
}

/// The whole Loadout: the latest list, empty where none was reported, and the kept turns in order.
pub(crate) fn whole(store: &Store, id: &str) -> Result<SessionLoadout, WriteError> {
    let items = store
        .session_loadout(id)?
        .and_then(|kept| ipc::decode::<LoadoutItems>("a Loadout", kept.as_bytes()).ok())
        .unwrap_or_default()
        .items;
    let mut turns: Vec<LoadoutTurn> = Vec::new();
    for (started_at, kept) in store.session_uses(id)? {
        let Ok(one) = ipc::decode::<LoadoutUse>("a Loadout use", kept.as_bytes()) else { continue };
        match turns.last_mut() {
            Some(turn) if turn.started_at.as_str() == started_at => turn.used.push(one),
            _ => turns.push(LoadoutTurn { started_at: ipc::Instant::carried(started_at), used: vec![one] }),
        }
    }
    Ok(SessionLoadout { items, turns })
}

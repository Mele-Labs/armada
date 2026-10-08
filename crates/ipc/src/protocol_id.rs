//! Which wire a build speaks. Two sides connect only when the IDs are equal.
//!
//! `build.rs` hashes the wire surface, so the ID moves when the wire does and
//! nobody bumps it. There is no order between two IDs: a mismatch says the
//! sides differ, not which one is older. `docs/practices/protocol.md` says what
//! the surface is.

use std::fmt;

use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/protocol_id.rs"));

/// A protocol ID as it travels: sixteen hex digits, compared whole. The empty
/// default is what a runtime file written before IDs reads as, and equals no real one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolId(String);

impl ProtocolId {
    /// What this build speaks.
    pub fn current() -> Self {
        Self(PROTOCOL_ID_HASH.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProtocolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

//! An error Bridge showed the person, told to the Operator's session. **One error is one item**: its id is
//! the error's fingerprint and the local day, so the same error is not told again that day.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::operator_errors::{BridgeErrorTaken, ReportBridgeError};

use super::BridgeError;
use crate::daemon::Fleet;

/// The shortest a token of hex digits is read as an id.
const HEX_ID: usize = 8;
/// The shortest a token with a digit in it is read as an id.
const MIXED_ID: usize = 8;

/// What identifies an error: its code and message, less what differs between two of the same fault.
/// **A token with a `/`, a `\` or a leading `~` is a path, one of eight or more hex digits or of
/// eight or more characters with a digit is an id, and any other digits are a number.** The result is
/// 16 hex digits, FNV-1a so it holds across Rust releases and a restart.
pub(crate) fn fingerprint(code: &str, message: &str) -> String {
    let shape: Vec<String> = message.split_whitespace().map(token_shape).collect();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in code.trim().bytes().chain(std::iter::once(b'\n')).chain(shape.join(" ").bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn token_shape(token: &str) -> String {
    if token.contains(['/', '\\']) || token.starts_with('~') {
        return "<path>".to_string();
    }
    let hex = token.trim_matches(|one: char| !one.is_ascii_alphanumeric());
    if hex.len() >= HEX_ID && hex.chars().all(|one| one.is_ascii_hexdigit()) {
        return "<id>".to_string();
    }
    if !token.chars().any(|one| one.is_ascii_digit()) {
        return token.to_string();
    }
    if token.chars().count() >= MIXED_ID && token.chars().any(|one| one.is_alphabetic()) {
        return "<id>".to_string();
    }
    let mut out = String::new();
    let mut digits = false;
    for one in token.chars() {
        match (one.is_ascii_digit(), digits) {
            (true, true) => {}
            (true, false) => out.push('#'),
            (false, _) => out.push(one),
        }
        digits = one.is_ascii_digit();
    }
    out
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The date of this machine's local day, as `YYYY-MM-DD`.
    async fn local_day(&self) -> String {
        const HALF_DAY: i64 = 12 * 3_600_000;
        let now = self.now().epoch_millis().unwrap_or(0);
        let began = crate::clock::local_day_began(now).await;
        crate::clock::rfc3339_utc(began + HALF_DAY).chars().take(10).collect()
    }

    /// `report_bridge_error`: tell the Operator's session of the error once for the local day. **Never fails**:
    /// while the Operator is off or its session is not running, or where Fleet serves no repository to put it in,
    /// it is let go and `repeated` is `false`.
    pub(crate) async fn bridge_error_taken(self: &Arc<Self>, error: ReportBridgeError) -> BridgeErrorTaken {
        let id = format!("error:{}:{}", fingerprint(&error.code, &error.message), self.local_day().await);
        let taken = |repeated: bool| BridgeErrorTaken { id: id.clone(), repeated };
        if !self.operator_on() {
            return taken(false);
        }
        let (manifest, job) = {
            let store = self.store().lock().await;
            let hosting = error.job_id.as_deref().and_then(|job| store.hosting(job).ok().flatten());
            let job = error.job_id.as_deref().map(|job| (job.to_string(), store.session(job).ok().flatten().and_then(|one| one.title).unwrap_or_default()));
            (hosting.map(|hosting| hosting.manifest_id), job)
        };
        let manifest = match manifest {
            Some(manifest) => manifest,
            None => match self.served_named(None) {
                Ok(served) => served.manifest().id().as_str().to_string(),
                Err(_) => return taken(false),
            },
        };
        let arrival = BridgeError {
            id: id.clone(),
            manifest_id: manifest,
            code: error.code,
            message: error.message,
            fields: error.fields.into_iter().map(|one| (one.key, one.value)).collect(),
            chain: error.chain,
            logs: error.logs,
            commit: error.commit,
            job,
            at: error.at,
        }
        .arrival();
        taken(self.arrival_handed(&arrival).await == super::session::Handed::Before)
    }
}

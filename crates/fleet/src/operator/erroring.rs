//! An error Bridge showed the person, handed to the Operator and done as ruled. **One error is one
//! item**: its id is the error's fingerprint and the local day, so the same error is not handed again
//! while it waits, is out on a call or is in the Log that day. It goes first in the Operator's queue,
//! and is asked about as a bug report is: a go-ahead, unless a rule he kept covers it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::operator_errors::{BridgeErrorTaken, ReportBridgeError};
use ipc::{ManifestId, Notice, SessionVoice, StartSession};

use super::{held, Arrival, BridgeError};
use crate::daemon::Fleet;

const OPERATOR: &str = "Operator";
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

    /// `report_bridge_error`: hand the error to the Operator once for the local day. **Never fails**:
    /// while the Operator is off, or where Fleet serves no repository to put it in, it is let go and
    /// `repeated` is `false`.
    pub(crate) async fn bridge_error_taken(&self, error: ReportBridgeError) -> BridgeErrorTaken {
        let id = format!("error:{}:{}", fingerprint(&error.code, &error.message), self.local_day().await);
        let taken = |repeated: bool| BridgeErrorTaken { id: id.clone(), repeated };
        if !self.operator_on() {
            return taken(false);
        }
        let seat = self.operator_seat();
        if seat.is_out(&id) || self.store().lock().await.operator_ruled(&id).unwrap_or(true) {
            return taken(true);
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
        let open = self.open_of(&manifest).await;
        let files = self.served_named(Some(&ManifestId::carried(&manifest))).ok().is_some_and(|served| self.backlog_of(&served).is_some());
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
            open,
            files,
        }
        .arrival();
        let handed = self.hand_once(arrival).await;
        taken(!handed)
    }

    /// An error no open issue covers, filed in the repository's beads with its words. The issue it
    /// was filed as.
    pub(super) async fn operator_filed_error(&self, manifest: &str, arrival: &Arrival) -> Option<String> {
        let served = self.served_named(Some(&ManifestId::carried(manifest))).ok()?;
        let backlog = self.backlog_of(&served)?;
        let (root, title) = (served.root().to_string(), arrival.subject.clone());
        let body = format!("{}\n\nFiled by the Operator from an error Bridge showed the person.", arrival.words);
        let filed = tokio::task::spawn_blocking(move || backlog.file(&root, &title, &body)).await;
        let Ok(Ok(reference)) = filed else { return None };
        held(&self.operator_seat().sources.filed).insert(reference.clone());
        self.tracking().forget(manifest);
        Some(reference)
    }

    /// An error the Operator ruled a Job of its own: started on it and briefed with every word the
    /// card carried. The Job's id.
    pub(super) async fn operator_started_error(self: &Arc<Self>, manifest: &str, arrival: &Arrival) -> Option<String> {
        let started = self
            .start_session(StartSession {
                manifest_id: ManifestId::carried(manifest),
                title: Some(arrival.subject.clone()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
            })
            .await;
        let record = started.ok()?;
        let job = record.id.as_str().to_string();
        let voice = SessionVoice::Session { id: OPERATOR.to_lowercase(), title: OPERATOR.to_string() };
        let brief = format!("Bridge showed the person this error. Find its cause and fix it.\n\n{}", arrival.words);
        if self.first_message_from(&job, voice, &brief).await.is_err() {
            let _ = self.closed_and_parked(ipc::CloseSession { session_id: record.id }).await;
            return None;
        }
        Some(job)
    }

    /// An error the Operator ruled to live Job `into`, whose work is near it: told it.
    pub(super) async fn operator_told_error(self: &Arc<Self>, arrival: &Arrival, into: &str) -> Result<(), String> {
        let text = format!("The Operator handed you an error Bridge showed the person, as your work is near it:\n\n{}\n\nFix it as part of your work, or say why it is not yours.", arrival.words);
        match self.told_by_fleet(into, &text, Notice::Operator).await {
            Ok(true) => Ok(()),
            _ => Err("the Job did not take the error".to_string()),
        }
    }
}

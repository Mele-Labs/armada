//! What calls the person, and which of it he has not opened, read by Fleet so a client does not work
//! it out: `get_calls`, `calls.changed` and `open_call`. `docs/practices/protocol/a-jobs-calls.md`.
//!
//! **A port of Electron Bridge's TypeScript, check for check**: `callsFromWaiting` in
//! `cockpit/waiting.ts`, the merge-line calls in `Dashboard.tsx`, `incomingOf` and `callOwnerOf` in
//! `cockpit/deck/store.ts`, and `session-attention.ts` in the main process. States and the words a
//! call says; what is played, shown or badged stays with the client.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::SessionId;

/// What a call is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandingCallKind {
    /// Something a Job waits on him for, or the Job's row itself where it needs him and no item does.
    Job,
    /// A repository's main reads red and nobody is on it.
    Main,
    /// A pull request of his whose checks failed or that cannot merge, or an `armada.yml` edit the forge would not auto-merge, and nobody is on it.
    Pull,
    /// A served checkout Fleet holds back from catching up with its base.
    Checkout,
    /// Another Mac waits for his Confirm here to pair: it pasted this Mac's link, or asked from its Settings.
    /// **The acts are the Phone Gateway's**: Confirm is `POST /admin/macs/pair/confirm` and Decline is
    /// `POST /admin/macs/forget`, each with `{ mac }` the call's `mac`. Never opened while it waits.
    MacPairing,
}

/// How hard it holds. **Blocked** is a Job's turn held on him (a question, a permission, a plan,
/// a slot or delivery choice, a done ask, a browser ask, a start card); anything else is waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallTone {
    Blocked,
    Waiting,
}

/// One call on the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingCall {
    /// Electron's own: `session:<id>:<item>`, `session:<id>:needs:<turn>`, `main:<root>`, `checkout:<root>`, `pull:<number>`;
    /// and `mac:<id>` for a Mac waiting to pair.
    pub key: String,
    /// Whom it is from, as `callOwnerOf` says it: `session:<id>` for a Job or for the Job already
    /// on a failure, else the key. **One badge for each owner.**
    pub owner: String,
    pub kind: StandingCallKind,
    /// The Job it is from, or the Job that owns the failure. Absent on a failure nobody owns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub title: String,
    pub text: String,
    /// When it began. A failure Fleet read no instant of reads `1970-01-01T00:00:00.000Z`.
    pub since: Instant,
    pub tone: CallTone,
    /// He has seen it: a Job call when he has been in the Job since it began or has it on screen
    /// and it is not asking him, a main, checkout or pull request call when `open_call` said so. A Mac waiting
    /// to pair asks him, so its call never is.
    pub opened: bool,
    /// The id of the paired Mac that holds the Job. **Absent means this Mac.** On a `mac_pairing` call, the id of
    /// the Mac waiting to pair, which Confirm and Decline name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
}

/// `get_calls`: every call standing, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingCalls {
    pub calls: Vec<StandingCall>,
}

impl StandingCalls {
    /// The calls still to be opened for a badge, the one that began last in front, **one for each
    /// owner**: `incomingOf`. A Job that asks him is never opened, so it stays here.
    pub fn unopened(&self) -> Vec<&StandingCall> {
        let mut unopened: Vec<&StandingCall> = self.calls.iter().filter(|one| !one.opened).collect();
        unopened.sort_by(|a, b| b.since.cmp(&a.since));
        let mut owners = std::collections::HashSet::new();
        unopened.retain(|one| owners.insert(one.owner.as_str()));
        unopened
    }
}

/// What a call is, to tell: a Job waiting, a Job Fleet gave up on, or another Mac waiting to pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallArrivalKind {
    Needs,
    Stopped,
    /// `session_id` is the call's key, `mac:<id>`, and names no Job.
    MacPairing,
}

/// News: a thing that began to wait on him since the last publish. Told once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallArrival {
    /// The Job's id, or on `mac_pairing` the call's key.
    pub session_id: SessionId,
    /// The Job's title, or `Job`; on `mac_pairing` the call's title.
    pub title: String,
    pub text: String,
    pub tone: CallTone,
    pub kind: CallArrivalKind,
    /// The call's key, as `StandingCall.key`. Set on `mac_pairing`, where `session_id` carries it too for
    /// readers that do not read this yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// What `calls.changed` carries: the calls whole, and the news since the last publish.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallsChanged {
    pub calls: Vec<StandingCall>,
    /// Empty where only openedness or the list moved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrived: Vec<CallArrival>,
}

/// `open_call`: he opened a main, checkout or pull request call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenStandingCall {
    pub key: String,
}

//! The link: for each linked Mac, one task that holds its `/api/mac/live` open and hands what comes
//! to this Fleet over loopback. A drop or a silent stream marks the Mac asleep, with why, and the
//! task opens it again, waiting 1 second and doubling to a minute.

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use ipc::{BugReports, Event, OtherMacBugReports, OtherMacEvent, OtherMacProjects, OtherMacSessions, Projects, SessionList, StreamMessage};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::mac_client::{encoded, signed, signed_whole};
use crate::macs::{frozen, said, MacHello};
use crate::{fleet_client, Gateway};

/// Ends a Mac's task when it is forgotten, or when the link itself stops.
struct Following(JoinHandle<()>);

impl Drop for Following {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Keep one task per linked Mac, started at pairing and stopped at forgetting. Runs until aborted,
/// which stops every task it started.
pub async fn link(gateway: Gateway) {
    let mut following: HashMap<(String, i64), Following> = HashMap::new();
    loop {
        let rows = gateway.pairing.store.lock().unwrap().paired_macs().unwrap_or_default();
        let linked: Vec<(String, i64)> = rows
            .into_iter()
            .filter(|row| row.state == store::MacState::Linked)
            .map(|row| (row.id, row.paired_at))
            .collect();
        following.retain(|key, _| linked.contains(key));
        for key in linked {
            following
                .entry(key.clone())
                .or_insert_with(|| Following(tokio::spawn(follow(gateway.clone(), key.0))));
        }
        gateway.macs.changed.notified().await;
    }
}

async fn follow(gateway: Gateway, mac: String) {
    let timing = gateway.macs.timing;
    let mut wait = timing.first_retry;
    loop {
        let mut opened = false;
        // A frozen link is not asleep: the other Mac answered, and its hello is read again until it matches.
        if let Some((since, why)) = held_open(&gateway, &mac, &mut opened).await {
            let _ = gateway.pairing.store.lock().unwrap().mac_asleep(&mac, since, &why);
        }
        if opened {
            wait = timing.first_retry;
        }
        tokio::time::sleep(wait).await;
        wait = (wait * 2).min(timing.longest_retry);
    }
}

/// A stream from the other Mac, one line at a time.
struct Lines {
    response: reqwest::Response,
    buffer: Vec<u8>,
}

enum Line {
    Text(String),
    Ended(String),
}

impl Lines {
    async fn next(&mut self, silence: Duration) -> Line {
        loop {
            if let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = self.buffer.drain(..=end).collect();
                return Line::Text(String::from_utf8_lossy(&line).trim().to_string());
            }
            match timeout(silence, self.response.chunk()).await {
                Ok(Ok(Some(bytes))) => self.buffer.extend_from_slice(&bytes),
                Ok(Ok(None)) => return Line::Ended(String::from("stopped answering")),
                Ok(Err(why)) => return Line::Ended(format!("stopped answering: {why}")),
                Err(_) => return Line::Ended(format!("went quiet for {} seconds", silence.as_secs())),
            }
        }
    }
}

/// Read the other Mac's hello, then open its stream and follow it until it drops. Answers when it
/// dropped and why, as a sentence; `opened` says whether it was open at all. **A Mac whose build speaks
/// another wire is not followed** (`macs::frozen`): nothing is read or handed, and the answer is `None`.
async fn held_open(gateway: &Gateway, mac: &str, opened: &mut bool) -> Option<(i64, String)> {
    let found = gateway.pairing.store.lock().unwrap().paired_mac(mac);
    let Ok(Some(row)) = found else {
        return Some((gateway.pairing.now(), String::from("This Mac is not paired with it any more.")));
    };
    let name = row.name.clone();
    let mut link = Link { gateway, mac, address: &row.address, name: &name };
    if let Err(why) = link.hello().await {
        return Some((gateway.pairing.now(), why));
    }
    // Read again on the retry's doubling pace, up to a minute, until the other Mac updates.
    if link.frozen() {
        return None;
    }
    let response = signed(gateway, mac, &row.address, "GET", "/api/mac/live", None, None).await;
    let response = match response {
        Ok(response) if response.status().is_success() => response,
        Ok(response) if response.status().as_u16() == 401 => {
            let why = format!("{name} does not know this Mac any more. Forget it here, then pair again.");
            return Some((gateway.pairing.now(), why));
        }
        Ok(response) => return Some((gateway.pairing.now(), format!("{name} answered {}.", response.status().as_u16()))),
        Err(why) => return Some((gateway.pairing.now(), format!("{name} could not be reached: {why}"))),
    };
    *opened = true;
    if let Err(why) = link.both().await {
        return Some((gateway.pairing.now(), why));
    }
    let mut heard_at = gateway.pairing.now();
    let mut lines = Lines { response, buffer: Vec::new() };
    let mut first = true;
    loop {
        let text = match lines.next(gateway.macs.timing.silence).await {
            Line::Text(text) => text,
            Line::Ended(why) => return Some((heard_at, format!("{name} {why}."))),
        };
        heard_at = gateway.pairing.now();
        let _ = gateway.pairing.store.lock().unwrap().touch_mac(mac, heard_at);
        if text.is_empty() {
            continue;
        }
        // A message from a build that says more than this one reads is passed over.
        let Ok(message) = ipc::decode::<StreamMessage>("a stream message", text.as_bytes()) else { continue };
        let handed = match message {
            // The first resync says what was just read whole on opening.
            StreamMessage::Resync(_) if first => Ok(()),
            StreamMessage::Resync(_) | StreamMessage::Missed(_) => link.both().await,
            StreamMessage::Event(delivered) => link.event(delivered.event).await,
        };
        first = false;
        if let Err(why) = handed {
            return Some((heard_at, why));
        }
    }
}

/// One open link, and what it hands this Fleet.
struct Link<'a> {
    gateway: &'a Gateway,
    mac: &'a str,
    address: &'a str,
    name: &'a str,
}

impl Link<'_> {
    /// Who the other Mac is now, its build among it, kept.
    async fn hello(&mut self) -> Result<(), String> {
        let hello: MacHello = self.read("/api/mac/hello", "a Mac").await?;
        let now = self.gateway.pairing.now();
        let _ = self.gateway.pairing.store.lock().unwrap().heard_from_mac(self.mac, now, &said(&hello, self.address));
        Ok(())
    }

    /// Whether the hello just kept says the other Mac's build speaks another wire.
    fn frozen(&self) -> bool {
        frozen_with(self.gateway, self.mac).is_some()
    }

    async fn both(&self) -> Result<(), String> {
        self.sessions().await?;
        self.projects().await?;
        let reports: BugReports = self.read("/api/mac/bug_reports", "a bug report list").await?;
        self.hand("PUT", "/other_macs/bug_reports", &OtherMacBugReports { mac: self.mac.to_string(), reports: reports.reports })
            .await
    }

    async fn sessions(&self) -> Result<(), String> {
        let list: SessionList = self.read("/api/mac/sessions", "a job list").await?;
        self.hand("PUT", "/other_macs/sessions", &OtherMacSessions { mac: self.mac.to_string(), sessions: list.sessions })
            .await
    }

    /// The other Mac's shared projects. **A Mac on a build from before shared projects answers 404**, and the
    /// link stays up without them.
    async fn projects(&self) -> Result<(), String> {
        let came = signed_whole(self.gateway, self.mac, self.address, "GET", "/api/mac/projects", None)
            .await
            .map_err(|why| format!("{} could not be reached: {why}", self.name))?;
        if came.status == 404 {
            return Ok(());
        }
        if !came.ok() {
            return Err(format!("{} answered {}: {}", self.name, came.status, came.sentence()));
        }
        let projects: Projects = ipc::decode("a project list", &came.body)
            .map_err(|_| format!("{} sent a project list this Mac cannot read. Update Armada on both.", self.name))?;
        self.hand("PUT", "/other_macs/projects", &OtherMacProjects { mac: self.mac.to_string(), projects: projects.projects }).await
    }

    async fn event(&self, event: Event) -> Result<(), String> {
        // A project that stops being shared takes its rows with it, so the list is read again.
        let projects = matches!(event, Event::ProjectsChanged(_));
        self.hand("POST", "/other_macs/event", &OtherMacEvent { mac: self.mac.to_string(), event }).await?;
        match projects {
            true => self.sessions().await,
            false => Ok(()),
        }
    }

    async fn read<T: DeserializeOwned>(&self, target: &str, what: &'static str) -> Result<T, String> {
        let came = signed_whole(self.gateway, self.mac, self.address, "GET", target, None)
            .await
            .map_err(|why| format!("{} could not be reached: {why}", self.name))?;
        if !came.ok() {
            return Err(format!("{} answered {}: {}", self.name, came.status, came.sentence()));
        }
        ipc::decode(what, &came.body).map_err(|_| format!("{} sent {what} this Mac cannot read. Update Armada on both.", self.name))
    }

    async fn hand<T: Serialize>(&self, method: &str, path: &str, value: &T) -> Result<(), String> {
        let not_taken = |why: String| format!("Armada on this Mac did not take what {} sent: {why}", self.name);
        let port = (self.gateway.fleet)().map_err(not_taken)?;
        let body = encoded(value).map_err(not_taken)?;
        let answer = fleet_client::send(port, method, path, Some(&body)).await.map_err(|why| not_taken(why.to_string()))?;
        match (200..300).contains(&answer.status) {
            true => Ok(()),
            false => Err(not_taken(format!("it answered {}", answer.status))),
        }
    }
}

/// [`crate::Gateway`]'s answer to Fleet's `send_other_mac`: `body` posted to Mac `mac` as `/api/mac/project_act`,
/// `/api/mac/act` or `/api/mac/update`, for the paths `/other_mac/project_act`, `/other_mac/act` and `/other_mac/update`
/// and no other. **While the link is frozen only the update is sent**, and the rest are refused as unsent. The status its Fleet
/// answered and the bytes, so a refusal there is said here as it was said.
pub fn send_other_mac(
    gateway: &Gateway,
    mac: &str,
    path: &str,
    body: Vec<u8>,
) -> impl Future<Output = Result<(u16, Vec<u8>), String>> + Send + 'static {
    let (gateway, mac, path) = (gateway.clone(), mac.to_string(), path.to_string());
    async move {
        let target = match path.as_str() {
            "/other_mac/project_act" => "/api/mac/project_act",
            "/other_mac/act" => "/api/mac/act",
            "/other_mac/update" => "/api/mac/update",
            _ => return Err(format!("{path} is not sent to another Mac.")),
        };
        let found = gateway.pairing.store.lock().unwrap().paired_mac(&mac);
        let row = match found {
            Ok(Some(row)) if row.state == store::MacState::Linked => row,
            _ => return Err(format!("This Mac is not linked to Mac {mac}.")),
        };
        // The update is the one call a frozen link carries: it is how the builds come to match.
        if target != "/api/mac/update" {
            if let Some(link) = frozen(&gateway.way_in, &row) {
                return Err(crate::macs::frozen_why(&link, &row.name));
            }
        }
        let came = signed_whole(&gateway, &row.id, &row.address, "POST", target, Some(body))
            .await
            .map_err(|why| format!("{} could not be reached: {why}", row.name))?;
        Ok((came.status, came.body))
    }
}

/// [`crate::Gateway`]'s answer to Fleet's `read_other_mac`: a `/other_mac/thread?…` path, read from
/// Mac `mac` as `/api/mac/thread?…`. Fleet asks nothing else of another Mac this way.
pub fn read_other_mac(gateway: &Gateway, mac: &str, path: &str) -> impl Future<Output = Result<(u16, Vec<u8>), String>> + Send + 'static {
    let (gateway, mac, path) = (gateway.clone(), mac.to_string(), path.to_string());
    async move {
        let rest = path.strip_prefix("/other_mac/thread").filter(|rest| rest.is_empty() || rest.starts_with('?'));
        let Some(rest) = rest else {
            return Err(format!("{path} is not read from another Mac."));
        };
        let found = gateway.pairing.store.lock().unwrap().paired_mac(&mac);
        let row = match found {
            Ok(Some(row)) if row.state == store::MacState::Linked => row,
            _ => return Err(format!("This Mac is not linked to Mac {mac}.")),
        };
        if let Some(link) = frozen(&gateway.way_in, &row) {
            return Err(crate::macs::frozen_why(&link, &row.name));
        }
        let came = signed_whole(&gateway, &row.id, &row.address, "GET", &format!("/api/mac/thread{rest}"), None)
            .await
            .map_err(|why| format!("{} could not be reached: {why}", row.name))?;
        Ok((came.status, came.body))
    }
}

/// How the link to Mac `mac` is frozen, as the store has it now, or `None` where it is not.
pub(crate) fn frozen_with(gateway: &Gateway, mac: &str) -> Option<ipc::MacLink> {
    let row = gateway.pairing.store.lock().unwrap().paired_mac(mac).ok().flatten()?;
    frozen(&gateway.way_in, &row)
}

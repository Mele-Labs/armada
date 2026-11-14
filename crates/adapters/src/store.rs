//! The central store over HTTP: [`HttpStore`] makes each [`Store`] call one
//! signed request and reads the one answer.
//! `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`,
//! *Fixing a red across machines*.
//!
//! Building a request and reading a reply are plain functions of values, apart
//! from sending, so the wire is tested without a network and the send is tested
//! against a socket on this machine.
//!
//! **A blocking client, as the `Store` trait is a plain trait.** `reqwest`'s
//! blocking client panics if it is made or dropped on an async worker, so a
//! caller builds and holds an `HttpStore` on a thread, or inside
//! `spawn_blocking`, as every other port here is called.
//!
//! **A 5xx is "unreachable", not "refused".** The store did not answer in a way
//! that says anything about the request, and the caller treats a take it could
//! not confirm as a take that did not happen.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use adapter_traits::{
    ClaimKey, ClaimKind, Claimant, Claimed, Contest, Holder, HolderClaims, HolderKind, ListedClaim, ListedWait, OwnerKey, Renewed, RepositoryKey, RunRow, Secret, Store, StoreDevice,
    StoreError,
};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};

use crate::store_device::{device_id, random, remember_device, sound_id, to_hex, DeviceKey};
use crate::store_signing;

/// How many owners one read asks for, so the signed query stays a short line.
const OWNERS_PER_READ: usize = 40;
/// The longest a refusal's own words are kept.
const SAID_AT_MOST: usize = 200;

/// One request, before it is signed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) method: &'static str,
    /// Path and query, as the signature covers them.
    pub(crate) target: String,
    pub(crate) body: Vec<u8>,
}

/// What came back.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct HolderWire {
    kind: String,
    id: String,
    machine: String,
    since: i64,
}

impl HolderWire {
    fn of(holder: &Holder) -> HolderWire {
        HolderWire {
            kind: holder.kind.as_wire().to_string(),
            id: holder.id.clone(),
            machine: holder.machine.clone(),
            since: holder.since,
        }
    }

    fn into_holder(self) -> Result<Holder, StoreError> {
        let kind = HolderKind::from_wire(&self.kind)
            .ok_or_else(|| StoreError::Refused(format!("The store named a holder kind this build does not know: {}.", self.kind)))?;
        Ok(Holder { kind, id: self.id, machine: self.machine, since: self.since })
    }
}

/// A claim's holder as the store spells it: a holder, with the person's name and the Job's title.
#[derive(Serialize, Deserialize)]
struct ClaimantWire {
    kind: String,
    id: String,
    machine: String,
    since: i64,
    name: String,
    title: String,
}

impl ClaimantWire {
    fn of(claimant: &Claimant) -> ClaimantWire {
        let HolderWire { kind, id, machine, since } = HolderWire::of(&claimant.holder);
        ClaimantWire { kind, id, machine, since, name: claimant.name.clone(), title: claimant.title.clone() }
    }

    fn into_claimant(self) -> Result<Claimant, StoreError> {
        let holder = HolderWire { kind: self.kind, id: self.id, machine: self.machine, since: self.since }.into_holder()?;
        Ok(Claimant { holder, name: self.name, title: self.title })
    }
}

#[derive(Serialize)]
struct ClaimBody<'a> {
    repo: &'a str,
    kind: &'static str,
    key: &'a str,
    holder: ClaimantWire,
}

/// A claim as the store answers it. Its `renewed` is not read yet.
#[derive(Deserialize)]
struct ClaimReply {
    holder: ClaimantWire,
}

/// Who a renewal is for: a holder without the `since` of any one claim.
#[derive(Serialize)]
struct RenewalBody<'a> {
    holder: RenewingWire<'a>,
}

#[derive(Serialize)]
struct RenewingWire<'a> {
    kind: &'static str,
    id: &'a str,
    machine: &'a str,
}

/// A renewal's answer. A store older than the contests leaves `contested` out, and it reads as none.
#[derive(Deserialize)]
struct RenewedWire {
    claims: u64,
    #[serde(default)]
    contested: Vec<ContestWire>,
}

/// One claim of the holder that another Job asked for since its last renewal.
#[derive(Deserialize)]
struct ContestWire {
    repo: String,
    kind: String,
    key: String,
    holder: ClaimantWire,
}

impl ContestWire {
    /// `None` for a contest this build cannot name: a kind, hash or holder kind it does not know.
    fn into_contest(self) -> Option<Contest> {
        Some(Contest {
            repository: RepositoryKey::from_hash(&self.repo)?,
            kind: ClaimKind::from_wire(&self.kind)?,
            key: ClaimKey::from_hash(&self.key)?,
            by: self.holder.into_claimant().ok()?,
        })
    }
}

/// `GET /v1/claims`: every live claim, grouped by holder.
#[derive(Deserialize)]
struct ListedWire {
    holders: Vec<GroupWire>,
}

/// One holder's group. The holder carries no `since`: each claim has its own.
#[derive(Deserialize)]
struct GroupWire {
    holder: GroupHolderWire,
    renewed: i64,
    #[serde(default)]
    claims: Vec<ListedClaimWire>,
    #[serde(default)]
    waits: Vec<ContestWire>,
}

#[derive(Deserialize)]
struct GroupHolderWire {
    kind: String,
    id: String,
    machine: String,
    name: String,
    title: String,
}

#[derive(Deserialize)]
struct ListedClaimWire {
    repo: String,
    kind: String,
    key: String,
    since: i64,
}

impl GroupWire {
    /// `None` for a group this build cannot name; within one, a claim or wait it cannot read is skipped.
    fn into_group(self) -> Option<HolderClaims> {
        let holder = HolderWire { kind: self.holder.kind, id: self.holder.id, machine: self.holder.machine, since: 0 }.into_holder().ok()?;
        let claims = self
            .claims
            .into_iter()
            .filter_map(|one| {
                Some(ListedClaim {
                    repository: RepositoryKey::from_hash(&one.repo)?,
                    kind: ClaimKind::from_wire(&one.kind)?,
                    key: ClaimKey::from_hash(&one.key)?,
                    since: one.since,
                })
            })
            .collect();
        let waits = self
            .waits
            .into_iter()
            .filter_map(ContestWire::into_contest)
            .map(|contest| ListedWait { repository: contest.repository, kind: contest.kind, key: contest.key, held_by: contest.by })
            .collect();
        Some(HolderClaims { holder: Claimant { holder, name: self.holder.name, title: self.holder.title }, renewed: self.renewed, claims, waits })
    }
}

/// The groups in a `GET /v1/claims` reply, in the store's order.
pub(crate) fn listed(reply: &Reply) -> Result<Vec<HolderClaims>, StoreError> {
    match succeeded(reply) {
        true => Ok(decoded::<ListedWire>("claims list", reply)?.holders.into_iter().filter_map(GroupWire::into_group).collect()),
        false => Err(not_answered(reply)),
    }
}

#[derive(Serialize)]
struct OwnerBody {
    holder: HolderWire,
}

/// One run, as the store spells it: a fact has no `port` key where it has none.
#[derive(Serialize, Deserialize)]
struct RunWire {
    workspace: String,
    kind: String,
    command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    succeeded: bool,
    last_ran: u64,
    source: String,
}

impl RunWire {
    fn of(run: &RunRow) -> RunWire {
        RunWire {
            workspace: run.workspace.clone(),
            kind: run.kind.clone(),
            command: run.command.clone(),
            port: run.port,
            succeeded: run.succeeded,
            last_ran: run.last_ran,
            source: run.source.clone(),
        }
    }

    fn into_row(self) -> RunRow {
        RunRow {
            workspace: self.workspace,
            kind: self.kind,
            command: self.command,
            port: self.port,
            succeeded: self.succeeded,
            last_ran: self.last_ran,
            source: self.source,
        }
    }
}

#[derive(Serialize)]
struct RunsBody {
    facts: Vec<RunWire>,
}

#[derive(Deserialize)]
struct RunsReply {
    facts: Vec<RunWire>,
}

#[derive(Deserialize)]
struct DevicesReply {
    devices: Vec<DeviceWire>,
}

#[derive(Deserialize)]
struct DeviceWire {
    device: String,
    machine: String,
}

/// The devices in a `GET /v1/devices` reply, in the store's order.
pub(crate) fn devices_held(reply: &Reply) -> Result<Vec<StoreDevice>, StoreError> {
    if !succeeded(reply) {
        return Err(not_answered(reply));
    }
    let devices = decoded::<DevicesReply>("devices", reply)?.devices;
    Ok(devices.into_iter().map(|one| StoreDevice { id: one.device, machine: one.machine }).collect())
}

#[derive(Serialize)]
struct EnrolBody<'a> {
    token: &'a str,
    public_key: &'a str,
    machine: &'a str,
}

#[derive(Deserialize)]
struct Enrolled {
    device: String,
}

fn body_of<T: Serialize>(value: &T) -> Result<Vec<u8>, StoreError> {
    ipc::encode(value)
        .map(String::into_bytes)
        .map_err(|why| StoreError::Refused(why.to_string()))
}

fn decoded<T: serde::de::DeserializeOwned>(what: &'static str, reply: &Reply) -> Result<T, StoreError> {
    ipc::decode(what, &reply.body).map_err(|why| StoreError::Refused(why.to_string()))
}

/// The error for a reply that was not the answer asked for.
pub(crate) fn not_answered(reply: &Reply) -> StoreError {
    let words = String::from_utf8_lossy(&reply.body);
    let words: String = words.trim().chars().take(SAID_AT_MOST).collect();
    let said = if words.is_empty() {
        format!("status {}", reply.status)
    } else {
        format!("status {}: {words}", reply.status)
    };
    if reply.status >= 500 {
        StoreError::Unreachable(said)
    } else {
        StoreError::Refused(said)
    }
}

fn succeeded(reply: &Reply) -> bool {
    (200..300).contains(&reply.status)
}

pub(crate) fn claim_call(repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey, claimant: &Claimant) -> Result<Call, StoreError> {
    let body = ClaimBody { repo: repository.as_str(), kind: kind.as_wire(), key: key.as_str(), holder: ClaimantWire::of(claimant) };
    Ok(Call { method: "POST", target: "/v1/claims".to_string(), body: body_of(&body)? })
}

pub(crate) fn claimed(reply: &Reply) -> Result<Claimed, StoreError> {
    match reply.status {
        200 => Ok(Claimed::Won),
        409 => Ok(Claimed::Held(decoded::<ClaimReply>("claim holder", reply)?.holder.into_claimant()?)),
        _ => Err(not_answered(reply)),
    }
}

fn claim_target(repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey) -> String {
    format!("/v1/claims/{}/{}/{}", repository.as_str(), kind.as_wire(), key.as_str())
}

pub(crate) fn held(reply: &Reply) -> Result<Option<Claimant>, StoreError> {
    match reply.status {
        200 => Ok(Some(decoded::<ClaimReply>("claim holder", reply)?.holder.into_claimant()?)),
        404 => Ok(None),
        _ => Err(not_answered(reply)),
    }
}

pub(crate) fn renewal_call(holder: &Holder) -> Result<Call, StoreError> {
    let body = RenewalBody { holder: RenewingWire { kind: holder.kind.as_wire(), id: &holder.id, machine: &holder.machine } };
    Ok(Call { method: "POST", target: "/v1/renewals".to_string(), body: body_of(&body)? })
}

/// The claims a holder still has and who asked for them. A contest this build cannot read is skipped,
/// not an error: the renewal itself still counts.
pub(crate) fn renewed(reply: &Reply) -> Result<Renewed, StoreError> {
    match succeeded(reply) {
        true => {
            let wire = decoded::<RenewedWire>("renewal", reply)?;
            Ok(Renewed { claims: wire.claims, contested: wire.contested.into_iter().filter_map(ContestWire::into_contest).collect() })
        }
        false => Err(not_answered(reply)),
    }
}

/// A delete is done when it succeeded or there was nothing to delete.
pub(crate) fn deleted(reply: &Reply) -> Result<(), StoreError> {
    if succeeded(reply) || reply.status == 404 {
        Ok(())
    } else {
        Err(not_answered(reply))
    }
}

fn stored(reply: &Reply) -> Result<(), StoreError> {
    if succeeded(reply) {
        Ok(())
    } else {
        Err(not_answered(reply))
    }
}

pub(crate) fn runs_call(repository: &RepositoryKey, runs: &[RunRow]) -> Result<Call, StoreError> {
    let body = RunsBody { facts: runs.iter().map(RunWire::of).collect() };
    Ok(Call { method: "PUT", target: format!("/v1/runs/{}", repository.as_str()), body: body_of(&body)? })
}

/// The runs in a reply; `None` where the store holds none.
pub(crate) fn runs_held(reply: &Reply) -> Result<Option<Vec<RunRow>>, StoreError> {
    match reply.status {
        200 => Ok(Some(decoded::<RunsReply>("runs", reply)?.facts.into_iter().map(RunWire::into_row).collect())),
        404 => Ok(None),
        _ => Err(not_answered(reply)),
    }
}

pub(crate) fn owners_target(keys: &[OwnerKey]) -> String {
    let keys: Vec<&str> = keys.iter().map(OwnerKey::as_str).collect();
    format!("/v1/owners?keys={}", keys.join(","))
}

/// The owners in a reply, **only for keys that were asked**: a store that
/// answers with more has said something this call did not ask.
pub(crate) fn owned(reply: &Reply, asked: &[OwnerKey]) -> Result<BTreeMap<OwnerKey, Holder>, StoreError> {
    if !succeeded(reply) {
        return Err(not_answered(reply));
    }
    let rows: BTreeMap<String, HolderWire> = decoded("owners", reply)?;
    let mut found = BTreeMap::new();
    for (key, holder) in rows {
        if let Some(key) = asked.iter().find(|asked| asked.as_str() == key) {
            found.insert(key.clone(), holder.into_holder()?);
        }
    }
    Ok(found)
}

/// This machine, signed in to the store.
///
/// No `Debug`: it holds the machine's private key.
pub struct HttpStore {
    base: String,
    device: String,
    key: DeviceKey,
    client: Client,
    now: fn() -> i64,
    nonce: fn() -> String,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
}

fn fresh_nonce() -> String {
    // Empty when the system's randomness cannot be read, which `send` refuses
    // rather than sign with a nonce the store would call a replay.
    random::<16>().map(|bytes| to_hex(&bytes)).unwrap_or_default()
}

fn client(timeout: Duration) -> Result<Client, String> {
    Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        // A redirect would send this machine's signature to a host it was not made for.
        .redirect(Policy::none())
        .build()
        .map_err(|why| format!("The store's client could not be made: {why}"))
}

impl HttpStore {
    /// The store at `base_url` as the machine in `dir` is enrolled with it, or
    /// `None` where this machine has not signed in.
    pub fn open(base_url: &str, dir: &Path, timeout: Duration) -> Result<Option<HttpStore>, String> {
        let Some(device) = device_id(dir)? else {
            return Ok(None);
        };
        let key = DeviceKey::open(dir)?;
        Ok(Some(HttpStore::new(base_url, device, key, timeout)?))
    }

    /// Enrol this machine's key with the store, once, with a device-flow access
    /// token the store reads the account from and drops. The device id it
    /// returns is kept beside the key; the token is not.
    ///
    /// The error says whether the store was out of reach or said no, so a
    /// sign-in can tell the person which.
    pub fn enrol(
        base_url: &str,
        dir: &Path,
        token: &Secret<String>,
        machine: &str,
        timeout: Duration,
    ) -> Result<HttpStore, StoreError> {
        let key = DeviceKey::open(dir).map_err(StoreError::Refused)?;
        let public_key = key.public_key();
        let body = body_of(&EnrolBody { token: token.expose(), public_key: &public_key, machine })?;
        let call = Call { method: "POST", target: "/v1/enrol".to_string(), body };
        let client = client(timeout).map_err(StoreError::Refused)?;
        let reply = send(&client, base_url, &call, &[])?;
        if !succeeded(&reply) {
            return Err(not_answered(&reply));
        }
        let device = decoded::<Enrolled>("enrolment", &reply)?.device;
        remember_device(dir, &device).map_err(StoreError::Refused)?;
        Ok(HttpStore { base: base_url.trim_end_matches('/').to_string(), device, key, client, now: unix_now, nonce: fresh_nonce })
    }

    /// The device id this store signs as.
    pub fn device(&self) -> &str {
        &self.device
    }

    fn new(base_url: &str, device: String, key: DeviceKey, timeout: Duration) -> Result<HttpStore, String> {
        Ok(HttpStore {
            base: base_url.trim_end_matches('/').to_string(),
            device,
            key,
            client: client(timeout)?,
            now: unix_now,
            nonce: fresh_nonce,
        })
    }

    #[cfg(test)]
    pub(crate) fn at(mut self, now: fn() -> i64, nonce: fn() -> String) -> HttpStore {
        self.now = now;
        self.nonce = nonce;
        self
    }

    #[cfg(test)]
    pub(crate) fn made(base_url: &str, device: &str, key: DeviceKey) -> HttpStore {
        HttpStore::new(base_url, device.to_string(), key, Duration::from_secs(5)).expect("a client")
    }

    fn send(&self, call: &Call) -> Result<Reply, StoreError> {
        let nonce = (self.nonce)();
        if nonce.is_empty() {
            return Err(StoreError::Refused("The system's random numbers could not be read.".to_string()));
        }
        let headers = store_signing::headers(&self.key, &self.device, call.method, &call.target, (self.now)(), &nonce, &call.body);
        send(&self.client, &self.base, call, &headers)
    }
}

fn send(client: &Client, base: &str, call: &Call, headers: &[(&'static str, String)]) -> Result<Reply, StoreError> {
    let method = reqwest::Method::from_bytes(call.method.as_bytes())
        .map_err(|why| StoreError::Refused(why.to_string()))?;
    let url = format!("{}{}", base.trim_end_matches('/'), call.target);
    let mut request = client.request(method, url);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    if !call.body.is_empty() {
        request = request.header("Content-Type", "application/json").body(call.body.clone());
    }
    // Everything `reqwest` raises is the request not completing: it names no
    // status, because a status comes back as a `Response`.
    let response = request.send().map_err(|why| StoreError::Unreachable(why.without_url().to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .bytes()
        .map_err(|why| StoreError::Unreachable(why.without_url().to_string()))?
        .to_vec();
    Ok(Reply { status, body })
}

impl Store for HttpStore {
    fn claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey, claimant: &Claimant) -> Result<Claimed, StoreError> {
        claimed(&self.send(&claim_call(repository, kind, key, claimant)?)?)
    }

    fn read_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey) -> Result<Option<Claimant>, StoreError> {
        let call = Call { method: "GET", target: claim_target(repository, kind, key), body: Vec::new() };
        held(&self.send(&call)?)
    }

    fn release_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey) -> Result<(), StoreError> {
        let call = Call { method: "DELETE", target: claim_target(repository, kind, key), body: Vec::new() };
        deleted(&self.send(&call)?)
    }

    /// **Two requests, not one**: delete whoever holds it, then claim. Two
    /// machines taking over at once leave one winner, and the other is told who.
    fn replace_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey, claimant: &Claimant) -> Result<Claimed, StoreError> {
        self.release_claim(repository, kind, key)?;
        self.claim(repository, kind, key, claimant)
    }

    fn clear_kind(&self, repository: &RepositoryKey, kind: ClaimKind) -> Result<(), StoreError> {
        let call = Call { method: "DELETE", target: format!("/v1/claims/{}/{}", repository.as_str(), kind.as_wire()), body: Vec::new() };
        deleted(&self.send(&call)?)
    }

    fn renew(&self, holder: &Holder) -> Result<Renewed, StoreError> {
        renewed(&self.send(&renewal_call(holder)?)?)
    }

    fn claims_listed(&self) -> Result<Vec<HolderClaims>, StoreError> {
        let call = Call { method: "GET", target: "/v1/claims".to_string(), body: Vec::new() };
        listed(&self.send(&call)?)
    }

    fn put_owner(&self, key: &OwnerKey, holder: &Holder) -> Result<(), StoreError> {
        let call = Call {
            method: "PUT",
            target: format!("/v1/owners/{}", key.as_str()),
            body: body_of(&OwnerBody { holder: HolderWire::of(holder) })?,
        };
        stored(&self.send(&call)?)
    }

    fn owners(&self, keys: &[OwnerKey]) -> Result<BTreeMap<OwnerKey, Holder>, StoreError> {
        let mut found = BTreeMap::new();
        for some in keys.chunks(OWNERS_PER_READ) {
            let call = Call { method: "GET", target: owners_target(some), body: Vec::new() };
            found.extend(owned(&self.send(&call)?, some)?);
        }
        Ok(found)
    }

    fn delete_owner(&self, key: &OwnerKey) -> Result<(), StoreError> {
        let call = Call { method: "DELETE", target: format!("/v1/owners/{}", key.as_str()), body: Vec::new() };
        deleted(&self.send(&call)?)
    }

    fn delete_device(&self, device: &str) -> Result<(), StoreError> {
        if !sound_id(device) {
            return Err(StoreError::Refused("That is not a device id the store would have given.".to_string()));
        }
        let call = Call { method: "DELETE", target: format!("/v1/devices/{device}"), body: Vec::new() };
        deleted(&self.send(&call)?)
    }

    fn signed_in_devices(&self) -> Result<Vec<StoreDevice>, StoreError> {
        let call = Call { method: "GET", target: "/v1/devices".to_string(), body: Vec::new() };
        devices_held(&self.send(&call)?)
    }

    fn put_runs(&self, repository: &RepositoryKey, runs: &[RunRow]) -> Result<(), StoreError> {
        stored(&self.send(&runs_call(repository, runs)?)?)
    }

    fn read_runs(&self, repository: &RepositoryKey) -> Result<Option<Vec<RunRow>>, StoreError> {
        let call = Call { method: "GET", target: format!("/v1/runs/{}", repository.as_str()), body: Vec::new() };
        runs_held(&self.send(&call)?)
    }
}

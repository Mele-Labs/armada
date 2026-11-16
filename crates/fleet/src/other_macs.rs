//! His other Macs, through this Mac's Phone Gateway: what Fleet hands the Gateway to send, always cut by
//! [`crate::share`], and what the Gateway hands in from the other Mac, kept with that Mac's id and served
//! beside this Mac's own. `docs/concepts/pocket.md`, *Another Mac*.
//!
//! **A Job held from another Mac is not one of this Mac's Jobs.** Its rows are kept in their own tables, so
//! every read of this Mac's ledger finds none by its id, and an act on one is sent to the Mac that holds it
//! (`crate::mac_acts`); what is said about it is relayed to Bridge and never told to Fleet's own listeners.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{ActAnswered, ActBegun, ActTurn, OtherMacs, Refusal, Sessions};
use ipc::{
    BugReport, BugReports, Event, JobAct, JobThread, JobThreadRead, MacAct, MacNamed, OpenOtherMacPage, OtherMacBugReports, OtherMacPageOpened, OtherMacEvent, OtherMacProjects,
    OtherMacSessions, ProjectActFromMac, QueuedList, SessionId, SessionList, SessionRecord, SessionRemoved, SessionState, TakeBack, WireError,
};
use serde::Serialize;
use store::{FromMac, KeptFromMac};

use crate::daemon::Fleet;
use crate::session_host::to_other_mac::MOVED_TO_MAC;

const MAC_UNNAMED: &str = "fleet.other_mac_unnamed";
const EVENT_NOT_KEPT: &str = "fleet.other_mac_event_not_kept";
const JOB_NOT_HELD: &str = "fleet.other_mac_job_not_held";
pub(crate) const MAC_UNREACHABLE: &str = "fleet.other_mac_unreachable";
pub(crate) const MAC_UNREADABLE: &str = "fleet.other_mac_unreadable";

/// Whether `record` passes `list_sessions`' filters, as the ledger's search reads them.
fn listed(record: &SessionRecord, manifest: Option<&str>, text: Option<&str>, state: Option<SessionState>) -> bool {
    if manifest.is_some_and(|manifest| record.manifest_id.as_ref().map(|id| id.as_str()) != Some(manifest)) {
        return false;
    }
    if state.is_some_and(|state| record.state != state) {
        return false;
    }
    let Some(text) = text.map(str::trim).filter(|text| !text.is_empty()) else {
        return true;
    };
    // `#12` and `slot-3` name a pull request and a slot as often as `12` and `3` do.
    let bare = text.trim_start_matches('#').trim_start_matches("slot-");
    let lower = text.to_lowercase();
    let holds = |within: &str| within.to_lowercase().contains(&lower);
    record.id.as_str() == text
        || record.title.as_deref().is_some_and(holds)
        || record.attachments.iter().any(|one| {
            (matches!(one.kind.as_str(), "pr" | "slot") && one.target == bare) || (one.kind == "branch" && holds(&one.target))
        })
}

/// A row of this Mac's as another Mac is sent it. **Its plan goes without `other_macs`**, which names Jobs as
/// this Mac sees them, the receiver's own among them.
fn sent_on(mut record: SessionRecord) -> SessionRecord {
    if let Some(plan) = record.plan.as_mut() {
        plan.other_macs.clear();
    }
    record
}

/// Newest seen first, then by id: the order `list_sessions` already uses.
fn in_list_order(sessions: &mut [SessionRecord]) {
    sessions.sort_by(|a, b| b.last_seen_at.as_str().cmp(a.last_seen_at.as_str()).then_with(|| a.id.as_str().cmp(b.id.as_str())));
}

/// Newest `at` first, ties by id: the order `list_bug_reports` already uses.
pub(crate) fn in_report_order(reports: &mut [BugReport]) {
    reports.sort_by(|a, b| b.at.as_str().cmp(a.at.as_str()).then_with(|| a.id.cmp(&b.id)));
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
    /// Everything of `list` held from his other Macs, read back. **A row a later build cannot read is left
    /// out** rather than failing the list: the next replace from its Mac writes it again.
    pub(crate) async fn from_macs<T: serde::de::DeserializeOwned>(&self, list: FromMac, what: &'static str) -> Result<Vec<(KeptFromMac, T)>, Refusal> {
        let kept = self.store().lock().await.held_from_macs(list).map_err(|why| self.ledger_fault(why))?;
        Ok(kept.into_iter().filter_map(|one| ipc::decode(what, one.body.as_bytes()).ok().map(|value| (one, value))).collect())
    }

    /// Every Job row held from his other Macs, each with its Mac.
    pub(crate) async fn sessions_from_macs(&self) -> Result<Vec<SessionRecord>, Refusal> {
        let held = self.from_macs::<SessionRecord>(FromMac::Sessions, "a Job row from another Mac").await?;
        Ok(held.into_iter().map(|(kept, mut record)| {
            record.mac = Some(kept.mac);
            record
        }).collect())
    }

    /// Every bug report held from his other Macs, each with its Mac.
    pub(crate) async fn bug_reports_from_macs(&self) -> Result<Vec<BugReport>, Refusal> {
        let held = self.from_macs::<BugReport>(FromMac::BugReports, "a bug report from another Mac").await?;
        Ok(held.into_iter().map(|(kept, mut report)| {
            report.mac = Some(kept.mac);
            report
        }).collect())
    }

    /// This Mac's own rows, `list_sessions` built them, with the rows held from his other Macs that pass the
    /// same filters, all in list order.
    pub(crate) async fn beside_other_macs(
        &self,
        mut own: Vec<SessionRecord>,
        manifest: Option<&str>,
        text: Option<&str>,
        state: Option<SessionState>,
    ) -> Result<Vec<SessionRecord>, Refusal> {
        let held = self.sessions_from_macs().await?;
        if held.is_empty() {
            return Ok(own);
        }
        // A Job that moved has a row on both Macs: the live one stands, wherever it is.
        let moved = |record: &SessionRecord| record.end_reason.as_deref() == Some(MOVED_TO_MAC);
        own.retain(|mine| !(moved(mine) && held.iter().any(|one| one.id == mine.id)));
        let kept: Vec<SessionId> = own.iter().map(|mine| mine.id.clone()).collect();
        own.extend(held.into_iter().filter(|record| !kept.contains(&record.id) && listed(record, manifest, text, state)));
        in_list_order(&mut own);
        Ok(own)
    }

    /// The Mac Job `job` is held from. **None where this Mac holds a Job by that id itself**, so another Mac
    /// can never stand in for one of this Mac's Jobs, unless it moved from here to that Mac.
    pub(crate) async fn mac_holding(&self, job: &str) -> Option<String> {
        let store = self.store().lock().await;
        if store.session(job).ok().flatten().is_some_and(|one| one.end_reason.as_deref() != Some(MOVED_TO_MAC)) {
            return None;
        }
        let held = store.held_from_macs(FromMac::Sessions).ok()?;
        held.into_iter().find(|one| one.id == job).map(|one| one.mac)
    }

    /// A Job held from Mac `mac`'s thread, read through the link from that Mac's own Fleet. **What it refuses
    /// is passed on as it was said there.**
    pub(crate) async fn thread_from_other_mac(&self, mac: &str, read: &JobThreadRead) -> Result<JobThread, Refusal> {
        let mut path = format!("/other_mac/thread?session_id={}", ipc::door::encoded(read.session_id.as_str()));
        for (name, value) in [("before", read.before.clone()), ("limit", read.limit.map(|n| n.to_string())), ("upto", read.upto.clone())] {
            if let Some(value) = value {
                path.push_str(&format!("&{name}={}", ipc::door::encoded(&value)));
            }
        }
        let answered = self.phone_switch()?.read_other_mac(mac, &path).await;
        let (status, bytes) = answered.map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREACHABLE, why, self.run_id())))?;
        if (200..300).contains(&status) {
            return ipc::decode("a Job's thread from another Mac", &bytes).map_err(|why| {
                Refusal::Fault(WireError::raised(MAC_UNREADABLE, format!("Mac {mac} sent a thread this build cannot read: {why}"), self.run_id()))
            });
        }
        Err(self.refusal_passed_on(mac, status, &bytes))
    }

    /// What Mac `mac` refused with, `status` and `bytes`, said again as it said it, with the status kept.
    pub(crate) fn refusal_passed_on(&self, mac: &str, status: u16, bytes: &[u8]) -> Refusal {
        let said: Option<WireError> = ipc::decode("a refusal from another Mac", bytes).ok();
        let (code, message) = match said {
            Some(said) => (said.code, said.message),
            None => (MAC_UNREADABLE.to_string(), format!("Mac {mac} answered {status}.")),
        };
        let error = WireError::raised(code, message, self.run_id());
        match status {
            404 => Refusal::NotFound(error),
            409 => Refusal::IllegalMove(error),
            422 => Refusal::Unacceptable(error),
            _ => Refusal::Fault(error),
        }
    }

    /// The Mac a call from the Gateway names. A blank one is refused, since every row is kept under it.
    pub(crate) fn mac_named(&self, mac: &str) -> Result<String, Refusal> {
        let mac = mac.trim();
        if mac.is_empty() {
            return Err(Refusal::Unacceptable(WireError::raised(MAC_UNNAMED, "A Mac to keep it under is needed.", self.run_id())));
        }
        Ok(mac.to_string())
    }

    pub(crate) fn encoded<T: Serialize>(&self, value: &T) -> Result<String, Refusal> {
        ipc::encode(value).map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREADABLE, why.to_string(), self.run_id())))
    }

    /// Every report this Mac and his other Macs hold, to Bridge whole, in list order.
    async fn every_bug_report_relayed(&self) -> Result<(), Refusal> {
        let mut reports = self.bug_reports_listed().await?.reports;
        in_report_order(&mut reports);
        self.events().relay(Event::BugReportsChanged(BugReports { reports }));
        Ok(())
    }

    async fn reports_held(&self, mac: &str, reports: Vec<BugReport>) -> Result<(), Refusal> {
        let rows = reports
            .into_iter()
            .map(|mut report| {
                report.mac = Some(mac.to_string());
                Ok((report.id.clone(), self.encoded(&report)?))
            })
            .collect::<Result<Vec<_>, Refusal>>()?;
        self.store().lock().await.replace_from_mac(FromMac::BugReports, mac, &rows).map_err(|why| self.ledger_fault(why))?;
        self.every_bug_report_relayed().await
    }

    /// `session.removed` for each row held from another Mac that went, to Bridge alone.
    fn relay_removed(&self, gone: Vec<String>) {
        self.calls_forgotten(&gone);
        for id in gone {
            self.events().relay(Event::SessionRemoved(SessionRemoved { session_id: SessionId::carried(id) }));
        }
    }

    /// The rows held from Mac `mac`, by id.
    async fn sessions_held_from(&self, mac: &str) -> Result<BTreeMap<String, SessionRecord>, Refusal> {
        let held = self.sessions_from_macs().await?;
        Ok(held.into_iter().filter(|one| one.mac.as_deref() == Some(mac)).map(|one| (one.id.as_str().to_string(), one)).collect())
    }
}

impl<H, V, W> OtherMacs for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn share_sessions(&self) -> Result<SessionList, Refusal> {
        let listed = self.list_sessions(None, None, None).await?;
        let mut sessions = Vec::new();
        // A row held from another Mac never goes back out, whatever its project here says, nor one that moved there.
        for record in listed.sessions.into_iter().filter(|one| one.mac.is_none() && one.end_reason.as_deref() != Some(MOVED_TO_MAC)) {
            let job = record.id.clone();
            if let Some(crossing) = self.of_job_for_other_mac(&job, record).await {
                sessions.push(sent_on(crossing.sent()));
            }
        }
        Ok(SessionList { sessions })
    }

    async fn share_projects(&self) -> Result<ipc::Projects, Refusal> {
        let own = api::Projects::list_projects(self).await?;
        Ok(self.projects_for_other_mac(own).sent())
    }

    async fn job_for_phone(&self, job: &str) -> bool {
        Fleet::job_for_phone(self, job).await
    }

    async fn run_for_phone(&self, run_id: &str) -> bool {
        Fleet::run_for_phone(self, run_id).await
    }

    async fn event_for_phone(&self, event: ipc::Event) -> Option<ipc::Event> {
        Fleet::event_for_phone(self, event).await
    }

    async fn start_from_phone(&self, start: &ipc::StartSession) -> Result<(), Refusal> {
        Fleet::start_from_phone(self, start).await
    }

    async fn projects_for_phone(&self, projects: ipc::Projects) -> ipc::Projects {
        Fleet::projects_for_phone(self, projects).await
    }

    async fn manifests_for_phone(&self, manifests: Vec<ipc::ManifestSummary>) -> Vec<ipc::ManifestSummary> {
        Fleet::manifests_for_phone(self, manifests).await
    }

    async fn project_act_from_mac(&self, sent: ProjectActFromMac) -> Result<ipc::Projects, Refusal> {
        self.project_act_run(sent).await
    }

    async fn share_bug_reports(&self) -> Result<BugReports, Refusal> {
        let mut own = self.bug_reports_listed().await?;
        own.reports.retain(|one| one.mac.is_none());
        Ok(self.bug_reports_for_other_mac(own).await.sent())
    }

    async fn begin_act_from_mac(&self, mac: String, sent: &MacAct) -> Result<ActBegun, Refusal> {
        self.act_from_mac_begun(mac, sent).await
    }

    async fn finish_act_from_mac(&self, turn: ActTurn, status: u16, body: &[u8]) {
        self.act_from_mac_ran(turn, status, body).await
    }

    async fn mac_holding_job(&self, job: &str) -> Option<String> {
        self.mac_holding_for_act(job).await
    }

    async fn mac_holding_pull(&self, repository: &str, number: u64) -> Option<(String, SessionId)> {
        self.pull_held_by_other_mac(repository, number).await
    }

    async fn act_on_other_mac(self: Arc<Self>, mac: String, act: JobAct) -> Result<ActAnswered, Refusal> {
        self.acted_on_other_mac(mac, act).await
    }

    async fn list_queued(&self, mac: Option<String>) -> Result<QueuedList, Refusal> {
        self.queued_listed(mac.as_deref()).await
    }

    async fn take_back_queued(&self, take: TakeBack) -> Result<QueuedList, Refusal> {
        self.queued_taken_back(take).await
    }

    async fn share_event(&self, event: Event) -> Option<Event> {
        match self.event_for_other_mac(event).await?.sent() {
            // The Mac it moved to holds it live, and this row ending it here would stand over that one.
            Event::SessionChanged(record) if record.end_reason.as_deref() == Some(MOVED_TO_MAC) => None,
            Event::SessionChanged(record) => Some(Event::SessionChanged(sent_on(record))),
            other => Some(other),
        }
    }

    async fn share_job_thread(self: Arc<Self>, read: JobThreadRead) -> Result<JobThread, Refusal> {
        self.act_from_other_mac(&read.session_id).await?;
        self.job_thread_served(read).await
    }

    async fn share_job_page(&self, job: SessionId, port: u16) -> Result<(), Refusal> {
        self.job_page_shared(job, port).await
    }

    async fn open_other_mac_page(&self, open: OpenOtherMacPage) -> Result<OtherMacPageOpened, Refusal> {
        self.other_mac_page_opened(open).await
    }

    async fn share_job_terminal(self: Arc<Self>, asked: ipc::TerminalFromMac) -> Result<api::TerminalShared, Refusal> {
        self.job_terminal_shared(asked).await
    }

    async fn share_job_terminal_output(&self, which: ipc::TerminalOfJob) -> Result<api::ObservedSessionRun, Refusal> {
        self.job_terminal_output_shared(which).await
    }

    async fn hold_other_mac_sessions(&self, held: OtherMacSessions) -> Result<MacNamed, Refusal> {
        let mac = self.mac_named(&held.mac)?;
        let before = self.sessions_held_from(&mac).await?;
        let rows: Vec<SessionRecord> = held
            .sessions
            .into_iter()
            .map(|mut record| {
                record.mac = Some(mac.clone());
                record
            })
            .collect();
        let kept = rows.iter().map(|one| Ok((one.id.as_str().to_string(), self.encoded(one)?))).collect::<Result<Vec<_>, Refusal>>()?;
        self.store().lock().await.replace_from_mac(FromMac::Sessions, &mac, &kept).map_err(|why| self.ledger_fault(why))?;
        let left_out = before.keys().filter(|id| !rows.iter().any(|one| one.id.as_str() == id.as_str()));
        self.relay_removed(left_out.cloned().collect());
        // A row that is as it was is no news, so a Mac waking after a sleep rings for what changed while it slept.
        let changed: Vec<SessionRecord> = rows.into_iter().filter(|record| before.get(record.id.as_str()) != Some(record)).collect();
        self.calls_heard_from_mac(&changed, before.is_empty());
        for record in changed {
            self.events().relay(Event::SessionChanged(record));
        }
        Ok(MacNamed { mac })
    }

    async fn hold_other_mac_projects(self: Arc<Self>, held: OtherMacProjects) -> Result<MacNamed, Refusal> {
        let mac = self.mac_named(&held.mac)?;
        self.copies_replaced(&mac, held.projects).await?;
        self.project_acts_flushing(&mac).await;
        Ok(MacNamed { mac })
    }

    async fn hold_other_mac_bug_reports(&self, held: OtherMacBugReports) -> Result<MacNamed, Refusal> {
        let mac = self.mac_named(&held.mac)?;
        self.reports_held(&mac, held.reports).await?;
        Ok(MacNamed { mac })
    }

    async fn hold_other_mac_event(self: Arc<Self>, crossed: OtherMacEvent) -> Result<MacNamed, Refusal> {
        let mac = self.mac_named(&crossed.mac)?;
        let job = match &crossed.event {
            Event::SessionChanged(record) => {
                let mut record = record.clone();
                record.mac = Some(mac.clone());
                let before = self.sessions_held_from(&mac).await?;
                if before.get(record.id.as_str()) != Some(&record) {
                    let body = self.encoded(&record)?;
                    let kept = self.store().lock().await.keep_from_mac(FromMac::Sessions, &mac, record.id.as_str(), &body);
                    kept.map_err(|why| self.ledger_fault(why))?;
                    self.calls_heard_from_mac(std::slice::from_ref(&record), false);
                    self.events().relay(Event::SessionChanged(record));
                }
                return Ok(MacNamed { mac });
            }
            Event::BugReportsChanged(reports) => {
                self.reports_held(&mac, reports.reports.clone()).await?;
                return Ok(MacNamed { mac });
            }
            // The list it carries is what that Mac shares now, so it is the copy. A Mac that has spoken is awake,
            // so what was queued for it goes now.
            Event::ProjectsChanged(projects) => {
                self.copies_replaced(&mac, projects.projects.clone()).await?;
                self.project_acts_flushing(&mac).await;
                return Ok(MacNamed { mac });
            }
            Event::SessionRow(row) => row.session_id.clone(),
            Event::SessionSubagent(subagent) => subagent.session_id.clone(),
            Event::JobThreadTail(tail) => tail.session_id.clone(),
            Event::JobThreadChanged(changed) => changed.session_id.clone(),
            other => {
                return Err(Refusal::Unacceptable(WireError::raised(
                    EVENT_NOT_KEPT,
                    format!("A {} event is not kept from another Mac.", other.kind()),
                    self.run_id(),
                )));
            }
        };
        if !self.sessions_held_from(&mac).await?.contains_key(job.as_str()) {
            return Err(Refusal::NotFound(WireError::raised(
                JOB_NOT_HELD,
                format!("Mac {mac} shares no Job named {}.", job.as_str()),
                self.run_id(),
            )));
        }
        self.events().relay(crossed.event);
        Ok(MacNamed { mac })
    }

    async fn message_from_mac(self: Arc<Self>, mac: String, message: ipc::JobMessage) -> Result<(), Refusal> {
        let mac = self.mac_named(&mac)?;
        self.message_from_mac_delivered(mac, message).await
    }

    async fn forget_other_mac(&self, named: MacNamed) -> Result<MacNamed, Refusal> {
        let mac = self.mac_named(&named.mac)?;
        let before = self.sessions_held_from(&mac).await?;
        self.store().lock().await.forget_mac(&mac).map_err(|why| self.ledger_fault(why))?;
        self.relay_removed(before.into_keys().collect());
        self.every_bug_report_relayed().await?;
        self.projects_relayed().await?;
        Ok(MacNamed { mac })
    }

    async fn hold_mac_claims(&self, claims: ipc::MacClaimsWaiting) -> Result<ipc::MacClaimsWaiting, Refusal> {
        self.calls_claimed(claims.claims.clone());
        Ok(claims)
    }

    async fn move_job_to_mac(self: Arc<Self>, asked: ipc::MoveJobToMac) -> Result<SessionRecord, Refusal> {
        self.job_moved_to_mac(asked).await
    }

    async fn move_from_mac(self: Arc<Self>, mac: String, job: SessionId) -> Result<SessionRecord, Refusal> {
        self.job_moved_from_mac(mac, job).await
    }

    async fn arrive_from_mac(self: Arc<Self>, mac: String, job: ipc::ArrivingJob) -> Result<SessionRecord, Refusal> {
        self.arrived_from(mac, job).await
    }

    async fn move_settled_from_mac(self: Arc<Self>, mac: String, settled: ipc::MoveSettled) -> Result<(), Refusal> {
        self.move_settled_from(mac, settled).await
    }

    async fn share_job_conversation(&self, mac: String, job: SessionId, from: u64) -> Result<Vec<u8>, Refusal> {
        self.conversation_shared(mac, job, from).await
    }
}

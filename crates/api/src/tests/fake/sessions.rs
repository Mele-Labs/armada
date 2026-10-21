//! The session ledger, as the fake answers it: a report is echoed back as the
//! row it would make, and a read finds nothing. What the ledger keeps and
//! matches is `fleet::sessioning`'s, against a real store.

use std::sync::Arc;

use ipc::{
    ManifestId, Owners, SessionList, SessionOrigin, SessionRecord, SessionReport, SessionState,
    SessionUsage,
};

use super::FakeDaemon;
use crate::{Refusal, Sessions};

impl Sessions for FakeDaemon {
    async fn report_session(&self, report: SessionReport) -> Result<SessionRecord, Refusal> {
        if let Some(row) = self.session_row.lock().expect("session row").clone() {
            return Ok(row);
        }
        let at = ipc::Instant::carried("2026-10-06T09:00:00.000Z");
        Ok(SessionRecord {
            id: report.session_id,
            harness: report.harness,
            origin: SessionOrigin::Terminal,
            manifest_id: None,
            cwd: String::new(),
            title: None,
            state: SessionState::Live,
            started_at: at.clone(),
            last_seen_at: at,
            last_turn_at: None,
            ended_at: None,
            end_reason: None,
            usage: SessionUsage::default(),
            attachments: Vec::new(),
            hosted: None,
            terminal: None,
            mod_out_of_date: false,
            waiting_for: Vec::new(),
            done_since: None,
            standing: None,
            held_until: None,
            context_breakdown: None,
            loadout_summary: None,
            helm: false,
            auto_land: false,
            activity: None,
            build: None,
            workflow: None,
            proposal: None,
            plan: None,
            touches: Vec::new(),
            workflow_step: None,
            judging: false,
            judge: None,
            settled: Vec::new(),
            waits: None,
        })
    }

    async fn rename_session(&self, rename: ipc::RenameSession) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: rename.session_id,
            fact: ipc::SessionFact::Titled {
                title: rename.title,
                named: true,
            },
        })
        .await
    }

    /// **The fake runs nothing**: what a run is, and its process group, are `fleet::session_running`'s.
    async fn start_session_run(
        self: std::sync::Arc<Self>,
        _start: ipc::StartSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.runs_nothing_here()
    }

    async fn run_session_shell(
        self: std::sync::Arc<Self>,
        _run: ipc::RunSessionShell,
    ) -> Result<ipc::SessionShellRan, Refusal> {
        self.runs_nothing_here()
    }

    async fn stop_session_run(
        &self,
        _stop: ipc::StopSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.runs_nothing_here()
    }

    async fn write_session_run(
        &self,
        _write: ipc::WriteSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.runs_nothing_here()
    }

    async fn resize_session_run(
        &self,
        _resize: ipc::ResizeSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.runs_nothing_here()
    }

    async fn observe_session_run(
        &self,
        _run_id: String,
    ) -> Result<crate::ObservedSessionRun, Refusal> {
        self.runs_nothing_here()
    }

    async fn report_session_check(
        &self,
        call: ipc::SessionCheckCall,
    ) -> Result<ipc::SessionCheckAnswer, Refusal> {
        Ok(ipc::SessionCheckAnswer { run: call.run.or(Some(1)), stop: None })
    }

    async fn ask_for_slot(&self, _call: ipc::AskForSlot) -> Result<ipc::SlotAsked, Refusal> {
        Ok(ipc::SlotAsked { standing: ipc::SlotAskStanding::Unasked })
    }

    async fn get_session_check_output(&self, run: u64) -> Result<ipc::CheckOutput, Refusal> {
        Ok(ipc::CheckOutput {
            attempt: 1,
            name: String::from("suite"),
            path: format!("session-check/{run}"),
            lines: vec![String::from("ok")],
            from_line: 1,
            total_lines: 1,
            bytes: 3,
            whole: true,
        })
    }

    async fn observe_session_check(&self, _run: u64) -> Result<crate::SessionCheckLive, Refusal> {
        self.runs_nothing_here()
    }

    async fn stop_session_check(&self, run: u64, force: bool) -> Result<ipc::SessionCheckAnswer, Refusal> {
        let stop = if force { ipc::SessionCheckStop::Force } else { ipc::SessionCheckStop::Graceful };
        Ok(ipc::SessionCheckAnswer { run: Some(run), stop: Some(stop) })
    }

    async fn start_another_session(
        self: Arc<Self>,
        _caller: Option<crate::Caller>,
        start: ipc::StartAnotherSession,
    ) -> Result<ipc::SessionHeld, Refusal> {
        Ok(ipc::SessionHeld {
            session_id: ipc::SessionId::carried("session"),
            title: start.title,
            fork: false,
        })
    }

    async fn change_waiting(
        self: Arc<Self>,
        _caller: Option<crate::Caller>,
        change: ipc::ChangeWaiting,
    ) -> Result<ipc::SessionWait, Refusal> {
        Ok(ipc::SessionWait {
            on: ipc::SessionLink { id: ipc::SessionId::carried("on"), title: String::from("On") },
            condition: change.condition.unwrap_or_default(),
            base: change.base.unwrap_or_default(),
            needs: String::new(),
            brief: String::new(),
            pull: None,
            part_title: None,
            by: ipc::WaitBy::You,
        })
    }

    async fn fork_this_session(
        &self,
        _caller: Option<crate::Caller>,
        fork: ipc::ForkThisSession,
    ) -> Result<ipc::SessionHeld, Refusal> {
        Ok(ipc::SessionHeld {
            session_id: ipc::SessionId::carried("session"),
            title: fork.title,
            fork: true,
        })
    }

    async fn claim_pull_request(
        &self,
        _caller: Option<crate::Caller>,
        claim: ipc::ClaimPullRequest,
    ) -> Result<ipc::PullRequestClaimed, Refusal> {
        Ok(ipc::PullRequestClaimed {
            number: claim.number,
            branch: String::new(),
            url: String::new(),
            holder_id: String::from("session"),
            auto_merge_refused: None,
        })
    }

    async fn take_main_red(
        &self,
        _caller: Option<crate::Caller>,
        _take: ipc::TakeMainRed,
    ) -> Result<ipc::MainRedTaken, Refusal> {
        Ok(ipc::MainRedTaken {
            root: String::from("/srv/shop"),
            claimed: true,
        })
    }

    async fn show_window(
        &self,
        _caller: Option<crate::Caller>,
        show: ipc::ShowWindow,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: show.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::Attached {
                attachment: ipc::AttachmentReport {
                    kind: String::from("artifact"),
                    target: show.url,
                    detail: Default::default(),
                },
            },
        })
        .await
    }

    async fn withdraw_window(
        &self,
        _caller: Option<crate::Caller>,
        withdraw: ipc::WithdrawWindow,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: withdraw.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn show_sketch(
        &self,
        _caller: Option<crate::Caller>,
        show: ipc::ShowSketch,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: show.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn record_plan(&self, _caller: Option<crate::Caller>, plan: ipc::RecordPlan) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: plan.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn approve_plan(self: std::sync::Arc<Self>, approve: ipc::ApprovePlan) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: approve.session_id,
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn judge_session(self: std::sync::Arc<Self>, judge: ipc::JudgeSession) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: judge.session_id,
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn send_concern(self: std::sync::Arc<Self>, send: ipc::SendConcern) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: send.session_id,
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn dismiss_concern(self: std::sync::Arc<Self>, dismiss: ipc::DismissConcern) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: dismiss.session_id,
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn send_plan_changes(self: std::sync::Arc<Self>, changes: ipc::SendPlanChanges) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: changes.session_id,
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn show_file(
        &self,
        _caller: Option<crate::Caller>,
        show: ipc::ShowFile,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: show.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn ask_in_window(
        &self,
        _caller: Option<crate::Caller>,
        ask: ipc::AskInWindow,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: ask.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn window_closed(&self, closed: ipc::WindowClosed) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: closed.session_id,
            fact: ipc::SessionFact::Attached {
                attachment: ipc::AttachmentReport {
                    kind: String::from("artifact"),
                    target: closed.url,
                    detail: Default::default(),
                },
            },
        })
        .await
    }

    async fn take_window_sign_in(&self, _take: ipc::TakeWindowSignIn) -> Result<ipc::WindowSignIn, Refusal> {
        Ok(ipc::WindowSignIn { sign_in: ipc::SignIn::default() })
    }

    async fn window_failed(self: std::sync::Arc<Self>, _failed: ipc::WindowFailed) -> Result<ipc::WalkTold, Refusal> {
        Ok(ipc::WalkTold { told: true })
    }

    async fn waiting_for(
        &self,
        _caller: Option<crate::Caller>,
        set: ipc::SetWaitingFor,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: set.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::Attached {
                attachment: ipc::AttachmentReport {
                    kind: String::from("waiting"),
                    target: String::new(),
                    detail: Default::default(),
                },
            },
        })
        .await
    }

    async fn finish_up(
        &self,
        _caller: Option<crate::Caller>,
        _finish: ipc::FinishUp,
    ) -> Result<ipc::FinishedUp, Refusal> {
        Ok(ipc::FinishedUp::default())
    }

    async fn mark_annotation_done(
        &self,
        _caller: Option<crate::Caller>,
        mark: ipc::MarkAnnotationDone,
    ) -> Result<ipc::AnnotationMarked, Refusal> {
        Ok(ipc::AnnotationMarked { path: format!("/repo/.armada/annotations/{}.json", mark.note_id) })
    }

    async fn dispute_fleet(
        self: std::sync::Arc<Self>,
        _caller: Option<crate::Caller>,
        _dispute: ipc::DisputeFleet,
    ) -> Result<ipc::FleetDisputed, Refusal> {
        Ok(ipc::FleetDisputed { dispute_id: String::from("dispute"), review_session_id: None, joined: false })
    }

    async fn all_done(
        &self,
        _caller: Option<crate::Caller>,
        done: ipc::AllDone,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: done.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn list_sessions(
        &self,
        _manifest_id: Option<ManifestId>,
        _text: Option<String>,
        state: Option<SessionState>,
    ) -> Result<SessionList, Refusal> {
        self.sessions_asked.lock().expect("sessions asked").push(state);
        let held = self.sessions_held.lock().expect("sessions held");
        Ok(SessionList {
            sessions: held
                .iter()
                .filter(|one| state.is_none_or(|state| one.state == state))
                .cloned()
                .collect(),
        })
    }

    async fn who_owns(
        &self,
        _kind: String,
        _target: String,
        _manifest_id: Option<ManifestId>,
    ) -> Result<Owners, Refusal> {
        Ok(Owners {
            holders: Vec::new(),
        })
    }

    async fn get_session_usage(
        &self,
        _session_id: ipc::SessionId,
        _since: Option<String>,
        _until: Option<String>,
    ) -> Result<ipc::SessionTurnUsage, Refusal> {
        Ok(ipc::SessionTurnUsage::default())
    }

    async fn get_usage_summary(
        &self,
        _since: Option<String>,
        _until: Option<String>,
        _tz_offset_minutes: i32,
    ) -> Result<ipc::UsageSummary, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised("fleet.session_unnamed", "a fake sums nothing", crate::tests::shapes::run_id())))
    }

    async fn get_usage_rules(&self, _since: Option<String>, _until: Option<String>) -> Result<ipc::UsageRules, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised("fleet.session_unnamed", "a fake watches nothing", crate::tests::shapes::run_id())))
    }

    async fn get_session_loadout(&self, _session_id: ipc::SessionId) -> Result<ipc::SessionLoadout, Refusal> {
        Ok(ipc::SessionLoadout::default())
    }

    async fn read_loadout_item(&self, _asked: ipc::LoadoutItemAsked) -> Result<ipc::LoadoutItemBody, Refusal> {
        Ok(ipc::LoadoutItemBody::NotRead { why: String::from("A fake reads nothing.") })
    }
}

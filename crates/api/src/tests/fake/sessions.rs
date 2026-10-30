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
            tools_missing: None,
            waiting_for: Vec::new(),
            done_since: None,
            done_summary: None,
            standing: None,
            held_until: None,
            context_breakdown: None,
            loadout_summary: None,
            helm: false,
            r#mod: false,
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
            closed: Vec::new(),
            waits: None,
            later: Vec::new(),
            held_open_by: Vec::new(),
            started: None,
            workflow_visits: Vec::new(),
            skipped_steps: Vec::new(),
            workflow_waiting_on: Vec::new(),
            workflow_copy: None,
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

    /// **The fake asks no proposer**, so it refuses as Fleet refuses without one.
    async fn name_session(&self, _name: ipc::NameSession) -> Result<SessionRecord, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.session_not_named",
            "the fake daemon asks no proposer, so this job was not named",
            crate::tests::shapes::run_id(),
        )))
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

    /// Opens nothing: answers the run id a shell for the Job would have.
    async fn open_terminal(
        self: std::sync::Arc<Self>,
        open: ipc::OpenTerminal,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        Ok(ipc::SessionRunUnderway { run_id: format!("shell-of-{}", open.session_id.as_str()) })
    }

    async fn start_terminal_run(
        self: std::sync::Arc<Self>,
        _start: ipc::StartTerminalRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.runs_nothing_here()
    }

    async fn type_in_terminal(
        self: std::sync::Arc<Self>,
        _caller: Option<crate::Caller>,
        _typed: ipc::TypeInTerminal,
    ) -> Result<ipc::TerminalTyped, Refusal> {
        self.runs_nothing_here()
    }

    async fn read_terminal(
        &self,
        _caller: Option<crate::Caller>,
        _asked: ipc::ReadTerminal,
    ) -> Result<ipc::TerminalScreen, Refusal> {
        self.runs_nothing_here()
    }

    async fn drive_browser(
        &self,
        _caller: Option<crate::Caller>,
        _session_id: Option<ipc::SessionId>,
        act: ipc::BrowserAct,
    ) -> Result<ipc::BrowserAnswer, Refusal> {
        let url = match act {
            ipc::BrowserAct::Open { url } => url,
            _ => String::from("http://localhost:5173/"),
        };
        Ok(ipc::BrowserAnswer::Page { url, title: String::from("A page"), snapshot: String::new(), truncated: false })
    }

    async fn answer_browser_request(
        &self,
        answer: ipc::AnswerBrowserRequest,
    ) -> Result<ipc::BrowserRequestAnswered, Refusal> {
        Ok(ipc::BrowserRequestAnswered { request_id: answer.request_id, session_id: ipc::SessionId::carried("s1") })
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
        caller: Option<crate::Caller>,
        start: ipc::StartAnotherSession,
    ) -> Result<ipc::SessionHeld, Refusal> {
        // Named by its title, so a batch's answers can be told apart. A call from no connection and a blank
        // title are refused as Fleet refuses them.
        if caller.is_none() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                ipc::AGENT_START_UNPLACED,
                "only a Job's own agent starts or forks a Job, and this call comes from none",
                crate::tests::shapes::run_id(),
            )));
        }
        if start.title.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.agent_start_empty",
                "a Job an agent starts needs a title and a first message",
                crate::tests::shapes::run_id(),
            )));
        }
        Ok(ipc::SessionHeld {
            session_id: ipc::SessionId::carried(format!("session-{}", start.title)),
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
            held: change.held.unwrap_or(true),
            workflow: None,
            model: None,
            effort: None,
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

    async fn claim_issue(
        &self,
        _caller: Option<crate::Caller>,
        claim: ipc::ClaimIssue,
    ) -> Result<ipc::IssueClaimed, Refusal> {
        Ok(ipc::IssueClaimed {
            issue: claim.issue,
            holder_id: String::from("session"),
            pull_requests: claim.pull_requests,
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

    async fn record_plan(&self, _caller: Option<crate::Caller>, plan: ipc::RecordPlan, seen: bool) -> Result<SessionRecord, Refusal> {
        self.plans_seen.lock().expect("plans seen").push(seen);
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: plan.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn answer_sketch(self: std::sync::Arc<Self>, answer: ipc::AnswerSketch) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: answer.session_id,
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn answer_document(self: std::sync::Arc<Self>, answer: ipc::AnswerDocument) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: answer.session_id,
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

    async fn get_shown_file(&self, _id: ipc::SessionId, path: String) -> Result<crate::ShownFile, Refusal> {
        Ok(crate::ShownFile::Text(ipc::ShownText { path, text: String::from("shown") }))
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

    async fn propose_draft(
        &self,
        _caller: Option<crate::Caller>,
        draft: ipc::ProposeDraft,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: draft.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted { said: None },
        })
        .await
    }

    async fn withdraw_draft(
        &self,
        _caller: Option<crate::Caller>,
        withdraw: ipc::WithdrawDraft,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: withdraw.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
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

    async fn resolve_pull_request_note(
        &self,
        _caller: Option<crate::Caller>,
        resolve: ipc::ResolvePullRequestNote,
    ) -> Result<ipc::PullRequestNote, Refusal> {
        Ok(ipc::PullRequestNote {
            id: resolve.note_id,
            repository: "armada/armada".into(),
            number: 7,
            path: "src/lib.rs".into(),
            line: 1,
            side: ipc::NoteSide::Right,
            body: "Too tight".into(),
            status: ipc::NoteStatus::Done,
            sent_to: None,
            sent_title: None,
            sent_at: None,
            created_at: "2026-10-27T09:00:00Z".into(),
        })
    }

    async fn dispute_fleet(
        self: std::sync::Arc<Self>,
        _caller: Option<crate::Caller>,
        _dispute: ipc::DisputeFleet,
    ) -> Result<ipc::FleetDisputed, Refusal> {
        Ok(ipc::FleetDisputed { dispute_id: String::from("dispute"), review_session_id: None, joined: false })
    }

    async fn report_bug(
        &self,
        _caller: Option<crate::Caller>,
        _report: ipc::ReportBug,
    ) -> Result<ipc::BugReported, Refusal> {
        Ok(ipc::BugReported { report_id: String::from("report") })
    }

    async fn list_bug_reports(&self) -> Result<ipc::BugReports, Refusal> {
        Ok(ipc::BugReports { reports: Vec::new() })
    }

    async fn keep_bug_report(&self, report_id: String) -> Result<ipc::BugReport, Refusal> {
        Ok(ipc::BugReport {
            id: report_id,
            session_id: String::from("session"),
            job_title: String::new(),
            title: String::from("Title"),
            what: String::from("What"),
            steps: Vec::new(),
            errors: Vec::new(),
            logs: Vec::new(),
            at: ipc::Instant::carried("2026-10-29T09:00:00Z"),
            kept: None,
        })
    }

    async fn drop_bug_report(&self, report_id: String) -> Result<ipc::BugReportDropped, Refusal> {
        Ok(ipc::BugReportDropped { id: report_id })
    }

    async fn defer_later(
        &self,
        _caller: Option<crate::Caller>,
        defer: ipc::DeferLater,
    ) -> Result<ipc::LaterDeferred, Refusal> {
        Ok(ipc::LaterDeferred { id: String::from("later-1"), sentence: defer.sentence, open: Vec::new() })
    }

    async fn settle_later(
        &self,
        _caller: Option<crate::Caller>,
        settle: ipc::SettleLater,
    ) -> Result<ipc::LaterSettled, Refusal> {
        Ok(ipc::LaterSettled { id: settle.id, sentence: String::new(), open: Vec::new() })
    }

    async fn list_closed_waiting(
        &self,
        _caller: Option<crate::Caller>,
        session_id: Option<ipc::SessionId>,
    ) -> Result<ipc::ClosedWaitingList, Refusal> {
        Ok(ipc::ClosedWaitingList {
            session_id: session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            closed: Vec::new(),
        })
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

    async fn recall(
        &self,
        _manifest_id: Option<ManifestId>,
        _query: String,
        _section: Option<String>,
    ) -> Result<ipc::Recalled, Refusal> {
        Ok(ipc::Recalled { entries: Vec::new() })
    }

    async fn get_logbook_for_subagent(&self, _session_id: String, _prompt: String) -> Result<ipc::ForSubagent, Refusal> {
        Ok(ipc::ForSubagent { text: String::new() })
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

    async fn get_session_system(self: std::sync::Arc<Self>, session_id: ipc::SessionId) -> Result<ipc::SessionSystem, Refusal> {
        Ok(ipc::SessionSystem {
            session_id,
            processes: Vec::new(),
            cpu_percent: None,
            memory_bytes: None,
            memory_of_bytes: None,
            disk_bytes: None,
            disk_of_bytes: None,
            worktrees: Vec::new(),
            logs: Vec::new(),
        })
    }

    async fn stop_process(
        self: std::sync::Arc<Self>,
        _caller: Option<crate::Caller>,
        stop: ipc::StopProcess,
    ) -> Result<ipc::ProcessStopped, Refusal> {
        Ok(ipc::ProcessStopped { pid: stop.pid.unwrap_or(0), command: String::new() })
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

    async fn get_attachment_history(&self, session_id: ipc::SessionId) -> Result<ipc::AttachmentHistory, Refusal> {
        // One Job's history says where its versions came from, so a route test can read it off the wire.
        if session_id.as_str() != "followed" {
            return Ok(ipc::AttachmentHistory::default());
        }
        let version = |from_message: &str, from_task: Option<&str>| ipc::AttachmentVersion {
            kind: "artifact".into(),
            manifest_id: None,
            target: "sketch:Flow".into(),
            state: ipc::AttachmentState::Standing,
            detail: Default::default(),
            at: ipc::Instant::carried("2026-10-22T09:00:00.000Z"),
            from_message: Some(from_message.into()),
            from_task: from_task.map(str::to_string),
        };
        Ok(ipc::AttachmentHistory { versions: vec![version("m1", Some("t3")), version("m2", None)] })
    }

    async fn read_loadout_item(&self, _asked: ipc::LoadoutItemAsked) -> Result<ipc::LoadoutItemBody, Refusal> {
        Ok(ipc::LoadoutItemBody::NotRead { why: String::from("A fake reads nothing.") })
    }
}

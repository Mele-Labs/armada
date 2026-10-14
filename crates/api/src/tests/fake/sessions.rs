//! The session ledger, as the fake answers it: a report is echoed back as the
//! row it would make, and a read finds nothing. What the ledger keeps and
//! matches is `fleet::sessioning`'s, against a real store.

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
            helm: false,
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
        &self,
        _start: ipc::StartSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.runs_nothing_here()
    }

    async fn stop_session_run(
        &self,
        _stop: ipc::StopSessionRun,
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
        Ok(ipc::SessionCheckAnswer { run: call.run.or(Some(1)) })
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

    async fn start_another_session(
        &self,
        _caller: Option<crate::Caller>,
        start: ipc::StartAnotherSession,
    ) -> Result<ipc::SessionHeld, Refusal> {
        Ok(ipc::SessionHeld {
            session_id: ipc::SessionId::carried("session"),
            title: start.title,
            fork: false,
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
            holder_kind: String::from("session"),
            holder_id: String::from("session"),
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

    async fn ask_in_window(
        &self,
        _caller: Option<crate::Caller>,
        ask: ipc::AskInWindow,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: ask.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted,
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

    async fn all_done(
        &self,
        _caller: Option<crate::Caller>,
        done: ipc::AllDone,
    ) -> Result<SessionRecord, Refusal> {
        self.report_session(SessionReport {
            harness: String::new(),
            session_id: done.session_id.unwrap_or_else(|| ipc::SessionId::carried("session")),
            fact: ipc::SessionFact::TurnCompleted,
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
}

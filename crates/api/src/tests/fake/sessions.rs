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

    async fn list_sessions(
        &self,
        _manifest_id: Option<ManifestId>,
        _text: Option<String>,
        _state: Option<SessionState>,
    ) -> Result<SessionList, Refusal> {
        Ok(SessionList {
            sessions: Vec::new(),
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

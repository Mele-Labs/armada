//! The store rows and the wire records, each spelled as the other, and the plugin version check.

use super::*;

/// The `version` the `armada` plugin declares, where it carries one: the installed copy under `home`
/// first, which is the one a session loads, and the repository's own where there is none.
pub(super) fn mod_version_in(home: &str, root: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        version: Option<String>,
    }
    let installed = crate::runtime::mod_dir(home).join(adapters::MOD_INSTALLED_MANIFEST);
    let bytes = std::fs::read(installed)
        .or_else(|_| std::fs::read(std::path::Path::new(root).join(adapters::MOD_MANIFEST)))
        .ok()?;
    ipc::decode::<Manifest>("the armada plugin's manifest", &bytes)
        .ok()?
        .version
}

/// Whether `reported` is behind `current`. No version reported is behind; a version that does not
/// read as dotted numbers is behind unless it is the same text.
pub(crate) fn mod_is_older(reported: Option<&str>, current: &str) -> bool {
    let Some(reported) = reported else {
        return true;
    };
    let parts = |text: &str| -> Option<Vec<u64>> { text.split('.').map(|one| one.parse().ok()).collect() };
    match (parts(reported), parts(current)) {
        (Some(had), Some(want)) => had < want,
        _ => reported != current,
    }
}

pub(super) fn origin_text(origin: ipc::SessionOrigin) -> &'static str {
    match origin {
        ipc::SessionOrigin::Terminal => "terminal",
        ipc::SessionOrigin::Bridge => "bridge",
    }
}

pub(super) fn attachment_state(state: ipc::AttachmentState) -> AttachmentState {
    match state {
        ipc::AttachmentState::Standing => AttachmentState::Standing,
        ipc::AttachmentState::Spent => AttachmentState::Spent,
        ipc::AttachmentState::GivenBack => AttachmentState::GivenBack,
    }
}

pub(super) fn session_state(state: SessionState) -> store::SessionState {
    match state {
        SessionState::Live => store::SessionState::Live,
        SessionState::Ended => store::SessionState::Ended,
    }
}

pub(super) fn wire_instant(at: &str) -> ipc::Instant {
    ipc::Instant::carried(at)
}

pub(super) fn attachment_wire(row: &KeptAttachment) -> ipc::Attachment {
    ipc::Attachment {
        kind: row.kind.clone(),
        manifest_id: (!row.manifest_id.is_empty()).then(|| ManifestId::carried(&row.manifest_id)),
        target: row.target.clone(),
        state: match row.state {
            AttachmentState::Standing => ipc::AttachmentState::Standing,
            AttachmentState::Spent => ipc::AttachmentState::Spent,
            AttachmentState::GivenBack => ipc::AttachmentState::GivenBack,
        },
        detail: row.detail.clone(),
        since: wire_instant(&row.since),
        changed_at: wire_instant(&row.changed_at),
    }
}

pub(super) fn version_wire(row: &store::KeptAttachmentVersion) -> ipc::AttachmentVersion {
    ipc::AttachmentVersion {
        kind: row.kind.clone(),
        manifest_id: (!row.manifest_id.is_empty()).then(|| ManifestId::carried(&row.manifest_id)),
        target: row.target.clone(),
        state: match row.state {
            AttachmentState::Standing => ipc::AttachmentState::Standing,
            AttachmentState::Spent => ipc::AttachmentState::Spent,
            AttachmentState::GivenBack => ipc::AttachmentState::GivenBack,
        },
        detail: row.detail.clone(),
        at: wire_instant(&row.at),
        from_message: row.from_message.clone(),
        from_task: row.from_task.clone(),
    }
}

pub(super) fn held_wire(row: &KeptAttachment, title: Option<String>) -> ipc::Ownership {
    ipc::Ownership {
        holder: ipc::Holder { kind: ipc::HolderKind::Session, id: row.holder.id.clone() },
        attachment: attachment_wire(row),
        title,
    }
}

pub(super) fn session_wire(session: &KeptSession, held: &[KeptAttachment]) -> SessionRecord {
    SessionRecord {
        id: ipc::SessionId::carried(&session.id),
        harness: session.harness.clone(),
        origin: if session.origin == "bridge" {
            ipc::SessionOrigin::Bridge
        } else {
            ipc::SessionOrigin::Terminal
        },
        manifest_id: session.manifest_id.as_deref().map(ManifestId::carried),
        cwd: session.cwd.clone(),
        title: session.title.clone(),
        state: match session.state {
            store::SessionState::Live => SessionState::Live,
            store::SessionState::Ended => SessionState::Ended,
        },
        started_at: wire_instant(&session.started_at),
        last_seen_at: wire_instant(&session.last_seen_at),
        last_turn_at: session.last_turn_at.as_deref().map(wire_instant),
        ended_at: session.ended_at.as_deref().map(wire_instant),
        end_reason: session.end_reason.clone(),
        usage: ipc::SessionUsage {
            context_tokens: session.figures.context_tokens,
            context_window: session.figures.context_window,
            cost_micros: session.figures.cost_micros,
        },
        attachments: held.iter().map(attachment_wire).collect(),
        hosted: None,
        terminal: None,
        mod_out_of_date: false,
        tools_missing: None,
        waiting_for: Vec::new(),
        done_since: None,
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
        held_open_by: None,
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
        workflow_waiting_on: Vec::new(),
        workflow_copy: None,
    }
}

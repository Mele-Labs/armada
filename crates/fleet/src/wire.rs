//! The redactions the `Queries` and `Commands` impls call by hand, split out of
//! `serving` and `commanding` so those files stay the thing that grows.
//!
//! Every function here is the visible line somebody had to write to put a
//! domain value on the wire. None of them is a `From` — the orphan rule puts one at this
//! boundary in `ipc`, and `ipc` has no `store` or `config` to convert from —
//! so the field-by-field decision is written here, where every type involved
//! is in scope.

use ipc::{
    DeclaredCheck, DeclaredJudge, ManifestId, ManifestSummary, StepId, WorkflowId, WorkflowStep,
    WorkflowSummary,
};

/// The wire shape of one declared Check.
///
/// A free function rather than a `From` impl: `ipc` already depends on
/// `core-model` and could hold this, but a `DeclaredCheck` is assembled from a
/// step rather than converted from one, and the assembly is Fleet's.
///
/// **The command crosses, and it comes off the resolved workflow.** A
/// `ResolvedCheck` holds the `run` this workflow froze, which is what the gate
/// runs; serving it from the live Manifest instead would show a command that is
/// not what ran the moment somebody edits `armada.yml` under a Job.
pub(crate) fn declared_check(check: &core_model::ResolvedCheck) -> DeclaredCheck {
    DeclaredCheck {
        kind: check.kind().to_string(),
        name: check.name().map(str::to_string),
        run: check.run().map(str::to_string),
        expect_exit_code: check.expects(),
        // **The frozen list, for `run`'s reason.** A Check's paths are lifted
        // out of the Manifest at Job creation like its command is, so serving
        // the live `armada.yml` would show a scope that is not the one this
        // Job's gate will decide against.
        when: check.when().map(|covers| {
            covers
                .patterns()
                .iter()
                .map(|pattern| pattern.as_str().to_string())
                .collect()
        }),
        // Absent where it runs everywhere, for `when`'s reason. #849.
        runs_at: Some(check.runs_at())
            .filter(|runs| *runs != core_model::RunsAt::Everywhere)
            .map(|runs| runs.as_wire().to_string()),
    }
}

/// The handoff-only Checks a step leaves to a later one, by name. #849.
pub(crate) fn held_for_handoff(
    workflow: &core_model::FrozenWorkflow,
    step: &core_model::StepId,
) -> Vec<String> {
    workflow
        .held_for_handoff(step)
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// Every Check a step declares, as a reader sees them.
///
/// **One function and two call sites**, because a preview of what a workflow
/// would freeze and a Job's account of what it did freeze have to draw the same
/// declaration. Two copies of the prepend below would agree until one changed.
///
/// **`every_manifest_check` leads the list where the step said it**, ahead of
/// the Checks it expanded to, because it is what those rows came from rather
/// than a Check beside them. It carries no name, no command and no paths — the
/// row is the declaration and the rows under it are the answer.
///
/// The case that makes it load-bearing rather than decorative is a repository
/// declaring no Checks, which `docs/concepts/manifest.md` sanctions: the
/// expansion is empty, and without this row a step that asked to be gated on
/// everything and got nothing is drawn exactly like a step that asked for
/// nothing. Those need opposite readings — one verified nothing while asking
/// to — and this is the row that says which happened, on the Job's own record
/// and without a transcript.
pub(crate) fn declared_checks(step: &core_model::ResolvedStep) -> Vec<DeclaredCheck> {
    let sweeping = step.gates_on_every_check().then(|| DeclaredCheck {
        kind: core_model::EVERY_MANIFEST_CHECK.to_string(),
        name: None,
        run: None,
        expect_exit_code: None,
        when: None,
        runs_at: None,
    });
    sweeping
        .into_iter()
        .chain(step.checks().iter().map(declared_check))
        .collect()
}

/// The word a step is drawn as, with its id standing in where there is none.
///
/// A blank label is a definition that declared the key and left it empty, and a
/// blank on the rail reads as a Fleet that lost the value.
fn reads_as(label: &str, step_id: &str) -> String {
    match label.trim().is_empty() {
        true => step_id.to_string(),
        false => label.to_string(),
    }
}

/// A workflow's steps with what each one declares, in the workflow's order.
///
/// **This is what the next Job would freeze**, and `get_job` answers from what
/// its Job already froze. The two can now differ, which is the point: a
/// workflow edited under a running Job shows the new declaration here and the
/// approved one there.
///
/// Both tiers and the gate, because this is read *before* a dispatch: after the
/// fact the rail says what happened, and here a person is agreeing to it.
pub(crate) fn declared(workflow: &config::ResolvedWorkflow) -> Vec<WorkflowStep> {
    workflow
        .steps()
        .iter()
        .map(|step| WorkflowStep {
            step_id: StepId::from(step.id()),
            label: reads_as(step.label(), step.id().as_str()),
            checks: declared_checks(step),
            // The rail's own narrowing, called rather than restated: an entry
            // that asks nothing and looks for nothing is not a Judge call, and
            // a preview that counted one would promise a call nothing makes.
            judge_checks: DeclaredJudge::firing(step.judge_checks()),
            advance_gate: step.advance_gate().into(),
            delivers: step.delivers(),
            held_for_handoff: held_for_handoff(workflow.frozen(), step.id()),
            verdict_routing_target: step.verdict_routing().values().next().map(StepId::from),
            // Zero is `ResolvedStep`'s own "no loop", the same sentinel
            // `ipc::StepDetail::of` reads off `StepPass` — absent on the wire
            // rather than a cap of zero, which is not a cap anything declared.
            iteration_cap: Some(step.iteration_cap()).filter(|cap| *cap > 0),
            phase: step.phase().into(),
            about: step.about().map(str::to_string),
        })
        .collect()
}

/// A path as the filesystem knows it, or as it was given where it cannot be
/// resolved.
///
/// A Manifest that has just been read exists, so the fallback covers the case
/// where it stopped existing between the read and the ask — and a path saying
/// where Fleet looked beats an empty string saying nothing.
pub(crate) fn canonical(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// One workflow as a caller sees it before naming one, with [`declared`]
/// carrying the steps.
///
/// The Manifest id travels on every row rather than beside the list: a
/// workflow is only nameable against the Manifest it was resolved from, and a
/// caller holding one row alone would otherwise have to remember which.
pub(crate) fn workflow_summary(
    workflow: &config::ResolvedWorkflow,
    manifest_id: &core_model::ManifestId,
    files: &[config::WorkflowFile],
) -> WorkflowSummary {
    let of_this = || files.iter().filter(|file| file.id() == workflow.id());
    WorkflowSummary {
        id: WorkflowId::from(workflow.id()),
        name: workflow.name().to_string(),
        version: workflow.version(),
        steps: declared(workflow),
        manifest_id: ManifestId::from(manifest_id),
        source: workflow.source().as_wire().to_string(),
        for_requests: workflow.for_requests().map(str::to_string),
        file: of_this()
            .find(|file| file.overridden_by().is_none())
            .map(|file| file.path().to_string_lossy().to_string())
            .unwrap_or_default(),
        overrides: of_this()
            .filter(|file| file.overridden_by().is_some())
            .map(|file| ipc::OverriddenWorkflow {
                source: file.source().as_wire().to_string(),
                file: file.path().to_string_lossy().to_string(),
            })
            .collect(),
        hides: of_this()
            .find(|file| file.overridden_by().is_none())
            .and_then(config::WorkflowFile::hides)
            .map(|place| place.as_wire().to_string()),
    }
}

/// One definition file as `get_workflow` answers it.
pub(crate) fn workflow_definition(file: &config::WorkflowFile) -> ipc::WorkflowDefinition {
    ipc::WorkflowDefinition {
        workflow_id: WorkflowId::from(file.id()),
        source: file.source().as_wire().to_string(),
        file: file.path().to_string_lossy().to_string(),
        definition: file.text().to_string(),
        overridden_by: file
            .overridden_by()
            .map(|place| place.as_wire().to_string()),
    }
}

/// A definition `config`'s catalogue left out, as `list_left_out_workflows` serves it.
pub fn left_out_workflow(left: &config::LeftOut) -> ipc::LeftOutWorkflow {
    ipc::LeftOutWorkflow {
        id: left.id().map(WorkflowId::from),
        source: left.source().as_wire().to_string(),
        file: left.path().to_string_lossy().to_string(),
        said: left.to_string(),
        instead: left.instead().map(|place| place.as_wire().to_string()),
    }
}

/// The Manifest a Fleet was started against, as the wire carries it.
///
/// **`repository` is not a name the Manifest declares.** `armada.yml` has no
/// key for one — `version`, `id`, `checks` and `commands` are the whole schema
/// — so what is carried is the directory the file was read from, which is a
/// fact rather than an invention. A person reading a Job wants to know which
/// project it runs against, and a ULID does not say.
pub(crate) fn manifest_summary(manifest: &config::Manifest, records_root: &str) -> ManifestSummary {
    let path = manifest.path();
    ManifestSummary {
        id: ManifestId::from(manifest.id()),
        repository: path
            .parent()
            .and_then(|dir| dir.file_name())
            .map(|name| name.to_string_lossy().to_string())
            // A Manifest at the filesystem root has no directory to name. Its
            // own path is the next most useful true thing.
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        // **Absolute, and it has to be.** Bridge derives the worktree from
        // this one and then hands the result to the OS to open. A relative
        // path answers a question about Fleet's working directory, which is
        // not a fact about the repository and is not a directory Bridge is
        // in: served as `./armada.yml`, every one of those opens resolved
        // against the Electron process and found nothing.
        //
        // `$HOME` therefore appears on this wire, which the log envelope and
        // the failure record both refuse. It is a different surface: those are
        // written down and read later, and this is two processes on one machine
        // agreeing where a file is.
        path: canonical(path),
        // **Absolute for the same reason, and never derived from `path`.**
        // Fleet's Job log, its Drones' transcripts, a Check's output, a
        // Judge's brief and a kept deliverable no longer sit anywhere under
        // the repository, so there is no arithmetic on `path` that would find
        // them — this is the one value Bridge can join them against.
        records_root: records_root.to_string(),
        version: manifest.version(),
        checks: manifest.check_names(),
        // The two words a `manifest_rule:` gate defers to, read off the live
        // cell like every other policy question. Absent from the file is the
        // policy's own default here, which is what the gate would resolve.
        auto_merge: manifest.auto_merge().as_written().to_string(),
        review_gate: manifest.review_gate().as_written().to_string(),
    }
}

/// A lease's `since`, seconds on the record, as the wire spells the instant.
pub(crate) fn since_of(secs: Option<u64>) -> Option<ipc::Instant> {
    secs.map(|secs| {
        ipc::Instant::carried(crate::clock::rfc3339_utc(
            i64::try_from(secs.saturating_mul(1000)).unwrap_or(i64::MAX),
        ))
    })
}

/// One pool slot. `since` is seconds on the wire's clock, read off the lease
/// record.
pub(crate) fn worktree_slot(one: &crate::leasing::PoolSlot) -> ipc::WorktreeSlot {
    use adapter_traits::SlotHeld;
    let reading = &one.reading;
    ipc::WorktreeSlot {
        manifest_id: ManifestId::carried(one.manifest.clone()),
        slot: reading.slot,
        path: reading.path.clone(),
        held: match &reading.held {
            SlotHeld::Unmade => ipc::SlotHolding::Unmade,
            SlotHeld::NotACheckout => ipc::SlotHolding::NotACheckout,
            SlotHeld::Busy => ipc::SlotHolding::Busy,
            SlotHeld::Free => ipc::SlotHolding::Free,
            SlotHeld::Hosted(id) => ipc::SlotHolding::HostedSession {
                session_id: id.clone(),
                session_title: one.session.as_ref().and_then(|session| session.title.clone()),
            },
            SlotHeld::Session(holder) => ipc::SlotHolding::Session {
                holder: holder.clone(),
                session_id: one.session.as_ref().map(|session| session.id.clone()),
                session_title: one.session.as_ref().and_then(|session| session.title.clone()),
            },
            SlotHeld::Stranded(why) => ipc::SlotHolding::Stranded { why: why.clone() },
        },
        base: one.base.clone(),
        branch: reading.branch.clone(),
        since: since_of(reading.since),
        warm: reading.warm,
        behind: reading.behind,
        closed: reading.closed,
        stranded: one.stranded.as_ref().map(|work| ipc::SlotStranded {
            uncommitted: work.uncommitted.clone(),
            commits: work
                .commits
                .iter()
                .map(|commit| ipc::SlotCommit {
                    sha: commit.sha.clone(),
                    subject: commit.subject.clone(),
                    home: match commit.home {
                        adapter_traits::CommitHome::OnlyHere => ipc::CommitHome::OnlyHere,
                        adapter_traits::CommitHome::OnRemote => ipc::CommitHome::OnRemote,
                        adapter_traits::CommitHome::OnMain => ipc::CommitHome::OnMain,
                    },
                })
                .collect(),
            unpushed: work.unpushed,
            last_holder: one.last_holder.clone(),
            told: one.told.clone().map(ipc::Instant::carried),
        }),
        rescue: one.rescue.as_ref().map(|kept| ipc::SlotFinding {
            state: match kept.state {
                store::RescueState::Reading => ipc::SlotFindingState::Reading,
                store::RescueState::Answered => ipc::SlotFindingState::Answered,
                store::RescueState::Stopped => ipc::SlotFindingState::Stopped,
                store::RescueState::Failed => ipc::SlotFindingState::Failed,
            },
            commit: kept.commit.clone(),
            uncommitted: kept.uncommitted,
            cut: kept.cut,
            read: kept.read.clone(),
            searched: kept.searched.clone(),
            verdict: kept.verdict.map(|verdict| match verdict {
                store::RescueVerdict::Unfinished => ipc::SlotVerdict::Unfinished,
                store::RescueVerdict::Scraps => ipc::SlotVerdict::Scraps,
            }),
            items: kept.items.clone(),
            summary: kept.summary.clone(),
            why: kept.why.clone(),
            cost_micros: kept.cost_micros,
        }),
    }
}

/// The adapter's word for what happened to a file, in the wire's spelling.
///
/// A function rather than a `From`: neither type belongs to this crate, so the impl could only
/// live in `ipc` — and that would put the harness seam's vocabulary under the wire crate to save a
/// `match` that is the boundary doing its job.
pub(crate) fn kind(change: adapter_traits::Change) -> ipc::ChangeKind {
    use adapter_traits::Change;
    match change {
        Change::Added => ipc::ChangeKind::Added,
        Change::Modified => ipc::ChangeKind::Modified,
        Change::Deleted => ipc::ChangeKind::Deleted,
        Change::Renamed => ipc::ChangeKind::Renamed,
        Change::Copied => ipc::ChangeKind::Copied,
        Change::TypeChanged => ipc::ChangeKind::TypeChanged,
        Change::Conflicted => ipc::ChangeKind::Conflicted,
        Change::Unreadable => ipc::ChangeKind::Unreadable,
    }
}

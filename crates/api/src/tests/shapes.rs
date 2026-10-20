//! The shapes the fake answers with, fixed and built out of `ipc` alone.
//!
//! Split from [`fake`](super::fake) at the 900-line refusal, along the seam
//! that module's header already names: these are the values it hands back.
//!
//! **This half carries the claim.** Every shape below is constructed without
//! `core-model` and every spelling goes through `from_wire`, so a shape a test
//! can build here is a shape a Bridge can build, and a spelling this can check
//! against the registry is one a client can check against the same registry.
//!
//! Nothing here reads the record and nothing here refuses. A value that had to
//! know what was asked for, or whether there was one, belongs on the other side
//! of the seam — which is what keeps these readable as the answer alone.

use ipc::{
    FleetHealth, HelmActionAuthority, ManifestConfig, ManifestId, ManifestSummary,
    ModelChoices, Probe, RunId, StepId, Unprobed, WorkflowId, WorkflowSummary,
};

mod scanning;
pub use scanning::{manifest_drift, repository_scan};

pub fn run_id() -> RunId {
    RunId::carried("01RUN")
}

/// The one call id this fake holds an open command for. Anything else is a
/// call it cannot explain.
pub const THE_CALL: &str = "toolu_01Haa";

/// The one Drone this fake is holding a slot for. Anything else names no live
/// process, which is a 404 and not a detail with nothing in it.
pub const THE_DRONE: &str = "01DRONE";

/// The one Manifest this fake serves, and therefore the only scope.
pub const THE_MANIFEST: &str = "01MF";

/// The one kept Check output this fake's record holds, spelled as
/// `fleet::check_output` names a file: step, attempt, ordinal. Anything else is
/// an output no row of the Job carries, which is a different answer from a
/// missing Job.
pub const THE_OUTPUT: &str = "implement.1.0.log";


/// The one retro item this fake knows, which it answers with a refusal that is
/// not a 404, so the routed-operations walk can tell the route is there.
pub const THE_LESSON: &str = "01JOB0-0";

/// The one kept frame this fake's record holds, spelled the way a frame is
/// named: the run's directory and the harness's own file name, joined.
///
/// **Two components, unlike [`THE_OUTPUT`].** A Check's output file name is
/// built by Fleet out of the row's key and is unique within the Job by
/// construction; a frame's name came from the repository's spec, so the run
/// directory in front of it is what identifies a row.
pub const THE_FRAME: &str = "implement.1/job-detail-refused.png";






/// Fleet on `main`, three commits behind `origin/main`, nothing restarting.
pub fn fleet_build() -> ipc::FleetBuildReport {
    ipc::FleetBuildReport {
        on: ipc::BuildSource::Main,
        commit: Some(String::from("0123456789abcdef0123456789abcdef01234567")),
        position: Some(ipc::BuildPosition {
            ahead: 0,
            behind: 3,
        }),
        restarting: None,
        stage: None,
        failed: None,
    }
}

/// The shipped numbers, in force because nothing has been saved.
pub fn limits() -> ipc::FleetLimits {
    let shipped = ipc::LimitValues {
        concurrency: 2,
        memory_spare_percent: 15,
        disk_floor_gib: 10,
        checks_at_once: 4,
    };
    ipc::FleetLimits {
        values: shipped,
        shipped,
    }
}

/// Nothing saved, so every preference reads its shipped default.
pub fn preferences() -> ipc::Preferences {
    ipc::Preferences::default()
}

/// A refused re-read: the shape with the most in it, because a refusal carries
/// what an adoption does not.
pub fn manifest_reading() -> ipc::ManifestReading {
    ipc::ManifestReading {
        path: "armada.yml".to_string(),
        at: ipc::Instant::carried("2026-09-04T09:00:00.000Z"),
        moved: Vec::new(),
        at_restart: Vec::new(),
        refused: Some(ipc::ManifestRefused {
            summary: "armada.yml was refused; `drone.poke_limit` is not a number".to_string(),
            faults: vec![ipc::ManifestFault {
                key: "drone.poke_limit".to_string(),
                fault: "is not a number".to_string(),
            }],
        }),
    }
}

/// The file behind the reading above, refused for the same key — so a test can
/// see the two answers are about one `armada.yml`.
pub fn manifest_file() -> ipc::ManifestFile {
    ipc::ManifestFile {
        path: "armada.yml".to_string(),
        text: "version: 1\nid: fake\ndrone:\n  poke_limit: soon\n".to_string(),
        // Refused for `poke_limit`, so it does not load and the forms have nothing.
        declared: None,
    }
}

/// One Check, and both policies at their defaults, for an edit's answer.
pub fn manifest_declared() -> ipc::ManifestDeclared {
    let words = |written: &str, offered: &[&str]| ipc::PolicyWords {
        written: written.to_string(),
        offered: offered.iter().map(|word| word.to_string()).collect(),
    };
    ipc::ManifestDeclared {
        id: None,
        version: None,
        checks: vec![ipc::NamedCheck {
            name: "lint".to_string(),
            check: ipc::CheckDraft {
                run: "pnpm -r lint".to_string(),
                expect_exit_code: 0,
                requires: Vec::new(),
                when: Vec::new(),
                narrow: None,
            },
        }],
        commands: Vec::new(),
        ports: Vec::new(),
        auto_merge: words("never", &["never", "checks-pass", "always"]),
        review_gate: words("human_always", &["human_always", "auto_if_judge_passes"]),
        freeze: false,
        cost_cap_micros_per_job: None,
        turn_cap_per_job: None,
        base: None,
        evidence: None,
        after_merge_checks: Vec::new(),
        setup_requires: Vec::new(),
        quiet_after_seconds: None,
        poke_limit: None,
        exclude_paths: Vec::new(),
        pr_mode: None,
    }
}

/// The one workflow the fake holds. A list, because the operation is one.
pub fn workflows() -> Vec<WorkflowSummary> {
    vec![WorkflowSummary {
        id: WorkflowId::carried("01WF"),
        name: "a-workflow".to_string(),
        version: 1,
        source: "armada".to_string(),
        for_requests: Some("A change somebody asked for by name".to_string()),
        file: "<carried by Armada>/a-workflow.json".to_string(),
        overrides: vec![ipc::OverriddenWorkflow {
            source: "kit".to_string(),
            file: "/kit/workflows/a-workflow.json".to_string(),
        }],
        hides: None,
        steps: vec![
            // Gated, and ungated. The pair is the distinction a preview's rail
            // turns on, so the fake carries both rather than one — and the
            // second is the step that stops for a person, which a preview drew
            // as a step with nothing on it.
            ipc::WorkflowStep {
                step_id: StepId::carried("implement"),
                label: "Implement the change".to_string(),
                checks: vec![ipc::DeclaredCheck {
                    runs_at: None,
                    kind: "manifest_check".to_string(),
                    name: Some("build".to_string()),
                    run: Some("cargo build --workspace --locked".to_string()),
                    expect_exit_code: Some(0),
                    // Covering everything, which is most Checks. The scoped
                    // case is asserted in `ipc`'s own round-trip.
                    when: None,
                }],
                judge_checks: vec![ipc::DeclaredJudge {
                    criteria: 2,
                    panel_size: None,
                }],
                advance_gate: ipc::AdvanceGate::from_wire("auto_if_judge_passes")
                    .expect("a gate the registry has"),
                delivers: false,
                held_for_handoff: Vec::new(),
                verdict_routing_target: None,
                iteration_cap: None,
                phase: ipc::StepPhase::Work,
                about: None,
            },
            ipc::WorkflowStep {
                step_id: StepId::carried("handoff"),
                label: "Hand the work back".to_string(),
                checks: Vec::new(),
                judge_checks: Vec::new(),
                advance_gate: ipc::AdvanceGate::from_wire("human_always")
                    .expect("a gate the registry has"),
                delivers: true,
                held_for_handoff: Vec::new(),
                verdict_routing_target: None,
                iteration_cap: None,
                phase: ipc::StepPhase::Delivery,
                about: None,
            },
        ],
        manifest_id: ManifestId::carried("01MF"),
    }]
}

/// The one Manifest the fake holds.
pub fn manifests() -> Vec<ManifestSummary> {
    vec![ManifestSummary {
        id: ManifestId::carried(THE_MANIFEST),
        repository: "a-repository".to_string(),
        path: "/a-repository/armada.yml".to_string(),
        records_root: "/records/a-repository-key".to_string(),
        version: 1,
        checks: vec!["build".to_string()],
        // Both policies' own defaults, which is what a file saying nothing
        // resolves to at the gate.
        auto_merge: "never".to_string(),
        review_gate: "human_always".to_string(),
        kit: None,
    }]
}

/// A small fixed tree, narrowed the way `fleet::files::search` narrows a real
/// checkout — a case-insensitive substring test against the relative path.
/// Enough for a route test to prove the query reaches the daemon and the
/// answer reaches the wire, without a filesystem behind the fake.
pub fn files_found(query: &str) -> Vec<String> {
    const TREE: &[&str] = &["src/main.rs", "src/lib.rs", "README.md", "Cargo.toml"];
    let needle = query.to_lowercase();
    TREE.iter()
        .filter(|path| needle.is_empty() || path.to_lowercase().contains(needle.as_str()))
        .map(|path| path.to_string())
        .collect()
}

/// Two repositories, and `storefront` clearly meant where the words say so.
/// Enough for a route test to prove the words reach the daemon and that an
/// answer with no pick leaves `manifest_id` out rather than sending `null`.
pub fn request_repository(words: &str) -> ipc::RequestRepository {
    let storefront = ipc::ManifestId::carried("storefront");
    let pocket = ipc::ManifestId::carried("pocket");
    if words.contains("storefront") {
        ipc::RequestRepository {
            manifest_id: Some(storefront.clone()),
            likely: vec![storefront, pocket],
        }
    } else {
        ipc::RequestRepository {
            manifest_id: None,
            likely: vec![pocket, storefront],
        }
    }
}

/// Two entries a quarter hour apart, cut to what the ask's bounds and limit leave. Enough for a
/// route test to prove the three bounds reach the daemon as the ask Bridge wrote.
pub fn fleet_timeline(ask: &ipc::TimelineAsk) -> ipc::FleetTimeline {
    let entry = |id: u64, at: &str, kind: ipc::TimelineKind, subject: ipc::TimelineSubject| {
        ipc::TimelineEvent {
            id,
            at: ipc::Instant::carried(at),
            kind,
            subject,
            title: None,
            tone: ipc::TimelineTone::Idle,
            says: "Started".to_string(),
            detail: None,
            items: Vec::new(),
            call: None,
            manifest_id: None,
            live: false,
            over: false,
        }
    };
    let all = [
        entry(
            1,
            "2026-10-12T10:00:00.000Z",
            ipc::TimelineKind::SessionStarted,
            ipc::TimelineSubject::Session { id: ipc::SessionId::carried("01SESSION") },
        ),
        entry(
            2,
            "2026-10-12T10:15:00.000Z",
            ipc::TimelineKind::MainRed,
            ipc::TimelineSubject::Main,
        ),
    ];
    let mut events: Vec<_> = all
        .into_iter()
        .filter(|one| ask.since.as_ref().is_none_or(|since| one.at >= *since))
        .filter(|one| ask.until.as_ref().is_none_or(|until| one.at < *until))
        .collect();
    let limit = ask.limit.unwrap_or(ipc::DEFAULT_TIMELINE_LIMIT) as usize;
    let cut = events.len() > limit;
    events.drain(..events.len().saturating_sub(limit));
    ipc::FleetTimeline { events, cut }
}

/// Three entries for the slot asked about, newest first: enough for a route test to prove the
/// manifest and the slot reach the daemon as the ask Bridge wrote.
pub fn slot_history(ask: &ipc::BayHistoryAsk) -> ipc::BayHistory {
    let holder = ipc::BayHolder { kind: ipc::BayHolderKind::Agent, id: "sess-1".to_string(), title: None };
    ipc::BayHistory {
        manifest_id: ask.manifest_id.clone(),
        slot: ask.slot,
        entries: vec![
            ipc::BayHistoryEntry::Checkout {
                at: ipc::Instant::carried("2026-10-12T10:10:00.000Z"),
                from: Some("bug-fix".to_string()),
                to: "main".to_string(),
                sha: "a".repeat(40),
            },
            ipc::BayHistoryEntry::Released { at: ipc::Instant::carried("2026-10-12T10:05:00.000Z"), holder: holder.clone() },
            ipc::BayHistoryEntry::Leased {
                at: ipc::Instant::carried("2026-10-12T10:00:00.000Z"),
                holder,
                branch: Some("bug-fix".to_string()),
            },
        ],
        cut: false,
    }
}

/// Two models and a default, which is the shape a picker draws from.
pub fn models() -> ModelChoices {
    ModelChoices {
        models: vec!["a-model".to_string(), "another-model".to_string()],
        default: "a-model".to_string(),
        session: "another-model".to_string(),
        harnesses: vec!["a-harness".to_string()],
    }
}

/// One probe and the half of the answer a surface must not drop: what nobody
/// asked.
pub fn health() -> FleetHealth {
    FleetHealth {
        probes: vec![Probe {
            module: "Fleet".to_string(),
            outcome: "pass".to_string(),
            detail: "answering".to_string(),
        }],
        not_probed: vec![Unprobed {
            owner: "adapters".to_string(),
            because: "it owns talking to anything outside Armada".to_string(),
        }],
        helm_action_authority: HelmActionAuthority::Acting,
        workflows_left_out: true,
    }
}

/// What the one Manifest declares, past the summary a picker reads.
pub fn manifest_config() -> ManifestConfig {
    ManifestConfig {
        id: ManifestId::carried(THE_MANIFEST),
        summary: manifests()[0].clone(),
        base: Some("main".to_string()),
        checks_as_written: vec!["build".to_string()],
        proved_after_a_merge: Vec::new(),
        commands: vec!["migrate".to_string()],
        servers: vec!["web".to_string()],
        ports: vec!["web".to_string()],
        exclude_paths: vec![".armada/".to_string()],
        cost_cap_micros: Some(5_000_000),
        turn_cap: Some(300),
        quiet_after_seconds: Some(120),
        poke_limit: Some(3),
    }
}

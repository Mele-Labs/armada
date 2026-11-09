//! The dispatch walk's mock scenario: the faces the sheet is drawn from, and the answers Fleet
//! gives a send. A walk step names a scene and the screen changes to it, so a step goes straight to
//! the state it shows. The Jobs come from the Jobs list's own fixture, so the `@` menu offers Jobs
//! the list holds.

use ipc::{SessionRecord, SessionState};

use super::face::{
    AutopilotField, AutopilotRead, Chip, Cycle, Face, Glyph, Kind, Mark, Mentions, Offered, Part, Reading, Ticket, Tuning, Wait,
};

/// Repositories the mock Mac serves, in the order the chips take: the first nine keyed, the rest under More.
const REPOSITORIES: [&str; 4] = ["sample-api", "armada", "sample-web", "sample-docs"];
const PAST_NINE: [&str; 8] = ["sample-ops", "sample-ml", "sample-mobile", "sample-infra", "sample-kit", "sample-auth", "sample-search", "sample-billing"];

fn chips(names: &[&str], first_key: u8) -> Vec<Chip> {
    names
        .iter()
        .enumerate()
        .map(|(at, name)| Chip { name: name.to_string().into(), key: (first_key as usize + at <= 9).then_some(first_key + at as u8), mark: Mark::Plain })
        .collect()
}

/// The live Jobs of the Jobs list, as the sheet offers them to wait on or tag.
pub fn live(records: &[SessionRecord]) -> Vec<Offered> {
    records
        .iter()
        .filter(|job| job.state == SessionState::Live)
        .filter_map(|job| job.title.as_ref().map(|title| Offered { id: job.id.as_str().to_string().into(), title: title.clone().into() }))
        .collect()
}

/// The sheet at rest: connected, both kinds, every repository a chip, no one named.
pub fn rest(records: &[SessionRecord]) -> Face {
    Face {
        connected: true,
        kinds: vec![Kind::Job, Kind::Autopilot],
        chips: chips(&REPOSITORIES, 1),
        more: Vec::new(),
        reading: Reading::Idle,
        tuning: Tuning {
            default_model: Some("Opus".into()),
            models: vec!["Opus".into(), "Sonnet".into(), "Haiku".into()],
            efforts: ["low", "medium", "high", "xhigh", "max"].iter().map(|one| one.to_string().into()).collect(),
        },
        queue: true,
        mentions: Mentions {
            jobs: live(records),
            files: [
                "crates/fleet/src/sessions.rs",
                "crates/ipc/src/hosted_sessions.rs",
                "crates/bridge-native/src/jobs.rs",
                "docs/practices/native-bridge.md",
                "apps/desktop/src/renderer/src/dispatch-panel/DispatchPanel.tsx",
                "packages/tokens/src/spacing.css",
            ]
            .iter()
            .map(|one| one.to_string().into())
            .collect(),
            tickets: vec![
                Ticket { glyph: Glyph::PullRequest, id: "3391".into(), reference: "#3391".into(), title: "Name the native pieces Jobs screens mounts".into(), standing: Some("open".into()) },
                Ticket { glyph: Glyph::Issue, id: "3356".into(), reference: "#3356".into(), title: "Bridge redraws too much".into(), standing: Some("open".into()) },
                Ticket { glyph: Glyph::Bug, id: "3342".into(), reference: "#3342".into(), title: "Checkout page loads twice".into(), standing: Some("open".into()) },
                Ticket { glyph: Glyph::Terminal, id: "armada-a3f2".into(), reference: "armada-a3f2".into(), title: "Dispatch sheet keeps its draft".into(), standing: None },
            ],
        },
        wait: None,
        read: None,
        refused: None,
    }
}

/// Fleet read a repository from the words: the one they name is read and chosen.
pub fn found(mut face: Face) -> Face {
    for chip in &mut face.chips {
        chip.mark = if chip.name == "sample-api" { Mark::Chosen } else { Mark::Plain };
        if chip.name == "sample-api" {
            chip.mark = Mark::Chosen;
        }
    }
    face.reading = Reading::Detected;
    face
}

/// Fleet found no repository in the words.
pub fn unresolved(mut face: Face) -> Face {
    for chip in face.chips.iter_mut().chain(face.more.iter_mut()) {
        chip.mark = Mark::Plain;
    }
    face.reading = Reading::Unresolved;
    face
}

/// The rail is scoped to one repository.
pub fn scoped(mut face: Face) -> Face {
    for chip in &mut face.chips {
        chip.mark = if chip.name == "armada" { Mark::Chosen } else { Mark::Plain };
    }
    face.reading = Reading::Scoped;
    face
}

/// Twelve repositories: nine chips with keys, three under More.
pub fn many(mut face: Face) -> Face {
    let mut all: Vec<&str> = REPOSITORIES.to_vec();
    all.extend(PAST_NINE);
    let shown = chips(&all[..9], 1);
    let rest = chips(&all[9..], 10);
    face.chips = shown;
    face.more = rest;
    face
}

/// The brief waits on a Job: `Starts when [Job]'s [PR lands] · from [main] · needs [..]`.
pub fn waiting(mut face: Face) -> Face {
    let mut jobs = face.mentions.jobs.clone().into_iter();
    let on = jobs.next().unwrap_or(Offered { id: "job".into(), title: "Job".into() });
    face.wait = Some(Wait {
        on,
        when: Cycle::of("lands", &[("ends", "ends"), ("plan:schema", "Schema change is done"), ("pushed", "PR is pushed"), ("lands", "PR lands")]),
        base: Cycle::of("main", &[("main", "main"), ("branch", "its PR branch")]),
        needs: Some(Cycle::of(
            "the schema change and the seed data",
            &[("the schema change and the seed data", "the schema change and the seed data"), ("the schema change", "the schema change"), ("the seed data", "the seed data")],
        )),
        his: Vec::new(),
        others: jobs.take(4).collect(),
    });
    face
}

/// Fleet read an Autopilot sentence.
pub fn autopilot_read(mut face: Face) -> Face {
    let field = |label, value: &str, placeholder, quoted: Option<&str>| AutopilotField {
        label,
        value: value.to_string().into(),
        placeholder,
        quoted: quoted.map(|one| one.to_string().into()),
    };
    face.read = Some(AutopilotRead {
        instruction: "Work through the open bugs labelled flaky".into(),
        fields: vec![
            field("At once", "2", "1", Some("two at a time")),
            field("Until", "", "no time set", None),
            field("Spend, in dollars", "20", "no cap", Some("twenty dollars")),
            field("Jobs", "", "no cap", None),
            field("Weekly usage, in percent", "", "no cap", None),
        ],
    });
    face.refused = None;
    face
}

/// Fleet refused to read it.
pub fn autopilot_refused(mut face: Face) -> Face {
    face.read = None;
    face.refused = Some("Autopilot needs a repository with a Manifest, and sample-docs has none".into());
    face
}

/// The window lost Fleet: everything in the sheet is off.
pub fn offline(mut face: Face) -> Face {
    face.connected = false;
    face
}

/// Which chip a hand picks, for the walk's mock of the proposer reading the wait.
pub fn others_of(face: &Face) -> Vec<Offered> {
    face.wait.as_ref().map(|wait| wait.others.clone()).unwrap_or_default()
}

/// A part set by hand, for the scene that shows the accent edge.
pub fn set_by_hand(face: &mut Face, part: Part) {
    if let Some(wait) = &mut face.wait {
        wait.cycled(part, false);
    }
}

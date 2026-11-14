//! The dispatch walk's mock scenario: the faces the sheet is drawn from, and the answers Fleet
//! gives a send. A walk step names a scene and the screen changes to it, so a step goes straight to
//! the state it shows. The Jobs come from the Jobs list's own fixture, so the `@` menu offers Jobs
//! the list holds.

use ipc::{SessionRecord, SessionState};

use super::face::{Cycle, Face, Glyph, InProject, Mark, Mentions, Offered, Reading, Ticket, Tuning, Wait};

/// More repositories in no project, for the scene with more rows than keys.
const PAST_NINE: [&str; 8] = ["sample-ops", "sample-ml", "sample-mobile", "sample-infra", "sample-kit", "sample-auth", "sample-search", "sample-billing"];

/// The picker's rows on the mock Mac, as a live Fleet's are made: his projects from the mock's, each with
/// its repositories, then every repository the mock serves in no project.
fn rows() -> Vec<(Option<InProject>, gpui::SharedString)> {
    let served = crate::mock::repositories();
    let repositories = crate::titlebar::repositories_of(&served, &[]);
    let projects = super::live::projects_of(&crate::titlebar::projects_of(&served, &crate::mock::projects()), &repositories);
    super::live::rows_of(&super::live::served(&served), &projects)
}

/// The live Jobs of the Jobs list, as the sheet offers them to wait on or tag.
pub fn live(records: &[SessionRecord]) -> Vec<Offered> {
    records
        .iter()
        .filter(|job| job.state == SessionState::Live)
        .filter_map(|job| job.title.as_ref().map(|title| Offered { id: job.id.as_str().to_string().into(), title: title.clone().into() }))
        .collect()
}

/// The sheet at rest: connected, every repository a chip, no one named.
pub fn rest(records: &[SessionRecord]) -> Face {
    Face {
        connected: true,
        chips: super::live::chips(&rows()).0,
        more: Vec::new(),
        reading: Reading::Idle,
        tuning: Tuning {
            default_model: Some("Opus".into()),
            models: vec!["Sonnet".into(), "Haiku".into()],
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
        refused: None,
    }
}

/// Fleet read a repository from the words: its first row, the project's, is chosen.
pub fn found(face: Face) -> Face {
    super::live::read(face, Some(&"sample-api".into()))
}

/// Fleet found no repository in the words.
pub fn unresolved(mut face: Face) -> Face {
    for chip in face.chips.iter_mut().chain(face.more.iter_mut()) {
        chip.mark = Mark::Plain;
    }
    face.reading = Reading::Unresolved;
    face
}

/// The window is scoped to one repository in no project: its row alone, chosen.
pub fn scoped(face: Face) -> Face {
    super::live::narrowed(face, None, &["armada".into()])
}

/// More rows than keys: nine with keys, the rest under More.
pub fn many(mut face: Face) -> Face {
    let mut all = rows();
    all.extend(PAST_NINE.iter().map(|name| (None, name.to_string().into())));
    let (shown, more) = super::live::chips(&all);
    face.chips = shown;
    face.more = more;
    face
}

/// The brief waits on a Job: `Starts when [Job]'s [PR lands] · from [main] · needs [..]`.
pub fn waiting(mut face: Face, tagged: Option<Offered>) -> Face {
    let on = tagged.or_else(|| face.mentions.jobs.first().cloned()).unwrap_or(Offered { id: "job".into(), title: "Job".into() });
    let others: Vec<Offered> = face.mentions.jobs.iter().filter(|job| job.id != on.id).take(4).cloned().collect();
    face.wait = Some(Wait {
        on,
        when: Cycle::of("lands", &[("ends", "ends"), ("plan:schema", "Schema change is done"), ("pushed", "PR is pushed"), ("lands", "PR lands")]),
        base: Cycle::of("main", &[("main", "main"), ("branch", "its PR branch")]),
        needs: Some(Cycle::of(
            "the schema change and the seed data",
            &[("the schema change and the seed data", "the schema change and the seed data"), ("the schema change", "the schema change"), ("the seed data", "the seed data")],
        )),
        his: Vec::new(),
        others,
    });
    face
}

/// Fleet is back: the sheet is live again.
pub fn online(mut face: Face) -> Face {
    face.connected = true;
    face
}

/// The window lost Fleet: everything in the sheet is off.
pub fn offline(mut face: Face) -> Face {
    face.connected = false;
    face
}

/// The notes Fleet's files hold when the layer comes on: written on other screens, so each stands in
/// the panel and has no pin here. Two are open (one with a Job), two are done.
pub fn saved_notes() -> Vec<crate::annotate::Saved> {
    use crate::annotate::Saved;
    vec![
        Saved { said: "Group the repositories under their project", on: "list", screen: Some("Projects"), done: false, sent: None },
        Saved { said: "Show the lane's last start", on: "region", screen: Some("Cockpit"), done: false, sent: Some("Move Bridge's thread to the list") },
        Saved { said: "Search keeps its scope after Esc", on: "field", screen: Some("Jobs"), done: true, sent: Some("Fix the merge line freshness chip") },
        Saved { said: "Name the Fleet dot's states", on: "button", screen: None, done: true, sent: None },
    ]
}

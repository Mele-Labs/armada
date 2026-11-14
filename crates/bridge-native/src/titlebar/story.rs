//! What the Fleet panel says about the build in words: the work the running build holds, named by its Job,
//! the line under Running, and the one sentence under Refresh. Worked out here from Fleet's report and the
//! offers; the panel draws it.

use ipc::{FleetBuildReport, PickedJob, PreviewOffers, PreviewState};

use super::face::{built_said, Build};

/// A piece of work in the running build, by its Job's title, or by its branch where no Job is named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held {
    pub name: String,
    /// Its branches, said on hover. Empty where only the Job's title is known.
    pub branches: Vec<String>,
    /// A Job offered one of its branches for the preview.
    pub offered: bool,
}

/// The work the running build holds, a Job once however many branches it has: the Jobs not yet kept, the Jobs
/// a revert would take out, then the offers the build holds. A branch takes its Job's title from its offer,
/// then from its pick. Fleet sends no whole list of what the build holds yet, so this is as much as it says.
pub fn held(report: &FleetBuildReport, offers: Option<&PreviewOffers>) -> Vec<Held> {
    let rows = offers.map_or(&[][..], |offers| offers.offers.as_slice());
    let title_of = |branch: &str| rows.iter().find(|offer| offer.branch == branch && !offer.title.trim().is_empty()).map(|offer| offer.title.clone());
    let mut found: Vec<(Option<String>, String)> = Vec::new();
    let taken_out = report.previous.iter().flat_map(|previous| previous.takes_out.iter());
    for name in report.trying.iter().chain(taken_out) {
        match report.picked.iter().find(|pick| pick.title.as_deref() == Some(name.as_str()) || pick.branch == *name) {
            Some(pick) => found.push((Some(pick.branch.clone()), title_of(&pick.branch).or_else(|| pick.title.clone()).unwrap_or_else(|| pick.branch.clone()))),
            None => found.push((None, title_of(name).unwrap_or_else(|| name.clone()))),
        }
    }
    for offer in rows.iter().filter(|offer| offer.in_build) {
        found.push((Some(offer.branch.clone()), title_of(&offer.branch).unwrap_or_else(|| offer.branch.clone())));
    }
    let mut work: Vec<Held> = Vec::new();
    for (branch, name) in found {
        let offered = branch.as_deref().is_some_and(|branch| rows.iter().any(|offer| offer.branch == branch));
        let at = match work.iter().position(|one| one.name == name) {
            Some(at) => at,
            None => {
                work.push(Held { name, branches: Vec::new(), offered: false });
                work.len() - 1
            }
        };
        let one = &mut work[at];
        one.offered |= offered;
        if let Some(branch) = branch.filter(|branch| !one.branches.contains(branch)) {
            one.branches.push(branch);
        }
    }
    work
}

/// The branches picked for the next build that the running build does not hold: the ones he added.
pub fn added<'a>(report: &'a FleetBuildReport, held: &[Held]) -> Vec<&'a PickedJob> {
    report
        .picked
        .iter()
        .filter(|pick| !held.iter().any(|one| one.branches.contains(&pick.branch) || pick.title.as_deref() == Some(one.name.as_str())))
        .collect()
}

/// "A", "A and B", "A, B and C".
fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// What Refresh builds: latest main with every ready offer, each Job named once.
pub fn refresh_said(offers: Option<&PreviewOffers>) -> String {
    let mut names: Vec<String> = Vec::new();
    for offer in offers.map_or(&[][..], |offers| offers.offers.as_slice()).iter().filter(|offer| offer.state == PreviewState::Ready) {
        let name = if offer.title.trim().is_empty() { offer.branch.clone() } else { offer.title.clone() };
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if names.is_empty() {
        "Restarts Fleet on latest main.".into()
    } else {
        format!("Restarts Fleet on latest main with {}.", and_list(&names))
    }
}

/// What Update to main builds.
pub const UPDATE_SAID: &str = "Restarts Fleet on latest main.";

/// The line beside Running: when it was built, and how far behind main it stands. None where neither is known.
pub fn running_said(held: &Build) -> Option<String> {
    let report = &held.report;
    let mut said: Vec<String> = Vec::new();
    said.extend(report.running.as_ref().map(|one| format!("built {}", built_said(&one.built_at))));
    match report.position {
        Some(at) if at.behind == 1 => said.push("1 commit behind main".into()),
        Some(at) if at.behind > 1 => said.push(format!("{} commits behind main", at.behind)),
        Some(_) => said.push("on latest main".into()),
        None => {}
    }
    (!said.is_empty()).then(|| said.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(json: &str) -> FleetBuildReport {
        ipc::decode::<FleetBuildReport>("FleetBuildReport", json.as_bytes()).expect("a report")
    }

    fn offers(json: &str) -> PreviewOffers {
        ipc::decode::<PreviewOffers>("PreviewOffers", json.as_bytes()).expect("offers")
    }

    fn offer(branch: &str, title: &str, state: &str, in_build: bool) -> String {
        format!(
            r#"{{"branch":"{branch}","tip":"abc","session_id":"s-1","title":"{title}","about":"x","offered_at":"2026-11-14T04:00:00Z","state":"{state}","in_build":{in_build}}}"#
        )
    }

    /// The owner's build on 14 Nov: one Job with two branches, three ended Jobs, and one offer waiting.
    fn his() -> (FleetBuildReport, PreviewOffers) {
        let report = report(
            r#"{"on":"preview","position":{"ahead":16,"behind":74},
                "picked":[{"branch":"native/settings-split-rows","title":"Native Settings"},{"branch":"native/settings-macs-models-width"},{"branch":"native/plan-sketches"}],
                "trying":["Native Settings","native/settings-macs-models-width","native/plan-sketches"]}"#,
        );
        let offered = offers(&format!(
            r#"{{"offers":[{},{},{}],"refresh":{{"id":"r1","branches":["armada/session-66faf58b"]}}}}"#,
            offer("native/settings-split-rows", "Native Settings", "ready", true),
            offer("native/settings-macs-models-width", "Native Settings", "ready", true),
            offer("armada/session-66faf58b", "Move a Job between Macs", "ready", false),
        ));
        (report, offered)
    }

    #[test]
    fn the_running_build_names_each_job_once_with_its_branches_on_hover() {
        let (report, offers) = his();
        let work = held(&report, Some(&offers));
        assert_eq!(
            work,
            vec![
                Held { name: "Native Settings".into(), branches: vec!["native/settings-split-rows".into(), "native/settings-macs-models-width".into()], offered: true },
                Held { name: "native/plan-sketches".into(), branches: vec!["native/plan-sketches".into()], offered: false },
            ]
        );
        assert!(added(&report, &work).is_empty(), "the picks default to the running build, so nothing is added");
    }

    #[test]
    fn a_branch_picked_beyond_the_running_build_is_one_he_added() {
        let (mut report, offers) = his();
        report.picked.push(PickedJob { branch: "native/refresh-set".into(), title: None });
        let work = held(&report, Some(&offers));
        assert_eq!(added(&report, &work).iter().map(|pick| pick.branch.as_str()).collect::<Vec<_>>(), ["native/refresh-set"]);
    }

    #[test]
    fn refresh_says_what_it_builds_naming_each_ready_job_once() {
        let (_, his_offers) = his();
        assert_eq!(refresh_said(Some(&his_offers)), "Restarts Fleet on latest main with Native Settings and Move a Job between Macs.");
        assert_eq!(refresh_said(None), "Restarts Fleet on latest main.");
        let three = offers(&format!(
            r#"{{"offers":[{},{},{},{}]}}"#,
            offer("a", "A", "ready", false),
            offer("b", "B", "checking", false),
            offer("c", "C", "ready", false),
            offer("d", "", "ready", false),
        ));
        assert_eq!(refresh_said(Some(&three)), "Restarts Fleet on latest main with A, C and d.");
    }

    #[test]
    fn running_says_how_far_behind_main_and_nothing_where_it_cannot_tell() {
        let build = |json: &str| super::super::face::build_of(&report(json));
        assert_eq!(running_said(&build(r#"{"on":"preview","position":{"ahead":16,"behind":74}}"#)).as_deref(), Some("74 commits behind main"));
        assert_eq!(running_said(&build(r#"{"on":"preview","position":{"ahead":2,"behind":1}}"#)).as_deref(), Some("1 commit behind main"));
        assert_eq!(running_said(&build(r#"{"on":"main","position":{"ahead":0,"behind":0}}"#)).as_deref(), Some("on latest main"));
        assert_eq!(running_said(&build(r#"{"on":"main"}"#)), None);
    }
}

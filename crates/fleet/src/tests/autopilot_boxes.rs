//! The claim, written before the code: **the boxes a person checked when they started Autopilot
//! are what the running Autopilot reports.** What goes on autopilot, and which kinds of decision a
//! helper may answer. Nothing acts on them yet: Fleet keeps them.
//!
//! A start replaces them along with everything else. A change of settings leaves them where they
//! were, since the Cockpit's Edit caps sends no boxes and must not clear what was checked.

use std::collections::BTreeSet;

use api::Autopilots;
use ipc::{AutopilotDecision, AutopilotSettings, AutopilotWork};

use super::session_host::rig_fitted;

fn settings(backlog: bool, running: bool, decisions: &[AutopilotDecision]) -> AutopilotSettings {
    AutopilotSettings {
        instruction: "Fix what is open".into(),
        at_once: 1,
        until: None,
        spend_cap_micros: None,
        session_cap: None,
        on_autopilot: AutopilotWork { backlog, running },
        decisions: decisions.iter().copied().collect::<BTreeSet<_>>(),
    }
}

#[tokio::test]
async fn boxes_checked_at_start_are_what_the_running_autopilot_reports() {
    let rig = rig_fitted(|_| {});
    let checked = [AutopilotDecision::Recommended, AutopilotDecision::Land, AutopilotDecision::Reads];
    let started = rig.fleet.start_autopilot(rig.manifest.clone(), settings(false, true, &checked)).await.expect("started");

    let wanted = AutopilotWork { backlog: false, running: true };
    let wanted_decisions: BTreeSet<_> = checked.into_iter().collect();
    assert_eq!((started.on_autopilot, &started.decisions), (wanted, &wanted_decisions), "the start's own answer");
    let got = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read");
    assert_eq!((got.on_autopilot, &got.decisions), (wanted, &wanted_decisions), "get");
    let listed = rig.fleet.list_autopilots().await.expect("listed").autopilots.remove(0);
    assert_eq!((listed.on_autopilot, &listed.decisions), (wanted, &wanted_decisions), "list");
}

#[tokio::test]
async fn a_start_with_no_boxes_reports_backlog_on_running_off_and_no_decisions() {
    let rig = rig_fitted(|_| {});
    let started = rig.fleet.start_autopilot(rig.manifest.clone(), settings(true, false, &[])).await.expect("started");
    assert_eq!(started.on_autopilot, AutopilotWork { backlog: true, running: false });
    assert!(started.decisions.is_empty());
}

#[tokio::test]
async fn starting_again_replaces_the_boxes_and_a_change_of_settings_leaves_them() {
    let rig = rig_fitted(|_| {});
    let first = [AutopilotDecision::Plan];
    rig.fleet.start_autopilot(rig.manifest.clone(), settings(false, true, &first)).await.expect("started");

    let mut changed = settings(true, false, &[AutopilotDecision::Walk]);
    changed.at_once = 3;
    let updated = rig.fleet.update_autopilot(rig.manifest.clone(), changed).await.expect("changed");
    assert_eq!(updated.at_once, 3);
    assert_eq!(updated.on_autopilot, AutopilotWork { backlog: false, running: true }, "an edit keeps the boxes");
    assert_eq!(updated.decisions, first.into_iter().collect::<BTreeSet<_>>());

    rig.fleet.stop_autopilot(rig.manifest.clone()).await.expect("stopped");
    let again = rig.fleet.start_autopilot(rig.manifest.clone(), settings(true, true, &[AutopilotDecision::Walk])).await.expect("started again");
    assert_eq!(again.on_autopilot, AutopilotWork { backlog: true, running: true }, "a start replaces them");
    assert_eq!(again.decisions, [AutopilotDecision::Walk].into_iter().collect::<BTreeSet<_>>());
}

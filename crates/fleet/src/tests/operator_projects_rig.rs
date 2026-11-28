//! What the tests of one Operator for each project share: a rig over two repositories, and who hears what.

use ipc::{ManifestId, OperatorSettingsSet, SetOperatorSettings};

use super::peer::Placing;
use super::projects::a_second_repository;
use super::session_host::{rig_placing, Rig};
use super::tmp::TempDir;

/// A rig serving two repositories, with no project holding either. The folder of the second is kept by the third value.
pub(super) fn projects_rig() -> (Rig, ManifestId, TempDir) {
    let rig = rig_placing(Some(Placing::nothing()));
    let home = TempDir::new();
    let second = a_second_repository(&rig.fleet, &home);
    (rig, second, home)
}

/// `project`'s live Operator session.
pub(super) async fn operator_of(rig: &Rig, project: &str) -> Option<String> {
    rig.fleet.operator_session_id(project).await
}

/// What `session`'s process was sent that holds `containing`, counted.
pub(super) fn heard_by(rig: &Rig, session: &str, containing: &str) -> usize {
    rig.stand_in.process_of(session).map_or(0, |process| process.sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count())
}

/// What he sets for `project`, through the act.
pub(super) async fn set_for(rig: &Rig, project: &str, set: OperatorSettingsSet) {
    rig.fleet.operator_settings_set(SetOperatorSettings { project: project.to_string(), set, unset: Vec::new() }).await.expect("set");
}

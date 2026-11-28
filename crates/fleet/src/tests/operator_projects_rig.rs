//! What the tests of one Operator for each project share: a rig over two repositories, and who hears what.

use std::sync::Arc;

use ipc::{ManifestId, OperatorSettingsSet, SetOperatorSettings};

use super::peer::Placing;
use super::projects::a_second_repository;
use super::session_host::{rig_placing, Rig};
use super::tmp::TempDir;

/// A rig serving two repositories, with no project holding either. The folder of the second is kept by the third value.
pub(super) fn projects_rig() -> (Rig, ManifestId, TempDir) {
    let (rig, second, home, _) = projects_rig_placed();
    (rig, second, home)
}

/// [`projects_rig`], and the plant a test places a caller on a session's process with.
pub(super) fn projects_rig_placed() -> (Rig, ManifestId, TempDir, Arc<Placing>) {
    let peers = Placing::nothing();
    let rig = rig_placing(Some(Arc::clone(&peers)));
    let home = TempDir::new();
    let second = a_second_repository(&rig.fleet, &home);
    (rig, second, home, peers)
}

/// A caller whose connection `session`'s process holds, as the door places one: port `from` on this Fleet's own.
pub(super) fn caller_on(rig: &Rig, peers: &Placing, session: &str, from: u16) -> api::Caller {
    let pid = rig.fleet.hosts().of(session).state().process.as_ref().and_then(|process| process.pid()).expect("a running process");
    let relay = 40_000 + u32::from(from);
    peers.holding(relay, from, rig.fleet.host().port);
    peers.started(pid, relay);
    api::Caller::at(format!("127.0.0.1:{from}").parse().expect("an address"))
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

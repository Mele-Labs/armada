//! A project as a test plants it: the file Fleet keeps in Kit's folder, and the read that holds it.

use std::path::PathBuf;

use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::tmp::TempDir;

/// The text of a version 2 file for a project of whole repositories.
pub(crate) fn text(id: &str, name: &str, members: &[&str]) -> String {
    let listed: String = members.iter().map(|one| format!("  - repository: {one}\n")).collect();
    let members = if members.is_empty() { String::from("members: []\n") } else { format!("members:\n{listed}") };
    format!("version: 2\nproject_id: {id}\nname: {name}\n{members}")
}

/// Where Kit keeps the file of project `id` on this Mac.
pub(crate) fn file(home: &TempDir, id: &str) -> PathBuf {
    home.path().join("kit").join("projects").join(format!("{id}.yml"))
}

/// Plant the file of a project on this Mac, and read every project again.
pub(crate) async fn planted(fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>, home: &TempDir, id: &str, name: &str, members: &[&str]) {
    let at = file(home, id);
    std::fs::create_dir_all(at.parent().expect("a folder")).expect("its folder");
    std::fs::write(at, text(id, name, members)).expect("its file");
    fleet.projects_reload().await;
}

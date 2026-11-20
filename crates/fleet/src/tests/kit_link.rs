//! Kit over the Mac link on one Fleet: the last-agreed marker tells a change on one side from a change on both, a
//! Kit edit lands in the harness's own home and is what this Mac offers, and the retired shared home is never read.

use std::sync::Arc;

use api::SharedHome;
use ipc::{GetKitFile, HomeKind, HomeProfile, KitFileKind, KitMark, SaveKitFile, SetKitLinkKind};
use testkit::{FakeWorkProduct, HeldFolder};

use crate::home::HomeFittings;
use crate::kit_link::{judged, Judged};
use crate::tests::tmp::TempDir;

fn stands(here: Option<&str>, there: Option<&str>, agreed: Option<&str>) -> (KitMark, bool) {
    seen(here, there, agreed, None)
}

/// As [`stands`], with `theirs` the other Mac's marker with this one, as its offer carried it.
fn seen(here: Option<&str>, there: Option<&str>, agreed: Option<&str>, theirs: Option<&str>) -> (KitMark, bool) {
    let Judged { mark, take, .. } = judged(here, there, agreed, theirs);
    (mark, take)
}

#[test]
fn the_last_agreed_marker_tells_a_change_on_one_side_from_a_change_on_both() {
    // Agreed at a; there changed to b: taken here.
    assert_eq!(stands(Some("a"), Some("b"), Some("a")), (KitMark::Differs, true));
    // Agreed at a; here changed to b: left for the other Mac to take.
    assert_eq!(stands(Some("b"), Some("a"), Some("a")), (KitMark::Differs, false));
    // Both changed since they agreed at a, or never agreed: a clash, and neither moves.
    assert_eq!(stands(Some("b"), Some("c"), Some("a")), (KitMark::Clash, false));
    assert_eq!(stands(Some("b"), Some("c"), None), (KitMark::Clash, false));
    // The same: agreed now.
    assert_eq!(judged(Some("a"), Some("a"), None, None), Judged { mark: KitMark::Same, take: false, agree: Some("a".into()) });
    assert_eq!(judged(Some("a"), Some("a"), Some("a"), None).agree, None, "nothing to record");
    // New on one side.
    assert_eq!(stands(Some("a"), None, None), (KitMark::NewHere, false));
    assert_eq!(stands(None, Some("a"), None), (KitMark::OnlyThere, true));
    // Taken away here after they agreed: not brought back.
    assert_eq!(stands(None, Some("a"), Some("a")), (KitMark::OnlyThere, false));
    assert_eq!(seen(None, Some("a"), None, Some("a")), (KitMark::OnlyThere, false), "nor where only the other Mac kept the marker");
}

/// Here sent `b` over agreed `a`; the other Mac took it, recorded `b`, and wrote `c` before this Mac had read that
/// it took it. Its marker `b` is this Mac's text, so only it moved since: `c` is taken, not held as a clash.
#[test]
fn an_edit_made_right_after_taking_the_other_macs_crosses_back_and_is_no_clash() {
    assert_eq!(seen(Some("b"), Some("c"), Some("a"), Some("b")), (KitMark::Differs, true));
    // The other side of the same moment: its own text moved past what it agreed, so this Mac's waits for the take.
    assert_eq!(seen(Some("c"), Some("b"), Some("b"), Some("a")), (KitMark::Differs, false));
    // Both moved past every marker either holds: a clash still.
    assert_eq!(seen(Some("d"), Some("c"), Some("a"), Some("b")), (KitMark::Clash, false));
    // Each marker names the other's text, so each says the other moved: neither is trusted, and it is held.
    assert_eq!(seen(Some("b"), Some("c"), Some("c"), Some("b")), (KitMark::Clash, false));
}

#[tokio::test]
async fn a_kit_edit_lands_in_the_harness_home_and_is_what_this_mac_offers() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    fleet.sharing_home_with(HomeFittings {
        mods: Arc::new(HeldFolder::new()),
        kit: Arc::new(HeldFolder::new()),
        harness: Arc::new(adapters::AgentHome::at(home.path(), "")),
    });
    let folder = adapters::kit_pieces::skills_folder(home.path()).join("docs");
    std::fs::create_dir_all(&folder).expect("the skill's folder");
    std::fs::write(folder.join("SKILL.md"), "---\nname: docs\n---\nfirst\n").expect("its file");
    fleet.set_kit_link_kind(SetKitLinkKind { kind: HomeKind::Skills, on: true }).await.expect("switched on");

    let read = fleet.kit_file_read(GetKitFile { kind: KitFileKind::Skills, name: "docs".into(), plugin: None }).await.expect("read");
    let edited = "---\nname: docs\n---\nsecond\n";
    let save = SaveKitFile { kind: KitFileKind::Skills, name: "docs".into(), plugin: None, read: read.text, text: edited.into() };
    fleet.kit_file_saved(save).await.expect("saved");

    // The harness's own folder is the adapter's to name, so it is found rather than named here.
    let saved = std::fs::read_dir(home.path()).expect("the home").filter_map(Result::ok).map(|entry| entry.path().join("skills/docs/SKILL.md")).find(|path| path.is_file());
    let on_disk = std::fs::read_to_string(saved.expect("in the harness's folder in the home")).expect("read back");
    assert_eq!(on_disk, edited);
    let offer = fleet.kit_offered(None).await.expect("offered");
    let docs = offer.pieces.iter().find(|piece| piece.name == "docs").expect("docs is offered");
    assert_eq!(docs.files.get("docs/SKILL.md").map(String::as_str), Some(edited), "the edit crosses from there");
}

#[tokio::test]
async fn the_shared_home_is_not_read_and_its_acts_name_the_link() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let profile = || HomeProfile { login: "ana".into() };

    let read = fleet.get_shared_home(profile()).await.expect("answered");
    assert!(read.home.is_none() && read.planned.is_none() && read.rows.is_empty(), "nothing read: {read:?}");
    assert!(fleet.get_home_kinds(profile()).await.expect("answered").kinds.iter().all(|kind| !kind.on));
    let refused = fleet.sync_shared_home(profile()).await.expect_err("a Sync is refused");
    assert_eq!(refused.error().code, "fleet.shared_home_retired");
    assert_eq!(refused.error().message, "Kit now crosses to your other Macs over the link.");
}

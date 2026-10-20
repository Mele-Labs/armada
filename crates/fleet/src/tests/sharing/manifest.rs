//! A person's own `armada.yml` as a Share piece: made from, and added to, the place Kit keeps it.

use std::path::PathBuf;

use api::Commands;
use ipc::{BindUnboundPiece, Landed, PackageText, PieceKind, PieceToCommit, RunMemoryPlace, RunPlace};

use super::{adding, mac, refused_with, Mac, REMOTE};
use crate::run_memory::places;
use crate::tests::tmp::TempDir;

const MANIFEST: &str = "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\ncommands:\n  dev:\n    run: pnpm dev\n";

impl Mac {
    fn manifest_file(&self) -> PathBuf {
        places::kit_manifest(&self.kit, &self.root)
    }

    fn keeps_manifest(&self, text: &str) {
        let at = self.manifest_file();
        std::fs::create_dir_all(at.parent().expect("a folder")).expect("made");
        std::fs::write(at, text).expect("written");
    }
}

#[tokio::test]
async fn a_manifest_is_shared_with_its_remote_read_with_every_run_line_and_added_to_kits_place() {
    let mine = mac(REMOTE);
    mine.keeps_manifest(MANIFEST);
    let made = mine.fleet.package_made(mine.sharing(PieceToCommit::Manifest)).await.expect("made");
    assert_eq!(made.file_name, "widgets.armada.toml");
    assert!(made.text.contains("acme/widgets") && !made.text.contains(&mine.root), "{}", made.text);
    assert_eq!(std::fs::read_to_string(mine.manifest_file()).expect("still kept"), MANIFEST, "the person's copy stays");

    let theirs = mac(REMOTE);
    let read = theirs.fleet.package_read(PackageText { text: made.text.clone() }).await.expect("read");
    let piece = &read.pieces[0];
    assert_eq!((piece.kind, piece.remote.as_deref(), piece.repository.as_deref(), piece.exists), (PieceKind::Manifest, Some(REMOTE), Some(theirs.root.as_str()), false));
    assert_eq!(piece.commands, ["make test", "pnpm dev"].map(String::from), "every Check and Command line it would add");

    let added = theirs.fleet.package_added(adding(&made.text, false)).await.expect("added");
    assert_eq!((added.pieces[0].landed, added.pieces[0].repository.as_deref()), (Landed::Added, Some(theirs.root.as_str())));
    assert_eq!(std::fs::read_to_string(theirs.manifest_file()).expect("in their Kit"), MANIFEST);
    assert!(theirs.fleet.package_read(PackageText { text: made.text }).await.expect("read").pieces[0].exists);
}

#[tokio::test]
async fn a_manifest_kit_already_holds_is_refused_unless_replace_is_asked() {
    let mine = mac(REMOTE);
    mine.keeps_manifest(MANIFEST);
    let text = mine.shared(PieceToCommit::Manifest).await;
    let theirs = mac(REMOTE);
    theirs.keeps_manifest("checks:\n  test:\n    run: make older\n");

    let (status, code, said) = refused_with(theirs.fleet.package_added(adding(&text, false)).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (422, "fleet.share_exists"));
    assert!(said.contains("manifest `armada.yml`") && said.ends_with("Add with replace to replace it."), "{said}");
    assert!(std::fs::read_to_string(theirs.manifest_file()).expect("there").contains("older"), "refused, so untouched");

    let replaced = theirs.fleet.package_added(adding(&text, true)).await.expect("replaced");
    assert_eq!(replaced.pieces[0].landed, Landed::Replaced);
    assert_eq!(std::fs::read_to_string(theirs.manifest_file()).expect("there"), MANIFEST);
}

#[tokio::test]
async fn a_manifest_for_an_unknown_remote_waits_unbound_and_is_written_when_bound() {
    let mine = mac("acme/gadgets");
    mine.keeps_manifest(MANIFEST);
    let text = mine.shared(PieceToCommit::Manifest).await;

    let theirs = mac(REMOTE);
    let added = theirs.fleet.package_added(adding(&text, false)).await.expect("added");
    assert_eq!(added.pieces[0].landed, Landed::Unbound);
    assert!(!theirs.manifest_file().exists(), "nothing is written for it");
    let held = theirs.fleet.unbound_listed();
    assert_eq!((held.pieces[0].kind, held.pieces[0].commands.len()), (PieceKind::Manifest, 2));

    let unbound_id = held.pieces[0].unbound_id.clone();
    let bound = theirs.fleet.unbound_bound(BindUnboundPiece { unbound_id, repository: theirs.root.clone(), replace: false }).await.expect("bound");
    assert_eq!(bound.landed, Landed::Added);
    assert_eq!(std::fs::read_to_string(theirs.manifest_file()).expect("in their Kit"), MANIFEST);
}

#[tokio::test]
async fn a_manifest_is_added_to_the_synced_folder_their_tiles_use_and_a_folder_gone_refuses_before_writing() {
    let mine = mac(REMOTE);
    mine.keeps_manifest(MANIFEST);
    let text = mine.shared(PieceToCommit::Manifest).await;

    let theirs = mac(REMOTE);
    let synced = TempDir::new();
    let folder = synced.path().join("Dropbox");
    std::fs::create_dir_all(&folder).expect("a synced folder");
    theirs
        .fleet
        .set_run_memory_place(RunMemoryPlace { repository: theirs.root.clone(), place: RunPlace::Synced, synced_folder: Some(folder.to_string_lossy().to_string()), cloud: None })
        .await
        .expect("chosen");
    theirs.fleet.package_added(adding(&text, false)).await.expect("added");
    assert_eq!(std::fs::read_to_string(places::kit_manifest(&folder, &theirs.root)).expect("in the synced folder"), MANIFEST);
    assert!(!theirs.manifest_file().exists(), "not in Kit's folder here");

    std::fs::remove_dir_all(&folder).expect("unmounted");
    let (status, code, said) = refused_with(theirs.fleet.package_added(adding(&text, true)).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (422, "fleet.share_refused"));
    assert!(said.contains("is not there"), "{said}");
}

#[tokio::test]
async fn a_manifest_the_person_does_not_keep_is_refused_and_so_is_one_that_is_not_a_mapping() {
    let mine = mac(REMOTE);
    let (_, code, said) = refused_with(mine.fleet.package_made(mine.sharing(PieceToCommit::Manifest)).await.expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), ("fleet.share_not_yours", "Your Kit holds no armada.yml for this repository."));

    let text = "version = 1\n[[piece]]\nkind = \"manifest\"\ntext = \"- a\\n- b\\n\"\n";
    let (_, code, said) = refused_with(mine.fleet.package_read(PackageText { text: text.into() }).await.expect_err("refused"));
    assert_eq!(code, "fleet.share_malformed");
    assert!(said.starts_with("Piece 1: the armada.yml is not a mapping of keys"), "{said}");
}

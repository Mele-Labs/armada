//! A person's own `armada.yml` for a repository, as `list_manifests` reports it on
//! [`ipc::ManifestSummary`]: which names came from Kit and which repository ones they hide.

use std::path::Path;

use crate::wire::manifest_summary;

const REPOSITORY: &str = "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n  \
                          lint:\n    run: make lint\n";

#[test]
fn the_summary_names_what_came_from_kit_and_what_it_hides() {
    let manifest = config::Manifest::with_kit(
        Path::new("/work/widgets/armada.yml"),
        REPOSITORY,
        Path::new("/kit/manifests/acme--widgets.yml"),
        "checks:\n  test:\n    run: make quick\n  lint:\n    run: make lint\n  e2e:\n    run: make e2e\n",
    )
    .expect("they merge");
    let kit = manifest_summary(&manifest, "/records").kit.expect("read from Kit");
    assert_eq!(kit.file, "/kit/manifests/acme--widgets.yml");
    assert!(!kit.whole);
    let names: Vec<(&str, &str, bool, bool, Option<bool>)> =
        kit.names.iter().map(|one| (one.kind.as_str(), one.name.as_str(), one.hides, one.added, one.gates)).collect();
    assert_eq!(
        names,
        [("check", "test", true, false, None), ("check", "lint", false, false, None), ("check", "e2e", false, true, Some(false))]
    );
    assert!(kit.not_read.is_empty(), "an added name is in names");
}

#[test]
fn a_repository_kit_holds_nothing_for_carries_no_kit_field() {
    let manifest = config::Manifest::parse(Path::new("/work/widgets/armada.yml"), REPOSITORY).expect("parses");
    let summary = manifest_summary(&manifest, "/records");
    assert_eq!(summary.kit, None);
    let wire = ipc::encode(&summary).expect("encodes");
    assert!(!wire.contains("\"kit\""), "absent rather than null");
}

/// Kit's copy at the place the repository's tiles are kept in: `crate::kit_manifests`.
mod placed {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use config::{KitCopy, KitRead};
    use store::{KeptPlace, KeptWhere};
    use testkit::FakeBucket;

    use crate::tests::red_claim::Planted;
    use super::REPOSITORY;
    use crate::kit_manifests::KitManifest;
    use crate::run_memory::places;
    use crate::tests::tmp::TempDir;

    const ROOT: &str = "/work/widgets";

    fn at(kit: &Path, held: KeptWhere, bucket: Option<Arc<FakeBucket>>) -> KitManifest {
        let reaching = match bucket {
            Some(bucket) => Planted::with_bucket(bucket),
            None => Planted::signed_out(),
        };
        KitManifest::at(kit, ROOT, &held, Arc::new(reaching))
    }

    fn synced(folder: &Path) -> KeptWhere {
        let place = KeptPlace { place: "synced".into(), synced_folder: Some(folder.to_string_lossy().to_string()), cloud: None };
        KeptWhere { kept_here: false, place: Some(place) }
    }

    fn plant(file: &Path, text: &str) {
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
        std::fs::write(file, text).expect("written");
    }

    #[test]
    fn a_synced_repositorys_copy_is_read_from_and_written_to_the_synced_folder() {
        let (kit, folder) = (TempDir::new(), TempDir::new());
        plant(&places::kit_manifest(kit.path(), ROOT), "checks: {}\n");
        let copy = at(kit.path(), synced(folder.path()), None);
        let file: PathBuf = places::kit_manifest(folder.path(), ROOT);
        assert_eq!(copy.label(), file);
        assert_eq!(copy.read(), KitRead::Absent, "Kit's folder here is not the place");

        copy.write(REPOSITORY).expect("written");
        assert_eq!(std::fs::read_to_string(&file).expect("in the synced folder"), REPOSITORY);
        assert_eq!(copy.read(), KitRead::Text(REPOSITORY.to_string()));
    }

    #[test]
    fn keep_on_this_mac_reads_kits_folder_whatever_place_was_chosen() {
        let (kit, folder) = (TempDir::new(), TempDir::new());
        plant(&places::kit_manifest(kit.path(), ROOT), "here: 1\n");
        plant(&places::kit_manifest(folder.path(), ROOT), "synced: 1\n");
        let held = KeptWhere { kept_here: true, ..synced(folder.path()) };
        let copy = at(kit.path(), held, None);
        assert_eq!(copy.read(), KitRead::Text("here: 1\n".to_string()));
        assert_eq!(copy.label(), places::kit_manifest(kit.path(), ROOT));
    }

    #[test]
    fn a_place_that_cannot_be_read_reads_as_no_copy_and_says_so() {
        let (kit, folder) = (TempDir::new(), TempDir::new());
        let gone = folder.path().join("Dropbox");
        let copy: Arc<dyn KitCopy> = Arc::new(at(kit.path(), synced(&gone), None));
        let KitRead::Unread(why) = copy.read() else { panic!("read as a copy") };
        assert!(why.contains("is not there"), "{why}");

        let repository = TempDir::new();
        plant(&repository.path().join("armada.yml"), REPOSITORY);
        let manifest = config::Manifest::load_over_copy(&repository.path().join("armada.yml"), Arc::clone(&copy)).expect("the repository's stands");
        assert_eq!(manifest.check_names(), ["lint", "test"]);
        let said = manifest.from_kit().and_then(|kit| kit.left_out.clone()).expect("named");
        assert!(said.contains("was left out") && said.contains("is not there"), "{said}");

        let unreachable = Arc::new(FakeBucket::new());
        unreachable.set_unreachable(true);
        let held = KeptWhere { kept_here: false, place: Some(KeptPlace { place: "cloud".into(), synced_folder: None, cloud: Some("s3://shop-runs/armada".into()) }) };
        let KitRead::Unread(why) = at(kit.path(), held, Some(unreachable)).read() else { panic!("read as a copy") };
        assert!(why.contains("could not be reached"), "{why}");
    }

    #[test]
    fn a_bucket_keeps_the_copy_as_an_object_beside_the_tiles() {
        let kit = TempDir::new();
        let bucket = Arc::new(FakeBucket::new());
        let held = KeptWhere { kept_here: false, place: Some(KeptPlace { place: "cloud".into(), synced_folder: None, cloud: Some("s3://shop-runs/armada".into()) }) };
        let copy = at(kit.path(), held, Some(Arc::clone(&bucket)));
        let object = places::manifest_object(ROOT);
        assert!(object.starts_with("manifests/") && object.ends_with(".yml"), "{object}");
        assert_eq!(copy.label(), PathBuf::from(format!("s3://shop-runs/armada/{object}")));
        assert_eq!(copy.read(), KitRead::Absent);

        copy.write(REPOSITORY).expect("put");
        assert_eq!(bucket.object(&object).expect("an object"), REPOSITORY.as_bytes());
        assert_eq!(copy.read(), KitRead::Text(REPOSITORY.to_string()));
        assert!(!places::kit_manifest(kit.path(), ROOT).exists(), "nothing on this Mac");
    }

    /// The reader that is not Fleet (`armada check`) finds the place in the store file.
    #[test]
    fn a_reader_with_only_the_store_file_finds_the_place_and_the_mark() {
        let (home, kit, folder) = (TempDir::new(), TempDir::new(), TempDir::new());
        let file = home.path().join("armada.db");
        let reaching = || -> Arc<dyn crate::red_claim::Reaching> { Arc::new(Planted::signed_out()) };
        let label = |kit: &Path| crate::kit_manifest_copy(&file, kit, ROOT, reaching()).label();
        assert_eq!(label(kit.path()), places::kit_manifest(kit.path(), ROOT), "no store file: no choice");

        let mut store = store::Store::open(&file).expect("opened");
        store.set_run_memory_place(ROOT, "synced", Some(&folder.path().to_string_lossy()), None).expect("chosen");
        assert_eq!(label(kit.path()), places::kit_manifest(folder.path(), ROOT));
        store.keep_on_this_mac(ROOT, true).expect("marked");
        assert_eq!(label(kit.path()), places::kit_manifest(kit.path(), ROOT), "Keep on this Mac wins");
    }
}

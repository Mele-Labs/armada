//! The Kit layout a repository's runs are written in, for `this_mac` and `synced`: one TOML file
//! a person can read, `runs/<key>.toml` under Kit's folder or under the folder they named. A
//! repository's flight controls share the key in `flight-controls/`: `crate::flight_controls`, and
//! a person's own `armada.yml` for it in `manifests/`: [`kit_manifest`].
//!
//! **Keyed by the repository's remote** (`owner--name`), so the same repository cloned twice, or
//! on two machines sharing a synced folder, is one file; a repository with no remote is keyed by
//! its path. A write reads the file first and merges, so a fact another machine added stays.

use std::path::{Path, PathBuf};

use adapters::BucketAddress;
use ipc::RunPlace;
use serde::{Deserialize, Serialize};
use store::{KeptPlace, KeptRun};

use super::cloud;

/// The folder inside Kit's, or a synced folder, that holds the files.
pub(crate) const FOLDER: &str = "runs";
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    repository: String,
    #[serde(default)]
    fact: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    workspace: String,
    kind: String,
    command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    succeeded: bool,
    /// Milliseconds since the epoch.
    last_ran: u64,
    source: String,
}

/// What names a repository on disk: its remote's `owner--name`, else its path made safe.
pub(crate) fn key_of(root: &str) -> String {
    match adapters::remote_of(Path::new(root)) {
        Some(address) => address.slug().replace('/', "--"),
        None => {
            let safe: String = root.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
            format!("path{safe}")
        }
    }
}

/// The folder inside Kit's that holds a person's own `armada.yml` for each repository.
pub(crate) const MANIFESTS: &str = "manifests";

/// Where Kit keeps a person's own `armada.yml` for the repository at `root`:
/// `manifests/<owner--name>.yml` under Kit's folder on this Mac. `docs/concepts/manifest.md`,
/// *Kit's copy*.
pub fn kit_manifest(kit: &Path, root: &str) -> PathBuf {
    kit.join(MANIFESTS).join(format!("{}.yml", key_of(root)))
}

/// Where a repository's pieces of the person's own are kept.
pub(crate) enum Where {
    /// Under this folder: Kit's own, or the synced folder.
    Folder(PathBuf),
    /// In this bucket, under its prefix; the text is the address as it was chosen.
    Bucket(BucketAddress, String),
    /// The place cannot be used now: the sentence that says why.
    Nowhere(String),
}

/// The place a repository's pieces are kept, and where in it: the one its runs are, `this_mac` where
/// none was chosen or where the choice would share a person's own.
pub(crate) fn where_kept(kit: &Path, held: Option<&KeptPlace>) -> (RunPlace, Where) {
    let this_mac = || Where::Folder(kit.to_path_buf());
    let Some(KeptPlace { place, synced_folder, cloud }) = held else {
        return (RunPlace::ThisMac, this_mac());
    };
    let place = super::place_of(place);
    let at = match place {
        RunPlace::ThisMac => this_mac(),
        RunPlace::Synced => match synced_folder {
            Some(folder) if Path::new(folder).is_dir() => Where::Folder(PathBuf::from(folder)),
            Some(folder) => Where::Nowhere(format!("the synced folder {folder} is not there")),
            None => Where::Nowhere(String::from("no synced folder was chosen")),
        },
        RunPlace::Cloud => match cloud.as_deref().map(cloud::Target::kept) {
            Some(Ok(cloud::Target::Bucket(address, text))) => Where::Bucket(address, text),
            Some(Ok(cloud::Target::Store)) => return (RunPlace::ThisMac, this_mac()),
            Some(Err(why)) => Where::Nowhere(why),
            None => Where::Nowhere(String::from("no cloud address was chosen")),
        },
        RunPlace::Repository => return (RunPlace::ThisMac, this_mac()),
    };
    (place, at)
}

/// The object a person's own `armada.yml` for a repository is, under a bucket's prefix: the same
/// layout as `manifests/<key>.yml` under a folder.
pub(crate) fn manifest_object(root: &str) -> String {
    format!("{MANIFESTS}/{}.yml", key_of(root))
}

/// A repository's file in `folder` under `base`: `runs`, or another kept in the same layout.
pub(crate) fn file_for(base: &Path, folder: &str, root: &str) -> PathBuf {
    base.join(folder).join(format!("{}.toml", key_of(root)))
}

/// The runs a file holds. A file that is not there holds none; one that will not read is an error,
/// since writing over it would lose what a person kept.
pub(super) fn read(path: &Path) -> Result<Vec<KeptRun>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => decode(&text, &path.display().to_string()),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(why) => Err(format!("{} could not be read: {why}", path.display())),
    }
}

/// The runs a document in this layout holds; `named` is what an error calls it, a path or an object.
pub(super) fn decode(text: &str, named: &str) -> Result<Vec<KeptRun>, String> {
    let file: File = toml::from_str(text).map_err(|why| format!("{named} could not be read: {why}"))?;
    if file.version != VERSION {
        return Err(format!("{named} is version {}, and this Fleet reads {VERSION}", file.version));
    }
    Ok(file
        .fact
        .into_iter()
        .map(|one| KeptRun {
            workspace: one.workspace,
            kind: one.kind,
            command: one.command,
            port: one.port,
            succeeded: one.succeeded,
            last_ran: one.last_ran,
            source: one.source,
            kept_in: None,
            why: None,
        })
        .collect())
}

/// `runs` as a document in this layout.
pub(crate) fn encode(root: &str, runs: &[KeptRun]) -> Result<String, String> {
    let file = File {
        version: VERSION,
        repository: root.to_string(),
        fact: runs
            .iter()
            .map(|one| Entry {
                workspace: one.workspace.clone(),
                kind: one.kind.clone(),
                command: one.command.clone(),
                port: one.port,
                succeeded: one.succeeded,
                last_ran: one.last_ran,
                source: one.source.clone(),
            })
            .collect(),
    };
    toml::to_string_pretty(&file).map_err(|why| why.to_string())
}

/// Write `runs` as the file, over a temporary file renamed into place.
pub(super) fn write(path: &Path, root: &str, runs: &[KeptRun]) -> Result<(), String> {
    let text = encode(root, runs)?;
    let folder = path.parent().ok_or_else(|| format!("{} has no folder", path.display()))?;
    std::fs::create_dir_all(folder).map_err(|why| format!("{} could not be made: {why}", folder.display()))?;
    let staging = path.with_extension("toml.writing");
    std::fs::write(&staging, text).map_err(|why| format!("{} could not be written: {why}", staging.display()))?;
    std::fs::rename(&staging, path).map_err(|why| format!("{} could not be written: {why}", path.display()))
}

/// The object a repository's file in `folder` is, under a bucket's prefix: the same layout as a file.
pub(crate) fn object_for(folder: &str, root: &str) -> String {
    format!("{folder}/{}.toml", key_of(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(command: &str, port: Option<u16>) -> KeptRun {
        KeptRun {
            workspace: "web".into(),
            kind: "dev_server".into(),
            command: command.into(),
            port,
            succeeded: true,
            last_ran: 1_790_848_800_000,
            source: "jobs".into(),
            kept_in: None,
            why: None,
        }
    }

    #[test]
    fn a_repository_with_no_remote_is_keyed_by_its_path() {
        assert_eq!(key_of("/work/shop"), "path-work-shop");
    }

    #[test]
    fn a_repositorys_flight_controls_share_the_key_under_a_folder_of_their_own() {
        let dir = crate::tests::tmp::TempDir::new();
        assert_eq!(
            file_for(dir.path(), "flight-controls", "/work/shop"),
            dir.path().join("flight-controls").join("path-work-shop.toml")
        );
        assert_eq!(object_for("flight-controls", "/work/shop"), "flight-controls/path-work-shop.toml");
        assert_eq!(object_for(FOLDER, "/work/shop"), "runs/path-work-shop.toml");
    }

    #[test]
    fn a_file_reads_back_what_was_written_and_a_missing_one_is_empty() {
        let dir = crate::tests::tmp::TempDir::new();
        let file = file_for(dir.path(), FOLDER, "/work/shop");
        assert_eq!(read(&file), Ok(Vec::new()));

        let runs = vec![run("pnpm dev", Some(5173)), run("pnpm preview", None)];
        write(&file, "/work/shop", &runs).expect("written");

        assert_eq!(read(&file), Ok(runs));
        assert!(!file.with_extension("toml.writing").exists(), "the staging file went");
    }

    #[test]
    fn a_file_that_will_not_read_is_an_error_and_not_an_empty_list() {
        let dir = crate::tests::tmp::TempDir::new();
        let file = file_for(dir.path(), FOLDER, "/work/shop");
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
        std::fs::write(&file, "version = 9\nrepository = \"x\"\n").expect("written");

        assert!(read(&file).expect_err("another version").contains("version 9"));
        std::fs::write(&file, "not = [toml").expect("written");
        assert!(read(&file).is_err());
    }
}

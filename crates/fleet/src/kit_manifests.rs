//! A person's own `armada.yml` for a repository, read from and written to the place the
//! repository's tiles are kept in. `docs/concepts/manifest.md`, *Kit's copy*.
//!
//! | place | kept in |
//! |---|---|
//! | `this_mac` | `manifests/<key>.yml` under Kit's folder |
//! | `synced` | the same under the folder the person named |
//! | `cloud`, a bucket | `<prefix>/manifests/<key>.yml` |
//! | `cloud`, Armada's store, and `repository` | the same as `this_mac` |
//! | any, while the repository is kept on this Mac | the same as `this_mac` |
//!
//! **A place that cannot be read reads as no copy, and says why**: a folder that is not mounted, a
//! bucket that refuses. The repository's own `armada.yml` stands, and `ManifestSummary.kit`
//! carries the sentence as `left_out`. **Read on each call**, so a re-read of the repository's
//! file reads the place again.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use config::{KitCopy, KitRead};

use crate::red_claim::Reaching;
use crate::run_memory::places::{self, Where};

enum At {
    File(PathBuf),
    Object { address: adapters::BucketAddress, object: String },
    Nowhere(String),
}

/// One repository's Kit copy of its `armada.yml`, at the place its tiles are kept.
pub struct KitManifest {
    label: PathBuf,
    at: At,
    reaching: Arc<dyn Reaching>,
}

impl std::fmt::Debug for KitManifest {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("KitManifest").field("label", &self.label).finish()
    }
}

impl KitManifest {
    /// The copy for the repository at `root`, where `held` says its pieces are kept.
    pub(crate) fn at(kit: &Path, root: &str, held: &store::KeptWhere, reaching: Arc<dyn Reaching>) -> KitManifest {
        let here = places::kit_manifest(kit, root);
        let (label, at) = match places::where_kept(kit, held).1 {
            Where::Folder(base) => {
                let file = places::kit_manifest(&base, root);
                (file.clone(), At::File(file))
            }
            Where::Bucket(address, text) => {
                let object = places::manifest_object(root);
                (PathBuf::from(format!("{}/{object}", text.trim_end_matches('/'))), At::Object { address, object })
            }
            Where::Nowhere(why) => {
                // Named where it would have been, so the sentence says which copy was not read.
                let label = match &held.place {
                    Some(store::KeptPlace { synced_folder: Some(folder), .. }) => places::kit_manifest(Path::new(folder), root),
                    _ => here,
                };
                (label, At::Nowhere(why))
            }
        };
        KitManifest { label, at, reaching }
    }

    /// Replace the copy with `text`, whole, at the place. A place that cannot take it refuses with
    /// the sentence that says why.
    pub fn write(&self, text: &str) -> Result<(), String> {
        match &self.at {
            At::File(path) => {
                let folder = path.parent().ok_or_else(|| format!("{} has no folder", path.display()))?;
                std::fs::create_dir_all(folder).map_err(|why| format!("{} could not be made: {why}", folder.display()))?;
                let staging = path.with_extension("yml.writing");
                std::fs::write(&staging, text).map_err(|why| format!("{} could not be written: {why}", staging.display()))?;
                std::fs::rename(&staging, path).map_err(|why| format!("{} could not be written: {why}", path.display()))
            }
            At::Object { address, object } => {
                let bucket = self.reaching.bucket(address).map_err(|why| why.said())?;
                bucket.put(object, text.as_bytes()).map_err(|why| why.said())
            }
            At::Nowhere(why) => Err(why.clone()),
        }
    }
}

impl KitCopy for KitManifest {
    fn label(&self) -> PathBuf {
        self.label.clone()
    }

    fn read(&self) -> KitRead {
        match &self.at {
            At::File(path) => config::KitFile(path.clone()).read(),
            At::Object { address, object } => match self.reaching.bucket(address).and_then(|bucket| bucket.get(object)) {
                Ok(None) => KitRead::Absent,
                Ok(Some(bytes)) => match String::from_utf8(bytes) {
                    Ok(text) => KitRead::Text(text),
                    Err(_) => KitRead::Unread(format!("the object {object} is not text")),
                },
                Err(why) => KitRead::Unread(why.said()),
            },
            At::Nowhere(why) => KitRead::Unread(why.clone()),
        }
    }
}

/// Kit's copy for the repository at `root`, at the place `store_file` says its pieces are kept.
/// **For a reader that is not Fleet** (`armada check` among them): the store is read as it stands,
/// and none, or one that will not read, is no choice, so Kit's folder on this Mac.
pub fn kit_manifest_copy(store_file: &Path, kit: &Path, root: &str, reaching: Arc<dyn Reaching>) -> Arc<dyn KitCopy> {
    let held = store::Store::peek_where_kept(store_file, root).unwrap_or_default();
    Arc::new(KitManifest::at(kit, root, &held, reaching))
}

//! The pictures of what a feature looks like in the running app: a Job's `add_feature_picture`, his Not this feature, and
//! the bytes the gallery draws. `docs/concepts/feature-catalog.md`, *Pictures*. The librarian's takes are `picture_taking`.
//!
//! **The file is Fleet's**, copied under the support folder by project and feature, so a Job's scratch folder may go.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AddFeaturePicture, CatalogRuleAct, FeaturePicture, FeaturePictureAdded, FeaturePictureRemoved, Instant, PictureBy, RemoveFeaturePicture};
use store::{picture_by, KeptPicture, SessionState, Store, WriteError};

use super::reading::Loaded;
use crate::daemon::Fleet;

/// Under Fleet's support folder: `<project>/<feature id>/<picture id>.png`.
pub(super) const DIR: &str = "feature-pictures";

/// The most a picture may weigh.
const MOST_BYTES: u64 = 20 * 1024 * 1024;

pub(super) const UNKNOWN: &str = "fleet.picture_unknown";
const SOURCE: &str = "fleet.picture_source";
const UNREADABLE: &str = "fleet.picture_unreadable";

/// What every PNG begins with.
const PNG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// The width and height a PNG's header gives, or `None` for bytes that are no PNG.
pub(crate) fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != PNG || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width > 0 && height > 0).then_some((width, height))
}

/// The pictures folder under the support folder of `home`.
pub(super) fn folder(home: &str) -> PathBuf {
    crate::runtime::support_dir(home).join(DIR)
}

/// Where a picture's file is kept, under [`folder`].
pub(super) fn file_of(project: &str, feature_id: &str, id: &str) -> String {
    format!("{}/{}/{id}.png", plain(project), plain(feature_id))
}

/// A name with anything that could leave its folder taken out.
fn plain(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') { c } else { '_' }).collect()
}

/// The standing pictures of `feature_id`, newest first.
pub(super) fn standing(store: &Store, feature_id: &str) -> Result<Vec<KeptPicture>, WriteError> {
    Ok(store.feature_pictures(feature_id)?.into_iter().filter(|one| one.removed_at.is_none()).collect())
}

pub(super) fn on_the_wire(kept: KeptPicture) -> FeaturePicture {
    let some = |text: String| Some(text).filter(|text| !text.is_empty());
    FeaturePicture {
        id: kept.id,
        url: some(kept.url),
        screen: some(kept.screen),
        commit: some(kept.at_commit),
        taken_at: Instant::carried(kept.taken_at),
        width: kept.width,
        height: kept.height,
        bytes: kept.bytes,
        by: if kept.taken_by == picture_by::LIBRARIAN { PictureBy::Librarian } else { PictureBy::Job },
        job: some(kept.job),
    }
}

/// Write `bytes` as the file `file` names under `home`'s pictures folder. **Blocking.**
pub(super) fn write_file(home: &str, file: &str, bytes: &[u8]) -> std::io::Result<()> {
    let path = folder(home).join(file);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

/// The bytes at `path`, where they are a PNG of at most [`MOST_BYTES`], with its size.
async fn read_png(path: &Path) -> Result<(Vec<u8>, (u32, u32)), String> {
    let meta = tokio::fs::metadata(path).await.map_err(|why| format!("{} cannot be read: {why}", path.display()))?;
    if meta.len() > MOST_BYTES {
        return Err(format!("{} is over 20 MB", path.display()));
    }
    let bytes = tokio::fs::read(path).await.map_err(|why| format!("{} cannot be read: {why}", path.display()))?;
    let size = png_size(&bytes).ok_or_else(|| format!("{} is not a PNG", path.display()))?;
    Ok((bytes, size))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn picture_unknown(&self, id: &str) -> Refusal {
        Refusal::NotFound(ipc::WireError::raised(UNKNOWN, format!("no picture `{id}`"), self.run_id()))
    }

    /// `add_feature_picture`. **Nothing is kept on a refusal.**
    pub(super) async fn feature_picture_added(&self, caller: Option<api::Caller>, add: AddFeaturePicture) -> Result<FeaturePictureAdded, Refusal> {
        let job = self.waiting_session(caller, add.session_id.as_ref())?;
        let given = |text: &Option<String>| text.as_deref().map(str::trim).filter(|text| !text.is_empty()).map(str::to_string);
        let (url, screen) = (given(&add.url), given(&add.screen));
        if url.is_none() && screen.is_none() {
            return Err(self.hosted_refusal(SOURCE, "give `url`, the page's address, or `screen`, the native walk's name where it has none"));
        }
        let (bytes, (width, height)) = read_png(Path::new(add.path.trim())).await.map_err(|why| self.hosted_refusal(UNREADABLE, &why))?;
        let feature = {
            let store = self.store().lock().await;
            let session = store.session(&job).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.hosted_refusal(SOURCE, "a job Fleet knows"))?;
            if session.state == SessionState::Ended {
                return Err(self.closed(&job));
            }
            let mut projects = self.projects_of_job_in(&store, &job);
            if let Some(named) = given(&add.project) {
                projects.insert(0, named);
            }
            let mut found = None;
            for project in &projects {
                let loaded = Loaded::read(&store, project).map_err(|why| self.ledger_fault(why))?;
                if let Some(feature) = loaded.feature_named(add.feature.trim()) {
                    found = Some(feature.clone());
                    break;
                }
            }
            found.ok_or_else(|| self.hosted_refusal(super::reading::UNKNOWN, &format!("no feature `{}` in this job's project", add.feature.trim())))?
        };
        let commit = self.commit_of_job(&job).await;
        let id = self.mint().ulid().as_str().to_lowercase();
        let kept = KeptPicture {
            file: file_of(&feature.project, &feature.id, &id),
            id,
            feature_id: feature.id.clone(),
            project: feature.project.clone(),
            url: url.unwrap_or_default(),
            screen: screen.unwrap_or_default(),
            at_commit: commit,
            taken_at: self.now().as_str().to_string(),
            width,
            height,
            bytes: bytes.len() as u64,
            job: job.clone(),
            taken_by: picture_by::JOB.to_string(),
            removed_at: None,
            removed_why: String::new(),
        };
        let (home, file) = (self.host().home.clone(), kept.file.clone());
        tokio::task::spawn_blocking(move || write_file(&home, &file, &bytes))
            .await
            .map_err(|why| self.hosted_fault(UNREADABLE, &why.to_string()))?
            .map_err(|why| self.hosted_fault(UNREADABLE, &format!("the picture could not be kept: {why}")))?;
        self.store().lock().await.keep_feature_picture(&kept).map_err(|why| self.ledger_fault(why))?;
        Ok(FeaturePictureAdded { feature: feature.slug, picture: on_the_wire(kept) })
    }

    /// `remove_feature_picture`. **The row and its file stay**, marked removed with when and why, and his words for why are
    /// kept as a Not this feature rule of the catalog, about the feature, as `reject_in_catalog` keeps one.
    pub(super) async fn feature_picture_removed(&self, caller: Option<api::Caller>, remove: RemoveFeaturePicture) -> Result<FeaturePictureRemoved, Refusal> {
        self.owner_or_operator_called(caller)?;
        let id = remove.picture.trim();
        let why = remove.why.as_deref().map(str::trim).unwrap_or_default();
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let picture = store.feature_picture(id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.picture_unknown(id))?;
        let removed = store.remove_feature_picture(id, &now, why).map_err(|why| self.ledger_fault(why))?;
        let slug = store.features_in(&picture.project).map_err(|why| self.ledger_fault(why))?.into_iter().find(|one| one.id == picture.feature_id).map(|one| one.slug).unwrap_or_default();
        let rule = match removed && !slug.is_empty() {
            true => {
                let about = super::catalog_rules::About { features: vec![slug.clone()], ..Default::default() };
                self.rule_kept(&mut store, &picture.project, Some(why), CatalogRuleAct::NotThisFeature, about).map_err(|why| self.ledger_fault(why))?
            }
            false => None,
        };
        let cover = standing(&store, &picture.feature_id).map_err(|why| self.ledger_fault(why))?.into_iter().next().map(|one| one.id);
        Ok(FeaturePictureRemoved { feature: slug, picture: picture.id, cover, rule })
    }

    /// `get_feature_picture`.
    pub(super) async fn feature_picture_bytes(&self, id: String) -> Result<Vec<u8>, Refusal> {
        let picture = self.store().lock().await.feature_picture(id.trim()).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.picture_unknown(&id))?;
        let path = folder(&self.host().home).join(&picture.file);
        tokio::fs::read(path).await.map_err(|_| self.picture_unknown(&id))
    }
}

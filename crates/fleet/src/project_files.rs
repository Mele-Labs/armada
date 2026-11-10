//! His projects as files, kept where he chose for each: this Mac, a synced folder or the cloud, and
//! **never a repository**. A project's members and name are the file's, and the store keeps only his
//! order and the last name it saw, shown when the place cannot be read.
//!
//! | place | the file |
//! |---|---|
//! | `this_mac` | `projects/<id>.yml` under Kit's folder |
//! | `synced` | the same under the folder he named |
//! | `cloud`, a bucket | `<prefix>/projects/<id>.yml` |
//! | `cloud`, Armada's store | the same as `this_mac`, as a repository's runs fall back |
//!
//! **Fleet reads every project at start and after every write of one, and holds them**
//! ([`Fleet::projects_held`]), so a lifecycle read never waits on a bucket. A place that cannot be
//! read leaves that project's file out with the reason and stops nothing: Fleet runs without it.

mod migrating;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{project_text, ProjectLeftOut, ProjectPlace, ProjectWritten, Projects};
use core_model::{Project, ScopeId};
use ipc::{RunCloud, RunPlace};
use store::KeptPlace;

use crate::daemon::Fleet;
use crate::piece_places::files_in;
use crate::run_memory::{cloud, place_word, places};

/// A project's file could not be written, or removed. A 500.
pub(crate) const UNWRITABLE: &str = "fleet.project_unwritable";
/// The place chosen for a project cannot be used. A 422.
pub(crate) const PLACE_REFUSED: &str = "fleet.project_place_refused";
/// Fleet holds no project by that id. A 404.
pub(crate) const NO_SUCH: &str = "fleet.no_such_project";

/// The projects as last read. A `std` lock, never held across an `.await`.
#[derive(Default)]
pub(crate) struct Held(RwLock<Arc<Projects>>);

/// Whether `id` may name a file: no separator, and not a dot-file.
pub(crate) fn file_safe(id: &str) -> bool {
    !id.is_empty() && !id.starts_with('.') && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn place_of_file(place: RunPlace) -> ProjectPlace {
    match place {
        RunPlace::Synced => ProjectPlace::Synced,
        RunPlace::Cloud => ProjectPlace::Cloud,
        RunPlace::ThisMac | RunPlace::Repository => ProjectPlace::ThisMac,
    }
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
    pub(crate) fn kit_folder(&self) -> PathBuf {
        PathBuf::from(&self.host().kit_home)
    }

    /// The projects as Fleet last read them. **Empty until the first read**, which `reconcile` makes.
    pub fn projects_held(&self) -> Arc<Projects> {
        Arc::clone(&self.project_files().0.read().unwrap_or_else(std::sync::PoisonError::into_inner))
    }

    fn project_unwritable(&self, named: &str, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(self.project_refusal(UNWRITABLE, format!("{named} could not be written: {why}")))
    }

    /// Where the file of project `id` is, from the place chosen for it.
    async fn project_kept(&self, id: &str) -> Result<(ProjectPlace, places::Where), Refusal> {
        let held = self.store().lock().await.project_place(id).map_err(|why| self.ledger_fault(why))?;
        let (place, at) = places::where_kept(&self.kit_folder(), held.as_ref());
        Ok((place_of_file(place), at))
    }

    /// Every project Fleet can see, read again and held: one for each row of his order, from the
    /// place chosen for it, and the files in Kit's folder that no row names. **Never refuses**:
    /// a file that will not read is set aside with the reason. The names the store last saw are
    /// brought level with the files'.
    pub async fn projects_reload(&self) -> Arc<Projects> {
        let rows: Vec<(String, String, Option<KeptPlace>)> = {
            let store = self.store().lock().await;
            let kept = store.projects().map(|kept| kept.projects).unwrap_or_default();
            kept.into_iter()
                .map(|one| {
                    let place = store.project_place(&one.id).ok().flatten();
                    (one.id, one.name, place)
                })
                .collect()
        };
        let kit = self.kit_folder();
        let (mut written, mut unread) = (Vec::new(), Vec::new());
        for (id, _, held) in &rows {
            let (place, at) = places::where_kept(&kit, held.as_ref());
            let place = place_of_file(place);
            if !file_safe(id) {
                unread.push(ProjectLeftOut::new(place, PathBuf::from(id), String::from("its id cannot name a file")));
                continue;
            }
            match at {
                places::Where::Folder(base) => {
                    let path = places::project_file(&base, id);
                    match tokio::fs::read_to_string(&path).await {
                        Ok(text) => written.push(ProjectWritten::new(place, path, text)),
                        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                            unread.push(ProjectLeftOut::new(place, path, String::from("the file is not there")))
                        }
                        Err(why) => unread.push(ProjectLeftOut::new(place, path.clone(), format!("{} could not be read: {why}", path.display()))),
                    }
                }
                places::Where::Bucket(address, _) => {
                    let object = places::project_object(id);
                    let (reaching, at) = (Arc::clone(self.reaching()), object.clone());
                    match cloud::blocking(move || reaching.bucket(&address)?.get(&at)).await {
                        Ok(Some(bytes)) => match String::from_utf8(bytes) {
                            Ok(text) => written.push(ProjectWritten::new(place, PathBuf::from(object), text)),
                            Err(_) => unread.push(ProjectLeftOut::new(place, PathBuf::from(object), String::from("is not text"))),
                        },
                        Ok(None) => unread.push(ProjectLeftOut::new(place, PathBuf::from(object), String::from("the object is not there"))),
                        Err(fault) => unread.push(ProjectLeftOut::new(place, PathBuf::from(object), fault.said().to_string())),
                    }
                }
                places::Where::Nowhere(why) => unread.push(ProjectLeftOut::new(place, PathBuf::from(places::project_object(id)), why)),
            }
        }
        let named = |path: &std::path::Path| path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string);
        for file in files_in(&kit.join(places::PROJECTS), config::PieceWritten::in_kit) {
            if named(file.path()).is_some_and(|stem| rows.iter().all(|(id, _, _)| id != &stem)) {
                written.push(ProjectWritten::new(ProjectPlace::ThisMac, file.path().to_path_buf(), file.text().to_string()));
            }
        }
        let projects = Arc::new(Projects::of(written, unread));
        {
            let mut store = self.store().lock().await;
            for (id, name, _) in &rows {
                let read = ScopeId::new(id).ok().and_then(|scope| projects.get(&scope).map(|one| one.name().as_str().to_string()));
                if let Some(read) = read.filter(|read| read != name) {
                    let _ = store.rename_project(id, &read);
                }
            }
        }
        *self.project_files().0.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::clone(&projects);
        projects
    }

    /// Write `project` as the file its place keeps. **The staged write**, over a temporary file.
    pub(crate) async fn project_stored(&self, project: &Project) -> Result<(), Refusal> {
        let held = self.store().lock().await.project_place(project.id().as_str()).map_err(|why| self.ledger_fault(why))?;
        self.project_stored_at(project, held.as_ref()).await
    }

    /// Write `project` as the file the place `held` keeps, which is chosen and not yet recorded.
    async fn project_stored_at(&self, project: &Project, held: Option<&KeptPlace>) -> Result<(), Refusal> {
        let id = project.id().as_str().to_string();
        if !file_safe(&id) {
            return Err(self.project_unwritable(&id, "its id cannot name a file"));
        }
        let text = project_text(project).map_err(|why| self.project_unwritable(&id, why))?;
        match places::where_kept(&self.kit_folder(), held).1 {
            places::Where::Folder(base) => {
                let path = places::project_file(&base, &id);
                let named = path.display().to_string();
                tokio::task::spawn_blocking(move || crate::piece_saving::write_over(&path, &text))
                    .await
                    .map_err(|why| self.project_unwritable(&named, why))?
                    .map_err(|why| self.project_unwritable(&named, why))
            }
            places::Where::Bucket(address, _) => {
                let object = places::project_object(&id);
                let (reaching, at) = (Arc::clone(self.reaching()), object.clone());
                cloud::blocking(move || reaching.bucket(&address)?.put(&at, text.as_bytes()))
                    .await
                    .map_err(|fault| Refusal::Unacceptable(self.project_refusal(fault.code(), fault.said().to_string())))
            }
            places::Where::Nowhere(why) => Err(self.project_unwritable(&format!("The project {id}"), why)),
        }
    }

    /// Take the file of project `id` away where Fleet keeps it itself: Kit's. A synced folder's and a
    /// bucket's are the person's, and are left.
    pub(crate) async fn project_file_removed(&self, id: &str) -> Result<(), Refusal> {
        let (place, at) = self.project_kept(id).await?;
        let places::Where::Folder(base) = at else { return Ok(()) };
        if place != ProjectPlace::ThisMac || !file_safe(id) {
            return Ok(());
        }
        let path = places::project_file(&base, id);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(why) => Err(self.project_unwritable(&path.display().to_string(), why)),
        }
    }

    /// Choose where project `id`'s file is kept, write it there, and read every project again.
    /// **`repository` is refused**: a project is not kept in one. The old place's file is taken away
    /// where it was Kit's; a synced folder's and a bucket's stay, and are never read again.
    pub async fn project_place_set(
        &self,
        id: &str,
        place: RunPlace,
        synced_folder: Option<String>,
        cloud: Option<RunCloud>,
    ) -> Result<(), Refusal> {
        let refused = |said: String| Refusal::Unacceptable(self.project_refusal(PLACE_REFUSED, said));
        if place == RunPlace::Repository {
            return Err(refused(String::from("a project is not kept in a repository: choose this Mac, a synced folder or the cloud")));
        }
        let project = ScopeId::new(id).ok().and_then(|scope| self.projects_held().get(&scope).cloned()).ok_or_else(|| {
            Refusal::NotFound(self.project_refusal(NO_SUCH, format!("Fleet holds no project {id} that reads, so it is not moved")))
        })?;
        let picked = places::place_chosen(place, synced_folder.as_deref(), cloud.as_ref()).map_err(refused)?;
        let chosen = KeptPlace {
            place: place_word(place).to_string(),
            synced_folder: picked.folder.clone(),
            cloud: picked.cloud.as_ref().map(cloud::Target::text),
        };
        if let places::Where::Bucket(address, _) = places::where_kept(&self.kit_folder(), Some(&chosen)).1 {
            let (reaching, at) = (Arc::clone(self.reaching()), places::project_object(id));
            cloud::blocking(move || reaching.bucket(&address)?.get(&at))
                .await
                .map_err(|fault| Refusal::Unacceptable(self.project_refusal(fault.code(), fault.said().to_string())))?;
        }
        let was = self.project_kept(id).await?;
        // The old place stays unless the new one takes the file.
        self.project_stored_at(&project, Some(&chosen)).await?;
        self.store()
            .lock()
            .await
            .set_project_place(id, &chosen.place, chosen.synced_folder.as_deref(), chosen.cloud.as_deref())
            .map_err(|why| self.ledger_fault(why))?;
        if let (ProjectPlace::ThisMac, places::Where::Folder(base)) = was {
            let old = places::project_file(&base, id);
            let moved_off = !matches!(self.project_kept(id).await?.1, places::Where::Folder(now) if places::project_file(&now, id) == old);
            if moved_off {
                let _ = std::fs::remove_file(old);
            }
        }
        self.projects_reload().await;
        Ok(())
    }
}

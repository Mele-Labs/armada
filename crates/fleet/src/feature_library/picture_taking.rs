//! The librarian's `pictures` part: one picture a feature, of the screen its files are tied to, taken with no window on his
//! screen. `docs/concepts/feature-catalog.md`, *Pictures*.
//!
//! **A feature's screen is the one its newest picture shows**, a Job's or the librarian's, and where it has none, the page
//! its files route to in a web app (`route_of`). A screen he took off the feature with Not this feature is never taken
//! again for it. **A page is taken on the project's running server**, at the picture's path; a native screen is the walk
//! of that name in Armada's kept build. The librarian's take replaces its own last take of the same screen, never a Job's,
//! and a screen whose app stands at the commit of that take is not taken again.

use std::collections::BTreeSet;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{ServerPhase, ServerState};
use store::{picture_by, KeptFeature, KeptPicture, WriteError};

use super::pictures::{file_of, folder, png_size, write_file};
use super::reading::Loaded;
use crate::daemon::Fleet;

/// A screen a feature is tied to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Screen {
    /// A page of a web app, by its path from the server's root: `/votes?tab=open`.
    Page(String),
    /// A native walk, by its name.
    Walk(String),
}

/// The path from the root of `url`, `/` where it has none.
fn path_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    match rest.find(['/', '?', '#']) {
        Some(at) if rest[at..].starts_with('/') => rest[at..].to_string(),
        Some(at) => format!("/{}", &rest[at..]),
        None => "/".to_string(),
    }
}

/// The scheme and host of `url`: `http://localhost:3000`.
fn origin_of(url: &str) -> String {
    let (scheme, rest) = url.split_once("://").unwrap_or(("http", url));
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    format!("{scheme}://{host}")
}

fn screen_of(picture: &KeptPicture) -> Option<Screen> {
    match (picture.url.is_empty(), picture.screen.is_empty()) {
        (false, _) => Some(Screen::Page(path_of(&picture.url))),
        (true, false) => Some(Screen::Walk(picture.screen.clone())),
        (true, true) => None,
    }
}

/// The page a web app serves the file at `path` as, where its framework routes by file: Next's `app/…/page.tsx` and
/// `pages/…`, and SvelteKit's `routes/…/+page.svelte`. `None` for any other file, and for a route with a segment filled
/// in at request time (`[id]`), which has no one address.
pub(crate) fn route_of(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').collect();
    let (last, folders) = segments.split_last()?;
    let (stem, ending) = last.rsplit_once('.')?;
    let script = matches!(ending, "tsx" | "jsx" | "ts" | "js" | "mdx");
    let (at, kind) = folders.iter().enumerate().find_map(|(at, one)| matches!(*one, "app" | "pages" | "routes").then_some((at, *one)))?;
    let mut route: Vec<&str> = folders[at + 1..].iter().copied().filter(|one| !(one.starts_with('(') && one.ends_with(')'))).collect();
    match kind {
        "app" if script && stem == "page" => {}
        "routes" if *last == "+page.svelte" => {}
        "pages" if script && !stem.starts_with('_') && route.first() != Some(&"api") => {
            if stem != "index" {
                route.push(stem);
            }
        }
        _ => return None,
    }
    if route.iter().any(|one| one.starts_with('[') || one.starts_with('@') || one.starts_with('_')) {
        return None;
    }
    Some(format!("/{}", route.join("/")))
}

/// The page a feature's own files route to: the shortest of their routes, the first by name among the shortest.
fn routed(feature: &KeptFeature, loaded: &Loaded, tracked: &[String]) -> Option<String> {
    tracked
        .iter()
        .filter(|path| loaded.map.owners(None, path).iter().any(|owner| owner.feature.id == feature.id))
        .filter_map(|path| route_of(path))
        .min_by(|one, other| one.matches('/').count().cmp(&other.matches('/').count()).then_with(|| one.cmp(other)))
}

/// Where the project's app can be reached now: the root of a running web server, and Armada's kept native Bridge.
#[derive(Clone, Debug, Default)]
pub(crate) struct Reach {
    /// The server's root and the commit its checkout stood at when it started.
    pub(crate) web: Option<(String, String)>,
    /// The kept native Bridge and the commit it was built from.
    pub(crate) native: Option<(PathBuf, String)>,
}

/// The running server of `repositories` a page is taken on: a main checkout's before a Job's slot's.
pub(crate) fn web_of(servers: &[ServerState], repositories: &[String]) -> Option<(String, String)> {
    let mut serving: Vec<&ServerState> = servers
        .iter()
        .filter(|one| one.phase == ServerPhase::Serving)
        .filter(|one| one.manifest_id.as_ref().is_some_and(|manifest| repositories.iter().any(|held| held == manifest.as_str())))
        .collect();
    serving.sort_by_key(|one| one.checkout.branch.is_some());
    let server = serving.first()?;
    let root = match (server.links.first(), server.ports.first()) {
        (Some(link), _) => origin_of(&link.url),
        (None, Some(port)) => format!("http://localhost:{}", port.port),
        (None, None) => return None,
    };
    Some((root, server.checkout.commit.clone().unwrap_or_default()))
}

/// What a run of the part did.
#[derive(Debug, Default)]
pub(super) struct Taken {
    /// Features tied to a screen the app can be reached on.
    pub(super) tied: usize,
    /// Features tied to a screen, reached or not.
    pub(super) screens: usize,
    pub(super) took: usize,
    /// Screens whose app stands where the last take was.
    pub(super) current: usize,
    /// Why each picture that failed did.
    pub(super) failed: Vec<String>,
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
    /// Where `project`'s app can be reached now, its repositories at `roots`.
    async fn reach_of(&self, project: &str, roots: &[PathBuf]) -> Reach {
        let repositories = self.repositories_of(&*self.store().lock().await, project);
        let web = web_of(&self.server_list().servers, &repositories);
        let native = match roots.iter().any(|root| root.join("crates/bridge-native/Cargo.toml").is_file()) {
            false => None,
            true => {
                let build = crate::native_walks::running_build(&self.host().home);
                let app = build.join("Armada.app/Contents/MacOS/Armada");
                let builds = crate::runtime::support_dir(&self.host().home).join(crate::kept_builds::BUILDS);
                let commit = crate::kept_builds::kept_at(&builds, crate::kept_builds::CURRENT).map(|kept| kept.commit).unwrap_or_default();
                app.is_file().then_some((app, commit))
            }
        };
        Reach { web, native }
    }

    /// The `pictures` part for `project`: one picture a feature tied to a screen its app can be reached on.
    pub(super) async fn pictures_taken(&self, project: &str, roots: &[PathBuf]) -> Result<Taken, WriteError> {
        let reach = self.reach_of(project, roots).await;
        let tracked: Vec<String> = match reach.web.is_some() {
            false => Vec::new(),
            true => {
                let roots = roots.to_vec();
                tokio::task::spawn_blocking(move || roots.iter().flat_map(|root| super::floor_files::tracked(root)).map(|(path, _)| path).collect()).await.unwrap_or_default()
            }
        };
        // Each feature's screen and the librarian's own last take of it, read at once.
        let mut asked: Vec<(KeptFeature, Screen, Option<KeptPicture>)> = Vec::new();
        {
            let store = self.store().lock().await;
            let loaded = Loaded::read(&store, project)?;
            for feature in loaded.map.features().iter().filter(|one| one.state != store::feature_state::PENDING) {
                let pictures = store.feature_pictures(&feature.id)?;
                let refused: BTreeSet<Screen> = pictures.iter().filter(|one| one.removed_at.is_some()).filter_map(screen_of).collect();
                let standing: Vec<&KeptPicture> = pictures.iter().filter(|one| one.removed_at.is_none()).collect();
                let screen = standing.first().and_then(|one| screen_of(one)).or_else(|| routed(feature, &loaded, &tracked).map(Screen::Page));
                let Some(screen) = screen.filter(|screen| !refused.contains(screen)) else { continue };
                let last = standing.iter().find(|one| one.taken_by == picture_by::LIBRARIAN && screen_of(one).as_ref() == Some(&screen)).map(|one| (*one).clone());
                asked.push((feature.clone(), screen, last));
            }
        }
        let mut taken = Taken { screens: asked.len(), ..Taken::default() };
        let home = self.host().home.clone();
        for (feature, screen, last) in asked {
            let (url, walk, commit) = match (&screen, &reach.web, &reach.native) {
                (Screen::Page(path), Some((root, commit)), _) => (format!("{root}{path}"), String::new(), commit.clone()),
                (Screen::Walk(name), _, Some((_, commit))) => (String::new(), name.clone(), commit.clone()),
                _ => continue,
            };
            taken.tied += 1;
            if last.as_ref().is_some_and(|last| !commit.is_empty() && last.at_commit == commit) {
                taken.current += 1;
                continue;
            }
            let id = self.mint().ulid().as_str().to_lowercase();
            let file = file_of(project, &feature.id, &id);
            let out = folder(&home).join(format!("{file}.taking"));
            let (capture, app, target) = (self.capture(), reach.native.as_ref().map(|(app, _)| app.clone()), if walk.is_empty() { url.clone() } else { walk.clone() });
            let shot = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent).map_err(|why| why.to_string())?;
                }
                let made = match app.filter(|_| !walk.is_empty()) {
                    Some(app) => capture.walk(&app, &target, &out),
                    None => capture.page(&target, &out),
                };
                let read = made.and_then(|()| std::fs::read(&out).map_err(|why| format!("it left no picture: {why}")));
                let _ = std::fs::remove_file(&out);
                read
            })
            .await
            .unwrap_or_else(|why| Err(why.to_string()));
            let (bytes, (width, height)) = match shot.and_then(|bytes| png_size(&bytes).map(|size| (bytes, size)).ok_or_else(|| "it left no PNG".to_string())) {
                Ok(read) => read,
                Err(why) => {
                    taken.failed.push(why);
                    continue;
                }
            };
            let size = bytes.len() as u64;
            let (writing, at) = (home.clone(), file.clone());
            if let Err(why) = tokio::task::spawn_blocking(move || write_file(&writing, &at, &bytes)).await.unwrap_or_else(|why| Err(std::io::Error::other(why))) {
                taken.failed.push(why.to_string());
                continue;
            }
            let kept = KeptPicture {
                id,
                feature_id: feature.id.clone(),
                project: project.to_string(),
                file,
                url: if let Screen::Page(_) = screen { url } else { String::new() },
                screen: if let Screen::Walk(name) = &screen { name.clone() } else { String::new() },
                at_commit: commit,
                taken_at: self.now().as_str().to_string(),
                width,
                height,
                bytes: size,
                job: String::new(),
                taken_by: picture_by::LIBRARIAN.to_string(),
                removed_at: None,
                removed_why: String::new(),
            };
            let mut store = self.store().lock().await;
            store.keep_feature_picture(&kept)?;
            if let Some(last) = last {
                if store.forget_librarian_picture(&last.id)? {
                    let _ = std::fs::remove_file(folder(&home).join(&last.file));
                }
            }
            taken.took += 1;
        }
        Ok(taken)
    }
}

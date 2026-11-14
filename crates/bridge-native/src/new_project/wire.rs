//! What the New project sheet reads and sends, each a request off the main thread on the port the Fleet
//! connection read, or answered at once by the walk's mock. `crates/ipc/operations/`: `list_repositories`,
//! `list_repository_areas`, `get_values`, `clone_repository`, `create_project`, `add_project` and `edit_project`.
//!
//! **Two searches ask Fleet as he types**: [`Wire::search_folders`], `GET /manifest/files` with `folders=true`, and
//! [`Wire::search_forge`], `GET /repositories/forge`, which both answer through [`Asked`].

use std::time::Duration;

use gpui::{App, AppContext, Entity, Task};
use ipc::{EditProject, ForgeRepositories, ModelChoices, NewProject, Projects, RepositoryAreas, RepositorySummary, ResolvedValues};
use serde::de::DeserializeOwned;

use crate::fleet::FleetConnection;

/// How long a clone may take before Bridge stops waiting: past Fleet's own ten minutes, so Fleet's answer
/// arrives first (`clone_repository.toml`).
const CLONE_PATIENCE: Duration = Duration::from_secs(11 * 60);

/// What a folder search is made against: a repository with a Manifest by its id, one Fleet serves with none
/// (a clone, a repository the sheet just made) by its root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Manifest(String),
    Root(String),
}

/// What Fleet said to a search as he typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked<T> {
    Found(T),
    /// Fleet's own sentence for a 422.
    Refused(String),
    /// Not asked, not answered, or answered with something the sheet cannot show.
    Down,
}

/// What Fleet said to a folder search.
pub type Folders = Asked<Vec<String>>;

/// What Fleet said to a search of his forge account.
pub type Forge = Asked<ForgeRepositories>;

/// The request for the repositories of his forge account matching `query`.
pub fn forge_path(query: &str) -> String {
    format!("/repositories/forge?q={}", crate::link::percent_encoded(query))
}

/// `GET path` off the main thread: the answer as `then` reads it, a 422 as Fleet's sentence, and anything else
/// the sheet cannot show as down.
fn asked<T: DeserializeOwned + Send + 'static, U: Send + 'static>(port: u16, path: String, what: &'static str, then: fn(T) -> U, cx: &App) -> Task<Asked<U>> {
    cx.background_spawn(async move {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return Asked::Down };
        let Ok((status, bytes)) = runtime.block_on(crate::transport::get(port, &path)) else { return Asked::Down };
        match status {
            200..=299 => ipc::decode::<T>(what, &bytes).map_or(Asked::Down, |found| Asked::Found(then(found))),
            422 => ipc::decode::<ipc::WireError>("WireError", &bytes).map_or(Asked::Down, |error| Asked::Refused(error.message)),
            _ => Asked::Down,
        }
    })
}

/// The request for a repository's folders matching `query`: Fleet takes one of `manifest_id` and `root`.
pub fn folders_path(place: &Place, query: &str) -> String {
    let named = match place {
        Place::Manifest(id) => format!("manifest_id={}", crate::link::percent_encoded(id)),
        Place::Root(root) => format!("root={}", crate::link::percent_encoded(root)),
    };
    format!("/manifest/files?q={}&{named}&folders=true", crate::link::percent_encoded(query))
}

pub enum Wire {
    Live(Entity<FleetConnection>),
    Mock(Box<Mock>),
}

enum Ask {
    Get(String),
    Post(String, String),
    /// A post Fleet answers when its work is done.
    Waiting(String, String, Duration),
}

fn request<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, cx: &App) -> Task<Result<T, String>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = match ask {
            Ask::Get(path) => runtime.block_on(crate::transport::get(port, &path)),
            Ask::Post(path, body) => runtime.block_on(crate::transport::post(port, &path, body)),
            Ask::Waiting(path, body, patience) => runtime.block_on(crate::transport::post_waiting(port, &path, body, patience)),
        }?;
        if !(200..300).contains(&status) {
            return Err(match ipc::decode::<ipc::WireError>("WireError", &bytes) {
                Ok(error) => error.message,
                Err(_) => format!("Fleet answered {status}"),
            });
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| why.to_string())
    })
}

fn body(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn unreachable<T: Send + 'static>() -> Task<Result<T, String>> {
    Task::ready(Err("Fleet is not connected".into()))
}

impl Wire {
    fn port(&self, cx: &App) -> Option<u16> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().port(),
            Wire::Mock(_) => None,
        }
    }

    /// The repositories Fleet serves, as the connection last read them; `None` before it has.
    pub fn repositories(&self, cx: &App) -> Option<Vec<RepositorySummary>> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().repositories.clone(),
            Wire::Mock(mock) => Some(mock.served.clone()),
        }
    }

    /// `GET /repository_areas`: every served repository's directories, two names deep.
    pub fn areas(&self, cx: &App) -> Task<Result<RepositoryAreas, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.areas.clone()));
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/repository_areas".into()), "RepositoryAreas", cx))
    }

    /// `GET /projects/values` for no project: what every project uses.
    pub fn values(&self, cx: &App) -> Task<Result<ResolvedValues, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.values.clone()));
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/projects/values".into()), "ResolvedValues", cx))
    }

    /// The mods on this Mac, as the connection last read them: the trackers among them are offered beside the built in ones.
    pub fn mods(&self, cx: &App) -> Vec<ipc::ModSummary> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().mods.clone().unwrap_or_default(),
            Wire::Mock(mock) => mock.mods.clone(),
        }
    }

    /// `POST /sessions/dispatch`: a Job started in a repository with `words` as its first message, by the route and
    /// client Bridge's own start uses. The answer is the Job's id, none where the Operator holds the idea.
    pub fn start_job(&mut self, manifest: &str, words: String, cx: &App) -> Task<Result<Option<String>, String>> {
        if let Wire::Mock(mock) = self {
            mock.started.push((manifest.to_string(), words));
            return Task::ready(Ok(Some("mock-tracker-mod".to_string())));
        }
        let Some(port) = self.port(cx) else { return unreachable() };
        match crate::custom_tracker::dispatch_body(manifest, None, &words) {
            Ok(body) => crate::custom_tracker::send(port, body, cx),
            Err(why) => Task::ready(Err(why)),
        }
    }

    /// `GET /models`: the models a review may run on.
    pub fn models(&self, cx: &App) -> Task<Result<Vec<String>, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.models.clone()));
        }
        let Some(port) = self.port(cx) else { return unreachable() };
        let read: Task<Result<ModelChoices, String>> = request(port, Ask::Get("/models".into()), "ModelChoices", cx);
        cx.background_spawn(async move { read.await.map(|choices| choices.models) })
    }

    /// `POST /repositories/clone`.
    pub fn clone_repository(&mut self, url: String, parent: String, cx: &App) -> Task<Result<RepositorySummary, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.served_new(&url, &parent));
        }
        let said = body(&ipc::CloneRepository { url, parent });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Waiting("/repositories/clone".into(), said, CLONE_PATIENCE), "RepositorySummary", cx))
    }

    /// `POST /repositories/create`: a new folder under `parent`, started as a git repository.
    pub fn create_repository(&mut self, name: String, parent: String, cx: &App) -> Task<Result<RepositorySummary, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.served_new(&name, &parent));
        }
        let said = body(&ipc::CreateProject { parent, name });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Waiting("/repositories/create".into(), said, CLONE_PATIENCE), "RepositorySummary", cx))
    }

    /// `POST /projects`. Fleet answers with every project, the new one last.
    pub fn add_project(&mut self, project: NewProject, cx: &App) -> Task<Result<Projects, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.added(project)));
        }
        let said = body(&project);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/projects".into(), said), "Projects", cx))
    }

    /// `POST /projects/project/:id/edit` with sharing turned on: `add_project` takes no sharing.
    pub fn share(&mut self, id: &str, cx: &App) -> Task<Result<Projects, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.projects.clone()));
        }
        let path = format!("/projects/project/{}/edit", crate::link::percent_encoded(id));
        let said = body(&EditProject { shared_with_macs: Some(true), ..EditProject::default() });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "Projects", cx))
    }

    // -- the folder search, and what Fleet does not serve yet ---------------------------------------------

    /// `GET /manifest/files?folders=true`: every folder of a repository whose path fuzzy-matches `query`, in
    /// Fleet's order. [`Folders::Down`] where Fleet cannot be asked or answered with anything but a refusal,
    /// and the sheet searches the folders it read.
    pub fn search_folders(&self, place: &Place, query: &str, cx: &App) -> Task<Folders> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.folders_found(place, query));
        }
        let Some(port) = self.port(cx) else { return Task::ready(Folders::Down) };
        asked(port, folders_path(place, query), "FilesFound", |found: ipc::FilesFound| found.folders, cx)
    }

    /// `GET /repositories/forge?q=`: the repositories his forge account can see whose owner and name hold every
    /// word of `query`, each marked where Fleet already serves it.
    pub fn search_forge(&self, query: &str, cx: &App) -> Task<Forge> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.forge_found(query));
        }
        let Some(port) = self.port(cx) else { return Task::ready(Forge::Down) };
        asked(port, forge_path(query), "ForgeRepositories", |found: ForgeRepositories| found, cx)
    }

    /// Whether this is the walk's Fleet, which answers at once and needs no wait for typing to stop.
    pub fn is_mock(&self) -> bool {
        matches!(self, Wire::Mock(_))
    }
}

/// The walk's Fleet: the walks' repositories and projects, folders for the first three, and what a project is
/// made with kept in its `values`, so the Job started in it can say how its work lands.
pub struct Mock {
    pub served: Vec<RepositorySummary>,
    pub areas: RepositoryAreas,
    pub values: ResolvedValues,
    pub models: Vec<String>,
    pub projects: Projects,
    /// The repositories his forge account sees in the walks.
    pub forge: ForgeRepositories,
    /// The mods on this Mac in the walks.
    pub mods: Vec<ipc::ModSummary>,
    /// Every Job the walk or a test started through a dispatch: the repository's Manifest id and the words.
    pub started: Vec<(String, String)>,
}

/// Folders below the two the areas read, as an index of every folder would hold them: what the walk's search
/// finds that the top folders do not show.
const MOCK_DEEP_FOLDERS: [(&str, &[&str]); 1] = [(
    "sample-web",
    &["web/cart/hooks", "web/account/settings", "services/pricing/rules", "services/pricing/rules/regional"],
)];

impl Mock {
    pub fn new() -> Mock {
        // A walk has no Fleet to say how he reads a time (fleet.rs holds its answer on connect), so the mock says it: a
        // 12-hour clock with no seconds, as every other stamp on his screen is drawn.
        crate::settings::time::hold(r#"{"hour12":true,"seconds":false,"date":false}"#);
        Mock {
            served: crate::mock::repositories(),
            areas: crate::mock::repository_areas(),
            values: ipc::decode("ResolvedValues", MOCK_VALUES.as_bytes()).unwrap_or_else(|why| panic!("the mock values do not decode: {why}")),
            models: vec!["opus".into(), "sonnet".into(), "haiku".into()],
            projects: crate::mock::projects(),
            forge: ipc::decode("ForgeRepositories", MOCK_FORGE.as_bytes()).unwrap_or_else(|why| panic!("the mock forge does not decode: {why}")),
            mods: crate::settings::walk::tracker_mods(),
            started: Vec::new(),
        }
    }

    /// What the walk's forge answers a search: each repository whose owner and name hold every word, and `cut`
    /// where the words are two letters or fewer, which stand in for more matching than Fleet lists. A search
    /// for `offline` stands in for a forge that will not list, with the sentence Fleet gives for no token.
    fn forge_found(&self, query: &str) -> Forge {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.iter().any(|word| word == "offline") {
            return Forge::Refused("GitHub has no token for this machine. Run `gh auth login`, then open it again.".into());
        }
        let repositories = self.forge.repositories.iter().filter(|one| words.iter().all(|word| format!("{}/{}", one.owner, one.name).to_lowercase().contains(word.as_str()))).cloned().collect();
        Forge::Found(ForgeRepositories { repositories, cut: query.trim().chars().count() <= 2 })
    }

    /// What the walk's Fleet answers a folder search: the areas' folders and the deeper ones, ranked as the
    /// sheet ranks its own. **A repository searched by its root is refused**, as Fleet refuses a root that is no
    /// longer a folder on this machine (`fleet.no_such_folder`, in `existing_folder`): the walk's repositories
    /// with no Manifest are not on a disk.
    fn folders_found(&self, place: &Place, query: &str) -> Folders {
        let id = match place {
            Place::Manifest(id) => id,
            Place::Root(root) => return Folders::Refused(format!("{root} is not a folder on this machine")),
        };
        let mut all: Vec<String> = self.areas.repositories.iter().filter(|one| one.manifest_id.as_str() == id).flat_map(|one| one.areas.iter()).flat_map(|area| {
            std::iter::once(area.dir.clone()).chain(area.kids.iter().map(|kid| format!("{}/{}", area.dir, kid.name)))
        }).collect();
        all.extend(MOCK_DEEP_FOLDERS.iter().filter(|(of, _)| of == id).flat_map(|(_, deep)| deep.iter().map(|path| path.to_string())));
        let mut found: Vec<(u32, String)> = all.into_iter().filter_map(|path| super::fuzzy(query, &path.to_lowercase()).map(|(score, _)| (score, path))).collect();
        found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.len().cmp(&b.1.len())).then(a.1.cmp(&b.1)));
        Folders::Found(found.into_iter().map(|(_, path)| path).collect())
    }

    /// A clone or a new repository, served as Fleet answers one: no Manifest, the folder's name its id.
    fn served_new(&mut self, from: &str, parent: &str) -> Result<RepositorySummary, String> {
        let name = from.trim_end_matches('/').trim_end_matches(".git").rsplit(['/', ':']).next().unwrap_or(from).to_string();
        if name.is_empty() {
            return Err("git refused the URL".into());
        }
        let root = format!("{}/{name}", parent.trim_end_matches('/'));
        let json = format!(r#"{{"root":"{root}","records_root":"/Users/user/Library/Application Support/Armada/records/{name}","implicit_id":"{name}"}}"#);
        let one: RepositorySummary = ipc::decode("RepositorySummary", json.as_bytes()).map_err(|why| why.to_string())?;
        self.served.push(one.clone());
        Ok(one)
    }

    fn added(&mut self, project: NewProject) -> Projects {
        let id = project.name.trim().to_lowercase().replace(' ', "-");
        self.projects.projects.push(ipc::Project {
            id,
            name: project.name.trim().to_string(),
            members: project.members.unwrap_or_default(),
            place: project.place.unwrap_or_default(),
            values: project.values,
            ..ipc::Project::default()
        });
        self.projects.clone()
    }
}

/// What every project uses on the walk's Fleet: Kit merges once checks pass, plans and pull requests reviewed.
const MOCK_VALUES: &str = r#"{
 "pr_mode": {"value": "ready", "from": "default"},
 "auto_merge": {"value": "checks-pass", "from": "kit"},
 "review_model": {"value": "opus", "from": "kit"},
 "layout": {"from": "default"},
 "theme": {"from": "default"},
 "density": {"value": "default", "from": "default"},
 "reviews": {"plans": "review", "pull_requests": "review_and_approve", "walks": "off", "from": "kit"},
 "agent_starts": {"value": "at_once", "from": "default"},
 "delivery": {"from": "default"}
}"#;

/// His forge account in the walks: a private repository, one Fleet serves already, and two more.
const MOCK_FORGE: &str = r#"{"repositories": [
 {"owner": "acme", "name": "billing", "url": "https://github.com/acme/billing", "private": true, "pushed_at": "2026-11-12T14:02:00Z"},
 {"owner": "acme", "name": "sample-api", "url": "https://github.com/acme/sample-api", "private": false, "pushed_at": "2026-11-11T09:30:00Z", "local_path": "/Users/user/Development/sample-api"},
 {"owner": "acme", "name": "ledger", "url": "https://github.com/acme/ledger", "private": true, "pushed_at": "2026-11-02T16:45:00Z"},
 {"owner": "acme", "name": "brand-site", "url": "https://github.com/acme/brand-site", "private": false}
]}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_forge_is_searched_by_the_words_typed_encoded() {
        assert_eq!(forge_path("acme"), "/repositories/forge?q=acme");
        assert_eq!(forge_path("acme/bill ing"), "/repositories/forge?q=acme%2Fbill%20ing");
    }

    #[test]
    fn a_repository_with_a_manifest_is_searched_by_its_id_and_one_without_by_its_root_each_for_folders() {
        assert_eq!(folders_path(&Place::Manifest("sample-web".into()), "chk"), "/manifest/files?q=chk&manifest_id=sample-web&folders=true");
        assert_eq!(
            folders_path(&Place::Root("/Users/user/Development/my app".into()), "web/c k"),
            "/manifest/files?q=web%2Fc%20k&root=%2FUsers%2Fuser%2FDevelopment%2Fmy%20app&folders=true"
        );
    }
}

//! What the New project sheet reads and sends, each a request off the main thread on the port the Fleet
//! connection read, or answered at once by the walk's mock. `crates/ipc/operations/`: `list_repositories`,
//! `list_repository_areas`, `get_values`, `clone_repository`, `create_project`, `add_project` and `edit_project`.
//!
//! **Three reads Fleet does not serve yet** each have one function here, which answers nothing until it does:
//! [`Wire::delivery`], [`Wire::search_folders`] and [`Wire::github_repositories`].

use std::time::Duration;

use gpui::{App, AppContext, Entity, Task};
use ipc::{EditProject, ModelChoices, NewProject, Projects, RepositoryAreas, RepositorySummary, ResolvedValues};
use serde::de::DeserializeOwned;

use crate::fleet::FleetConnection;

/// How long a clone may take before Bridge stops waiting: past Fleet's own ten minutes, so Fleet's answer
/// arrives first (`clone_repository.toml`).
const CLONE_PATIENCE: Duration = Duration::from_secs(11 * 60);

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
    pub fn add_project(&mut self, project: NewProject, delivery: Option<String>, cx: &App) -> Task<Result<Projects, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.added(project, delivery)));
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

    // -- what Fleet does not serve yet -----------------------------------------------------------------

    /// How a project's work lands, `auto_land`, `open_pr` or `keep_local`, as its settings say. **Fleet keeps no
    /// `delivery` value yet**, so a live project has none and the Job's line is not drawn.
    /// Owed: a `delivery` field in `ipc::ValuesSet`, sent in `NewProject.values` and read back here.
    pub fn delivery(&self, project: &str) -> Option<String> {
        match self {
            Wire::Live(_) => None,
            Wire::Mock(mock) => mock.delivery.iter().find(|(id, _)| id == project).map(|(_, word)| word.clone()),
        }
    }

    /// Every folder of a repository whose path fuzzy-matches `query`, best first. **Fleet serves no folder
    /// search on `/manifest/files` yet**, so this is `None` and the sheet searches the folders it read.
    pub fn search_folders(&self, _manifest: &str, _query: &str) -> Option<Vec<String>> {
        None
    }

    /// The repositories his GitHub account sees, for the clone field to search. **Fleet serves no such list
    /// yet**: none, and the field offers nothing as he types.
    pub fn github_repositories(&self) -> Vec<String> {
        Vec::new()
    }
}

/// The walk's Fleet: the walks' repositories and projects, folders for the first three, and what a project is
/// made with kept, so the Job started in it can say how its work lands.
pub struct Mock {
    pub served: Vec<RepositorySummary>,
    pub areas: RepositoryAreas,
    pub values: ResolvedValues,
    pub models: Vec<String>,
    pub projects: Projects,
    delivery: Vec<(String, String)>,
}

impl Mock {
    pub fn new() -> Mock {
        Mock {
            served: crate::mock::repositories(),
            areas: crate::mock::repository_areas(),
            values: ipc::decode("ResolvedValues", MOCK_VALUES.as_bytes()).unwrap_or_else(|why| panic!("the mock values do not decode: {why}")),
            models: vec!["opus".into(), "sonnet".into(), "haiku".into()],
            projects: crate::mock::projects(),
            delivery: Vec::new(),
        }
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

    fn added(&mut self, project: NewProject, delivery: Option<String>) -> Projects {
        let id = project.name.trim().to_lowercase().replace(' ', "-");
        if let Some(word) = delivery {
            self.delivery.push((id.clone(), word));
        }
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
 "agent_starts": {"value": "at_once", "from": "default"}
}"#;

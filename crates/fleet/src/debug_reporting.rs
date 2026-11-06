//! `get_debug_report`: what Fleet knows for the text a person pastes to whoever helps them.
//! `ipc::debug_report` lays it out, and `armada report` prints the same layout without Fleet.
//!
//! **Names, versions and states.** A Job's title is the one sentence a person wrote here, and
//! everything else is a name Fleet or the CLI printed. The reads are bounded: files are read as they
//! stand, the log from its last megabyte, and the CLI's version is the one a Job's opening line gave,
//! asked of the CLI for two seconds only where no Job has one.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::debug_report::{self, FleetFacts, JobFacts, ModFacts, RepositoryFacts};
use store::{KeptInit, KeptSession, SessionSearch, SessionState};

use crate::daemon::Fleet;
use crate::run_memory::places::{self, Where};

const REPORT_FAILED: &str = "fleet.debug_report_failed";

/// How long the CLI is asked its version, where no Job has told Fleet.
const VERSION_WITHIN: Duration = Duration::from_secs(2);

/// A Job's title on one line, short enough to head a block.
fn title_of(session: &KeptSession) -> String {
    let title = session.title.as_deref().unwrap_or("untitled");
    let one: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
    one.chars().take(80).collect()
}

/// What a Job came up without, in the words the report prints. Only the mod is known of a Job whose
/// opening line has not been read.
fn missing_of(mod_version: &Option<String>, init: &Option<KeptInit>) -> Vec<String> {
    let mut missing = Vec::new();
    match init {
        Some(init) => {
            if !init.mod_loaded {
                missing.push("mod".to_string());
            }
            if init.door != "connected" {
                missing.push("door".to_string());
            }
            if !init.task_tools {
                missing.push("task tools".to_string());
            }
            if init.skills.is_empty() {
                missing.push("armada: skills".to_string());
            }
        }
        None if mod_version.is_none() => missing.push("mod".to_string()),
        None => {}
    }
    missing
}

/// One live Job's block.
fn job_of(session: &KeptSession, hosted: bool, init: Option<KeptInit>) -> JobFacts {
    JobFacts {
        title: title_of(session),
        id: session.id.chars().take(8).collect(),
        hosted,
        mod_version: session.mod_version.clone(),
        door: init.as_ref().map_or_else(|| "not read".to_string(), |init| init.door.clone()),
        missing: missing_of(&session.mod_version, &init),
        tools: init.as_ref().map(|init| init.tools.clone()),
        skills: init.as_ref().map(|init| init.skills.clone()),
    }
}

/// The repository's name: its remote's, else its folder's.
fn name_of(root: &Path) -> String {
    adapters::remote_of(root)
        .and_then(|address| address.slug().rsplit('/').next().map(str::to_string))
        .or_else(|| root.file_name().map(|name| name.to_string_lossy().into_owned()))
        .unwrap_or_else(|| root.display().to_string())
}

impl Fleet
{
    /// `get_debug_report`: every part Fleet can read, laid out. `root` names the repository for its
    /// block, and none leaves the block out.
    pub async fn debug_report(&self, root: Option<String>) -> Result<ipc::DebugReport, Refusal> {
        let fault = |why: &str| self.hosted_fault(REPORT_FAILED, why);
        let (jobs, place) = {
            let store = self.store().lock().await;
            let live = store
                .find_sessions(&SessionSearch { state: Some(SessionState::Live), ..Default::default() })
                .map_err(|why| fault(&format!("the jobs were not read: {why:?}")))?;
            let mut jobs = Vec::new();
            for session in live {
                let hosted = store.hosting(&session.id).map_err(|why| fault(&format!("a job was not read: {why:?}")))?.is_some();
                let init = store.init_of(&session.id).map_err(|why| fault(&format!("a job was not read: {why:?}")))?;
                jobs.push((session, hosted, init));
            }
            let place = match &root {
                Some(root) => store.run_memory_place(root).ok().flatten(),
                None => None,
            };
            (jobs, place)
        };
        // The CLI's version as a Job's opening line gave it, newest Job first.
        let told = jobs.iter().filter_map(|(_, _, init)| init.as_ref()).map(|init| init.harness_version.clone()).find(|version| !version.is_empty());
        let jobs: Vec<JobFacts> = jobs.into_iter().map(|(session, hosted, init)| job_of(&session, hosted, init)).collect();

        let home = self.host().home.clone();
        let path = self.host().path.clone();
        let kit = PathBuf::from(&self.host().kit_home);
        let harness = Arc::clone(self.harness());
        let taken = self.now();
        let read = tokio::task::spawn_blocking(move || {
            let home_dir = Path::new(&home);
            let copy = crate::runtime::mod_dir(&home);
            let harness_version = told.or_else(|| harness.version_within(&path, VERSION_WITHIN));
            let install = adapters::debug_facts::install_of(&path, harness_version);
            let mod_facts = ModFacts {
                copy_filled: adapters::agent_mod::is_populated(&copy),
                copy_version: adapters::debug_facts::mod_copy_version(&copy),
                installed_version: adapters::debug_facts::mod_installed_version(home_dir),
            };
            let repository = root.map(|root| repository_of(&root, &kit, place.as_ref()));
            let taken = debug_report::to_the_second(taken.as_str());
            let trouble = adapters::debug_facts::recent_log(home_dir, &taken);
            debug_report::render(&debug_report::Facts {
                home,
                install,
                fleet: Some(FleetFacts { protocol: ipc::ProtocolId::current().to_string(), mod_facts, repository, jobs }),
                trouble,
                taken,
            })
        })
        .await
        .map_err(|_| fault("the readings did not finish"))?;
        Ok(ipc::DebugReport { text: read })
    }
}

/// Whether the repository's `armada.yml` is there and which place keeps it. **The repository's own
/// file first**, then Kit's copy where it is a file: a bucket is not asked, since a report waits on
/// nobody's network.
fn repository_of(root: &str, kit: &Path, held: Option<&store::KeptPlace>) -> RepositoryFacts {
    let name = name_of(Path::new(root));
    if Path::new(root).join("armada.yml").is_file() {
        return RepositoryFacts { name, armada_yml: "present".into(), kept_in: "repository".into() };
    }
    let (place, at) = places::where_kept(kit, held);
    let kept_in = crate::run_memory::place_word(place).to_string();
    let armada_yml = match at {
        Where::Folder(base) if places::kit_manifest(&base, root).is_file() => "present",
        Where::Folder(_) => "absent",
        Where::Bucket(..) | Where::Nowhere(_) => "not read",
    };
    RepositoryFacts { name, armada_yml: armada_yml.into(), kept_in }
}

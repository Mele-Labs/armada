//! Mock data: Fleet's own answers, written as JSON beside the crate and decoded through `ipc` as a
//! live answer is, so a fixture that no longer matches the wire fails to decode instead of drawing.
//! A screen on mock data is the same screen; only where its data comes from differs.

use std::collections::{BTreeMap, HashMap};

use std::sync::Mutex;

use ipc::{IssueList, JobThread, ModelChoices, SessionList, SessionQuick, SessionRecord, SettingValue, WorkflowChange, WorkflowChanges};
use serde::Deserialize;

/// The scenario the last [`jobs`] call named, which the message box reads its catalogue by.
static SCENARIO: Mutex<String> = Mutex::new(String::new());

/// The Jobs of a named scenario, as `GET /sessions` would answer them.
pub fn jobs(scenario: &str) -> Result<Vec<SessionRecord>, String> {
    if let Ok(mut held) = SCENARIO.lock() {
        *held = scenario.to_string();
    }
    let bytes: &[u8] = match scenario {
        "jobs-many" | "job-view" | "workflow-card" => include_bytes!("../fixtures/jobs-many.json"),
        "job-bands" => include_bytes!("../fixtures/jobs-bands.json"),
        // The same Jobs, with the first holding a plan, a sketch, a document and two drafts to read.
        "review-palette" => include_bytes!("../fixtures/review-palette.json"),
        "job-view-composer" => include_bytes!("../fixtures/jobs-composer.json"),
        // The whole Jobs tour: the review palette's Jobs, with the Judge's reading on the first and
        // the second held Job holding its three waits.
        "jobs-screens" => include_bytes!("../fixtures/jobs-screens.json"),
        other => return Err(format!("no mock scenario named {other}")),
    };
    ipc::decode::<SessionList>("SessionList", bytes).map(|list| list.sessions).map_err(|why| why.to_string())
}

/// The threads Fleet would serve for a scenario's Jobs, by Job id, as `GET /sessions/thread` answers
/// them. A Job with none stays on its skeleton, as one Fleet has not answered for does.
pub fn threads(scenario: &str) -> Result<HashMap<String, JobThread>, String> {
    #[derive(Deserialize)]
    struct Threads {
        threads: Vec<JobThread>,
    }
    let bytes: &[u8] = match scenario {
        "jobs-screens" => return screens_threads(),
        "job-view" | "review-palette" => include_bytes!("../fixtures/job-threads.json"),
        // The thread the box was walked against: no question or plan waiting, so the quick bar draws.
        "job-view-composer" => include_bytes!("../fixtures/composer-threads.json"),
        "job-bands" => include_bytes!("../fixtures/bands-threads.json"),
        // The Audit Job's thread ending on a proposed workflow's draft.
        "workflow-card" => include_bytes!("../fixtures/workflow-card-threads.json"),
        _ => return Ok(HashMap::new()),
    };
    let read = ipc::decode::<Threads>("job threads", bytes).map_err(|why| why.to_string())?;
    Ok(read.threads.into_iter().map(|thread| (thread.session_id.as_str().to_string(), thread)).collect())
}

/// The workflow changes Fleet would list for the scenario last named by [`jobs`], by change id, as
/// `GET /workflows/changes` answers them. Empty for a scenario that proposes no workflow.
pub fn workflow_changes() -> Result<BTreeMap<i64, WorkflowChange>, String> {
    let named = SCENARIO.lock().map(|held| held.clone()).unwrap_or_default();
    let bytes: &[u8] = match named.as_str() {
        "workflow-card" => include_bytes!("../fixtures/workflow-card-changes.json"),
        _ => return Ok(BTreeMap::new()),
    };
    let read = ipc::decode::<WorkflowChanges>("WorkflowChanges", bytes).map_err(|why| why.to_string())?;
    Ok(read.changes.into_iter().map(|change| (change.id, change)).collect())
}

/// The tour's threads, each from the fixture it was walked against: the first Job's question, plan and
/// cards and the long thread of the second from `job-threads`, the Audit Job's quiet thread from
/// `composer-threads` so its quick bar draws, and the held Job's from `screens-threads`.
fn screens_threads() -> Result<HashMap<String, JobThread>, String> {
    #[derive(Deserialize)]
    struct Threads {
        threads: Vec<JobThread>,
    }
    let read = |name: &'static str, bytes: &[u8]| ipc::decode::<Threads>(name, bytes).map(|read| read.threads).map_err(|why| why.to_string());
    let mut all = HashMap::new();
    for (id, thread) in read("job threads", include_bytes!("../fixtures/job-threads.json"))?.into_iter().map(|thread| (thread.session_id.as_str().to_string(), thread)) {
        if id != "mock-04" {
            all.insert(id, thread);
        }
    }
    for thread in read("composer threads", include_bytes!("../fixtures/composer-threads.json"))? {
        if thread.session_id.as_str() == "mock-04" {
            all.insert("mock-04".to_string(), thread);
        }
    }
    for thread in read("screens threads", include_bytes!("../fixtures/screens-threads.json"))? {
        all.insert(thread.session_id.as_str().to_string(), thread);
    }
    Ok(all)
}

/// What the message box reads from Fleet beside the Job: the models, the quick bar's look and
/// answers, the repository's files and its issues.
pub struct Catalogue {
    pub models: ModelChoices,
    pub look: SettingValue,
    pub quick: SessionQuick,
    pub files: Vec<String>,
    pub issues: IssueList,
    /// What a mock helper says to Talk in each Job, by Job id.
    pub talk: HashMap<String, crate::voice::Script>,
}

/// The catalogue of the scenario last named, where it has one.
pub fn catalogue() -> Option<Catalogue> {
    #[derive(Deserialize)]
    struct Files {
        paths: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Talk {
        job: String,
        #[serde(flatten)]
        script: crate::voice::Script,
    }
    #[derive(Deserialize)]
    struct Talks {
        scripts: Vec<Talk>,
    }
    let named = SCENARIO.lock().ok()?.clone();
    if named != "job-view-composer" && named != "jobs-screens" {
        return None;
    }
    let read = || -> Result<Catalogue, String> {
        let why = |why: ipc::Undecodable| why.to_string();
        Ok(Catalogue {
            models: ipc::decode("ModelChoices", include_bytes!("../fixtures/composer-models.json")).map_err(why)?,
            look: ipc::decode("thread.look", include_bytes!("../fixtures/composer-look.json")).map_err(why)?,
            quick: ipc::decode("SessionQuick", include_bytes!("../fixtures/composer-quick.json")).map_err(why)?,
            files: ipc::decode::<Files>("files", include_bytes!("../fixtures/composer-files.json")).map_err(why)?.paths,
            issues: ipc::decode("IssueList", include_bytes!("../fixtures/composer-issues.json")).map_err(why)?,
            talk: ipc::decode::<Talks>("talk", include_bytes!("../fixtures/composer-talk.json")).map_err(why)?.scripts.into_iter().map(|one| (one.job, one.script)).collect(),
        })
    };
    read().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_scenario_decodes_on_this_build_s_wire() {
        assert!(super::jobs("jobs-many").expect("decodes").len() > 10);
        assert_eq!(super::threads("job-view").expect("decodes").len(), 3);
        assert!(super::jobs("job-view-composer").expect("decodes").len() > 10);
        assert!(super::catalogue().is_some_and(|catalogue| catalogue.models.offered.len() == 3));
        assert!(super::jobs("jobs-screens").expect("decodes").len() > 10);
        let screens = super::threads("jobs-screens").expect("decodes");
        for id in ["mock-01", "mock-02", "mock-04", "mock-06"] {
            assert!(screens.contains_key(id), "{id}");
        }
        assert!(super::catalogue().is_some());
    }

    #[test]
    fn workflow_card_scenario_decodes_with_its_change() {
        assert!(super::jobs("workflow-card").expect("decodes").len() > 10);
        let threads = super::threads("workflow-card").expect("decodes");
        let changes = super::workflow_changes().expect("decodes");
        let change = changes.get(&31).expect("change 31");
        let shown = threads["mock-04"].items.iter().find_map(|item| match item {
            ipc::ThreadItem::Row { row: ipc::SessionRow::Draft { text, change: Some(31), .. } } => Some(text.clone()),
            _ => None,
        });
        // The card draws lanes only where the change drafts the text the row shows.
        assert_eq!(change.after.as_deref(), shown.as_deref());
        assert_eq!(change.after_steps.as_ref().map(Vec::len), Some(5));
    }
}

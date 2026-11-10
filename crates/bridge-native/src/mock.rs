//! Mock data: Fleet's own answers, written as JSON beside the crate and decoded through `ipc` as a
//! live answer is, so a fixture that no longer matches the wire fails to decode instead of drawing.
//! A screen on mock data is the same screen; only where its data comes from differs.

use std::collections::HashMap;

use std::sync::Mutex;

use ipc::{IssueList, JobThread, ModelChoices, SessionList, SessionQuick, SessionRecord, SettingValue};
use serde::Deserialize;

/// The scenario the last [`jobs`] call named, which the message box reads its catalogue by.
static SCENARIO: Mutex<String> = Mutex::new(String::new());

/// The Jobs of a named scenario, as `GET /sessions` would answer them.
pub fn jobs(scenario: &str) -> Result<Vec<SessionRecord>, String> {
    if let Ok(mut held) = SCENARIO.lock() {
        *held = scenario.to_string();
    }
    let bytes: &[u8] = match scenario {
        "jobs-many" | "job-view" => include_bytes!("../fixtures/jobs-many.json"),
        "job-bands" => include_bytes!("../fixtures/jobs-bands.json"),
        // The same Jobs, with the first holding a plan, a sketch, a document and two drafts to read.
        "review-palette" => include_bytes!("../fixtures/review-palette.json"),
        "job-view-composer" => include_bytes!("../fixtures/jobs-composer.json"),
        // The whole Jobs tour: the review palette's Jobs, with the Judge's reading on the first and
        // the second held Job holding its three waits.
        "jobs-screens" => include_bytes!("../fixtures/jobs-screens.json"),
        // One Job with only its plan waiting on him, as Fleet served it.
        "needs-plan" => include_bytes!("../fixtures/needs-plan.json"),
        // Five Jobs each running one kind of call after a finished one, and one whose run is over.
        "tool-rows" => include_bytes!("../fixtures/jobs-tools.json"),
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
        "needs-plan" => include_bytes!("../fixtures/needs-plan-threads.json"),
        "tool-rows" => include_bytes!("../fixtures/tools-threads.json"),
        _ => return Ok(HashMap::new()),
    };
    let read = ipc::decode::<Threads>("job threads", bytes).map_err(|why| why.to_string())?;
    Ok(read.threads.into_iter().map(|thread| (thread.session_id.as_str().to_string(), thread)).collect())
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
        assert_eq!(super::jobs("tool-rows").expect("decodes").len(), 6);
        assert_eq!(super::threads("tool-rows").expect("decodes").len(), 6);
    }
}

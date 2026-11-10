//! Mock data: Fleet's own answers, written as JSON beside the crate and decoded through `ipc` as a
//! live answer is, so a fixture that no longer matches the wire fails to decode instead of drawing.
//! A screen on mock data is the same screen; only where its data comes from differs.

use std::collections::HashMap;

use ipc::{JobThread, SessionList, SessionRecord};
use serde::Deserialize;

/// The Jobs of a named scenario, as `GET /sessions` would answer them.
pub fn jobs(scenario: &str) -> Result<Vec<SessionRecord>, String> {
    let bytes: &[u8] = match scenario {
        "jobs-many" | "job-view" => include_bytes!("../fixtures/jobs-many.json"),
        "job-bands" => include_bytes!("../fixtures/jobs-bands.json"),
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
        "job-view" => include_bytes!("../fixtures/job-threads.json"),
        "job-bands" => include_bytes!("../fixtures/bands-threads.json"),
        _ => return Ok(HashMap::new()),
    };
    let read = ipc::decode::<Threads>("job threads", bytes).map_err(|why| why.to_string())?;
    Ok(read.threads.into_iter().map(|thread| (thread.session_id.as_str().to_string(), thread)).collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_scenario_decodes_on_this_build_s_wire() {
        assert!(super::jobs("jobs-many").expect("decodes").len() > 10);
        assert_eq!(super::threads("job-view").expect("decodes").len(), 3);
    }
}

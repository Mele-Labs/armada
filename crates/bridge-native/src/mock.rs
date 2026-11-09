//! Mock data: Fleet's own answers, written as JSON beside the crate and decoded through `ipc` as a
//! live answer is, so a fixture that no longer matches the wire fails to decode instead of drawing.
//! A screen on mock data is the same screen; only where its data comes from differs.

use ipc::{SessionList, SessionRecord};

/// The Jobs of a named scenario, as `GET /sessions` would answer them.
pub fn jobs(scenario: &str) -> Result<Vec<SessionRecord>, String> {
    let bytes: &[u8] = match scenario {
        "jobs-many" => include_bytes!("../fixtures/jobs-many.json"),
        other => return Err(format!("no mock scenario named {other}")),
    };
    ipc::decode::<SessionList>("SessionList", bytes).map(|list| list.sessions).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_scenario_decodes_on_this_build_s_wire() {
        assert!(super::jobs("jobs-many").expect("decodes").len() > 10);
    }
}

//! `buildkite-build`: one Buildkite build, each command step a part, read with
//! `BUILDKITE_API_TOKEN` from the environment the watch was given.
//!
//! **A failed step says what failed**: its own log is read, and the part carries the exact error
//! lines in it and its last lines (`super::evidence`).

use std::collections::BTreeMap;
use std::time::Duration;

use adapter_traits::Secret;
use core_model::{Reading, ReadingState};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserialize;

use super::{evidence, folded, Found};

pub(super) const READER: &str = "buildkite-build";
pub(super) const TAKES: &[&str] = &["organization", "pipeline", "build"];

const API_BASE: &str = "https://api.buildkite.com/v2";
const TOKEN: &str = "BUILDKITE_API_TOKEN";
const TIMEOUT: Duration = Duration::from_secs(30);
/// The longest a refusal's own words are kept.
const SAID_AT_MOST: usize = 300;
/// The most failed steps whose logs are read, so a build with dozens failing stays one reading.
const MOST_LOGS: usize = 10;

#[derive(Deserialize)]
pub(crate) struct Build {
    pub(crate) number: Option<u64>,
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) web_url: Option<String>,
    #[serde(default)]
    pub(crate) jobs: Vec<Job>,
}

#[derive(Deserialize)]
pub(crate) struct Job {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) step_key: Option<String>,
    #[serde(default)]
    pub(crate) state: Option<String>,
    #[serde(default)]
    pub(crate) soft_failed: bool,
    #[serde(default)]
    pub(crate) retried: bool,
    #[serde(default)]
    pub(crate) web_url: Option<String>,
    #[serde(default)]
    pub(crate) log_url: Option<String>,
}

/// A step's log, as the API answers it.
#[derive(Deserialize)]
struct Log {
    #[serde(default)]
    content: String,
}

pub(super) fn read(organization: &str, pipeline: &str, build: &str) -> Result<Reading, String> {
    let token = std::env::var(TOKEN)
        .ok()
        .filter(|token| !token.trim().is_empty())
        .map(Secret::new)
        .ok_or_else(|| format!("{TOKEN} is not set in the environment this watch was given."))?;
    let client = Client::builder()
        .timeout(TIMEOUT)
        .connect_timeout(TIMEOUT)
        .build()
        .map_err(|why| why.to_string())?;
    let fetch = |url: &str| -> Result<Vec<u8>, String> {
        let response = client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", token.expose().trim()))
            .header(ACCEPT, "application/json")
            .send()
            .map_err(|why| format!("Buildkite did not answer: {why}"))?;
        let status = response.status().as_u16();
        let body = response.bytes().map_err(|why| format!("Buildkite's answer stopped arriving: {why}"))?;
        if status >= 400 {
            let said: String = String::from_utf8_lossy(&body).chars().take(SAID_AT_MOST).collect();
            return Err(format!("Buildkite answered {status}: {said}"));
        }
        Ok(body.to_vec())
    };
    let url = format!("{API_BASE}/organizations/{organization}/pipelines/{pipeline}/builds/{build}");
    let build: Build = ipc::decode("a Buildkite build", &fetch(&url)?).map_err(|why| why.to_string())?;
    // A log that will not read leaves its part with a link and no lines, never a broken reading.
    let mut logs = BTreeMap::new();
    for job in build.jobs.iter().filter(|job| job.kind == "script" && !job.retried && matches!(job.state.as_deref(), Some("failed" | "timed_out")) && !job.soft_failed).take(MOST_LOGS) {
        let (Some(id), Some(log_url)) = (&job.id, &job.log_url) else { continue };
        if let Ok(log) = fetch(log_url).and_then(|body| ipc::decode::<Log>("a Buildkite job's log", &body).map_err(|why| why.to_string())) {
            logs.insert(id.clone(), log.content);
        }
    }
    Ok(reading(build, pipeline, &logs))
}

/// What a build comes to: its command steps as parts, a retried step by its latest attempt, and
/// each failed one with the error lines and end of its log in `logs`, by job id.
pub(crate) fn reading(build: Build, pipeline: &str, logs: &BTreeMap<String, String>) -> Reading {
    let finished = matches!(build.state.as_str(), "passed" | "failed" | "canceled" | "skipped" | "not_run");
    let found: Vec<Found> = build
        .jobs
        .into_iter()
        .filter(|job| job.kind == "script" && !job.retried)
        .map(|job| {
            let state = step_state(job.state.as_deref().unwrap_or(""), job.soft_failed, finished);
            let (error, tail) = match job.id.as_ref().and_then(|id| logs.get(id)) {
                Some(log) if state == ReadingState::Failed => evidence::of_log(log),
                _ => (None, None),
            };
            Found {
                state,
                name: job.name.or(job.label).or(job.step_key).unwrap_or_else(|| String::from("a step")),
                link: job.web_url,
                error,
                tail,
            }
        })
        .collect();
    let of = match build.number {
        Some(number) => format!("{pipeline} build {number}"),
        None => format!("{pipeline}'s build"),
    };
    if found.is_empty() && finished {
        let state = match build.state.as_str() {
            "passed" => ReadingState::Passed,
            _ => ReadingState::Failed,
        };
        return Reading {
            state,
            finished,
            says: format!("{of} ended {} and ran no step.", build.state),
            link: build.web_url,
            parts: Vec::new(),
        };
    }
    let mut read = folded(found, &of, finished);
    read.link = build.web_url.or(read.link);
    read
}

/// A step that has not passed once the build is finished is failed: one the build skipped leaves
/// the old code running, so it is waiting until then, as `docs/concepts/watch.md` asks of every
/// script.
fn step_state(state: &str, soft_failed: bool, finished: bool) -> ReadingState {
    match state {
        "passed" => ReadingState::Passed,
        "failed" if soft_failed => ReadingState::Passed,
        "failed" | "timed_out" => ReadingState::Failed,
        _ if finished => ReadingState::Failed,
        "running" | "canceling" | "timing_out" | "assigned" | "accepted" => ReadingState::Running,
        _ => ReadingState::Waiting,
    }
}

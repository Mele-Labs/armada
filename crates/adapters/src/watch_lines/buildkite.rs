//! `buildkite-build`: one Buildkite build, each command step a part, read with
//! `BUILDKITE_API_TOKEN` from the environment the watch was given.

use std::time::Duration;

use adapter_traits::Secret;
use core_model::{Reading, ReadingState};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserialize;

use super::{folded, Found};

pub(super) const READER: &str = "buildkite-build";
pub(super) const TAKES: &[&str] = &["organization", "pipeline", "build"];

const API_BASE: &str = "https://api.buildkite.com/v2";
const TOKEN: &str = "BUILDKITE_API_TOKEN";
const TIMEOUT: Duration = Duration::from_secs(30);
/// The longest a refusal's own words are kept.
const SAID_AT_MOST: usize = 300;

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
    let url = format!("{API_BASE}/organizations/{organization}/pipelines/{pipeline}/builds/{build}");
    let response = client
        .get(&url)
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
    let build: Build = ipc::decode("a Buildkite build", &body).map_err(|why| why.to_string())?;
    Ok(reading(build, pipeline))
}

/// What a build comes to: its command steps as parts, a retried step by its latest attempt.
pub(crate) fn reading(build: Build, pipeline: &str) -> Reading {
    let finished = matches!(build.state.as_str(), "passed" | "failed" | "canceled" | "skipped" | "not_run");
    let found: Vec<Found> = build
        .jobs
        .into_iter()
        .filter(|job| job.kind == "script" && !job.retried)
        .map(|job| Found {
            state: step_state(job.state.as_deref().unwrap_or(""), job.soft_failed, finished),
            name: job.name.or(job.label).or(job.step_key).unwrap_or_else(|| String::from("a step")),
            link: job.web_url,
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

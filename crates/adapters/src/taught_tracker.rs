//! A tracker this build was never written for, taught by a mod's `tracker.toml`: how to list,
//! search and read over a command line or HTTP, and a `jq` filter from each answer to Armada's
//! issues. `docs/concepts/mods.md`, *Tracker mods*; `docs/concepts/tracker.md`, *Choosing a tracker*.
//!
//! [`file`] reads and checks the file without running anything, [`running`] runs what it names
//! within bounds, and [`mapping`] turns what `jq` printed into issues.

use std::sync::Arc;

use adapter_traits::{
    NotTracked, OpenIssue, Outgoing, QueryGuide, ReadIssue, RequestSender, Secret, Tracker,
};

use crate::HttpRequests;

mod file;
mod mapping;
mod running;

pub use file::{problems, read, TrackerFile, MOST_BYTES};
use file::{Call, Transport};
use running::{failed_with, Escape, Fills};

/// How many open issues one listing keeps, newest edit first, as Jira's.
const LISTED: usize = 200;

/// How many issues one search keeps, as the trait says.
const SEARCHED: usize = 100;

/// The tracker mods on this machine, as a provider name this build has no tracker for is looked
/// up in. **Implemented where the mods are read**, so what counts as valid and tried is said once.
pub trait TaughtTrackers: Send + Sync {
    /// `None` where no mod is called `name`. The file where that mod is a tracker that is valid and
    /// passed a try of the file as it now stands; why it cannot be used where not.
    fn taught(&self, name: &str) -> Option<Result<TrackerFile, String>>;
}

/// A tracker read from its file.
pub struct TaughtTracker {
    file: TrackerFile,
}

impl TaughtTracker {
    pub fn over(file: TrackerFile) -> TaughtTracker {
        TaughtTracker { file }
    }

    /// The rows `call` answers with, filled from `fills`. **Every refusal is scrubbed of the
    /// credential**, which is read here, for this call, and dropped with it.
    fn rows(&self, call: &Call, asked: Fills<'_>, doing: &'static str) -> Result<Option<Vec<String>>, NotTracked> {
        let credential = self.credential().map_err(|why| NotTracked::of(doing, why))?;
        let fills = Fills { query: asked.query, reference: asked.reference, credential: credential.as_ref() };
        let answer = match &call.transport {
            Transport::Cli { argv } => {
                let filled: Vec<String> = argv.iter().map(|one| fills.fill(one, Escape::Plain)).collect();
                let ran = running::run(&filled, None).map_err(|why| NotTracked::of(doing, fills.scrubbed(&why)))?;
                if !ran.succeeded {
                    return Err(NotTracked::of(doing, fills.scrubbed(&failed_with(&ran))));
                }
                ran.stdout
            }
            Transport::Http { method, url, headers, body } => {
                let outgoing = Outgoing {
                    method: method.clone(),
                    url: fills.fill(url, Escape::Url),
                    headers: headers.iter().map(|(name, value)| (name.clone(), fills.fill(value, Escape::Plain))).collect(),
                    body: body.as_deref().map(|body| fills.fill(body, Escape::Json)),
                };
                let came = HttpRequests::within(running::WITHIN)
                    .send(outgoing, running::MOST_OUTPUT)
                    .map_err(|why| NotTracked::of(doing, fills.scrubbed(&why.said())))?;
                if came.status == 404 && fills.reference.is_some() {
                    return Ok(None);
                }
                if came.status >= 400 {
                    let said = String::from_utf8_lossy(&came.body);
                    return Err(NotTracked::of(doing, fills.scrubbed(&format!("it answered {}: {said}", came.status))));
                }
                if came.truncated {
                    return Err(NotTracked::of(doing, format!("the answer was larger than {} bytes", running::MOST_OUTPUT)));
                }
                came.body
            }
        };
        let jq = ["jq".to_string(), "-r".to_string(), mapping::program(&call.map)];
        let mapped = running::run(&jq, Some(&answer)).map_err(|why| NotTracked::of(doing, fills.scrubbed(&why)))?;
        if !mapped.succeeded {
            return Err(NotTracked::of(doing, fills.scrubbed(&mapping::refusal_of(&mapped.stderr))));
        }
        let text = String::from_utf8_lossy(&mapped.stdout);
        Ok(Some(text.lines().filter(|line| !line.is_empty()).map(str::to_string).collect()))
    }

    /// The secret the credential command prints, or `None` where the file names none.
    fn credential(&self) -> Result<Option<Secret<String>>, String> {
        let Some(argv) = &self.file.credential else { return Ok(None) };
        let ran = running::run(argv, None).map_err(|why| format!("the credential was not read: {why}"))?;
        if !ran.succeeded {
            return Err(format!("the credential was not read: {}", failed_with(&ran)));
        }
        let secret = String::from_utf8(ran.stdout).map_err(|_| "the credential command printed something not UTF-8".to_string())?;
        let secret = secret.trim_end_matches(['\n', '\r']).to_string();
        match secret.is_empty() {
            true => Err("the credential command printed nothing".to_string()),
            false => Ok(Some(Secret::new(secret))),
        }
    }

    fn issues(&self, call: &Call, query: &str, doing: &'static str, most: usize) -> Result<Vec<OpenIssue>, NotTracked> {
        let fills = Fills { query: Some(query), ..Fills::default() };
        let rows = self.rows(call, fills, doing)?.unwrap_or_default();
        let mut issues: Vec<OpenIssue> = rows.iter().filter_map(|line| mapping::open_issue_of(line)).collect();
        // RFC 3339 in one zone sorts as text; a tracker that wrote no time keeps its own order.
        issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        issues.truncate(most);
        Ok(issues)
    }
}

impl Tracker for TaughtTracker {
    fn open_issues(&self, _in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        let doing = "listing the open issues";
        let (call, query) = match &self.file.list {
            Some(list) => (list, ""),
            None => (&self.file.search, self.file.guide.open.as_str()),
        };
        let mut issues = self.issues(call, query, doing, usize::MAX)?;
        issues.retain(|one| !one.closed);
        issues.truncate(LISTED);
        Ok(issues)
    }

    fn read_issue(&self, _in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Ok(None);
        }
        let fills = Fills { reference: Some(reference), ..Fills::default() };
        let rows = self.rows(&self.file.read, fills, "reading the issue")?;
        Ok(rows.and_then(|rows| rows.first().and_then(|line| mapping::read_issue_of(line))))
    }

    fn search_issues(&self, _in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        self.issues(&self.file.search, query, "searching the issues", SEARCHED)
    }

    fn query_guide(&self) -> QueryGuide {
        self.file.guide.clone()
    }
}

/// A Manifest's provider that names a tracker mod, looked up at each call: **on the blocking
/// thread the call runs on**, and as the mod stands then, so a try that passes after Fleet
/// started is honoured without a restart.
pub(crate) struct Taught {
    pub(crate) name: String,
    pub(crate) mods: Arc<dyn TaughtTrackers>,
    /// What to say where no mod has the name either.
    pub(crate) unknown: String,
}

impl Taught {
    fn tracker(&self, doing: &'static str) -> Result<TaughtTracker, NotTracked> {
        match self.mods.taught(&self.name) {
            Some(Ok(file)) => Ok(TaughtTracker::over(file)),
            Some(Err(why)) => Err(NotTracked::of(doing, why)),
            None => Err(NotTracked::of(doing, self.unknown.clone())),
        }
    }
}

impl Tracker for Taught {
    fn open_issues(&self, in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        self.tracker("listing the open issues")?.open_issues(in_repo)
    }

    fn read_issue(&self, in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        self.tracker("reading the issue")?.read_issue(in_repo, reference)
    }

    fn search_issues(&self, in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        self.tracker("searching the issues")?.search_issues(in_repo, query)
    }

    fn query_guide(&self) -> QueryGuide {
        match self.tracker("teaching the query language") {
            Ok(tracker) => tracker.query_guide(),
            Err(not) => QueryGuide { guide: not.said, ..QueryGuide::default() },
        }
    }
}

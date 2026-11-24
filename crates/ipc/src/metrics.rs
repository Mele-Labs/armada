//! A project's live metrics: the recipe the librarian writes once, and what Fleet read with it. `docs/practices/protocol/projects.md`,
//! *Metrics*.
//!
//! **A recipe is run with no model.** It names one call, over a command line, HTTP or an MCP server, as a tracker mod's
//! call does, and a `jq` filter from the answer to rows. A block on the board shows one recipe, as kind `metric:<id>`.
//!
//! **A token is never on the wire's way out.** A recipe names the credential it needs; the token itself crosses once,
//! in `SaveMetricToken`, into this Mac's keychain, and no answer carries it.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// How often a recipe is read while its dashboard is open, where it names none.
pub const METRIC_EVERY_SECS: u32 = 60;

/// One call, as a tracker mod's call is written. `{credential}` in a field is filled with the token, and only in the
/// fields that take it: `env` of a cli or an mcp call, `arguments`, `headers` and `body`, never `argv`, `server` or `url`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum MetricCall {
    /// A program and its arguments, run with no shell, and the environment variables set on it. A token is passed in `env`,
    /// which a process list does not show, and never in `argv`.
    Cli {
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env: Vec<MetricHeader>,
    },
    Http {
        method: String,
        url: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        headers: Vec<MetricHeader>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
    },
    /// A server started on standard input and output for the call, and one tool called on it. `arguments` is JSON text.
    Mcp {
        server: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env: Vec<MetricHeader>,
        tool: String,
        #[serde(default = "no_arguments")]
        arguments: String,
    },
}

fn no_arguments() -> String {
    "{}".to_string()
}

/// A header of an HTTP call, or a variable of an MCP server's environment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricHeader {
    pub name: String,
    pub value: String,
}

/// The token a recipe signs in with. **One per tool on each Mac**: two recipes naming `sentry` share one token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricCredential {
    /// The tool, as the keychain item is named: `sentry`, `grafana`.
    pub name: String,
    /// What to make, in a sentence: "A Sentry auth token with read access".
    pub how: String,
    /// Where it is made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// What the librarian writes once. `map` is a `jq` filter whose output reads as [`MetricAnswer`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricRecipe {
    /// Unique in the project, lower case with dashes. The block that shows it is `metric:<id>`.
    pub id: String,
    pub title: String,
    pub call: MetricCall,
    pub map: String,
    /// How long a reading stands before Bridge's next ask fetches again. Absent is [`METRIC_EVERY_SECS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every_secs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<MetricCredential>,
}

impl MetricRecipe {
    /// The board block kind that shows this recipe.
    pub fn block_kind(&self) -> String {
        format!("metric:{}", self.id)
    }

    pub fn every_secs(&self) -> u32 {
        self.every_secs.unwrap_or(METRIC_EVERY_SECS)
    }
}

/// A value of a row: a number Graph can draw, or words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Number(f64),
    Text(String),
}

/// One row of an answer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricRow {
    pub label: String,
    pub value: MetricValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// When the tool says the value stood. Rows that carry it are a series, and Graph draws them in time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

/// What `map` prints, read as one JSON object.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MetricAnswer {
    pub rows: Vec<MetricRow>,
    /// An image the tool renders, such as a panel. **Picture is offered only where an answer gives one.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
}

/// One kept value of the first numeric row, from a reading Fleet made while the dashboard was open.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricPoint {
    pub at: Instant,
    pub value: f64,
}

/// Where a recipe stands on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MetricState {
    /// The last fetch read.
    Read,
    /// The recipe names a credential this Mac's keychain does not hold.
    NoToken { credential: MetricCredential },
    /// The call or the map failed. `why` is scrubbed of the token.
    Broken { why: String },
}

/// One recipe's reading, as Bridge draws its block.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricReading {
    pub id: String,
    pub title: String,
    #[serde(flatten)]
    pub state: MetricState,
    /// The last answer that read, kept through a broken fetch. Absent where none ever read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<MetricAnswer>,
    /// When `answer` was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at: Option<Instant>,
    /// The values kept from this Fleet's readings, oldest first, for Graph where the answer is not a series.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kept: Vec<MetricPoint>,
}

/// `read_metrics`' answer: a reading for each recipe the project keeps, in the file's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricReadings {
    pub project: String,
    pub readings: Vec<MetricReading>,
}

/// `try_metric` and `save_metric`: a recipe to run once, or to run and keep.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TryMetric {
    pub recipe: MetricRecipe,
}

/// `save_metric`'s answer: the recipe as kept and its first reading. **A recipe that cannot read is not kept**, unless
/// the only thing missing is this Mac's token.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricSaved {
    pub project: String,
    pub recipe: MetricRecipe,
    pub reading: MetricReading,
}

/// `save_metric_token`: a token for one credential name, into this Mac's keychain. **`Debug` leaves the token out**, so a
/// request that is logged or a test that fails does not print it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMetricToken {
    pub credential: String,
    pub token: String,
}

impl std::fmt::Debug for SaveMetricToken {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("SaveMetricToken").field("credential", &self.credential).finish_non_exhaustive()
    }
}

//! A project's live metric recipes, as its file keeps them under `look.metrics`. `docs/practices/protocol/projects.md`,
//! *Metrics*.
//!
//! **Plain words, kept as written.** What a recipe is for, and how it is run, is `adapters::metrics`'s and Fleet's;
//! this is what the file holds, so the Project's layer can carry it without a wire type.

use alloc::string::String;
use alloc::vec::Vec;

/// One call, as a tracker mod's call is written. `{credential}` is filled into `env`, `arguments`, `headers` and
/// `body` only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricCall {
    Cli { argv: Vec<String> },
    Http { method: String, url: String, headers: Vec<(String, String)>, body: Option<String> },
    /// `arguments` is JSON text.
    Mcp { server: Vec<String>, env: Vec<(String, String)>, tool: String, arguments: String },
}

/// The token a recipe signs in with, named by the tool: two recipes naming `sentry` share one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricCredential {
    pub name: String,
    pub how: String,
    pub url: Option<String>,
}

/// What the librarian wrote once. `map` is a `jq` filter from the call's answer to rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricRecipe {
    pub id: String,
    pub title: String,
    pub call: MetricCall,
    pub map: String,
    pub every_secs: Option<u32>,
    pub credential: Option<MetricCredential>,
}

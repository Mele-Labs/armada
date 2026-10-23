//! Reading the link a request names, so the request carries what the page says.
//!
//! **Fleet's own process, never a Drone's**: it is unsandboxed and already has a network, which
//! is the whole reason this call exists instead of asking an agent to reach the same link from
//! inside a sandbox that has neither.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{LinkLookup, LookupCall};
use tokio::process::Command;
use tokio::time::timeout;

/// How long a request's own link may take to resolve before the request goes through
/// unresolved. The failure this waits out is a slow network, not a model that will not answer.
pub(crate) const LOOKUP_BUDGET: Duration = Duration::from_secs(10);

/// What became of the request's own link, once it was looked at.
pub(crate) enum Enriched {
    /// Nothing in the request named a link this could resolve.
    AsGiven,
    /// Resolved, and appended: the URL is never replaced, only added to.
    Resolved(String),
    /// Named a link, and resolving it did not work. The request still goes through exactly as
    /// written; only the log says why nothing was added.
    Failed,
}

/// Look once for a link in `request` and answer with what came of it. **A link read by the
/// lookup itself is read off this thread**, where it may wait on the network; one it renders as
/// a call is run here.
pub(crate) async fn enrich(request: &str, links: Arc<dyn LinkLookup + Send + Sync>) -> Enriched {
    let read = {
        let (links, request) = (Arc::clone(&links), request.to_string());
        tokio::task::spawn_blocking(move || links.read_link(&request)).await.ok().flatten()
    };
    if let Some(read) = read {
        return match read {
            Ok(text) => Enriched::Resolved(text),
            Err(_) => Enriched::Failed,
        };
    }
    match links.resolve(request) {
        None => Enriched::AsGiven,
        Some(call) => match resolved(&call).await {
            Ok(text) => Enriched::Resolved(text),
            Err(_) => Enriched::Failed,
        },
    }
}

/// Run one rendered lookup and answer with what it printed.
pub(crate) async fn resolved(call: &LookupCall) -> Result<String, String> {
    let mut spawning = Command::new(call.program());
    spawning
        .args(call.args())
        .stdin(std::process::Stdio::null())
        // A call that outran its budget is dropped, not left running.
        .kill_on_drop(true);
    let output = timeout(LOOKUP_BUDGET, spawning.output())
        .await
        .map_err(|_| "the lookup took too long".to_string())?
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(match output.status.code() {
            Some(code) => format!("the lookup exited {code}"),
            None => "the lookup was killed".to_string(),
        });
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match text.is_empty() {
        true => Err("the lookup said nothing".to_string()),
        false => Ok(text),
    }
}

#[cfg(test)]
mod tests {
    use adapter_traits::IssueAddress;

    use super::*;

    /// A lookup that reads links itself as told, and renders no call.
    struct Reading(Option<Result<String, String>>);

    impl LinkLookup for Reading {
        fn resolve(&self, _request: &str) -> Option<LookupCall> {
            None
        }

        fn read_link(&self, _request: &str) -> Option<Result<String, String>> {
            self.0.clone()
        }

        fn issue(&self, _request: &str) -> Option<IssueAddress> {
            None
        }

        fn edited(&self, _issue: &IssueAddress) -> Option<LookupCall> {
            None
        }
    }

    #[tokio::test]
    async fn a_link_the_lookup_reads_itself_is_resolved_or_failed_by_what_it_said() {
        let said = Arc::new(Reading(Some(Ok("Title\n\nBody".to_string()))));
        assert!(matches!(enrich("a link", said).await, Enriched::Resolved(text) if text == "Title\n\nBody"));
        let refused = Arc::new(Reading(Some(Err("it would not answer".to_string()))));
        assert!(matches!(enrich("a link", refused).await, Enriched::Failed));
    }

    #[tokio::test]
    async fn a_lookup_that_reads_nothing_itself_and_renders_no_call_leaves_the_request_as_given() {
        assert!(matches!(enrich("a link", Arc::new(Reading(None))).await, Enriched::AsGiven));
    }
}

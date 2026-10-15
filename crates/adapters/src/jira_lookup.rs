//! Resolving a bare Jira link, for the site a Manifest's `tracker:` names.
//! `crate::issue_lookup`'s counterpart; `docs/concepts/tracker.md`, *Jira*.
//!
//! **One shape**: `https://<site>/browse/KEY-123`, with or without the scheme.
//! The call is a shell pipeline because a `LookupCall` is one process and the
//! answer needs `jq` after `acli`. The key and the filter travel as positional
//! parameters, so neither is ever spliced into the script.

use adapter_traits::{IssueAddress, LinkLookup, LookupCall};

use crate::jira::{self, JiraSettings};

/// `acli` fails loudly, `jq` on nothing does not: a failed read stops here
/// rather than resolving to an empty page.
const VIEW_THROUGH_JQ: &str = r#"out=$(acli jira workitem view "$1" --fields "$3" --json) || exit $?; printf '%s' "$out" | jq -r "$2""#;

pub(crate) struct JiraLookup {
    settings: JiraSettings,
}

impl JiraLookup {
    pub(crate) fn over(settings: JiraSettings) -> JiraLookup {
        JiraLookup { settings }
    }

    fn call(key: &str, filter: String, fields: &str) -> LookupCall {
        LookupCall::rendered(
            "sh",
            vec![
                "-c".into(),
                VIEW_THROUGH_JQ.into(),
                "sh".into(),
                key.into(),
                filter,
                fields.into(),
            ],
        )
    }
}

impl LinkLookup for JiraLookup {
    fn resolve(&self, request: &str) -> Option<LookupCall> {
        let key = self.settings.key_in(request)?;
        Some(JiraLookup::call(&key, jira::text_filter(), "summary,description"))
    }

    /// The key as the reference, and the canonical address, never the link as typed.
    fn issue(&self, request: &str) -> Option<IssueAddress> {
        let key = self.settings.key_in(request)?;
        Some(IssueAddress::at(&key, &self.settings.browse(&key)))
    }

    fn edited(&self, issue: &IssueAddress) -> Option<LookupCall> {
        let key = self.settings.key_in(issue.url())?;
        Some(JiraLookup::call(&key, jira::edited_filter(), "updated"))
    }
}

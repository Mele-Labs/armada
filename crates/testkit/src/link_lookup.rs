//! A [`LinkLookup`] scripted by a test, standing in for whatever real one
//! `fleet` is assembled with.
//!
//! No link shape lives here, deliberately: recognising a real one is
//! `adapters`' job, tested there against real link text. A fixture that
//! matched real links would be asserting `fleet`'s own runner against a
//! shape only `adapters` is supposed to know.

use std::sync::Mutex;

use adapter_traits::{IssueAddress, LinkLookup, LookupCall};

/// Matches every request against one fragment and answers with one script.
///
/// `prints`, when given, rides on the argument list rather than inside the
/// script text — the same reason [`crate::FakeJudge`] never interpolates an
/// answer into its shell string: a fixture's text should not have to be shell
/// quoting-safe to be usable here.
#[derive(Debug)]
pub struct FakeLinkLookup {
    fragment: Option<&'static str>,
    script: &'static str,
    prints: Option<String>,
    calls: Mutex<usize>,
    /// The issue a matching request links, and when it was last edited.
    issue: Option<IssueAddress>,
    edited_at: Mutex<Option<String>>,
}

impl FakeLinkLookup {
    /// Resolves nothing. What most fixtures want: the request the test wrote
    /// carries no link at all, and this is the fake saying so.
    pub fn resolving_nothing() -> FakeLinkLookup {
        FakeLinkLookup {
            fragment: None,
            script: "",
            prints: None,
            calls: Mutex::new(0),
            issue: None,
            edited_at: Mutex::new(None),
        }
    }

    /// Any request containing `fragment` resolves to a call that prints
    /// `text` on success.
    pub fn resolving(fragment: &'static str, text: &str) -> FakeLinkLookup {
        FakeLinkLookup {
            fragment: Some(fragment),
            script: "printf %s \"$0\"",
            prints: Some(text.to_string()),
            calls: Mutex::new(0),
            issue: None,
            edited_at: Mutex::new(None),
        }
    }

    /// Any request containing `fragment` resolves to a call that fails —
    /// the network blip, the private repository, the deleted issue.
    pub fn failing_to_resolve(fragment: &'static str) -> FakeLinkLookup {
        FakeLinkLookup {
            fragment: Some(fragment),
            script: "exit 7",
            prints: None,
            calls: Mutex::new(0),
            issue: None,
            edited_at: Mutex::new(None),
        }
    }

    /// How many times `resolve` matched and rendered a call. What a test
    /// asserts to say the fetch ran exactly once per request, not once per
    /// Job a multi-Job plan mints from it.
    pub fn resolved_count(&self) -> usize {
        *self.calls.lock().expect("not poisoned")
    }

    /// The same fake, saying a matching request links this issue. Spike 022,
    /// slice 4.
    pub fn linking(mut self, reference: &str, url: &str) -> FakeLinkLookup {
        self.issue = Some(IssueAddress::at(reference, url));
        self
    }

    /// From now on the issue reads as last edited at `instant`, as the forge
    /// would answer after somebody edits it.
    pub fn edit_issue_at(&self, instant: &str) {
        *self.edited_at.lock().expect("not poisoned") = Some(instant.to_string());
    }
}

impl LinkLookup for FakeLinkLookup {
    fn resolve(&self, request: &str) -> Option<LookupCall> {
        let fragment = self.fragment?;
        if !request.contains(fragment) {
            return None;
        }
        *self.calls.lock().expect("not poisoned") += 1;
        let mut args = vec!["-c".to_string(), self.script.to_string()];
        if let Some(text) = &self.prints {
            args.push(text.clone());
        }
        Some(LookupCall::rendered("/bin/sh", args))
    }

    fn issue(&self, request: &str) -> Option<IssueAddress> {
        let fragment = self.fragment?;
        request.contains(fragment).then(|| self.issue.clone())?
    }

    /// Prints the instant [`edit_issue_at`](FakeLinkLookup::edit_issue_at)
    /// last set, or fails where nobody set one — a forge that will not say.
    fn edited(&self, issue: &IssueAddress) -> Option<LookupCall> {
        if self.issue.as_ref() != Some(issue) {
            return None;
        }
        let args = match self.edited_at.lock().expect("not poisoned").clone() {
            Some(instant) => vec!["-c".to_string(), "printf %s \"$0\"".to_string(), instant],
            None => vec!["-c".to_string(), "exit 7".to_string()],
        };
        Some(LookupCall::rendered("/bin/sh", args))
    }
}

//! What the fake forge does with a form's edit to `armada.yml`. Its own file for
//! `merging`'s reason: it is scripted through `&self`, because nobody saves a
//! form until the Fleet holding this fake exists.

use adapter_traits::{
    ManifestEditSent, ManifestEditToSend, ManifestOnTheBase, NotDelivered, PullRequestFor,
};

use super::FakeVcs;

/// What the fake answers about edits, and every one it was sent.
#[derive(Debug, Default)]
pub(super) struct ManifestEdits {
    /// Whether the Manifest is tracked and not ignored. **No by default**, so a
    /// case that does not script it writes to disk as every case did before.
    pub(super) by_pull_request: bool,
    /// What the base holds. Absent is a base the fake cannot read.
    pub(super) on_the_base: Option<ManifestOnTheBase>,
    pub(super) sent: Vec<ManifestEditToSend>,
}

impl FakeVcs {
    /// Say that the Manifest is a tracked file, so an edit goes out as a pull
    /// request, or that it is not.
    pub fn now_manifest_goes_by_pull_request(&self, yes: bool) {
        self.manifest_edits.lock().expect("not poisoned").by_pull_request = yes;
    }

    /// Say what the base branch holds of the Manifest, and the commit that holds it.
    pub fn now_base_holds_manifest(&self, commit: &str, text: &str) {
        self.manifest_edits.lock().expect("not poisoned").on_the_base = Some(ManifestOnTheBase {
            commit: commit.to_string(),
            text: text.to_string(),
        });
    }

    /// Every edit sent, in order. **A pull request is one that is `Open`**, so
    /// a test counting the opened ones counts those.
    pub fn manifest_edits_sent(&self) -> Vec<ManifestEditToSend> {
        self.manifest_edits.lock().expect("not poisoned").sent.clone()
    }

    pub(super) fn manifest_goes_by_pull_request(&self) -> bool {
        self.manifest_edits.lock().expect("not poisoned").by_pull_request
    }

    pub(super) fn manifest_on_the_base_is(&self) -> Result<ManifestOnTheBase, NotDelivered> {
        self.manifest_edits
            .lock()
            .expect("not poisoned")
            .on_the_base
            .clone()
            .ok_or_else(|| NotDelivered::of("reading the Manifest on the base", "nothing scripted"))
    }

    /// The commit is numbered by how many were sent, so a second edit's parent
    /// can be told from the first's.
    pub(super) fn manifest_edit_sent(
        &self,
        sending: &ManifestEditToSend,
    ) -> Result<ManifestEditSent, NotDelivered> {
        let number = {
            let mut edits = self.manifest_edits.lock().expect("not poisoned");
            edits.sent.push(sending.clone());
            edits.sent.len()
        };
        let (pull_request, auto_merge) = match &sending.to {
            PullRequestFor::Open { .. } => {
                let url = String::from("https://forge.invalid/armada/pull/1");
                let auto_merge = self.auto_merge_asked(&url);
                (url, auto_merge)
            }
            PullRequestFor::Update { pull_request, .. } => (pull_request.clone(), Ok(())),
        };
        Ok(ManifestEditSent {
            commit: format!("{number:040x}"),
            pull_request,
            auto_merge,
        })
    }
}

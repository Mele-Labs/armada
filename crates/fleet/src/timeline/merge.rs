//! A pull request's moves and main's colour, read off two consecutive reads of
//! the merge lines.
//!
//! **There is no event to hear here**: `merge_lines.changed` carries the list
//! whole, so what happened is what differs between the read before and this
//! one. The first read is only a baseline, because everything on it was there
//! already. A pull request is dated by the read that saw it, apart from a merge,
//! which the forge dates itself.

use std::collections::BTreeSet;

use ipc::{
    HubPullCi, HubQueueState, MainCiState, MergeLineHub, MergeLines, TimelineKind, TimelineTone,
};

use super::{Fact, Subject};

/// The last merge lines read, and which repositories' main stood red.
#[derive(Default)]
pub(crate) struct Between {
    last: Option<MergeLines>,
    /// Roots whose main read red and has not read green since. **A running
    /// commit between the two does not clear it**: red to running to green is
    /// one repair, and it is green that ends it.
    red: BTreeSet<String>,
}

impl Between {
    /// Take this read and answer what it shows happened since the last,
    /// `manifests` naming each root's repository and `at` dating what the forge
    /// did not.
    pub(crate) fn read(
        &mut self,
        now: &MergeLines,
        at: &str,
        manifests: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<Fact> {
        let mut facts = Vec::new();
        for line in &now.lines {
            let Some(hub) = &line.hub else { continue };
            let manifest = manifests(&line.root);
            let before = self
                .last
                .as_ref()
                .map(|last| last.lines.iter().find(|one| one.root == line.root).and_then(|one| one.hub.as_ref()));
            if let Some(Some(before)) = before {
                facts.extend(pulls(before, hub, at, &manifest));
            }
            if let (Some(_), Some(main)) = (&self.last, &hub.main) {
                facts.extend(self.main(&line.root, main, &manifest));
            }
            // Seen on the baseline read as well, so a main already red when
            // Fleet started ends with a green.
            match hub.main.as_ref().map(|main| main.state) {
                Some(MainCiState::Red) => {
                    self.red.insert(line.root.clone());
                }
                Some(MainCiState::Green) => {
                    self.red.remove(&line.root);
                }
                _ => {}
            }
        }
        self.last = Some(now.clone());
        facts
    }

    fn main(&self, root: &str, main: &ipc::MainStanding, manifest: &Option<String>) -> Option<Fact> {
        let was_red = self.red.contains(root);
        let fact = |at: &str, kind, tone, says: &str| {
            Fact::new(at, kind, Subject::Main, tone, says).in_repository(manifest.clone())
        };
        match main.state {
            MainCiState::Red if !was_red => {
                let since = main.red_since.as_ref().unwrap_or(&main.read_at);
                Some(
                    fact(since.as_str(), TimelineKind::MainRed, TimelineTone::Bad, "Main red")
                        .detail(main.failed.first().map(|job| job.name.clone())),
                )
            }
            MainCiState::Green if was_red => Some(fact(
                main.read_at.as_str(),
                TimelineKind::MainGreen,
                TimelineTone::Ok,
                "Main green",
            )),
            _ => None,
        }
    }
}

/// What happened to the pull requests between two reads of one repository's hub.
fn pulls(before: &MergeLineHub, now: &MergeLineHub, at: &str, manifest: &Option<String>) -> Vec<Fact> {
    use TimelineKind as K;
    use TimelineTone as T;
    let pull = |number: u64, url: &str, title: &str, at: &str, kind, tone, says: &str| {
        Fact::new(at, kind, Subject::Pull(number, Some(url.to_string())), tone, says)
            .titled(Some(title.to_string()))
            .in_repository(manifest.clone())
    };
    let mut facts = Vec::new();
    // Another author's pull request is not the person's to follow, in the queue or out of it.
    for one in now.pull_requests.iter().filter(|one| one.mine) {
        let on = |kind, tone, says: &str| pull(one.number, &one.url, &one.title, at, kind, tone, says);
        let was = before.pull_requests.iter().find(|was| was.number == one.number);
        if was.is_none() {
            if now.merged.iter().chain(&before.merged).any(|done| done.number == one.number) {
                continue;
            }
            facts.push(on(K::PrOpened, T::Idle, "Opened").detail(Some(one.branch.clone())));
        }
        // A pull request first seen is compared with nothing, so checks already running and auto-merge
        // already set when it opened are entered with it: they will not change again to be seen.
        if was.map(|was| &was.ci) != Some(&one.ci) {
            facts.extend(one.ci.map(|ci| match ci {
                HubPullCi::Passed => on(K::PrChecks, T::Ok, "Checks passed"),
                HubPullCi::Running => on(K::PrChecks, T::Run, "Checks running").live(),
                HubPullCi::Failed => on(K::PrChecks, T::Bad, "Checks failed"),
                HubPullCi::WaitingOnMain => on(K::PrChecks, T::Idle, "Waiting on main"),
            }));
        }
        if was.map(|was| &was.queue) != Some(&one.queue) {
            facts.extend(one.queue.map(|queue| match (queue.state, queue.position) {
                (HubQueueState::InQueue | HubQueueState::Queued, Some(place)) => {
                    on(K::PrQueued, T::Idle, &format!("Queued, place {place}"))
                }
                (HubQueueState::InQueue | HubQueueState::Queued, None) => on(K::PrQueued, T::Idle, "Queued"),
                (HubQueueState::WaitingForCi, _) => on(K::PrQueued, T::Idle, "Auto-merge waits on checks"),
                (HubQueueState::AwaitingChecks, _) => on(K::PrQueued, T::Run, "Queue checks running").live(),
                (HubQueueState::Mergeable, _) => on(K::PrQueued, T::Idle, "Mergeable"),
                (HubQueueState::Unmergeable, _) => on(K::PrQueued, T::Bad, "Cannot merge"),
            }));
        }
    }
    for done in &now.merged {
        let new = !before.merged.iter().any(|was| was.number == done.number);
        // A merge seen with nothing before it to compare is a hub that came back
        // empty, not forty pull requests merging at once.
        let was_open = before.pull_requests.iter().any(|was| was.number == done.number);
        if new && (was_open || !before.merged.is_empty()) {
            facts.push(
                pull(done.number, &done.url, &done.title, done.merged_at.as_str(), K::PrMerged, T::Ok, "Merged")
                    .detail(Some(done.branch.clone()))
                    .over(),
            );
        }
    }
    facts
}

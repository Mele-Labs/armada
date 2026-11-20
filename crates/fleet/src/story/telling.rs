//! A Job's story: its asks, open ones first, with what he was shown and decided, what was built and what was handed on.
//! `docs/concepts/job-story.md`, *A Job's story*.
//!
//! **An artifact goes to the ask he last spoke on before it was first kept.** Fleet records no message an artifact answers,
//! so the time is the one fact there is; what came before any of his asks stands in `shown_otherwise`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Ask, AskState, Instant, JobArtifact, JobStory, SessionId, StoryAsk, StoryBuilt, StoryHanded, StoryShown};

use crate::daemon::Fleet;

/// What the story is built from, an ask at a time.
pub(crate) struct Entry {
    pub ask: Ask,
    pub built: Vec<StoryBuilt>,
    pub handed: Vec<StoryHanded>,
}

fn open(state: AskState) -> bool {
    !matches!(state, AskState::Built | AskState::Dropped)
}

fn shown_of(artifact: &JobArtifact) -> StoryShown {
    StoryShown {
        artifact: artifact.id.clone(),
        title: artifact.title.clone(),
        form: artifact.form,
        state: artifact.state,
        words: artifact.words.clone(),
        votes: artifact.votes.clone(),
        presses: artifact.presses.clone(),
        shown_at: artifact.revisions.first().map(|one| one.at.clone()).unwrap_or_else(|| Instant::carried("")),
    }
}

/// Where an ask stands, in the plain words a line of the done card carries.
fn standing(entry: &Entry) -> String {
    match entry.ask.state {
        AskState::Backlog => String::from("backlog"),
        AskState::Building => String::from("being built"),
        AskState::Built => match entry.built.last() {
            Some(merge) => format!("built, pull request {}", merge.pull_request),
            None => String::from("built"),
        },
        AskState::Handed => match entry.handed.last() {
            Some(hand) => format!("handed to {}", hand.to_title.clone().unwrap_or_else(|| hand.to.as_str().to_string())),
            None => String::from("handed"),
        },
        AskState::Dropped => String::from("dropped"),
    }
}

/// The story of `session`: the asks it holds, grouped, with each artifact under the ask he last spoke on. Pure.
pub(crate) fn story_of(session: &SessionId, mut entries: Vec<Entry>, artifacts: &[JobArtifact]) -> JobStory {
    entries.sort_by(|a, b| (!open(a.ask.state), a.ask.opened_at.as_str(), &a.ask.ask_id).cmp(&(!open(b.ask.state), b.ask.opened_at.as_str(), &b.ask.ask_id)));
    // When he spoke on each ask in this Job: its first words and each revision or answer written here.
    let spoke: Vec<Vec<&str>> = entries
        .iter()
        .map(|entry| {
            let first = (&entry.ask.said_in == session).then(|| entry.ask.opened_at.as_str());
            let later = entry.ask.said.iter().filter(|one| &one.session_id == session).map(|one| one.at.as_str());
            first.into_iter().chain(later).collect()
        })
        .collect();
    let mut shown: Vec<Vec<StoryShown>> = entries.iter().map(|_| Vec::new()).collect();
    let mut otherwise = Vec::new();
    let mut each: Vec<StoryShown> = artifacts.iter().map(shown_of).collect();
    each.sort_by(|a, b| a.shown_at.as_str().cmp(b.shown_at.as_str()));
    for one in each {
        let owner = spoke
            .iter()
            .enumerate()
            .filter_map(|(at, times)| times.iter().copied().filter(|time| *time <= one.shown_at.as_str()).max().map(|time| (time, at)))
            .max_by(|a, b| a.0.cmp(b.0).then(b.1.cmp(&a.1)))
            .map(|(_, at)| at);
        match owner {
            Some(at) => shown[at].push(one),
            None => otherwise.push(one),
        }
    }
    let lead = entries.iter().map(|entry| format!("{}: {}", entry.ask.title, standing(entry))).collect();
    let asks = entries
        .into_iter()
        .zip(shown)
        .map(|(entry, shown)| StoryAsk { open: open(entry.ask.state), ask: entry.ask, shown, built: entry.built, handed: entry.handed })
        .collect();
    JobStory { session_id: session.clone(), asks, shown_otherwise: otherwise, lead, features: Vec::new() }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_job_story`.
    pub(crate) async fn job_story_read(&self, session: &SessionId) -> Result<JobStory, Refusal> {
        let asks = self.job_asks_listed(session).await?.asks;
        let artifacts = self.job_artifacts(session.as_str()).await?;
        let mut entries = Vec::new();
        let features;
        {
            let store = self.store().lock().await;
            features = self.features_of_job_in(&store, session.as_str(), &asks).map_err(|why| self.ledger_fault(why))?;
            for ask in asks {
                let built = store
                    .merges_of_ask(&ask.ask_id)
                    .map_err(|why| self.ledger_fault(why))?
                    .into_iter()
                    .map(|one| StoryBuilt { pull_request: one.pull_request, merged_at: Instant::carried(one.merged_at) })
                    .collect();
                let mut handed = Vec::new();
                for hand in store.handoffs_of_ask(&ask.ask_id).map_err(|why| self.ledger_fault(why))? {
                    let to_title = store.session(&hand.to_session).ok().flatten().and_then(|one| one.title);
                    handed.push(StoryHanded {
                        from: SessionId::carried(hand.from_session),
                        to: SessionId::carried(hand.to_session),
                        to_title,
                        at: Instant::carried(hand.at),
                    });
                }
                entries.push(Entry { ask, built, handed });
            }
        }
        Ok(JobStory { features, ..story_of(session, entries, &artifacts) })
    }
}

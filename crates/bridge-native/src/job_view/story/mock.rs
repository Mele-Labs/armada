//! A Job's story for a walk, a picture and the tests, with no Fleet: the three asks of the approved mock (a change request
//! being built, a bug handed on, a bug fixed) and one thing shown before any ask. It stands on the features the feature
//! screens' mock holds, so a Filed under row opens a page that exists.

use ipc::{Ask, AskKind, AskState, AskWords, ArtifactForm, ArtifactState, Instant, JobStory, MessageKind, PageAct, PagePress, PressDid, SessionId, StoryAsk, StoryBuilt, StoryHanded, StoryShown, VoteCast, VoteForm, VoteResult};

use super::model::Features;
use crate::features::page_model::Titles;

/// The Job the mock story is of.
pub const JOB: &str = "job-feature-catalog";
const OTHER: &str = "job-one-artifact";

fn at(day: u32) -> Instant {
    at_h(day, 12)
}

fn at_h(day: u32, hour: u32) -> Instant {
    Instant::carried(format!("2026-11-{day:02}T{hour:02}:00:00Z"))
}

/// The titles of the Jobs the story names.
pub fn titles() -> Titles {
    [(JOB, "Feature Catalog"), (OTHER, "One artifact that grows")].into_iter().map(|(id, title)| (id.to_string(), title.to_string())).collect()
}

/// The titles of the features of the mock project.
pub fn features() -> Features {
    crate::features::mock::library().areas.iter().flat_map(|area| &area.features).map(|row| (row.slug.clone(), row.title.clone())).collect()
}

fn ask(id: &str, kind: AskKind, title: &str, words: &str, state: AskState, opened: u32, moved: u32, features: &[&str]) -> Ask {
    Ask {
        ask_id: id.into(),
        kind,
        title: title.into(),
        first_words: words.into(),
        first_row: format!("row-{id}"),
        said_in: SessionId::carried(JOB),
        building: matches!(state, AskState::Building | AskState::Built).then(|| SessionId::carried(JOB)),
        features: features.iter().map(|slug| slug.to_string()).collect(),
        state,
        state_at: at(moved),
        opened_at: at(opened),
        said: Vec::new(),
    }
}

fn words(id: &str, kind: MessageKind, said: &str, day: u32) -> AskWords {
    AskWords { session_id: SessionId::carried(JOB), row_id: id.into(), kind, words: said.into(), at: at(day) }
}

fn story_ask(ask: Ask, open: bool) -> StoryAsk {
    StoryAsk { ask, shown: Vec::new(), built: Vec::new(), handed: Vec::new(), open }
}

fn shown(artifact: &str, title: &str, form: ArtifactForm, state: ArtifactState, said: Option<&str>, day: u32) -> StoryShown {
    StoryShown { artifact: artifact.into(), title: title.into(), form, state, words: said.map(str::to_string), votes: Vec::new(), presses: Vec::new(), shown_at: at(day) }
}

/// The story of `job`: every Job shows the same one.
pub fn story(job: &str) -> JobStory {
    let mut catalog = story_ask(ask("ask-1", AskKind::ChangeRequest, "Feature catalog", "every feature gets a page the agents read before they touch its files", AskState::Building, 12, 16, &["start-a-job", "reviews"]), true);
    catalog.ask.said = vec![words("row-a", MessageKind::Revision, "decided kinds as sections", 16)];
    let mut design = shown("documents/feature-catalog", "Feature catalog design", ArtifactForm::Page, ArtifactState::Approved, None, 15);
    design.votes = vec![VoteCast { revision: 2, vote: "layout".into(), form: VoteForm::Pick, result: VoteResult::Pick { take: "rows".into(), notes: Default::default() }, at: at(15) }];
    catalog.shown = vec![design];

    let mut reading = story_ask(ask("ask-2", AskKind::Bug, "A reading page never shows", "a page i am reading never shows up in the tab", AskState::Handed, 13, 14, &["start-a-job"]), true);
    reading.handed = vec![StoryHanded { from: SessionId::carried(JOB), to: SessionId::carried(OTHER), to_title: Some("One artifact that grows".into()), at: at(14) }];

    let mut votes = story_ask(ask("ask-3", AskKind::Bug, "Votes show as never cast", "they show as i never voted", AskState::Built, 15, 15, &["start-a-job"]), false);
    votes.ask.said = vec![words("row-b", MessageKind::Answer, "yes, per revision", 13)];
    let mut sent = shown("sketches/votes", "Votes per revision", ArtifactForm::Sketch, ArtifactState::SentBack, Some("keep each take's votes"), 14);
    sent.presses = vec![PagePress { revision: 1, action: "start".into(), item: Some("take-a".into()), act: PageAct::Start, at: at(14), did: PressDid::Sent }];
    votes.shown = vec![sent];
    votes.handed = vec![StoryHanded { from: SessionId::carried(JOB), to: SessionId::carried(OTHER), to_title: Some("One artifact that grows".into()), at: at_h(15, 9) }];
    votes.built = vec![StoryBuilt { pull_request: "3720".into(), merged_at: at_h(15, 15) }];

    JobStory {
        session_id: SessionId::carried(job),
        asks: vec![catalog, reading, votes],
        shown_otherwise: vec![shown("sketches/start-card", "Start card with the model picker", ArtifactForm::Sketch, ArtifactState::Dismissed, None, 11)],
        lead: vec!["Feature catalog: being built".into(), "A reading page never shows: handed to One artifact that grows".into(), "Votes show as never cast: fixed in #3720".into()],
    }
}

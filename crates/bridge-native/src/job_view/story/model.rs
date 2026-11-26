//! What a Job's Story tab draws, worked out from the `JobStory` Fleet read: each ask's head line and the rows under it
//! when it is unfolded. Nothing here touches the screen or the wire.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ipc::{ArtifactForm, ArtifactState, AskKind, AskState, AskWords, JobFeature, JobStory, MessageKind, SessionId, StoryAsk, StoryBuilt, StoryHanded, StoryShown, VoteResult};

use crate::features::page_model::{day, has_job, instant, job_title, Hue, Titles};

/// The titles of the features of the Job's project, by slug.
pub type Features = HashMap<String, String>;

/// Where an ask stands, as its head and a done card's lead line say it: a word, the registry glyph and its hue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stand {
    pub word: &'static str,
    pub glyph: &'static str,
    pub hue: Hue,
}

pub fn stand(state: AskState) -> Stand {
    match state {
        AskState::Building => Stand { word: "Open", glyph: "circle-dot", hue: Hue::Running },
        AskState::Handed => Stand { word: "Handed", glyph: "circle", hue: Hue::Handed },
        AskState::Backlog => Stand { word: "Backlog", glyph: "circle-dashed", hue: Hue::NotStarted },
        AskState::Built => Stand { word: "Fixed", glyph: "circle-check", hue: Hue::Done },
        AskState::Dropped => Stand { word: "Dropped", glyph: "circle-x", hue: Hue::Muted },
    }
}

pub fn kind_word(kind: AskKind) -> &'static str {
    match kind {
        AskKind::Bug => "Bug",
        AskKind::ChangeRequest => "Change request",
        AskKind::Idea => "Idea",
        AskKind::Fix => "Fix",
    }
}

/// One row under an unfolded ask: a 16px glyph, the words, and a muted line.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: String,
    pub glyph: &'static str,
    pub hue: Hue,
    /// What the glyph's tooltip says.
    pub said: String,
    pub text: String,
    /// The words are drawn in the glyph's hue instead of the default ink.
    pub tinted: bool,
    pub meta: String,
    /// A second muted line: what he voted and pressed on what he was shown.
    pub also: String,
    /// The feature this row opens, where it is the line that files the ask under one.
    pub feature: Option<String>,
    /// His own words whole, drawn as the thread draws them: headings, lists and code as formatted, not one quoted line.
    pub words: Option<String>,
}

/// One ask as the tab draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub ask_id: String,
    /// `1.`: its place as drawn: grouped by feature, open ones first in a group.
    pub number: String,
    pub title: String,
    pub kind: &'static str,
    /// What came of it, in one line: merged, handed, decided or answered, open or not.
    pub outcome: String,
    pub stand: Stand,
    pub rows: Vec<Row>,
    /// The feature this card opens a group of, where it is the first of its group. The last group has none.
    pub heading: Option<Heading>,
}

/// The feature a group of asks is filed under.
#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub slug: String,
    pub title: String,
    /// The slug of its area, which gives its hue.
    pub area: String,
}

/// A feature the Job is on, as the section above the asks lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureLine {
    pub slug: String,
    pub title: String,
    /// The slug of its area, which gives its hue.
    pub area: String,
    /// How the Job touched it: `wrote 12 files · named in 9 commits`.
    pub how: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Built {
    /// The features the Job is on, in the order Fleet gave them. The section is drawn only where this is not empty.
    pub features: Vec<FeatureLine>,
    pub cards: Vec<Card>,
    /// What he was shown before any ask, unnumbered, under a heading the view draws only where this is not empty.
    pub before: Vec<Row>,
}

impl Built {
    /// The place of the ask `ask_id`.
    pub fn find(&self, ask_id: &str) -> Option<usize> {
        self.cards.iter().position(|card| card.ask_id == ask_id)
    }

    /// The ask unfolded where none was asked for: the first open one, else the first.
    pub fn first_open(&self) -> Option<usize> {
        self.cards.iter().position(|card| card.stand.hue == Hue::Running).or(if self.cards.is_empty() { None } else { Some(0) })
    }
}

fn first_line(words: &str) -> String {
    plain(words).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// His words as plain text: his markdown marks (emphasis, code ticks, a heading's hashes, a link's address) are not drawn.
fn plain(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    let mut rest = words.trim_start().trim_start_matches(['#', '>']).trim_start();
    while let Some(one) = rest.chars().next() {
        rest = &rest[one.len_utf8()..];
        match one {
            '*' | '`' => {}
            // `[name](address)` reads as its name.
            '[' => match rest.find("](").and_then(|close| rest[close..].find(')').map(|end| (close, close + end))) {
                Some((close, end)) if !rest[..close].contains(['[', '\n']) => {
                    out.push_str(&rest[..close]);
                    rest = &rest[end + 1..];
                }
                _ => out.push(one),
            },
            _ => out.push(one),
        }
    }
    out
}

/// How he decided about what he was shown, in Armada's voice.
pub fn decided(shown: &StoryShown) -> String {
    let words = shown.words.as_deref().map(first_line).filter(|words| !words.is_empty());
    match shown.state {
        ArtifactState::Approved => words.map_or_else(|| "Approved".into(), |words| format!("Approved: {words}")),
        ArtifactState::SentBack => words.map_or_else(|| "Sent back".into(), |words| format!("Sent back: {words}")),
        ArtifactState::Dismissed => "Dismissed".into(),
        ArtifactState::Asking => "Waiting".into(),
        ArtifactState::Reading => "Shown".into(),
        ArtifactState::Withdrawn => "Taken back".into(),
    }
}

fn form_word(form: ArtifactForm) -> &'static str {
    match form {
        ArtifactForm::Sketch => "Sketch",
        ArtifactForm::Draft => "Draft",
        ArtifactForm::Page => "Document",
    }
}

/// His votes and presses on it as one muted line, with no figure beside what it counts.
fn also(shown: &StoryShown) -> String {
    let mut parts: Vec<String> = Vec::new();
    for cast in &shown.votes {
        parts.push(match &cast.result {
            VoteResult::Pick { take, .. } => format!("Picked {take}"),
            VoteResult::Rank { order, .. } => format!("Ranked {}", order.join(", ")),
            VoteResult::Score { .. } => "Scored".into(),
        });
    }
    for press in &shown.presses {
        parts.push(match &press.item {
            Some(item) => format!("Pressed {} on {item}", press.action),
            None => format!("Pressed {}", press.action),
        });
    }
    parts.join(". ")
}

fn shown_row(shown: &StoryShown) -> Row {
    let decision = decided(shown);
    let (glyph, hue) = match shown.state {
        ArtifactState::Approved => ("circle-check", Hue::Done),
        ArtifactState::SentBack => ("undo-2", Hue::Warn),
        ArtifactState::Dismissed => ("circle-x", Hue::Muted),
        ArtifactState::Asking => ("circle-help", Hue::Warn),
        ArtifactState::Reading => ("eye", Hue::Muted),
        ArtifactState::Withdrawn => ("ban", Hue::Muted),
    };
    Row {
        key: format!("shown-{}", shown.artifact),
        glyph,
        hue,
        said: decision.clone(),
        text: shown.title.clone(),
        tinted: false,
        meta: format!("{}, {}. {decision}", form_word(shown.form), day(instant(&shown.shown_at))),
        also: also(shown),
        feature: None,
        words: None,
    }
}

fn said_row(words: &AskWords) -> Row {
    let what = if words.kind == MessageKind::Answer { "answer" } else { "revision" };
    Row {
        key: format!("said-{}", words.row_id),
        glyph: if words.kind == MessageKind::Answer { "message-square-reply" } else { "message-square-text" },
        hue: Hue::Muted,
        said: what.into(),
        text: format!("\"{}\"", first_line(&words.words)),
        tinted: false,
        meta: format!("You, {}, {what}", day(instant(&words.at))),
        also: String::new(),
        feature: None,
        words: Some(words.words.trim().to_string()),
    }
}

/// One thing that happened to an ask, with when, for its rows.
struct Happened {
    at: DateTime<Utc>,
    row: Row,
}

fn handed_text(handed: &StoryHanded, mine: &SessionId, titles: &Titles) -> String {
    let taken = handed.to == *mine;
    let other = if taken { job_title(titles, &handed.from) } else { handed.to_title.clone().filter(|title| !title.is_empty()).or_else(|| job_title(titles, &handed.to)) };
    match (taken, other) {
        (true, Some(title)) => format!("Taken from {title}"),
        (true, None) => "Taken".into(),
        (false, Some(title)) => format!("Handed to {title}"),
        (false, None) => "Handed".into(),
    }
}

fn pull_number(built: &StoryBuilt) -> &str {
    built.pull_request.trim().trim_start_matches('#')
}

/// What came of an ask, in one line: what was merged, where it was handed, or the last thing he decided or answered, and
/// whether it is still open. An ask with nothing yet says where it stands.
pub fn outcome(story: &StoryAsk, mine: &SessionId, titles: &Titles) -> String {
    if !story.built.is_empty() {
        let merged = story.built.iter().map(|built| format!("#{}", pull_number(built))).collect::<Vec<_>>().join(", ");
        return if story.open { format!("Merged {merged}, still open") } else { format!("Merged {merged}") };
    }
    if let Some(handed) = story.handed.last() {
        return handed_text(handed, mine, titles);
    }
    let decisions = story.shown.iter().map(|shown| (instant(&shown.shown_at), decided(shown))).chain(story.ask.said.iter().map(|words| {
        let line = first_line(&words.words);
        (instant(&words.at), if words.kind == MessageKind::Answer { format!("Answered: {line}") } else { line })
    }));
    let latest = decisions.max_by_key(|(at, _)| *at).map(|(_, said)| said);
    match (story.open, latest) {
        (true, Some(said)) => format!("Open: {said}"),
        (true, None) => "Open".into(),
        (false, Some(said)) => said,
        (false, None) => stand(story.ask.state).word.into(),
    }
}

/// The group an ask is filed under: the place in the Job's features, most touched first, of the first of them it touches.
pub fn group_of(story: &StoryAsk, features: &[JobFeature]) -> Option<usize> {
    features.iter().position(|one| story.ask.features.contains(&one.slug))
}

/// The asks in the order they are drawn: grouped by feature as the Job's features stand, those under none last, open ones
/// first in a group and Fleet's order kept otherwise.
pub fn arranged(story: &JobStory) -> Vec<(Option<usize>, &StoryAsk)> {
    let mut asks: Vec<(Option<usize>, &StoryAsk)> = story.asks.iter().map(|one| (group_of(one, &story.features), one)).collect();
    asks.sort_by_key(|(group, one)| (group.unwrap_or(usize::MAX), !one.open));
    asks
}

fn happenings(story: &StoryAsk, mine: &SessionId, titles: &Titles) -> Vec<Happened> {
    let mut out: Vec<Happened> = Vec::new();
    for words in &story.ask.said {
        out.push(Happened { at: instant(&words.at), row: said_row(words) });
    }
    for shown in &story.shown {
        out.push(Happened { at: instant(&shown.shown_at), row: shown_row(shown) });
    }
    for built in &story.built {
        let number = pull_number(built);
        let text = format!("Merged #{number}");
        out.push(Happened {
            at: instant(&built.merged_at),
            row: Row { key: format!("built-{number}"), glyph: "git-merge", hue: Hue::Done, said: "Merged".into(), text: text.clone(), tinted: true, meta: day(instant(&built.merged_at)), also: String::new(), feature: None, words: None },
        });
    }
    for handed in &story.handed {
        let text = handed_text(handed, mine, titles);
        out.push(Happened {
            at: instant(&handed.at),
            row: Row { key: format!("handed-{}", handed.at.as_str()), glyph: "arrow-right-left", hue: Hue::Handed, said: "Handed".into(), text: text.clone(), tinted: true, meta: day(instant(&handed.at)), also: String::new(), feature: None, words: None },
        });
    }
    // What happened first stands first; the order Fleet gave breaks a tie.
    out.sort_by_key(|one| one.at);
    out
}

fn card(at: usize, heading: Option<Heading>, story: &StoryAsk, mine: &SessionId, titles: &Titles, features: &Features) -> Card {
    let ask = &story.ask;
    let started = format!("You, {}{}", day(instant(&ask.opened_at)), if has_job(&ask.said_in) { ", how it started" } else { ", to the Operator, how it started" });
    let mut rows = vec![Row {
        key: format!("start-{}", ask.ask_id),
        glyph: "message-square-quote",
        hue: Hue::Muted,
        said: "How it started".into(),
        text: format!("\"{}\"", first_line(&ask.first_words)),
        tinted: false,
        meta: started,
        also: String::new(),
        feature: None,
        words: Some(ask.first_words.trim().to_string()),
    }];
    let happened = happenings(story, mine, titles);
    rows.extend(happened.into_iter().map(|one| one.row));
    if story.open {
        rows.push(Row { key: "open".into(), glyph: "circle-dot", hue: Hue::Running, said: "Open".into(), text: "Open".into(), tinted: true, meta: String::new(), also: String::new(), feature: None, words: None });
    }
    for slug in &ask.features {
        let title = features.get(slug).cloned().unwrap_or_else(|| slug.clone());
        rows.push(Row {
            key: format!("filed-{slug}"),
            glyph: "library",
            hue: Hue::Muted,
            said: "Feature".into(),
            text: format!("Filed under {title}"),
            tinted: false,
            meta: "Feature".into(),
            also: String::new(),
            feature: Some(slug.clone()),
            words: None,
        });
    }
    Card { ask_id: ask.ask_id.clone(), number: format!("{}.", at + 1), title: ask.title.clone(), kind: kind_word(ask.kind), outcome: outcome(story, mine, titles), stand: stand(ask.state), rows, heading }
}

fn counted(count: u32, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// How the Job touched a feature, each way it did joined by a dot: a way with nothing in it is left out.
pub fn how_touched(feature: &JobFeature) -> String {
    let wrote = (feature.files_written > 0).then(|| format!("wrote {}", counted(feature.files_written, "file", "files")));
    let named = (feature.commits_named > 0).then(|| format!("named in {}", counted(feature.commits_named, "commit", "commits")));
    let filed = (feature.asks > 0).then(|| format!("{} filed", counted(feature.asks, "ask", "asks")));
    [wrote, named, filed].into_iter().flatten().collect::<Vec<_>>().join(" · ")
}

/// The tab's cards, grouped by feature and numbered as drawn.
pub fn build(story: &JobStory, titles: &Titles, features: &Features) -> Built {
    Built {
        features: story.features.iter().map(|one| FeatureLine { slug: one.slug.clone(), title: one.title.clone(), area: one.area.clone(), how: how_touched(one) }).collect(),
        cards: {
            let mut seen = None;
            arranged(story)
                .into_iter()
                .enumerate()
                .map(|(at, (group, ask))| {
                    let heading = if group != seen { group.and_then(|group| story.features.get(group)).map(|one| Heading { slug: one.slug.clone(), title: one.title.clone(), area: one.area.clone() }) } else { None };
                    seen = group;
                    card(at, heading, ask, &story.session_id, titles, features)
                })
                .collect()
        },
        before: story.shown_otherwise.iter().map(shown_row).collect(),
    }
}

/// The lines a done card leads with: Fleet's words for each ask, with where the ask stands, in the order Fleet gave.
pub fn lead(story: &JobStory) -> Vec<(Stand, String)> {
    story.lead.iter().enumerate().map(|(at, line)| (stand(story.asks.get(at).map_or(AskState::Building, |ask| ask.ask.state)), line.clone())).collect()
}

//! What a Job's Story tab draws, worked out from the `JobStory` Fleet read: a timeline of rows oldest first, each with when
//! it happened and the time its portion took, and what an opened row holds. Nothing here touches the screen or the wire.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ipc::{ArtifactForm, ArtifactState, Ask, AskKind, AskState, AskWords, JobFeature, JobStory, MessageKind, PullRequestCommit, SessionId, StoryAsk, StoryBuilt, StoryHanded, StoryShown, VoteResult};

use crate::features::page_model::{instant, job_title, Hue, Titles};

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


// ---- the timeline ----------------------------------------------------------------------------

/// Who a row in the timeline is with. One glyph each; no word is drawn on the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// His words: an ask, and what he added to it.
    You,
    /// Another Job: an ask handed to it or taken from it.
    Job,
    /// What he was shown, and what he did with it.
    Artifact,
    /// A pull request that built an ask.
    Pull,
}

impl Kind {
    /// The order the filter tabs stand in.
    pub const ALL: [Kind; 4] = [Kind::You, Kind::Job, Kind::Artifact, Kind::Pull];

    pub fn glyph(self) -> &'static str {
        match self {
            Kind::You => "user",
            Kind::Job => "link",
            Kind::Artifact => "layers",
            Kind::Pull => "git-pull-request",
        }
    }

    pub fn hue(self) -> Hue {
        match self {
            Kind::You => Hue::Warn,
            Kind::Job => Hue::Handed,
            Kind::Artifact => Hue::Running,
            Kind::Pull => Hue::Muted,
        }
    }

    /// What the glyph's tooltip and name say.
    pub fn said(self) -> &'static str {
        match self {
            Kind::You => "You",
            Kind::Job => "Another Job",
            Kind::Artifact => "Artifact",
            Kind::Pull => "Pull request",
        }
    }

    /// The word on the filter tab.
    pub fn tab(self) -> &'static str {
        match self {
            Kind::You => "You",
            Kind::Job => "Jobs",
            Kind::Artifact => "Artifacts",
            Kind::Pull => "PRs",
        }
    }
}

/// Which rows the timeline shows. Opening a row leaves it as it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Only(Kind),
}

impl Filter {
    pub fn word(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Only(kind) => kind.tab(),
        }
    }

    pub fn admits(self, kind: Kind) -> bool {
        match self {
            Filter::All => true,
            Filter::Only(only) => only == kind,
        }
    }
}

/// Whose time a portion was: his while he was looking at what he was shown, the Job's otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    You,
    Job,
}

/// The time a row's portion took: from the row to the next row, or to now where the row still waits on him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spent {
    pub minutes: Option<i64>,
    pub who: Who,
    /// The bar's share of the longest portion, 0 where there is no figure, otherwise 4 to 100.
    pub share: u8,
}

impl Spent {
    pub const NONE: Spent = Spent { minutes: None, who: Who::Job, share: 0 };

    /// `45m`, `3h`, `2h 5m`, `2d`; nothing where the portion has no end yet.
    pub fn said(&self) -> String {
        match self.minutes {
            None => String::new(),
            Some(minutes) => spoken(minutes),
        }
    }
}

pub fn spoken(minutes: i64) -> String {
    match minutes {
        m if m < 1 => "<1m".into(),
        m if m < 60 => format!("{m}m"),
        m if m >= 1440 => format!("{}d", m / 1440),
        m if m % 60 == 0 => format!("{}h", m / 60),
        m => format!("{}h {}m", m / 60, m % 60),
    }
}

/// When, as the column says it: `now`, `12m ago`, `3h ago`, `2d ago`.
pub fn relative(now: DateTime<Utc>, at: DateTime<Utc>) -> String {
    let seconds = (now - at).num_seconds();
    match seconds {
        s if s < 60 => "now".into(),
        s if s < 3600 => format!("{}m ago", s / 60),
        s if s < 86_400 => format!("{}h ago", s / 3600),
        s => format!("{}d ago", s / 86_400),
    }
}

/// One line of a chat: his on the right in the accent, the Job's on the left.
#[derive(Clone, Debug, PartialEq)]
pub struct Bubble {
    pub mine: bool,
    /// Who spoke, drawn under a bubble that is not his.
    pub who: String,
    pub said: String,
    /// Relative; empty where Fleet did not keep when.
    pub when: String,
}

/// A feature an ask is filed under: a row of its card opens the feature's page.
#[derive(Clone, Debug, PartialEq)]
pub struct Filed {
    pub slug: String,
    pub title: String,
}

/// The merge a pull request row stands for.
#[derive(Clone, Debug, PartialEq)]
pub struct Merge {
    pub number: u64,
    pub at: DateTime<Utc>,
}

/// What an opened row holds.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Card {
    /// Where the ask stands, as a fixed-width pill with its meaning in the tooltip.
    pub stand: Option<Stand>,
    /// What the pill's state means, for its tooltip.
    pub means: &'static str,
    pub bubbles: Vec<Bubble>,
    pub filed: Vec<Filed>,
    pub merge: Option<Merge>,
    /// The Job an ask was handed to or taken from, which a button opens.
    pub job: Option<(String, String)>,
    /// What the hand was about: the ask's title.
    pub about: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub key: String,
    pub kind: Kind,
    pub at: DateTime<Utc>,
    pub title: String,
    /// The explained status under the title, in plain words. Empty where there is none.
    pub says: String,
    /// The Job is waiting on him for this.
    pub waiting: bool,
    pub when: String,
    pub spent: Spent,
    pub card: Card,
}

/// One press an opened row offers, in the order the keys visit them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    Feature(String),
    Commit { number: u64, sha: String, headline: String },
    Pull(u64),
    Job(String),
}

/// One line of a pull request's own history: its commits and its merge.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub glyph: &'static str,
    pub hue: Hue,
    pub said: String,
    pub sha: Option<String>,
    pub when: String,
    pub link: Option<Link>,
}

/// A pull request's commits and its merge in the order they happened. A commit with no time stands first.
pub fn pull_events(merge: &Merge, commits: &[PullRequestCommit], now: DateTime<Utc>) -> Vec<Event> {
    let mut out: Vec<(DateTime<Utc>, Event)> = commits
        .iter()
        .map(|commit| {
            let at = commit.at.as_deref().and_then(|at| DateTime::parse_from_rfc3339(at).ok()).map(|at| at.with_timezone(&Utc));
            let event = Event {
                glyph: "git-commit-horizontal",
                hue: Hue::Muted,
                said: commit.headline.clone(),
                sha: Some(commit.sha.chars().take(7).collect()),
                when: at.map(|at| relative(now, at)).unwrap_or_default(),
                link: Some(Link::Commit { number: merge.number, sha: commit.sha.clone(), headline: commit.headline.clone() }),
            };
            (at.unwrap_or(DateTime::<Utc>::MIN_UTC), event)
        })
        .collect();
    out.sort_by_key(|(at, _)| *at);
    let merged = Event { glyph: "git-merge", hue: Hue::Done, said: "Merged".into(), sha: None, when: relative(now, merge.at), link: None };
    let mut events: Vec<Event> = out.into_iter().map(|(_, event)| event).collect();
    events.push(merged);
    events
}

impl Entry {
    /// The presses the opened row offers, in the order they are drawn: its features, its commits, then the button.
    pub fn links(&self) -> Vec<Link> {
        let mut links: Vec<Link> = self.card.filed.iter().map(|filed| Link::Feature(filed.slug.clone())).collect();
        links.extend(self.card.job.iter().map(|(id, _)| Link::Job(id.clone())));
        links
    }

    /// With the commits of its pull request, which are read after the story.
    pub fn links_with(&self, commits: &[PullRequestCommit]) -> Vec<Link> {
        let mut links = self.links();
        if let Some(merge) = &self.card.merge {
            links.extend(commits.iter().map(|commit| Link::Commit { number: merge.number, sha: commit.sha.clone(), headline: commit.headline.clone() }));
            links.push(Link::Pull(merge.number));
        }
        links
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Built {
    /// The features the Job is on, in the order Fleet gave them: the strip over the thread draws them.
    pub features: Vec<FeatureLine>,
    /// Oldest first.
    pub entries: Vec<Entry>,
    /// What he was shown before any ask, drawn under its own heading where this is not empty.
    pub before: Vec<Entry>,
}

impl Built {
    /// The place of the ask `ask_id`.
    pub fn find(&self, ask_id: &str) -> Option<usize> {
        let key = ask_key(ask_id);
        self.entries.iter().position(|entry| entry.key == key)
    }

    /// The filter tabs: All, then each kind the story holds a row of, in a fixed order.
    pub fn filters(&self) -> Vec<Filter> {
        let held = |kind: Kind| self.entries.iter().chain(&self.before).any(|entry| entry.kind == kind);
        std::iter::once(Filter::All).chain(Kind::ALL.into_iter().filter(|kind| held(*kind)).map(Filter::Only)).collect()
    }

    /// The places of the timeline's rows the filter lets through.
    pub fn visible(&self, filter: Filter) -> Vec<usize> {
        self.entries.iter().enumerate().filter(|(_, entry)| filter.admits(entry.kind)).map(|(at, _)| at).collect()
    }

    /// Whether the group of what he was shown before any ask is drawn under the filter.
    pub fn before_shown(&self, filter: Filter) -> bool {
        !self.before.is_empty() && filter.admits(Kind::Artifact)
    }

    /// The row the keys start on: the newest that waits on him, else the newest.
    pub fn newest(&self, filter: Filter) -> Option<usize> {
        let shown = self.visible(filter);
        shown.iter().rev().find(|at| self.entries[**at].waiting).or(shown.last()).copied()
    }
}

pub fn ask_key(ask_id: &str) -> String {
    format!("ask-{ask_id}")
}

/// The rows oldest first, with the order Fleet gave breaking a tie.
pub fn arrange(mut entries: Vec<Entry>) -> Vec<Entry> {
    entries.sort_by_key(|entry| entry.at);
    entries
}

/// Each row's portion: from the row to the next, or to `now` for the last where it waits on him. Time after something
/// shown to him is his; every other portion is the Job's. The bar is the portion's share of the longest.
pub fn durations(entries: &mut [Entry], now: DateTime<Utc>) {
    let ends: Vec<Option<DateTime<Utc>>> = (0..entries.len()).map(|at| entries.get(at + 1).map(|next| next.at).or(entries[at].waiting.then_some(now))).collect();
    let minutes: Vec<Option<i64>> = entries.iter().zip(&ends).map(|(entry, end)| end.map(|end| (end - entry.at).num_minutes().max(0))).collect();
    let longest = minutes.iter().flatten().copied().max().unwrap_or(0).max(1);
    for (entry, minutes) in entries.iter_mut().zip(minutes) {
        let who = if entry.kind == Kind::Artifact { Who::You } else { Who::Job };
        let share = minutes.map_or(0, |minutes| (100 * minutes / longest).clamp(4, 100) as u8);
        entry.spent = Spent { minutes, who, share };
    }
}

/// What a state of an artifact says he did, in his own terms.
pub fn said_of(state: ArtifactState) -> &'static str {
    match state {
        ArtifactState::Approved => "You approved it",
        ArtifactState::SentBack => "You sent it back with a note",
        ArtifactState::Dismissed => "You dismissed it",
        ArtifactState::Asking => "Asking you",
        ArtifactState::Reading => "Shown to you",
        ArtifactState::Withdrawn => "The Job took it back",
    }
}

/// What a state of an ask means, for the pill's tooltip.
pub fn means(state: AskState) -> &'static str {
    match state {
        AskState::Building => "A Job is building it",
        AskState::Handed => "Passed to another Job, which builds it",
        AskState::Backlog => "Decided and not started",
        AskState::Built => "Built and merged",
        AskState::Dropped => "Dropped",
    }
}

/// His votes and presses on what he was shown, each as a line of his own with when.
fn touches(shown: &StoryShown, now: DateTime<Utc>) -> Vec<(DateTime<Utc>, Bubble)> {
    let said = |at: &ipc::Instant, said: String| {
        let at = instant(at);
        (at, Bubble { mine: true, who: "You".into(), said, when: relative(now, at) })
    };
    let votes = shown.votes.iter().map(|cast| {
        said(
            &cast.at,
            match &cast.result {
                VoteResult::Pick { take, .. } => format!("Picked {take}"),
                VoteResult::Rank { order, .. } => format!("Ranked {}", order.join(", ")),
                VoteResult::Score { .. } => "Scored".into(),
            },
        )
    });
    let presses = shown.presses.iter().map(|press| {
        said(
            &press.at,
            match &press.item {
                Some(item) => format!("Pressed {} on {item}", press.action),
                None => format!("Pressed {}", press.action),
            },
        )
    });
    let mut out: Vec<_> = votes.chain(presses).collect();
    out.sort_by_key(|(at, _)| *at);
    out
}

fn shown_entry(shown: &StoryShown, job: &str, now: DateTime<Utc>) -> Entry {
    let at = instant(&shown.shown_at);
    let mut bubbles = vec![Bubble {
        mine: false,
        who: job.to_string(),
        said: format!("Showed you the {} {}", form_word(shown.form).to_lowercase(), shown.title),
        when: relative(now, at),
    }];
    if let Some(words) = shown.words.as_deref().map(str::trim).filter(|words| !words.is_empty()) {
        bubbles.push(Bubble { mine: true, who: "You".into(), said: words.to_string(), when: String::new() });
    }
    bubbles.extend(touches(shown, now).into_iter().map(|(_, bubble)| bubble));
    Entry {
        key: format!("shown-{}", shown.artifact),
        kind: Kind::Artifact,
        at,
        title: shown.title.clone(),
        says: said_of(shown.state).into(),
        waiting: shown.state == ArtifactState::Asking,
        when: relative(now, at),
        spent: Spent::NONE,
        card: Card { bubbles, ..Card::default() },
    }
}

const SENT_BACK: &str = "Sent back part \"";
const PUT_AGAIN: &str = " Change it and put it again with add_part under the same id.";
const PREVIEW_NOTE: &str = "On your preview note \"";

/// What he said, without the harness's wrapper: the line sent when he sends back an artifact, or his note on a preview.
pub fn his_words(raw: &str) -> String {
    let raw = raw.trim();
    let body = if let Some(rest) = raw.strip_prefix(SENT_BACK) {
        let rest = rest.split_once("\" of \"").map_or(rest, |(_, rest)| rest);
        rest.split_once("\". ").map_or("", |(_, said)| said)
    } else if let Some(rest) = raw.strip_prefix(PREVIEW_NOTE) {
        rest.split_once("\": ").map_or("", |(_, said)| said)
    } else {
        raw
    };
    body.trim().strip_suffix(PUT_AGAIN.trim_start()).unwrap_or(body).trim().to_string()
}

fn ask_entry(story: &StoryAsk, job: &str, mine: &SessionId, titles: &Titles, features: &Features, now: DateTime<Utc>) -> Entry {
    let ask = &story.ask;
    let at = instant(&ask.opened_at);
    let you = |said: &str, at: DateTime<Utc>| Bubble { mine: true, who: "You".into(), said: his_words(said), when: relative(now, at) };
    let theirs = |said: String, at: DateTime<Utc>| Bubble { mine: false, who: job.to_string(), said, when: relative(now, at) };
    let mut said = vec![(at, you(&ask.first_words, at))];
    said.extend(ask.said.iter().map(|words| (instant(&words.at), you(&words.words, instant(&words.at)))));
    said.retain(|(_, bubble)| !bubble.said.is_empty());
    let mut job_said: Vec<(DateTime<Utc>, Bubble)> = Vec::new();
    for shown in &story.shown {
        let at = instant(&shown.shown_at);
        job_said.push((at, theirs(format!("Showed you the {} {}", form_word(shown.form).to_lowercase(), shown.title), at)));
    }
    for built in &story.built {
        let at = instant(&built.merged_at);
        job_said.push((at, theirs(format!("Merged pull request #{}", pull_number(built)), at)));
    }
    for handed in &story.handed {
        let at = instant(&handed.at);
        let other = if handed.to == *mine { &handed.from } else { &handed.to };
        let name = job_title(titles, other).or_else(|| handed.to_title.clone().filter(|title| !title.is_empty())).unwrap_or_else(|| "another Job".into());
        job_said.push((at, theirs(format!("Passed it to {name}"), at)));
    }
    for reply in &story.replies {
        let at = instant(&reply.at);
        job_said.push((at, theirs(reply.words.trim().to_string(), at)));
    }
    said.extend(job_said);
    said.sort_by_key(|(at, _)| *at);
    let bubbles: Vec<Bubble> = said.into_iter().map(|(_, bubble)| bubble).collect();
    Entry {
        key: ask_key(&ask.ask_id),
        kind: Kind::You,
        at,
        title: ask.title.clone(),
        says: outcome(story, mine, titles),
        waiting: false,
        when: relative(now, at),
        spent: Spent::NONE,
        card: Card {
            stand: Some(stand(ask.state)),
            means: means(ask.state),
            bubbles,
            filed: ask.features.iter().map(|slug| Filed { slug: slug.clone(), title: features.get(slug).cloned().unwrap_or_else(|| slug.clone()) }).collect(),
            ..Card::default()
        },
    }
}

fn handed_entry(handed: &StoryHanded, ask: &Ask, mine: &SessionId, titles: &Titles, now: DateTime<Utc>) -> Entry {
    let at = instant(&handed.at);
    let other = if handed.to == *mine { &handed.from } else { &handed.to };
    Entry {
        key: format!("handed-{}-{}", ask.ask_id, handed.at.as_str()),
        kind: Kind::Job,
        at,
        title: handed_text(handed, mine, titles),
        says: String::new(),
        waiting: false,
        when: relative(now, at),
        spent: Spent::NONE,
        card: Card { job: Some((other.as_str().to_string(), job_title(titles, other).unwrap_or_else(|| handed.to_title.clone().unwrap_or_default()))), about: ask.title.clone(), ..Card::default() },
    }
}

fn built_entry(built: &StoryBuilt, now: DateTime<Utc>) -> Entry {
    let at = instant(&built.merged_at);
    let number = pull_number(built);
    Entry {
        key: format!("built-{number}"),
        kind: Kind::Pull,
        at,
        title: format!("Pull request #{number}"),
        says: "Merged".into(),
        waiting: false,
        when: relative(now, at),
        spent: Spent::NONE,
        card: Card { merge: number.parse().ok().map(|number| Merge { number, at }), ..Card::default() },
    }
}

/// The timeline of a story, as drawn: oldest first, each row with its portion of time.
pub fn build(story: &JobStory, titles: &Titles, features: &Features, now: DateTime<Utc>) -> Built {
    let job = job_title(titles, &story.session_id).unwrap_or_else(|| "Job".into());
    let mut entries = Vec::new();
    for one in &story.asks {
        entries.push(ask_entry(one, &job, &story.session_id, titles, features, now));
        entries.extend(one.shown.iter().map(|shown| shown_entry(shown, &job, now)));
        entries.extend(one.handed.iter().map(|handed| handed_entry(handed, &one.ask, &story.session_id, titles, now)));
        entries.extend(one.built.iter().map(|built| built_entry(built, now)));
    }
    let mut entries = arrange(entries);
    durations(&mut entries, now);
    let mut before: Vec<Entry> = story.shown_otherwise.iter().map(|shown| shown_entry(shown, &job, now)).collect();
    before.sort_by_key(|entry| entry.at);
    Built {
        features: story.features.iter().map(|one| FeatureLine { slug: one.slug.clone(), title: one.title.clone(), area: one.area.clone(), how: how_touched(one) }).collect(),
        entries,
        before,
    }
}

/// The lines a done card leads with: Fleet's words for each ask, with where the ask stands, in the order Fleet gave.
pub fn lead(story: &JobStory) -> Vec<(Stand, String)> {
    story.lead.iter().enumerate().map(|(at, line)| (stand(story.asks.get(at).map_or(AskState::Building, |ask| ask.ask.state)), line.clone())).collect()
}

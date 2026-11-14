//! What the Reviews screen draws, as one view model the mock fills today and Fleet's views will.
//!
//! **Shaped for Fleet's half**, which keeps the head he last saw, the files he marked viewed, the
//! inbox state of each pull request, the diff since he looked and the critique of a range. Each type
//! says which of those it stands for. Wire types are reused where they already fit.

use ipc::{PullRequestCheckRow, PullRequestCommit, PullRequestEntry, PullRequestFileStatus, PullReviewChecks, PullReviewList, PullReviewThread};

/// The four inbox groups, in the order they are drawn. Fleet's inbox state of a pull request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    FirstLook,
    Changed,
    Waiting,
    Done,
}

impl Group {
    pub const ALL: [Group; 4] = [Group::FirstLook, Group::Changed, Group::Waiting, Group::Done];

    pub fn said(self) -> &'static str {
        match self {
            Group::FirstLook => "Needs first look",
            Group::Changed => "Changed since you looked",
            Group::Waiting => "Waiting on author",
            Group::Done => "Done",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Group::FirstLook => "eye",
            Group::Changed => "git-commit-horizontal",
            Group::Waiting => "hourglass",
            Group::Done => "circle-check",
        }
    }
}

/// One pull request in the inbox: a `PullReviewRow` with the inbox state Fleet adds.
#[derive(Clone, Debug, PartialEq)]
pub struct InboxRow {
    pub number: u64,
    /// `owner/name`, as the forge names it.
    pub repository: String,
    pub title: String,
    pub author: String,
    /// How long since it was updated, as the row says it. Fleet sends RFC 3339.
    pub age: String,
    pub group: Group,
    /// Commits pushed since he looked: the chip on a changed row.
    pub new_commits: Option<u32>,
    /// The forge lists it is on, which the filter picks between.
    pub lists: Vec<PullReviewList>,
    pub checks: PullReviewChecks,
    /// Where his own review stands.
    pub mine: Verdict,
}

impl InboxRow {
    /// The repository's own name, without its owner.
    pub fn short_repository(&self) -> &str {
        self.repository.rsplit('/').next().unwrap_or(&self.repository)
    }

    /// The row's name, as VoiceOver reads it and a walk aims at it.
    pub fn name(&self) -> String {
        format!("#{} {}", self.number, self.title)
    }
}

/// Where his own review of a pull request stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    NotReviewed,
    Commented,
    ChangesRequested,
    Approved,
}

impl Verdict {
    pub fn said(self) -> &'static str {
        match self {
            Verdict::NotReviewed => "Not reviewed",
            Verdict::Commented => "Commented",
            Verdict::ChangesRequested => "Changes requested",
            Verdict::Approved => "Approved",
        }
    }

    /// The registry's glyph for a review's verdict, as the pull request panel draws it.
    pub fn glyph(self) -> &'static str {
        match self {
            Verdict::NotReviewed => "clock",
            Verdict::Commented => "scroll-text",
            Verdict::ChangesRequested => "ban",
            Verdict::Approved => "check",
        }
    }

    /// What he did, after "You".
    pub fn done(self) -> &'static str {
        match self {
            Verdict::NotReviewed => "have not reviewed",
            Verdict::Commented => "commented",
            Verdict::ChangesRequested => "requested changes",
            Verdict::Approved => "approved",
        }
    }
}

/// His last review, and what came after it: Fleet's review state of a pull request.
#[derive(Clone, Debug, PartialEq)]
pub struct Mine {
    pub verdict: Verdict,
    /// The short sha he reviewed at, said only in a tooltip.
    pub on: Option<String>,
    /// The day he reviewed, as the chip says it: `Tuesday`, `yesterday`, `today`.
    pub day: Option<String>,
    /// How long since he was asked, while he has not reviewed it.
    pub requested: Option<String>,
    /// Commits since his review.
    pub commits: u32,
    /// Replies to his comments since his review.
    pub replies: u32,
}

impl Mine {
    /// The bar's state chip.
    pub fn chip(&self) -> String {
        match (&self.day, self.verdict) {
            (Some(day), verdict) if verdict != Verdict::NotReviewed => format!("You {} {day}", verdict.done()),
            (_, verdict) => verdict.said().to_string(),
        }
    }

    /// The chip's tooltip, which carries the commit.
    pub fn tip(&self) -> String {
        match &self.on {
            Some(on) if self.verdict != Verdict::NotReviewed => format!("Your review: {} at {on}", self.verdict.said()),
            _ => format!("Your review: {}", self.verdict.said()),
        }
    }

    pub fn fresh(&self) -> bool {
        self.commits > 0 || self.replies > 0
    }

    /// What came after his review, or how long he has been asked.
    pub fn since(&self) -> Option<String> {
        if self.verdict == Verdict::NotReviewed {
            return self.requested.as_ref().map(|age| format!("Requested from you {age}"));
        }
        if !self.fresh() {
            return Some("Nothing new since".into());
        }
        let commits = match self.commits {
            0 => None,
            1 => Some("1 commit".to_string()),
            many => Some(format!("{many} commits")),
        };
        let replies = match self.replies {
            0 => None,
            1 => Some("1 reply to you".to_string()),
            many => Some(format!("{many} replies to you")),
        };
        Some(format!("Since then: {}", commits.into_iter().chain(replies).collect::<Vec<_>>().join(", ")))
    }
}

/// The inbox: its rows, and whether a refresh is reading the forge again.
#[derive(Clone, Debug, Default)]
pub struct Inbox {
    pub rows: Vec<InboxRow>,
    pub refreshing: bool,
}

/// Where he last looked, from Fleet's seen head.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    /// The day, as a person says it: `Tuesday`, `yesterday`.
    pub day: String,
    pub time: String,
    pub old: String,
    pub new: String,
    /// The commit he looked at is gone after a force push, so there is no range to read.
    pub gone: bool,
}

impl Seen {
    /// `on Tuesday at 14:10`, or `yesterday at 16:40`.
    pub fn when(&self) -> String {
        match self.day.as_str() {
            "today" | "yesterday" => format!("{} at {}", self.day, self.time),
            day => format!("on {day} at {}", self.time),
        }
    }

    /// The commits, `old → new`, said only in a tooltip.
    pub fn commits(&self) -> String {
        format!("{} → {}", self.old, self.new)
    }
}

/// Whether he marked a file viewed, from Fleet's viewed files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Viewed {
    No,
    Yes,
    /// He viewed it, then the author changed it: the box clears itself.
    ChangedAfter,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageFile {
    pub path: String,
    pub status: PullRequestFileStatus,
    pub additions: u64,
    pub deletions: u64,
    pub viewed: Viewed,
    /// The author changed it since he looked.
    pub since: bool,
    /// Its patch over the whole pull request.
    pub patch: String,
    /// Its patch since he looked, from Fleet's compare diff.
    pub patch_since: Option<String>,
}

/// An answer to a comment of his, since he looked.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub path: String,
    pub line: u64,
    pub author: String,
    pub body: String,
}

/// One thing a critique found, on one line.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub path: String,
    pub line: u64,
    /// One or two plain sentences: what is wrong, what to do.
    pub comment: String,
}

/// The range a critique reads: in words, and as commits for a tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    /// After "Critique of": `the 1 commit since you looked`.
    pub said: String,
    pub commits: String,
}

/// A critique of a range: Fleet's range critique, a review run.
#[derive(Clone, Debug, PartialEq)]
pub enum Critique {
    None,
    Running { range: Span },
    Finished { range: Span, findings: Vec<Finding> },
}

/// The written summary of the commits since he looked: written when he opens a pull request that
/// changed, and kept for its head.
#[derive(Clone, Debug, PartialEq)]
pub enum Summary {
    None,
    Writing,
    Written(String),
}

/// How serious a finding on the Overview is, which hues its card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagKind {
    Blocker,
    Risk,
    PossibleBug,
}

impl FlagKind {
    pub fn said(self) -> &'static str {
        match self {
            FlagKind::Blocker => "Blocker",
            FlagKind::Risk => "Risk",
            FlagKind::PossibleBug => "Possible bug",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            FlagKind::Blocker => "circle-x",
            FlagKind::Risk => "triangle-alert",
            FlagKind::PossibleBug => "circle-question-mark",
        }
    }
}

/// A blocker, risk or possible bug Find problems wrote, in the change set it sits in.
#[derive(Clone, Debug, PartialEq)]
pub struct Flag {
    pub id: String,
    pub kind: FlagKind,
    /// The change set's id.
    pub set: String,
    pub title: String,
    /// One plain sentence or two: why it matters.
    pub said: String,
    pub path: String,
    pub line: u64,
    /// The suggestion's title beside its hunk: what to do.
    pub fix: String,
    /// The suggestion's words, which become the draft comment.
    pub comment: String,
}

/// A change set's worst finding, as Fleet works it out: its glyph in the rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Blocked,
    Risky,
    Clear,
}

/// Whether a change set keeps one of the repository's practices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    Met,
    Missed,
    DoesNotApply,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Practice {
    pub said: String,
    pub kept: Kept,
}

/// One hunk of a change set, in the order the change runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Hunk {
    pub path: String,
    pub patch: String,
    /// The diff line a finding points at, counted from the hunk's first drawn line.
    pub hot: Option<usize>,
    /// The finding drawn beside it.
    pub flag: Option<String>,
}

/// Part of a pull request that does one thing, across the files it touches.
#[derive(Clone, Debug, PartialEq)]
pub struct ChangeSet {
    pub id: String,
    pub title: String,
    pub purpose: String,
    pub files: u32,
    pub mark: Mark,
    /// What kind of change it is, after "Practices for": `migrations`.
    pub kind: String,
    /// Where its practices were read from and when, as Armada keeps them.
    pub kept_note: String,
    pub practices: Vec<Practice>,
    pub hunks: Vec<Hunk>,
}

impl ChangeSet {
    /// `Adds a stage column. 2 files`
    pub fn line(&self) -> String {
        match self.files {
            1 => format!("{} 1 file", self.purpose),
            many => format!("{} {many} files", self.purpose),
        }
    }
}

/// What the Overview draws: why the author made it, what Find problems wrote, and its change sets.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Overview {
    pub why: String,
    pub flags: Vec<Flag>,
    pub sets: Vec<ChangeSet>,
}

/// What a change set's Explain tools are asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Explain {
    HowItWorks,
    HowToTest,
}

impl Explain {
    pub fn said(self) -> &'static str {
        match self {
            Explain::HowItWorks => "How it works",
            Explain::HowToTest => "How to test",
        }
    }
}

/// A pull request's page.
#[derive(Clone, Debug)]
pub struct Page {
    pub number: u64,
    pub repository: String,
    pub title: String,
    pub author: String,
    pub branch: String,
    pub base: String,
    /// The head's short sha.
    pub head: String,
    pub body: String,
    pub mine: Mine,
    /// Absent where he has never opened it.
    pub seen: Option<Seen>,
    /// Commits since he looked, newest first.
    pub new_commits: Vec<PullRequestCommit>,
    pub files: Vec<PageFile>,
    pub replies: Vec<Reply>,
    pub checks: Vec<PullRequestCheckRow>,
    pub conversation: Vec<PullRequestEntry>,
    pub threads: Vec<PullReviewThread>,
    pub critique: Critique,
    pub summary: Summary,
    pub overview: Overview,
    /// The Job that holds it, whose notes Send to job sends.
    pub holder: Option<String>,
    /// His notes for that Job, unsent.
    pub notes: Vec<String>,
    /// Drawn from what his last visit kept, until the refresh brings the fresh page: its checks, the
    /// commits since he looked, replies to him and the summary wait in grey.
    pub kept: bool,
    /// The refresh found it closed, or gone from the forge.
    pub ended: Option<Ended>,
}

/// How a kept pull request ended before he came back to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// Fleet's view says so; the mock forge closes nothing.
    #[allow(dead_code)]
    Closed,
    Gone,
}

impl Ended {
    /// What the page says in place of Your review, and its registry glyph.
    pub fn said(self) -> (&'static str, &'static str) {
        match self {
            Ended::Closed => ("git-pull-request-closed", "Closed"),
            Ended::Gone => ("circle-slash", "No longer on the forge"),
        }
    }
}

impl Page {
    /// Whether the author changed it since he looked: the page opens on What changed.
    pub fn changed(&self) -> bool {
        self.seen.as_ref().is_some_and(|seen| seen.gone || !self.new_commits.is_empty())
    }

    pub fn set(&self, id: &str) -> Option<&ChangeSet> {
        self.overview.sets.iter().find(|set| set.id == id)
    }

    pub fn flag(&self, id: &str) -> Option<&Flag> {
        self.overview.flags.iter().find(|flag| flag.id == id)
    }

    /// The range a critique reads: the author's changes since he looked, or else the whole pull request.
    pub fn range(&self) -> Span {
        let whose = possessive(&self.author);
        match self.seen.as_ref().filter(|seen| !seen.gone) {
            Some(seen) => Span { said: format!("{whose} changes since you looked"), commits: seen.commits() },
            None => Span { said: format!("{whose} changes in this pull request"), commits: format!("{} → {}", self.base, self.head) },
        }
    }
}

/// A draft comment on a line, written before Submit.
#[derive(Clone, Debug, PartialEq)]
pub struct DraftOf {
    pub id: String,
    pub number: u64,
    pub path: String,
    pub line: u64,
    pub body: String,
    /// The finding it came from.
    pub finding: Option<String>,
}

impl DraftOf {
    pub fn place(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }
}

/// `Mira's`, from the login `mira`; `your` for his own.
pub fn possessive(author: &str) -> String {
    if author == "you" {
        return "your".into();
    }
    let mut chars = author.chars();
    let first = chars.next().map(|first| first.to_uppercase().collect::<String>()).unwrap_or_default();
    format!("{first}{}'s", chars.as_str())
}

/// `Mira`, from the login `mira`.
pub fn named(author: &str) -> String {
    let mut chars = author.chars();
    let first = chars.next().map(|first| first.to_uppercase().collect::<String>()).unwrap_or_default();
    format!("{first}{}", chars.as_str())
}

/// `on Tuesday`, `today`, `yesterday`: a day after a verb.
pub fn on_day(day: &str) -> String {
    match day {
        "today" | "yesterday" => day.to_string(),
        day => format!("on {day}"),
    }
}

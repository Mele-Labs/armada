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

/// Where a review tool's answer goes: a problem found is a suggestion in Review comments, the rest
/// read under the tool's own row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    FindProblems,
    Explain,
    WalkThrough,
}

impl Section {
    pub const ALL: [Section; 3] = [Section::FindProblems, Section::Explain, Section::WalkThrough];

    pub fn said(self) -> &'static str {
        match self {
            Section::FindProblems => "Find problems",
            Section::Explain => "Explain",
            Section::WalkThrough => "Walk through",
        }
    }
}

/// One review tool, as `crates/config/review-tools` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tool {
    pub id: &'static str,
    pub name: &'static str,
    pub section: Section,
}

pub const TOOLS: [Tool; 9] = [
    Tool { id: "risks", name: "Risks", section: Section::FindProblems },
    Tool { id: "bugs", name: "Possible bugs", section: Section::FindProblems },
    Tool { id: "api-breaking-changes", name: "API breaks", section: Section::FindProblems },
    Tool { id: "how-it-works", name: "How it works", section: Section::Explain },
    Tool { id: "user-impact", name: "Who it affects", section: Section::Explain },
    Tool { id: "how-to-test", name: "How to test", section: Section::Explain },
    Tool { id: "why", name: "Why", section: Section::Explain },
    Tool { id: "walk", name: "Walk", section: Section::WalkThrough },
    Tool { id: "tour", name: "Tour", section: Section::WalkThrough },
];

/// A tool's last run on this pull request.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolRun {
    Running,
    /// `on` is whose changes it read, after "Ran on": `Tuesday's changes`. An explanation keeps its words.
    Ran { on: String, said: Option<String> },
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
    /// Each tool's last run, by its id; a tool not here has not run.
    pub tools: std::collections::HashMap<&'static str, ToolRun>,
    /// The Job that holds it, whose notes Send to job sends.
    pub holder: Option<String>,
    /// His notes for that Job, unsent.
    pub notes: Vec<String>,
}

impl Page {
    /// Whether the author changed it since he looked: the page opens on What changed.
    pub fn changed(&self) -> bool {
        self.seen.as_ref().is_some_and(|seen| seen.gone || !self.new_commits.is_empty())
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

/// `on Tuesday`, `today`, `yesterday`: a day after a verb.
pub fn on_day(day: &str) -> String {
    match day {
        "today" | "yesterday" => day.to_string(),
        day => format!("on {day}"),
    }
}

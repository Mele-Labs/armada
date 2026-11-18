//! What the Reviews screen draws, as one view model the mock fills today and Fleet's views will.
//!
//! **Shaped for Fleet's half**, which keeps the head he last saw, the files he marked viewed, the
//! inbox state of each pull request, the diff since he looked and the critique of a range. Each type
//! says which of those it stands for. Wire types are reused where they already fit.

use ipc::{PullRequestCheckRow, PullRequestCommit, PullRequestEntry, PullRequestFileStatus, PullRequestReviewer, PullRequestReviewerKind, PullReviewChecks, PullReviewList, PullReviewThread};

/// The inbox groups, in the order they are drawn. A row lands in the first that takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    Requested,
    LookAgain,
    Yours,
    Team,
    Done,
}

impl Group {
    pub const ALL: [Group; 5] = [Group::Requested, Group::LookAgain, Group::Yours, Group::Team, Group::Done];

    pub fn said(self) -> &'static str {
        match self {
            Group::Requested => "Review requested from you",
            Group::LookAgain => "You reviewed, look again",
            Group::Yours => "Yours",
            Group::Team => "Asked of your team",
            Group::Done => "Done",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Group::Requested => "eye",
            Group::LookAgain => "git-commit-horizontal",
            Group::Yours => "user",
            Group::Team => "users",
            Group::Done => "circle-check",
        }
    }

    /// The groups drawn folded until he opens them.
    pub fn starts_folded(self) -> bool {
        matches!(self, Group::Team | Group::Done)
    }
}

/// The group a pull request lands in: the first that takes it. `changed` is Fleet's inbox state
/// "changed since you looked".
pub fn group_of(lists: &[PullReviewList], mine: Verdict, changed: bool) -> Group {
    if lists.contains(&PullReviewList::Requested) {
        Group::Requested
    } else if mine != Verdict::NotReviewed && changed {
        Group::LookAgain
    } else if lists.contains(&PullReviewList::Mine) {
        Group::Yours
    } else if lists.contains(&PullReviewList::Team) {
        Group::Team
    } else {
        Group::Done
    }
}

/// The rows in the order the inbox draws them: group by group, each in the order it came, but
/// **Review requested from you has the one he has waited on longest first**.
pub fn in_order(rows: &[InboxRow]) -> Vec<&InboxRow> {
    let instant = |at: &str| chrono::DateTime::parse_from_rfc3339(at).ok();
    // A time that is missing or does not read goes after every one that does.
    let waited = |row: &InboxRow| {
        let asked = row.asked_at.as_deref().and_then(instant);
        let updated = instant(&row.updated_at);
        (asked.is_none(), asked, updated.is_none(), updated)
    };
    Group::ALL
        .iter()
        .flat_map(|group| {
            let mut rows: Vec<&InboxRow> = rows.iter().filter(|row| row.group() == *group).collect();
            if *group == Group::Requested {
                rows.sort_by_key(|row| waited(row));
            }
            rows
        })
        .collect()
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
    /// When it was updated, RFC 3339, as Fleet sent it.
    pub updated_at: String,
    /// When he was last asked to review it, RFC 3339, where Fleet knows.
    pub asked_at: Option<String>,
    /// Fleet's inbox state is changed since he looked.
    pub changed: bool,
    /// Commits pushed since he looked: the chip on a changed row.
    pub new_commits: Option<u32>,
    /// The forge lists it is on, which the filter picks between.
    pub lists: Vec<PullReviewList>,
    pub checks: PullReviewChecks,
    /// Where his own review stands.
    pub mine: Verdict,
}

impl InboxRow {
    pub fn group(&self) -> Group {
        group_of(&self.lists, self.mine, self.changed)
    }

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

/// Whether a finished review may say it found nothing: no findings, and no tool that failed, since
/// a run that failed read nothing.
pub fn found_nothing(findings: &[Finding], failed: &[String]) -> bool {
    findings.is_empty() && failed.is_empty()
}

/// A critique of a range: Fleet's range critique, a review run, with what he asked it to look for.
#[derive(Clone, Debug, PartialEq)]
pub enum Critique {
    None,
    Running { range: Span, asked: ReviewAsk },
    Finished { range: Span, findings: Vec<Finding>, asked: ReviewAsk },
}

/// A review tool that makes findings, which he can ask a review to look for. The mock names them
/// now; Fleet's review tools list them later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewTool {
    pub id: String,
    /// As its checkbox says it: `API breaking changes`.
    pub name: String,
    /// As a sentence says it after another: `API breaking changes`, `risks`.
    pub said: String,
    /// Ticked until he picks for himself.
    pub first: bool,
}

/// How hard a review looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Effort {
    Quick,
    #[default]
    Standard,
    Thorough,
}

impl Effort {
    pub const ALL: [Effort; 3] = [Effort::Quick, Effort::Standard, Effort::Thorough];

    pub fn name(self) -> &'static str {
        match self {
            Effort::Quick => "Quick",
            Effort::Standard => "Standard",
            Effort::Thorough => "Thorough",
        }
    }

    /// Its rough time and cost.
    pub fn costs(self) -> &'static str {
        match self {
            Effort::Quick => "About a minute, reads only the changed lines",
            Effort::Standard => "A few minutes, reads each changed file",
            Effort::Thorough => "Several minutes, reads around each change",
        }
    }
}

/// What he asks a review to look for, how hard, and anything to focus on: remembered per repository,
/// and the shape that goes to Fleet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewAsk {
    /// The ticked tools' ids, in the list's order.
    pub tools: Vec<String>,
    pub effort: Effort,
    pub focus: String,
}

impl ReviewAsk {
    /// The ask before he has made one: the tools ticked first, at Standard.
    pub fn first(tools: &[ReviewTool]) -> ReviewAsk {
        ReviewAsk { tools: tools.iter().filter(|tool| tool.first).map(|tool| tool.id.clone()).collect(), effort: Effort::default(), focus: String::new() }
    }

    pub fn has(&self, tool: &str) -> bool {
        self.tools.iter().any(|one| one == tool)
    }

    /// Ticks or unticks a tool, keeping the list's order.
    pub fn toggle(&mut self, tool: &str, list: &[ReviewTool]) {
        let on = !self.has(tool);
        self.tools = list.iter().filter(|one| if one.id == tool { on } else { self.has(&one.id) }).map(|one| one.id.clone()).collect();
    }

    /// What was asked, in words: `Possible bugs and risks, thorough`.
    pub fn said(&self, list: &[ReviewTool]) -> String {
        format!("{}, {}", self.what(list), self.effort.name().to_lowercase())
    }

    /// What was asked to look for, without how hard: `Possible bugs and risks`. Fleet takes no effort.
    pub fn what(&self, list: &[ReviewTool]) -> String {
        let named: Vec<&str> = list.iter().filter(|tool| self.has(&tool.id)).map(|tool| tool.said.as_str()).collect();
        let what = match named.as_slice() {
            [] => String::new(),
            [one] => (*one).to_string(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        };
        let mut chars = what.chars();
        chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
    }
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

    /// `1 possible bug`, `2 risks`.
    pub fn counted(self, count: usize) -> String {
        let (one, many) = match self {
            FlagKind::Blocker => ("blocker", "blockers"),
            FlagKind::Risk => ("risk", "risks"),
            FlagKind::PossibleBug => ("possible bug", "possible bugs"),
        };
        format!("{count} {}", if count == 1 { one } else { many })
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

/// What the findings still open on a change set come to: its glyph in the rail and on its Overview row.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Mark {
    /// The worst finding's kind; None when Find problems found nothing in the set.
    pub worst: Option<FlagKind>,
    /// Its findings in words, worst first: `1 blocker and 2 risks`. Empty when there are none.
    pub said: String,
}

impl Mark {
    /// What the findings on set `set` come to.
    pub fn of(set: &str, flags: &[&Flag]) -> Mark {
        let counts: Vec<String> = [FlagKind::Blocker, FlagKind::Risk, FlagKind::PossibleBug]
            .into_iter()
            .filter_map(|kind| {
                let count = flags.iter().filter(|flag| flag.set == set && flag.kind == kind).count();
                (count > 0).then(|| kind.counted(count))
            })
            .collect();
        let worst = [FlagKind::Blocker, FlagKind::Risk, FlagKind::PossibleBug].into_iter().find(|kind| flags.iter().any(|flag| flag.set == set && flag.kind == *kind));
        let said = match counts.as_slice() {
            [] => String::new(),
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        };
        Mark { worst, said }
    }
}

/// Whether a change set keeps one of the repository's practices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    Met,
    Missed,
    DoesNotApply,
}

impl Kept {
    /// The result the other way round, as "Check is wrong" turns it. Does not apply stays.
    pub fn flipped(self) -> Kept {
        match self {
            Kept::Met => Kept::Missed,
            Kept::Missed => Kept::Met,
            Kept::DoesNotApply => Kept::DoesNotApply,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Practice {
    pub said: String,
    pub kept: Kept,
}

/// What he says is wrong with a practice's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrong {
    /// It does not belong in this area's checklist: it folds into the set-aside line.
    DoesNotApply,
    /// The result is the reverse: met is unmet, unmet is met.
    CheckIsWrong,
}

impl Wrong {
    pub fn said(self) -> &'static str {
        match self {
            Wrong::DoesNotApply => "Doesn't apply here",
            Wrong::CheckIsWrong => "Check is wrong",
        }
    }

    /// What the reason field asks.
    pub fn asks(self) -> &'static str {
        match self {
            Wrong::DoesNotApply => "Why it does not apply here",
            Wrong::CheckIsWrong => "Why the check is wrong",
        }
    }
}

/// A practice by pull request, change set and its place in the set's checklist.
pub type PracticeAt = (u64, String, usize);

/// His correction of one practice and his reason. The mock keeps it in the screen's state; this is
/// the shape Fleet will store.
#[derive(Clone, Debug, PartialEq)]
pub struct Correction {
    pub wrong: Wrong,
    pub reason: String,
}

/// One hunk of a change set, in the order the change runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Hunk {
    pub path: String,
    /// One sentence under its path: what this part does, and why it comes here in the order.
    pub caption: String,
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
    /// Find problems has read this commit. The mock's own; the live page asks Fleet's runs.
    pub read: bool,
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
    /// Who is asked to review, and who reviewed unasked, as the forge lists them.
    pub reviewers: Vec<PullRequestReviewer>,
    /// Where it is on the forge. Empty where Fleet did not say.
    pub address: String,
    pub draft: bool,
}

/// Where the pull request stands on the forge, as the head's state chip says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullState {
    Open,
    Draft,
    Closed,
    Gone,
}

impl PullState {
    /// How it ended wins over whether it was a draft.
    pub fn of(draft: bool, ended: Option<Ended>) -> PullState {
        match (ended, draft) {
            (Some(Ended::Closed), _) => PullState::Closed,
            (Some(Ended::Gone), _) => PullState::Gone,
            (None, true) => PullState::Draft,
            (None, false) => PullState::Open,
        }
    }

    pub fn said(self) -> &'static str {
        match self {
            PullState::Open => "Open",
            PullState::Draft => "Draft",
            PullState::Closed => "Closed",
            PullState::Gone => "No longer on the forge",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            PullState::Open => "git-pull-request",
            PullState::Draft => "git-pull-request-draft",
            PullState::Closed => "git-pull-request-closed",
            PullState::Gone => "circle-slash",
        }
    }
}

/// Who is still asked to review, as Your review says it: whether he is, and the others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    pub you: bool,
    /// `Also asked: tobias, @payments-team` beside his own chip, `Asked: …` without it. None when
    /// nobody else is asked.
    pub others: Option<String>,
}

/// Still asked is a reviewer with no review: the forge drops a person from its requests once they
/// review, and a team never has one. `me` is his own login.
pub fn asked(reviewers: &[PullRequestReviewer], me: &str) -> Asked {
    // A person who reviewed is no longer asked; a team never answers, so it stays asked.
    let still: Vec<&PullRequestReviewer> = reviewers.iter().filter(|reviewer| reviewer.verdict.is_none()).collect();
    let mine = |reviewer: &&PullRequestReviewer| reviewer.kind == PullRequestReviewerKind::Person && reviewer.login.eq_ignore_ascii_case(me);
    let you = still.iter().any(mine);
    let others: Vec<String> = still
        .iter()
        .filter(|reviewer| !mine(reviewer))
        .map(|reviewer| match reviewer.kind {
            PullRequestReviewerKind::Team => format!("@{}", reviewer.login),
            PullRequestReviewerKind::Person => reviewer.login.clone(),
        })
        .collect();
    let lead = if you { "Also asked" } else { "Asked" };
    Asked { you, others: (!others.is_empty()).then(|| format!("{lead}: {}", others.join(", "))) }
}

/// The page's files under their folders, folders in path order and files by name within each. A
/// file at the top of the repository is under the folder `""`.
pub fn by_folder(files: &[PageFile]) -> Vec<(String, Vec<&PageFile>)> {
    let mut folders: std::collections::BTreeMap<&str, Vec<&PageFile>> = std::collections::BTreeMap::new();
    for file in files {
        let folder = file.path.rsplit_once('/').map_or("", |(folder, _)| folder);
        folders.entry(folder).or_default().push(file);
    }
    folders
        .into_iter()
        .map(|(folder, mut files)| {
            files.sort_by(|a, b| a.path.cmp(&b.path));
            (folder.to_string(), files)
        })
        .collect()
}

/// What the description's hover buttons ask of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rewrite {
    Summarize,
    Explain,
    Simplify,
}

impl Rewrite {
    pub const ALL: [Rewrite; 3] = [Rewrite::Summarize, Rewrite::Explain, Rewrite::Simplify];

    /// As its button says it.
    pub fn name(self) -> &'static str {
        match self {
            Rewrite::Summarize => "Summarize",
            Rewrite::Explain => "Explain",
            Rewrite::Simplify => "Simplify",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Rewrite::Summarize => "scroll-text",
            Rewrite::Explain => "circle-question-mark",
            Rewrite::Simplify => "sliders-horizontal",
        }
    }

    /// The small label over its answer.
    pub fn label(self) -> &'static str {
        match self {
            Rewrite::Summarize => "Summary",
            Rewrite::Explain => "Explanation",
            Rewrite::Simplify => "Simpler",
        }
    }

    /// The question Fleet's Ask run is put, in words it keeps lower-cased.
    pub fn question(self) -> &'static str {
        match self {
            Rewrite::Summarize => "summarize this pull request's description in two or three sentences, in markdown",
            Rewrite::Explain => "explain this pull request's description to a reader new to this code: what it changes and why, in markdown",
            Rewrite::Simplify => "rewrite this pull request's description in plain, simpler words, keeping every fact, in markdown",
        }
    }
}

/// An answer to a question put to the pull request, while it is written and once it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Writing,
    Written(String),
    Failed(String),
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
    /// The diff lines it sits on, as [`excerpt`] cuts them. None when no hunk holds its line.
    pub excerpt: Option<Vec<crate::diff::DiffLine>>,
}

impl DraftOf {
    pub fn place(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }
}

/// The lines a draft comment sits on, as a thread's excerpt draws them: up to four ending at its
/// line, under their hunk's head. None where no hunk of the file reaches it.
pub fn excerpt(files: &[PageFile], path: &str, line: u64) -> Option<Vec<crate::diff::DiffLine>> {
    crate::diff::excerpt(&files.iter().find(|file| file.path == path)?.patch, line, false)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn flagged(set: &str, kind: FlagKind) -> Flag {
        Flag { id: "f".into(), kind, set: set.into(), title: String::new(), said: String::new(), path: String::new(), line: 0, fix: String::new(), comment: String::new() }
    }

    #[test]
    fn a_review_that_failed_never_reads_as_one_that_found_nothing() {
        let finding = Finding { id: "f".into(), title: "t".into(), path: "a.rs".into(), line: 1, comment: "c".into() };
        let failed = vec!["risks failed: the answer was refused".to_string()];
        assert!(found_nothing(&[], &[]), "a clean run with no findings found nothing");
        assert!(!found_nothing(&[], &failed), "a failed run found nothing to report, which is not nothing");
        assert!(!found_nothing(&[finding.clone()], &[]));
        assert!(!found_nothing(&[finding], &failed));
    }

    #[test]
    fn a_review_ask_says_what_it_looks_for_and_how_hard() {
        let tool = |id: &str, said: &str, first: bool| ReviewTool { id: id.into(), name: said.into(), said: said.into(), first };
        let list = [tool("bugs", "possible bugs", true), tool("risks", "risks", true), tool("api", "API breaking changes", false)];
        let mut ask = ReviewAsk::first(&list);
        assert_eq!(ask.said(&list), "Possible bugs and risks, standard");
        ask.toggle("api", &list);
        ask.effort = Effort::Thorough;
        assert_eq!(ask.tools, ["bugs", "risks", "api"]);
        assert_eq!(ask.said(&list), "Possible bugs, risks and API breaking changes, thorough");
        ask.toggle("bugs", &list);
        assert_eq!(ask.said(&list), "Risks and API breaking changes, thorough");
    }

    #[test]
    fn a_set_is_marked_by_its_worst_finding_and_counts_each_kind() {
        let flags = [flagged("a", FlagKind::Risk), flagged("a", FlagKind::Blocker), flagged("a", FlagKind::Risk), flagged("b", FlagKind::PossibleBug)];
        let flags: Vec<&Flag> = flags.iter().collect();
        assert_eq!(Mark::of("a", &flags), Mark { worst: Some(FlagKind::Blocker), said: "1 blocker and 2 risks".into() });
        assert_eq!(Mark::of("b", &flags), Mark { worst: Some(FlagKind::PossibleBug), said: "1 possible bug".into() });
        assert_eq!(Mark::of("c", &flags), Mark::default());
    }

    #[test]
    fn the_state_chip_says_how_it_ended_before_whether_it_is_a_draft() {
        assert_eq!(PullState::of(false, None), PullState::Open);
        assert_eq!(PullState::of(true, None), PullState::Draft);
        assert_eq!(PullState::of(true, Some(Ended::Closed)), PullState::Closed);
        assert_eq!(PullState::of(false, Some(Ended::Gone)), PullState::Gone);
    }

    fn reviewer(login: &str, kind: PullRequestReviewerKind, verdict: Option<ipc::PullRequestVerdict>) -> PullRequestReviewer {
        PullRequestReviewer { login: login.into(), kind, verdict, name: None, url: None, team: None }
    }

    #[test]
    fn the_asked_line_names_the_others_still_asked_and_marks_a_team() {
        use PullRequestReviewerKind::{Person, Team};
        let reviewers = [
            reviewer("Wren", Person, None),
            reviewer("tobias", Person, None),
            reviewer("jun", Person, Some(ipc::PullRequestVerdict::Approved)),
            reviewer("payments-team", Team, None),
        ];
        assert_eq!(asked(&reviewers, "wren"), Asked { you: true, others: Some("Also asked: tobias, @payments-team".into()) });
        assert_eq!(asked(&reviewers, "kit"), Asked { you: false, others: Some("Asked: Wren, tobias, @payments-team".into()) });
        assert_eq!(asked(&reviewers[..1], "wren"), Asked { you: true, others: None });
        assert_eq!(asked(&reviewers[2..3], "wren"), Asked { you: false, others: None });
    }

    #[test]
    fn files_sit_under_their_folders_in_path_order() {
        let file = |path: &str| PageFile {
            path: path.into(),
            status: PullRequestFileStatus::Modified,
            additions: 1,
            deletions: 0,
            viewed: Viewed::No,
            since: false,
            patch: String::new(),
            patch_since: None,
        };
        let files = [file("src/jobs/retry.rs"), file("README.md"), file("src/db/schema.rs"), file("src/jobs/invoice.rs")];
        let folders: Vec<(String, Vec<&str>)> = by_folder(&files).into_iter().map(|(folder, files)| (folder, files.iter().map(|file| file.path.as_str()).collect())).collect();
        assert_eq!(
            folders,
            [
                (String::new(), vec!["README.md"]),
                ("src/db".to_string(), vec!["src/db/schema.rs"]),
                ("src/jobs".to_string(), vec!["src/jobs/invoice.rs", "src/jobs/retry.rs"]),
            ]
        );
    }
}

#[cfg(test)]
mod group_tests {
    use super::*;
    use PullReviewList::*;

    #[test]
    fn requested_comes_first_even_when_he_reviewed_and_it_changed() {
        assert_eq!(group_of(&[Requested, Mine], Verdict::Approved, true), Group::Requested);
        assert_eq!(group_of(&[Requested], Verdict::NotReviewed, false), Group::Requested);
    }

    #[test]
    fn reviewed_and_changed_is_look_again_unless_requested() {
        assert_eq!(group_of(&[Assigned], Verdict::ChangesRequested, true), Group::LookAgain);
        assert_eq!(group_of(&[Mine, Team], Verdict::Commented, true), Group::LookAgain);
        assert_eq!(group_of(&[Assigned], Verdict::NotReviewed, true), Group::Done);
        assert_eq!(group_of(&[Assigned], Verdict::Approved, false), Group::Done);
    }

    #[test]
    fn mine_then_team_then_done() {
        assert_eq!(group_of(&[Mine], Verdict::NotReviewed, false), Group::Yours);
        assert_eq!(group_of(&[Team, Mine], Verdict::NotReviewed, false), Group::Yours);
        assert_eq!(group_of(&[Team], Verdict::NotReviewed, false), Group::Team);
        assert_eq!(group_of(&[Team, Assigned], Verdict::Approved, false), Group::Team);
        assert_eq!(group_of(&[Assigned], Verdict::NotReviewed, false), Group::Done);
    }

    fn row(number: u64, lists: &[PullReviewList], updated_at: &str, asked_at: Option<&str>) -> InboxRow {
        InboxRow {
            number,
            repository: "acme/billing".into(),
            title: format!("Pull {number}"),
            author: "tobias".into(),
            age: String::new(),
            updated_at: updated_at.into(),
            asked_at: asked_at.map(str::to_string),
            changed: false,
            new_commits: None,
            lists: lists.to_vec(),
            checks: PullReviewChecks::Passed,
            mine: Verdict::NotReviewed,
        }
    }

    fn numbers(rows: &[InboxRow]) -> Vec<u64> {
        in_order(rows).iter().map(|row| row.number).collect()
    }

    #[test]
    fn review_requested_from_you_puts_the_longest_wait_first() {
        let rows = [
            row(1, &[Requested], "2026-11-16T12:00:00Z", Some("2026-11-16T09:00:00Z")),
            row(2, &[Requested], "2026-11-10T12:00:00Z", None),
            row(3, &[Requested], "2026-11-16T12:00:00Z", Some("2026-11-12T09:00:00+02:00")),
            row(4, &[Requested], "2026-11-09T12:00:00Z", None),
            row(5, &[Requested], "2026-11-16T12:00:00Z", Some("2026-11-12T08:00:00Z")),
        ];
        // #3 was asked at 07:00 UTC, before #5; an unknown time goes after, the oldest update first.
        assert_eq!(numbers(&rows), [3, 5, 1, 4, 2]);
    }

    #[test]
    fn a_time_that_does_not_read_is_no_time_and_other_groups_keep_their_order() {
        let rows = [
            row(10, &[Mine], "2026-11-01T00:00:00Z", None),
            row(11, &[Requested], "2026-11-16T12:00:00Z", Some("yesterday")),
            row(12, &[Requested], "2026-11-16T12:00:00Z", Some("2026-11-15T09:00:00Z")),
            row(13, &[Mine], "2026-10-01T00:00:00Z", None),
            row(14, &[Team], "2026-11-16T12:00:00Z", Some("2026-11-01T09:00:00Z")),
            row(15, &[Team], "2026-11-16T12:00:00Z", Some("2026-10-01T09:00:00Z")),
        ];
        assert_eq!(numbers(&rows), [12, 11, 10, 13, 14, 15]);
    }

    #[test]
    fn team_and_done_start_folded() {
        let folded: Vec<Group> = Group::ALL.into_iter().filter(|group| group.starts_folded()).collect();
        assert_eq!(folded, [Group::Team, Group::Done]);
    }
}

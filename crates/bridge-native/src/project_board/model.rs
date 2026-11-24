//! What each block draws, worked out from what Fleet said. A block's rows are [`Item`]s whatever the block is, so List, Table and
//! Cards are drawn once each (`blocks.rs`) and a block's own code is only the function that turns its data into items.
//! Nothing here touches the screen or the wire.

use chrono::{DateTime, Utc};
use ipc::{
    AppRun, AppRunDraft, AppRunState, JobLamp, LookedTag, ManifestConfig, ProjectOverview, RankedIssue, RankedPull, SessionRecord, SessionState, SinceYouLooked,
    TrackedIssue,
};

use super::layout::BlockKind;
use crate::look::{token, Token};
use crate::reviews::model::InboxRow;

/// A repository of the project, by Fleet's manifest id and the name the picker gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    pub id: String,
    pub name: String,
}

/// The project the board is of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub repositories: Vec<Repository>,
}

impl Project {
    pub fn holds(&self, manifest: &str) -> bool {
        self.repositories.iter().any(|one| one.id == manifest)
    }

    pub fn repository_name(&self, manifest: &str) -> Option<&str> {
        self.repositories.iter().find(|one| one.id == manifest).map(|one| one.name.as_str())
    }
}

/// What a press on an item or one of its buttons asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Press {
    /// A Job opens on the Jobs screen.
    Job(String),
    /// The Operator's page comes to the front.
    Operator,
    Pull { repository: String, number: u64 },
    /// An address opens in the browser.
    Url(String),
    /// A feature opens on the Features tab.
    Feature(String),
    /// A run, started on the checkout the board is on.
    RunStart { manifest: String, name: String },
    RunStop { manifest: String, name: String },
    /// The tail of a server's output.
    Logs { server: String, title: String },
    /// Fleet proposes a run for a repository that declares none.
    Draft { manifest: String },
    /// One Check or Command of the manifest, run in the main checkout. A Command flagged `confirm: true` asks first.
    Command { manifest: String, name: String, confirm: bool },
}

/// A button on a row. A row's buttons stand in fixed columns, so a row without one leaves its place empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    pub glyph: &'static str,
    /// What the tooltip says and VoiceOver reads.
    pub name: String,
    pub press: Press,
}

/// One row of a block, drawn as a line, a table row or a card.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// What its element id is made of.
    pub key: String,
    /// The glyph that says what the row is or the state it is in, in its hue, with its words in the tooltip.
    pub glyph: &'static str,
    pub hue: Token,
    pub said: String,
    /// The glyph turns: the row is on its way to another state.
    pub turning: bool,
    pub title: String,
    /// The quiet words at the right: an age, a port, who it is for.
    pub meta: Option<String>,
    /// A chip with a fill: what kind of row it is.
    pub tag: Option<(String, Token)>,
    pub press: Option<Press>,
    /// A heading drawn above this row and the rows after it, up to the next that has one.
    pub section: Option<String>,
    /// One slot for each column of buttons the block has.
    pub buttons: Vec<Option<Button>>,
    /// The librarian's reason for the row's place, a line under its title, where it ranked the list.
    pub why: Option<String>,
}

impl Item {
    pub(super) fn new(key: impl Into<String>, glyph: &'static str, hue: Token, said: impl Into<String>, title: impl Into<String>) -> Item {
        Item { key: key.into(), glyph, hue, said: said.into(), turning: false, title: title.into(), meta: None, tag: None, press: None, section: None, buttons: Vec::new(), why: None }
    }

    pub(super) fn meta(mut self, meta: impl Into<String>) -> Item {
        let meta: String = meta.into();
        self.meta = Some(meta).filter(|meta| !meta.is_empty());
        self
    }

    pub(super) fn tag(mut self, words: impl Into<String>, hue: Token) -> Item {
        self.tag = Some((words.into(), hue));
        self
    }

    pub(super) fn press(mut self, press: Press) -> Item {
        self.press = Some(press);
        self
    }

    pub(super) fn why(mut self, why: Option<String>) -> Item {
        self.why = why.filter(|why| !why.is_empty());
        self
    }

    pub(super) fn section(mut self, heading: impl Into<String>) -> Item {
        self.section = Some(heading.into());
        self
    }

    pub(super) fn buttons(mut self, buttons: Vec<Option<Button>>) -> Item {
        self.buttons = buttons;
        self
    }

    /// The name VoiceOver reads for the row.
    pub fn name(&self) -> String {
        match &self.meta {
            Some(meta) => format!("{}, {meta}", self.title),
            None => self.title.clone(),
        }
    }
}

/// One room of the code map, drawn in the Features block's Picture.
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub module: String,
    pub kind: String,
    pub hue: Token,
}

/// What a block has to draw.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub items: Vec<Item>,
    pub tiles: Vec<Tile>,
}

/// A read that may not have answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    Reading,
    Failed(String),
    Ready(Shown),
}

impl Fill {
    pub fn items(items: Vec<Item>) -> Fill {
        Fill::Ready(Shown { items, tiles: Vec::new() })
    }
}

/// `4m`, `3h`, `2d`: how long since `at`, as RFC 3339.
pub fn age(at: &str, now: DateTime<Utc>) -> Option<String> {
    let then = DateTime::parse_from_rfc3339(at).ok()?;
    let minutes = (now - then.with_timezone(&Utc)).num_minutes().max(0);
    Some(match minutes {
        0 => "now".into(),
        1..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h", minutes / 60),
        _ => format!("{}d", minutes / 1440),
    })
}

// ---- Needs you -------------------------------------------------------------------------------------------------------

/// The Jobs waiting on him in this project, then the Operator's cards that no listed Job stands for. A Job is in the project
/// Fleet says it was started for, else the one that holds its repository.
pub fn needs(project: &Project, jobs: &[SessionRecord], cards: &[ipc::operator_page::OperatorTurnItem], now: DateTime<Utc>) -> Vec<Item> {
    let in_project = |job: &SessionRecord| match job.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()) {
        Some(id) => id == project.id,
        None => job.manifest_id.as_ref().is_some_and(|id| project.holds(id.as_str())),
    };
    let waiting: Vec<&SessionRecord> = jobs
        .iter()
        .filter(|job| job.state == SessionState::Live && job.snoozed_until.is_none() && job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs) && in_project(job))
        .collect();
    let mut rows: Vec<Item> = waiting
        .iter()
        .map(|job| {
            let item = job.waiting_for.iter().find(|item| item.source != ipc::WaitingSource::Done);
            let since = item.and_then(|item| age(item.since.as_str(), now));
            let title = crate::jobs::titled(job.title.as_deref());
            Item::new(format!("job-{}", job.id.as_str()), "bell-ring", token::YOU, "Needs you", title)
                .meta(item.map(|item| item.text.clone()).unwrap_or_default())
                .tag(since.map_or_else(|| "Job".to_string(), |since| format!("Job, {since}")), token::YOU)
                .press(Press::Job(job.id.as_str().to_string()))
        })
        .collect();
    for card in cards {
        let mine = card.manifest_id.as_deref().is_some_and(|id| project.holds(id)) || card.session_id.as_ref().is_some_and(|id| waiting.iter().any(|job| job.id == *id));
        let stood_for = card.session_id.as_ref().is_some_and(|id| waiting.iter().any(|job| job.id == *id));
        if !mine || stood_for {
            continue;
        }
        let title = crate::operator::face::card_title(card).to_string();
        rows.push(
            Item::new(format!("card-{}", card.id), "bot", token::OPERATOR, "Operator card", title)
                .meta(age(card.raised_at.as_str(), now).unwrap_or_default())
                .tag("Card", token::OPERATOR)
                .press(Press::Operator),
        );
    }
    rows
}

// ---- Run -------------------------------------------------------------------------------------------------------------

/// The glyph, the hue and the words of a process's state.
fn state_of(state: AppRunState) -> (&'static str, Token, &'static str, bool) {
    match state {
        AppRunState::Ready => ("circle-dot", token::RUN_SERVING, "Serving", false),
        AppRunState::Starting => ("loader-circle", token::RUN_STARTING, "Starting", true),
        AppRunState::Failed => ("circle-x", token::RUN_FAILED, "Failed", false),
        AppRunState::Exited => ("circle-dashed", token::RUN_STOPPED, "Stopped", false),
        AppRunState::NotStarted => ("circle-dashed", token::RUN_STOPPED, "Not started", false),
    }
}

/// A process of each run, with its state, its port and the buttons Open, Logs and Stop (Start where it is not running), in
/// those three columns.
pub fn run_rows(project: &Project, manifest: &str, runs: &[AppRun]) -> Vec<Item> {
    let many = project.repositories.len() > 1;
    let repository = project.repository_name(manifest).unwrap_or_default().to_string();
    let mut rows = Vec::new();
    for run in runs {
        let up = |state: AppRunState| matches!(state, AppRunState::Ready | AppRunState::Starting);
        for process in &run.processes {
            let (glyph, hue, said, turning) = state_of(process.state);
            let mut item = Item::new(format!("run-{manifest}-{}-{}", run.name, process.name), glyph, hue, said, process.name.clone());
            item.turning = turning;
            let port = process.port.map(|port| format!(":{port}"));
            item = item.meta(port.unwrap_or_default());
            let label = if many { format!("{repository}, {}", run.label) } else { run.label.clone() };
            item = item.tag(label, token::SKETCH_GREEN);
            let open = process.links.first().filter(|_| process.state == AppRunState::Ready).map(|link| Button {
                glyph: "square-arrow-out-up-right",
                name: format!("Open {}", link.name.clone().unwrap_or_else(|| process.name.clone())),
                press: Press::Url(link.url.clone()),
            });
            let logs = process.server_id.as_ref().map(|server| Button { glyph: "scroll-text", name: format!("Logs of {}", process.name), press: Press::Logs { server: server.clone(), title: process.name.clone() } });
            let last = if up(process.state) {
                Button { glyph: "square", name: format!("Stop {}", run.label), press: Press::RunStop { manifest: manifest.to_string(), name: run.name.clone() } }
            } else {
                Button { glyph: "play", name: format!("Start {}", run.label), press: Press::RunStart { manifest: manifest.to_string(), name: run.name.clone() } }
            };
            rows.push(item.buttons(vec![open, logs, Some(last)]));
        }
    }
    rows
}

/// The processes Fleet proposes for a repository that declares no run, as rows to read. Nothing is saved.
pub fn draft_rows(manifest: &str, draft: &AppRunDraft) -> Vec<Item> {
    draft.processes.iter().map(|process| Item::new(format!("draft-{manifest}-{}", process.name), "square-terminal", token::SKETCH_GREEN, "Proposed, not saved", process.name.clone()).meta(process.command.clone()).tag("Draft", token::SKETCH_GREEN)).collect()
}

/// The row that offers the draft where a repository declares no run.
pub fn offer_draft(project: &Project, manifest: &str) -> Item {
    let name = project.repository_name(manifest).unwrap_or("Repository").to_string();
    Item::new(format!("offer-{manifest}"), "square-terminal", token::RUN_STOPPED, "No run declared", name)
        .tag("No run", token::RUN_STOPPED)
        .buttons(vec![None, None, Some(Button { glyph: "pencil-line", name: "Draft a run".into(), press: Press::Draft { manifest: manifest.to_string() } })])
}

// ---- Reviews ---------------------------------------------------------------------------------------------------------

/// Requested of him, assigned to him and his own, each pull request once under the first list it is on.
pub fn reviews(rows: &[(Option<String>, &InboxRow)], project: &Project) -> Vec<Item> {
    use ipc::PullReviewList::{Assigned, Mine, Requested};
    let mut items = Vec::new();
    for (list, words, hue, glyph) in [(Requested, "Requested", token::SKETCH_AMBER, "git-pull-request-arrow"), (Assigned, "Assigned", token::SKETCH_BLUE, "git-pull-request"), (Mine, "Mine", token::SKETCH_TEAL, "git-pull-request")] {
        for (manifest, row) in rows {
            let first = [Requested, Assigned, Mine].into_iter().find(|one| row.lists.contains(one));
            let in_project = manifest.as_deref().is_none_or(|id| project.holds(id));
            if first != Some(list) || !in_project || row.ignored {
                continue;
            }
            let age = row.age.clone();
            items.push(
                Item::new(format!("pull-{}", row.number), glyph, hue, words, format!("#{} {}", row.number, row.title))
                    .meta(if list == Requested { row.author.clone() } else { age })
                    .tag(words, hue)
                    .press(Press::Pull { repository: row.repository.clone(), number: row.number }),
            );
        }
    }
    items
}

// ---- Issues ----------------------------------------------------------------------------------------------------------

/// The features a ranked row matched, by name, for the column that names them.
fn feature_names(features: &[ipc::MatchedFeature]) -> String {
    features.iter().map(|one| one.title.as_str()).collect::<Vec<_>>().join(", ")
}

/// The issues assigned to him, then the ones Fleet ranked as ones he might pick up, in its order and each once. The chip says
/// which kind a row is, and the column beside it names the features a ranked one matched.
pub fn issues(mine: &[TrackedIssue], ranked: &[RankedIssue]) -> Vec<Item> {
    let mut items: Vec<Item> = mine
        .iter()
        .map(|issue| Item::new(format!("issue-{}", issue.reference), "ticket", token::SKETCH_TEAL, "Assigned to you", format!("{} {}", issue.reference, issue.title)).tag("Yours", token::SKETCH_TEAL).press(Press::Url(issue.url.clone())))
        .collect();
    for one in ranked.iter().filter(|one| !mine.iter().any(|mine| mine.reference == one.issue.reference)) {
        let issue = &one.issue;
        items.push(
            Item::new(format!("issue-{}", issue.reference), "ticket", token::SKETCH_GREY, "Open and unassigned", format!("{} {}", issue.reference, issue.title))
                .meta(feature_names(&one.features))
                .tag("Might pick up", token::SKETCH_GREY)
                .press(Press::Url(issue.url.clone()))
                .why(one.why.clone()),
        );
    }
    items
}

/// The pull requests of other people Fleet ranked, each once, as the second section of Reviews. One a list of Reviews already
/// holds is left to that list.
pub fn interested(pulls: &[RankedPull], held: &[Item]) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for pull in pulls.iter().filter(|pull| !held.iter().any(|one| one.key == format!("pull-{}", pull.number))) {
        let item = Item::new(format!("interested-{}", pull.number), "git-pull-request", token::SKETCH_GREY, "Open pull request of someone else", format!("#{} {}", pull.number, pull.title))
            .meta(feature_names(&pull.features))
            .tag("Might be interested", token::SKETCH_GREY)
            .press(Press::Pull { repository: pull.repository.clone(), number: pull.number });
        items.push(if items.is_empty() { item.section("Might be interested") } else { item });
    }
    items
}

// ---- Features --------------------------------------------------------------------------------------------------------

/// What moved since he looked, in Fleet's order, and the code map's rooms for the Picture.
pub fn features(since: Option<&SinceYouLooked>, overview: Option<&ProjectOverview>) -> Shown {
    let items = since
        .map(|read| {
            read.features
                .iter()
                .map(|one| {
                    let (words, hue) = crate::features::tag_mark(one.tag);
                    let glyph = match one.tag {
                        LookedTag::New => "circle-plus",
                        LookedTag::Changed => "git-merge",
                        LookedTag::NeedsYou => "bell-ring",
                    };
                    Item::new(format!("feature-{}", one.slug), glyph, hue, words, one.title.clone()).meta(one.what.clone()).tag(words, hue).press(Press::Feature(one.slug.clone()))
                })
                .collect()
        })
        .unwrap_or_default();
    let hues = [token::SKETCH_BLUE, token::SKETCH_TEAL, token::SKETCH_GREEN, token::SKETCH_AMBER, token::SKETCH_ORANGE, token::SKETCH_RED, token::ASK_FEATURES];
    let mut areas: Vec<&str> = Vec::new();
    let tiles = overview
        .map(|read| {
            read.rooms
                .iter()
                .map(|room| {
                    let hue = match room.area.as_deref() {
                        Some(area) => {
                            let at = areas.iter().position(|one| *one == area).unwrap_or_else(|| {
                                areas.push(area);
                                areas.len() - 1
                            });
                            hues[at % hues.len()]
                        }
                        None => token::SKETCH_GREY,
                    };
                    Tile { module: room.module.clone(), kind: room.kind.clone(), hue }
                })
                .collect()
        })
        .unwrap_or_default();
    Shown { items, tiles }
}

// ---- Checks and commands ---------------------------------------------------------------------------------------------

/// The Checks, Commands and Servers the manifest declares, each Check and Command with a Run button.
pub fn checks(project: &Project, manifests: &[ManifestConfig]) -> Vec<Item> {
    let many = project.repositories.len() > 1;
    let mut items = Vec::new();
    for config in manifests {
        let manifest = config.id.as_str().to_string();
        let of = if many { project.repository_name(&manifest).map(str::to_string) } else { None };
        let run = |name: &str, confirm: bool| Some(Button { glyph: "play", name: format!("Run {name}"), press: Press::Command { manifest: manifest.clone(), name: name.to_string(), confirm } });
        for name in &config.checks_as_written {
            items.push(Item::new(format!("check-{manifest}-{name}"), "list-checks", token::SKETCH_GREY, "Check", name.clone()).meta(of.clone().unwrap_or_default()).tag("Check", token::SKETCH_GREY).buttons(vec![run(name, false)]));
        }
        for name in &config.commands {
            items.push(Item::new(format!("command-{manifest}-{name}"), "terminal", token::SKETCH_BLUE, "Command", name.clone()).meta(of.clone().unwrap_or_default()).tag("Command", token::SKETCH_BLUE).buttons(vec![run(name, config.confirm_commands.contains(name))]));
        }
        for name in &config.servers {
            items.push(Item::new(format!("server-{manifest}-{name}"), "square-terminal", token::SKETCH_GREEN, "Server", name.clone()).meta(of.clone().unwrap_or_default()).tag("Server", token::SKETCH_GREEN).buttons(vec![None]));
        }
        for name in &config.ports {
            items.push(Item::new(format!("port-{manifest}-{name}"), "circle-dot", token::SKETCH_TEAL, "Port", name.clone()).meta(of.clone().unwrap_or_default()).tag("Port", token::SKETCH_TEAL).buttons(vec![None]));
        }
    }
    items
}

/// The columns of buttons a block draws: a row's buttons stand in the same places down the block.
pub fn button_columns(kind: BlockKind) -> usize {
    match kind {
        BlockKind::Run => 3,
        BlockKind::Checks => 1,
        _ => 0,
    }
}

//! A Job's pull requests (`PullsBody`, `PullRow`): each one a row framed in its state's hue, its number
//! and title, then its branch and what it waits on, and the act that moves it on. The Ledger draws the
//! same row.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, ClipboardItem, Context, FontWeight, Role, SharedString, Window};
use ipc::AttachmentState;

use super::ledger::Line;
use super::menus::Pick;
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// Where a pull request stands (`SessionPullRequestState`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PullState {
    Open,
    Draft,
    Merged,
    /// Closed without merging. Classic draws it as Open (`fleet-reads.ts` stateOf); native says Closed.
    Closed,
}

/// Its Checks, as the row's mark reads them (`checksOf`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PullChecks {
    Passed,
    Running,
    Failed,
}

/// A pull request a Job holds, with what Fleet last read of it (`pullRequestOf`).
#[derive(Clone, Debug)]
pub(super) struct Pull {
    pub number: u64,
    pub title: String,
    pub branch: String,
    pub address: String,
    pub state: PullState,
    pub checks: PullChecks,
    /// The Checks failing, by name: under the gate where `checks` is Failed, outside it otherwise.
    pub failing: String,
    pub auto: bool,
    pub queued: bool,
    /// The Job holding it, whose repository its acts name.
    pub job: String,
    pub manifest: Option<String>,
    /// The Job it came from, where that is not the Job whose list this is.
    pub from: Option<String>,
    /// The branch it is based on, where Fleet read it.
    pub base: Option<String>,
    /// A stack's root that has children: the branch it is based on, drawn as a muted word (`rootBase`).
    pub root_base: Option<String>,
    /// The number of the pull request it is stacked on, where that is not the row above (`chip`).
    pub stacked_on: Option<u64>,
    /// Whether the forge would merge it now. Unknown from a Fleet before it carried one.
    pub merge: ipc::PullRequestMergeState,
}

/// What the forge's merge verdict lets a pull request do, as the head, the panel's act and the row's act read it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Gate {
    /// No verdict: its Checks decide, as they did before Fleet carried one.
    Checks,
    /// Clean, unstable or with hooks: the forge merges it now.
    Merges,
    /// A rule of the base branch stops it, such as a review it asks for.
    Blocked,
    Behind,
    Conflicts,
}

/// The gate a merge verdict stands for. A draft's verdict is its state's, which is drawn already.
pub(super) fn gate_of(merge: ipc::PullRequestMergeState) -> Gate {
    use ipc::PullRequestMergeState as Merge;
    match merge {
        Merge::Clean | Merge::Unstable | Merge::HasHooks => Gate::Merges,
        Merge::Blocked => Gate::Blocked,
        Merge::Behind => Gate::Behind,
        Merge::Dirty => Gate::Conflicts,
        Merge::Draft | Merge::Unknown => Gate::Checks,
    }
}

/// The verdict a ledger row's `merge_state` detail spells, Unknown where it carries none.
fn merge_named(word: Option<&str>) -> ipc::PullRequestMergeState {
    use ipc::PullRequestMergeState as Merge;
    [Merge::Clean, Merge::Blocked, Merge::Behind, Merge::Dirty, Merge::Unstable, Merge::HasHooks, Merge::Draft]
        .into_iter()
        .find(|merge| Some(merge.word()) == word)
        .unwrap_or(Merge::Unknown)
}

/// An act that moves a pull request on, and the route Fleet takes it at (`PRESS_ROUTE`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PullAct {
    Ready,
    Merge,
    AutoMerge,
}

impl PullAct {
    fn route(self) -> &'static str {
        match self {
            PullAct::Ready => "ready",
            PullAct::Merge => "merge",
            PullAct::AutoMerge => "auto_merge",
        }
    }
}

impl Pull {
    /// Merged or closed: nothing moves it on any more.
    pub(super) fn ended(&self) -> bool {
        matches!(self.state, PullState::Merged | PullState::Closed)
    }

    /// Whether a Job holds it. One an issue names that no Job holds names no Job.
    pub(super) fn held(&self) -> bool {
        !self.job.is_empty()
    }
}

/// A pull request an issue names that no Job holds (`LinkedPullRequests`): its Checks are the forge's
/// read of it, filled in as the panel reads its page.
pub(super) fn unheld(manifest: Option<String>, one: &ipc::LinkedPullRequest) -> Pull {
    Pull {
        number: one.number,
        title: one.title.clone(),
        branch: String::new(),
        address: one.url.clone(),
        state: match one.state {
            ipc::LinkedPullRequestState::Merged => PullState::Merged,
            ipc::LinkedPullRequestState::Closed => PullState::Closed,
            ipc::LinkedPullRequestState::Open => PullState::Open,
        },
        checks: PullChecks::Running,
        failing: String::new(),
        auto: false,
        queued: false,
        job: String::new(),
        manifest,
        from: None,
        base: None,
        root_base: None,
        stacked_on: None,
        merge: ipc::PullRequestMergeState::Unknown,
    }
}

/// The repository path and number of a forge pull request's address, as Classic's `PULL` reads one
/// (`shared-panel/store.ts`): `https://<host>/<owner>/<name>/pull/<number>`, nothing after the number
/// but one slash. Any other address, `/files` after the number or an issue's, is not one.
pub(super) fn pull_address(address: &str) -> Option<(&str, u64)> {
    let rest = address.strip_prefix("https://").or_else(|| address.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let (repository, number) = path.rsplit_once("/pull/")?;
    if host.is_empty() || repository.split('/').any(str::is_empty) || number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((repository, number.parse().ok()?))
}

/// The Manifest a pull request's repository path names, by the name its `armada.yml`'s directory has,
/// as Classic's `repositoryOf` matches it. A repository no Manifest is read from has none. Classic's
/// fall back to the only Manifest there is is left out: it would open another repository's number.
pub(super) fn manifest_named(repositories: &[ipc::RepositorySummary], repository: &str) -> Option<String> {
    let whole = repository.to_lowercase();
    let name = repository.rsplit('/').next().unwrap_or(repository).to_lowercase();
    repositories
        .iter()
        .filter_map(|one| one.manifest.as_ref())
        .find(|manifest| {
            let held = manifest.repository.to_lowercase();
            held == whole || held == name
        })
        .map(|manifest| manifest.id.as_str().to_string())
}

/// A pull request no Job holds, opened from its address: its title is the merge line's where one holds it,
/// and its Checks the forge's read of its page, filled in as the panel reads it.
pub(super) fn addressed(manifest: String, address: &str, number: u64, title: String) -> Pull {
    Pull {
        number,
        title,
        branch: String::new(),
        address: address.to_string(),
        state: PullState::Open,
        checks: PullChecks::Running,
        failing: String::new(),
        auto: false,
        queued: false,
        job: String::new(),
        manifest: Some(manifest),
        from: None,
        base: None,
        root_base: None,
        stacked_on: None,
        merge: ipc::PullRequestMergeState::Unknown,
    }
}

/// A pull request's Checks as the forge's read of its page says them, with the failing ones by name.
pub(super) fn checks_of_page(page: &ipc::PullRequestPage) -> (PullChecks, String) {
    use ipc::PullRequestCheckState as State;
    let failing: Vec<&str> = page.checks.iter().filter(|one| one.state == State::Failed).map(|one| one.name.as_str()).collect();
    let checks = if !failing.is_empty() {
        PullChecks::Failed
    } else if page.checks.iter().any(|one| one.state == State::Running) {
        PullChecks::Running
    } else {
        PullChecks::Passed
    };
    (checks, failing.join(", "))
}

/// The pull request with its Checks and merge verdict as its page says them once the page is read, so the
/// head, the failing checks and the act agree with the tabs. Before, the ledger's row stands, though it can
/// be stale; so does its verdict where the page could not say one.
pub(super) fn with_page_read(mut pull: Pull, page: Option<&ipc::PullRequestPage>) -> Pull {
    if let Some(page) = page {
        (pull.checks, pull.failing) = checks_of_page(page);
        if page.merge_state != ipc::PullRequestMergeState::Unknown {
            pull.merge = page.merge_state;
        }
    }
    pull
}

/// A pull request attachment as its row reads it. A closed one is let go; a merged one is `spent`.
pub(super) fn pull_of(record: &ipc::SessionRecord, one: &ipc::Attachment, from: Option<String>) -> Option<Pull> {
    if !matches!(one.kind.as_str(), "pr" | "pull_request") || one.state == AttachmentState::GivenBack {
        return None;
    }
    let detail = |key: &str| one.detail.get(key).map(String::as_str);
    let number: u64 = one.target.trim_start_matches('#').parse().ok()?;
    let checks = match detail("checks") {
        Some("passed") => PullChecks::Passed,
        Some("failed") => PullChecks::Failed,
        _ => PullChecks::Running,
    };
    let state = if one.state == AttachmentState::Spent || detail("state") == Some("merged") {
        PullState::Merged
    } else if detail("state") == Some("draft") {
        PullState::Draft
    } else {
        PullState::Open
    };
    Some(Pull {
        number,
        title: detail("title").unwrap_or_default().to_string(),
        branch: detail("branch").unwrap_or_default().to_string(),
        address: detail("address").unwrap_or_default().to_string(),
        state,
        checks,
        failing: detail("failing").unwrap_or_default().to_string(),
        auto: detail("auto_merge") == Some("true"),
        queued: detail("queued") == Some("true"),
        job: record.id.as_str().to_string(),
        manifest: record.manifest_id.as_ref().map(|id| id.as_str().to_string()),
        from,
        base: detail("base").filter(|base| !base.is_empty()).map(str::to_string),
        root_base: None,
        stacked_on: None,
        merge: merge_named(detail("merge_state")),
    })
}

/// Whether the held row says something about the pull request it did not when its page was read: its
/// state, Checks, queue, auto-merge, base or the forge's merge verdict. The page is read again then.
pub(super) fn row_moved(was: &Pull, now: &Pull) -> bool {
    (was.state, was.checks, was.queued, was.auto, was.merge, &was.failing, &was.base) != (now.state, now.checks, now.queued, now.auto, now.merge, &now.failing, &now.base)
}

/// Where a stacked row's lane starts and ends (`StackRow.lane`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Lane {
    pub first: bool,
    pub last: bool,
}

/// Pull requests in the order a person reads them, a stack parent first (`stackedRows`, `pull-stacks.ts`).
/// `pulls` come oldest first. A pull request is stacked where its base is another open pull request's
/// branch; a merged parent has been retargeted by the forge, so its children stand alone. Roots and
/// unstacked pull requests keep newest first; children follow their parent depth-first, lowest number
/// first. A stack of one draws no lane.
pub(super) fn stacked_rows(pulls: Vec<Pull>) -> Vec<(Pull, Option<Lane>)> {
    let parents: Vec<Option<usize>> = (0..pulls.len())
        .map(|at| {
            let base = pulls[at].base.as_deref()?;
            (0..pulls.len()).find(|&other| other != at && pulls[other].state != PullState::Merged && !pulls[other].branch.is_empty() && pulls[other].branch == base)
        })
        .collect();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); pulls.len()];
    for (at, parent) in parents.iter().enumerate() {
        if let Some(parent) = parent {
            children[*parent].push(at);
        }
    }
    for kids in &mut children {
        kids.sort_by_key(|&kid| pulls[kid].number);
    }
    fn walk(at: usize, parent: Option<usize>, children: &[Vec<usize>], seen: &mut [bool], rows: &mut Vec<(usize, Option<usize>)>) {
        seen[at] = true;
        rows.push((at, parent));
        for &kid in &children[at] {
            if !seen[kid] {
                walk(kid, Some(at), children, seen, rows);
            }
        }
    }
    let mut seen = vec![false; pulls.len()];
    let mut order: Vec<(usize, Option<Lane>, Option<String>, Option<u64>)> = Vec::new();
    for at in (0..pulls.len()).rev() {
        if parents[at].is_some() || seen[at] {
            continue;
        }
        let mut stack = Vec::new();
        walk(at, None, &children, &mut seen, &mut stack);
        let many = stack.len() > 1;
        for (ix, &(one, parent)) in stack.iter().enumerate() {
            let lane = many.then_some(Lane { first: ix == 0, last: ix == stack.len() - 1 });
            let root_base = (parent.is_none() && !children[one].is_empty()).then(|| pulls[one].base.clone()).flatten();
            let above = ix.checked_sub(1).map(|up| stack[up].0);
            let chip = parent.filter(|&parent| Some(parent) != above).map(|parent| pulls[parent].number);
            order.push((one, lane, root_base, chip));
        }
    }
    // Pull requests each on the other have no root: they stand alone.
    for at in (0..pulls.len()).rev() {
        if !seen[at] {
            order.push((at, None, None, None));
        }
    }
    order
        .into_iter()
        .map(|(at, lane, root_base, chip)| {
            let mut pull = pulls[at].clone();
            pull.root_base = root_base;
            pull.stacked_on = chip;
            (pull, lane)
        })
        .collect()
}

/// A pull request's Checks in words, the tooltip on its Checks mark (`checksSaid`).
pub(super) fn checks_said(pull: &Pull) -> String {
    if pull.checks == PullChecks::Failed {
        return format!("Checks failed: {}", pull.failing);
    }
    let red = match (pull.failing.is_empty(), pull.checks) {
        (true, _) => String::new(),
        (false, PullChecks::Running) => format!("; failing: {}", pull.failing),
        (false, _) => format!("; not required, failing: {}", pull.failing),
    };
    format!("{}{red}", if pull.checks == PullChecks::Running { "Checks running" } else { "Checks passed" })
}

/// The frame's hue: its state's, or the failed hue where its Checks refused an open one (`data-tone`).
fn frame_hue(pull: &Pull) -> Token {
    match pull.state {
        PullState::Merged => token::STEP_ADVANCED,
        PullState::Draft | PullState::Closed => token::STATUS_NOT_STARTED,
        PullState::Open if pull.checks == PullChecks::Failed => token::STEP_FAILED,
        PullState::Open => token::STEP_RUNNING,
    }
}

/// Where it stands on its way in, as a fact after its Checks: a draft, the merge queue or auto-merge (`standingOf`).
fn standing_of(pull: &Pull) -> Option<(&'static str, &'static str, &'static str, Token)> {
    if pull.state == PullState::Draft {
        return Some(("git-pull-request", "Draft, not ready for review", "Draft", token::STATUS_NOT_STARTED));
    }
    if pull.queued {
        return Some(("clock", "In the merge queue, merges when its turn comes", "Queued", token::ACCENT));
    }
    if pull.checks == PullChecks::Running && pull.auto {
        return Some(("git-merge", "Auto-merge on, merges when every Check has passed", "Auto-merge", token::ACCENT));
    }
    if pull.auto && gate_of(pull.merge) == Gate::Blocked {
        return Some(("git-merge", "Auto-merge on, merges once nothing blocks it", "Auto-merge", token::ACCENT));
    }
    None
}

/// The act that moves it on, where one fits (`actOf`): its glyph, name and tooltip.
fn act_of(pull: &Pull) -> Option<(PullAct, &'static str, &'static str, &'static str)> {
    if pull.ended() || pull.queued {
        return None;
    }
    if pull.state == PullState::Draft {
        return Some((PullAct::Ready, "git-pull-request-arrow", "Ready for review", "Marks the pull request ready for review"));
    }
    // The forge's verdict decides where Fleet carried one; a Fleet before it leaves the Checks to.
    match gate_of(pull.merge) {
        Gate::Merges => Some((PullAct::Merge, "git-merge", "Merge", "Merges it now")),
        Gate::Blocked if !pull.auto => Some((PullAct::AutoMerge, "git-merge", "Enable auto-merge", "Enable auto-merge, merges it once nothing blocks it")),
        Gate::Blocked | Gate::Behind | Gate::Conflicts => None,
        Gate::Checks => match pull.checks {
            PullChecks::Passed => Some((PullAct::Merge, "git-merge", "Merge", "Merges it now, every Check has passed")),
            PullChecks::Running if !pull.auto => Some((PullAct::AutoMerge, "git-merge", "Enable auto-merge", "Enable auto-merge, merges it when every Check has passed")),
            _ => None,
        },
    }
}

/// The pull request that stands for a started Job: its open one, else the last it merged (`pullOf`).
pub(super) fn pull_of_child(child: &ipc::SessionRecord) -> Option<Pull> {
    let all: Vec<Pull> = child.attachments.iter().filter_map(|one| pull_of(child, one, None)).collect();
    let open = all.iter().position(|pull| !pull.ended());
    open.or(all.len().checked_sub(1)).map(|at| all[at].clone())
}

/// A linked Job row's pull request facts: Merged or Open, then its Checks (`SessionLayers` row.pr).
pub(super) fn linked_facts(look: &Look, key: &str, pull: &Pull) -> [AnyElement; 2] {
    let (icon, word, hue) = match pull.state {
        PullState::Merged => ("git-merge", "Merged", token::STEP_ADVANCED),
        PullState::Closed => ("git-pull-request-closed", "Closed", token::STATUS_NOT_STARTED),
        PullState::Open | PullState::Draft => ("git-pull-request", "Open", token::STEP_RUNNING),
    };
    let (check_icon, check_word, check_hue) = match pull.checks {
        PullChecks::Passed => ("circle-check", "Passed", token::STEP_ADVANCED),
        PullChecks::Running => ("loader-circle", "Running", token::STEP_RUNNING),
        PullChecks::Failed => ("circle-x", "Failed", token::STEP_FAILED),
    };
    // Classic's linked row names no failing Check outside the gate (`linked.ts` checksSaid).
    let said = match pull.checks {
        PullChecks::Failed => format!("Checks failed: {}", pull.failing),
        PullChecks::Running => "Checks running".to_string(),
        PullChecks::Passed => "Checks passed".to_string(),
    };
    [
        fact(look, format!("linked-pull:{key}"), icon, word.to_string(), word, hue),
        fact(look, format!("linked-checks:{key}"), check_icon, said, check_word, check_hue),
    ]
}

/// A fact on the row's second line: its glyph, which carries the tooltip, and its word, filled in its hue.
/// Chips that show only whole: a one-line wrap whose second line is clipped, so a chip without room
/// drops out rather than being cut. The empty first child lets even the first chip wrap away.
fn spare(look: &Look, chips: impl IntoIterator<Item = Option<AnyElement>>) -> gpui::Div {
    let gap = look.length(token::SPACE_2);
    div()
        .min_w_0()
        .flex()
        .flex_wrap()
        .items_center()
        .h(look.length(token::LEADING_2XS))
        .overflow_hidden()
        .child(div().w_0().h_full())
        .children(chips.into_iter().flatten().map(|chip| div().flex_none().pl(gap).child(chip)))
}

fn fact(look: &Look, id: String, icon: &'static str, said: String, word: &'static str, hue: Token) -> AnyElement {
    state_chip(look, id, icon, said, word, hue).into_any_element()
}

/// A state as a chip: its registry glyph and its word, filled in its hue with no outline. The
/// tooltip says what the word is short for.
pub(crate) fn state_chip(look: &Look, id: String, icon: &'static str, said: String, word: impl Into<SharedString>, hue: Token) -> gpui::Stateful<gpui::Div> {
    let ink = look.color(hue);
    div()
        .id(SharedString::from(id))
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .pl(look.length(token::SPACE_1))
        .pr(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_SM))
        .bg(ink.opacity(0.14))
        .text_color(ink)
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .tooltip(tip(said))
        .child(glyph(look, icon, Size::Small, ink))
        .child(word.into())
}

impl JobView {
    /// A Job's pull requests, then those of each Job it started with the Job each came from (`PullsBody`).
    pub(super) fn pulls_of(&self, id: &str, cx: &gpui::App) -> Vec<Pull> {
        let Some(job) = self.record(id, cx) else { return Vec::new() };
        let mut out: Vec<Pull> = job.attachments.iter().filter_map(|one| pull_of(job, one, None)).collect();
        for one in job.attachments.iter().filter(|one| one.kind == "started_to") {
            if let Some(child) = self.record(&one.target, cx) {
                let name = child.title.clone().unwrap_or_else(|| child.id.as_str().to_string());
                out.extend(child.attachments.iter().filter_map(|one| pull_of(child, one, Some(name.clone()))));
            }
        }
        out
    }

    /// The pull requests known in `manifest`, by number and address: a `pr` row any Job holds, then those on
    /// the merge line of its repository. A `#N` in words is a link only to one of these.
    pub(super) fn known_pulls(&self, manifest: &str, cx: &gpui::App) -> Vec<(u64, String)> {
        // Read straight off the rows `pull_of` would take, without drawing up each: the list holds hundreds of Jobs.
        let mut known: Vec<(u64, String)> = self
            .list
            .read(cx)
            .records()
            .iter()
            .filter(|record| record.manifest_id.as_ref().is_some_and(|id| id.as_str() == manifest))
            .flat_map(|record| &record.attachments)
            .filter(|one| matches!(one.kind.as_str(), "pr" | "pull_request") && one.state != AttachmentState::GivenBack)
            .filter_map(|one| {
                let number: u64 = one.target.trim_start_matches('#').parse().ok()?;
                let address = one.detail.get("address").filter(|address| !address.is_empty())?;
                Some((number, address.clone()))
            })
            .collect();
        let mut line = |served: &[ipc::RepositorySummary], lines: &[ipc::MergeLineView]| {
            let roots: Vec<&str> = served.iter().filter(|one| one.manifest.as_ref().is_some_and(|held| held.id.as_str() == manifest)).map(|one| one.root.as_str()).collect();
            for view in lines.iter().filter(|line| roots.contains(&line.root.as_str())).flat_map(|line| &line.pulls) {
                if !known.iter().any(|(number, _)| *number == view.pull.number) {
                    known.push((view.pull.number, view.pull.url.clone()));
                }
            }
        };
        match &self.fleet {
            Some(fleet) => {
                let held = fleet.read(cx).held();
                line(held.repositories.as_deref().unwrap_or_default(), held.merge_lines.as_deref().unwrap_or_default());
            }
            None => line(&crate::mock::repositories(), &crate::mock::merge_lines().unwrap_or_default()),
        }
        known
    }

    /// A forge pull request's address pressed in words opens in the PR tab: the pull request a Job holds at
    /// that address, else one no Job holds in the Manifest its repository names. False where it is no pull
    /// request's address or no Manifest is read from its repository, and the browser takes it.
    pub(super) fn open_pull_address(&mut self, address: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((repository, number)) = pull_address(address) else { return false };
        let same = |one: &str| one.trim_end_matches('/').eq_ignore_ascii_case(address.trim_end_matches('/'));
        let held = self.list.read(cx).records().iter().find_map(|record| record.attachments.iter().filter_map(|one| pull_of(record, one, None)).find(|pull| same(&pull.address)));
        let pull = match held {
            Some(pull) => pull,
            None => {
                let (served, lines) = match &self.fleet {
                    Some(fleet) => {
                        let held = fleet.read(cx).held();
                        (held.repositories.clone().unwrap_or_default(), held.merge_lines.clone().unwrap_or_default())
                    }
                    None => (crate::mock::repositories(), crate::mock::merge_lines().unwrap_or_default()),
                };
                let Some(manifest) = manifest_named(&served, repository) else { return false };
                // The forge's page carries no title: the merge line has one for a pull request in it.
                let title = lines.iter().flat_map(|line| &line.pulls).find(|view| same(&view.pull.url)).map(|view| view.pull.title.clone()).unwrap_or_default();
                addressed(manifest, address, number, title)
            }
        };
        self.open_pull(pull, window, cx);
        true
    }

    /// Pull Requests: every row as the Ledger draws it. None, and nothing is drawn.
    pub(super) fn pane_pulls(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let lines: Vec<Line> = self.pulls_of(id, cx).into_iter().map(pull_line).collect();
        if lines.is_empty() {
            return None;
        }
        Some(labelled(div().id("pane-pulls").flex().flex_col().children(self.ledger_rows(lines, "pull_request", look, cx)), Role::List, "Pull requests").into_any_element())
    }

    /// The row (`PullRow`): framed in its hue, its lead glyph says merged or open, its number in the hue
    /// before the title; then the Job it came from, its branch, its Checks and where it stands. Its act and
    /// its Judge stand at the right; a right press opens its menu.
    pub(super) fn pull_row(&self, line: Line, pull: Pull, group: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let judged = self.row_judge(&line, group, look, cx);
        let hue = look.color(frame_hue(&pull));
        let key = line.key.clone();
        let number = pull.number;
        let (lead, lead_said) = match pull.state {
            PullState::Merged => ("git-merge", "Merged"),
            PullState::Closed => ("git-pull-request-closed", "Closed"),
            PullState::Draft => ("git-pull-request", "Draft"),
            PullState::Open => ("git-pull-request", "Open"),
        };
        let lead_ink = look.color(match pull.state {
            PullState::Merged => token::STEP_ADVANCED,
            PullState::Draft | PullState::Closed => token::STATUS_NOT_STARTED,
            PullState::Open => token::STEP_RUNNING,
        });
        let lead = div()
            .id(SharedString::from(format!("pull-glyph:{key}")))
            .flex_none()
            .w(look.length(token::SPACE_3))
            .h(look.length(token::LEADING_XS))
            .flex()
            .items_center()
            .justify_center()
            .tooltip(tip(lead_said))
            .child(glyph(look, lead, Size::Small, lead_ink));
        // A title longer than two lines ends in an ellipsis, its whole in the tooltip, as a Jobs list row's does.
        let title = div()
            .id(SharedString::from(format!("pull-title:{key}")))
            .min_w_0()
            .line_clamp(2)
            .text_ellipsis()
            .tooltip(tip(format!("#{number} {}", pull.title)))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .child(
                gpui::StyledText::new(format!("#{number} {}", pull.title)).with_highlights([(
                    0..format!("#{number}").len(),
                    gpui::HighlightStyle { color: Some(hue), font_weight: Some(FontWeight(look.number(token::WEIGHT_HEADING))), ..Default::default() },
                )]),
            );
        // A root with children says what it is based on; a row whose parent is not above names it.
        let based = pull.root_base.clone().map(|base| div().flex_none().px(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_MUTED).opacity(0.14)).child(SharedString::from(format!("base: {base}"))));
        let on = pull.stacked_on.map(|parent| {
            let said = format!("Stacked on pull request #{parent}");
            let accent = look.color(token::ACCENT);
            labelled(
                div()
                    .id(SharedString::from(format!("pull-on:{key}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(accent.opacity(0.14))
                    .text_color(accent)
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .child(glyph(look, "link", Size::Small, accent))
                    .child(SharedString::from(format!("on #{parent}"))),
                Role::Link,
                said,
            )
        });
        let from = pull.from.clone().map(|from| div().flex_none().px(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_MUTED).opacity(0.14)).child(SharedString::from(from)));
        let branch = (!pull.branch.is_empty()).then(|| {
            div().max_w(look.length(token::SPACE_12) * 4.).truncate().px(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_DEFAULT).opacity(0.08)).font_family(mono(look)).child(SharedString::from(pull.branch.clone()))
        });
        let (check_icon, check_word, check_hue) = match pull.checks {
            PullChecks::Passed => ("circle-check", "Passed", token::STEP_ADVANCED),
            PullChecks::Running => ("loader-circle", "Running", token::STEP_RUNNING),
            PullChecks::Failed => ("circle-x", "Failed", token::STEP_FAILED),
        };
        let checks = fact(look, format!("pull-checks:{key}"), check_icon, checks_said(&pull), check_word, check_hue);
        let standing = standing_of(&pull).map(|(icon, said, word, hue)| fact(look, format!("pull-standing:{key}"), icon, said.to_string(), word, hue));
        // A description the Job proposed for it waits on him in the panel (`PullRequestDraft`).
        let waiting = self.record(&pull.job, cx).and_then(|job| job.pull_request_drafts.iter().find(|draft| draft.number == pull.number && pull.manifest.as_deref().is_none_or(|manifest| draft.manifest_id.as_str() == manifest))).map(|draft| {
            fact(look, format!("pull-draft:{key}"), "file-pen-line", format!("A new description waits on you: revision {}", draft.revision), "Description", token::STATUS_AWAITING_REVIEW)
        });
        let main = div()
            .id(SharedString::from(format!("ledger-row:{key}")))
            .flex_1()
            .min_w_0()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .child(lead)
            .child(
                div().flex_1().min_w_0().flex().flex_col().child(title).child(
                    // The checks chip leads and always shows whole; a narrow pane drops the others
                    // whole from the end, and never cuts one.
                    div()
                        .flex()
                        .items_center()
                        .min_w_0()
                        .overflow_hidden()
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_MUTED))
                        .whitespace_nowrap()
                        .child(div().flex_none().child(checks))
                        .child(spare(look, [waiting, standing, based.map(IntoElement::into_any_element), on.map(IntoElement::into_any_element), from.map(IntoElement::into_any_element), branch.map(IntoElement::into_any_element)])),
                ),
            );
        let pressed = pull.clone();
        let main = main.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.open_pull(pressed.clone(), window, cx)));
        let main = match self.shown.as_ref().and_then(|job| self.pages.get(job)) {
            Some(page) => {
                let spot = self.spot(page, &format!("pull-menu:{key}"), cx);
                let held = pull.clone();
                let entries = Rc::new(move || pull_menu(&held));
                let pick = pull_pick(pull.clone());
                self.menu_on(main.track_focus(&spot.focus), &spot, format!("Pull request #{number}"), spot.focus.clone(), entries, pick, cx)
            }
            None => main,
        };
        let main = labelled(main, Role::Button, format!("Pull request #{number} {}", pull.title)).into_any_element();
        let act = self.pull_act_button(&pull, &key, look, cx);
        let (press, under) = judged.map_or((None, None), |(press, under)| (Some(press), under));
        let acts = div().flex_none().flex().items_center().children(act).child(press.unwrap_or_else(|| div().flex_none().size(look.length(token::H_CONTROL_SM)).into_any_element()));
        let border = look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4));
        let fill = look.color(token::BG_RAISED).blend(hue.opacity(0.07));
        let hover = look.color(token::BG_RAISED).blend(hue.opacity(0.13));
        let edge = look.length(token::BORDER_WIDTH);
        let frame = div()
            .id(SharedString::from(format!("pull-frame:{key}")))
            .relative()
            .overflow_hidden()
            .my(look.length(token::SPACE_1))
            .border(edge)
            .border_color(border)
            .rounded(look.length(token::RADIUS_MD))
            .bg(fill)
            .hover(move |style| style.bg(hover))
            .child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.).rounded_l(look.length(token::RADIUS_MD) - edge).bg(hue))
            .child(div().flex().items_center().gap(look.length(token::SPACE_1)).pr(look.length(token::SPACE_1)).child(main).child(acts));
        div().flex().flex_col().child(frame).children(under).into_any_element()
    }

    /// The icon button that moves it on, pending while Fleet takes it and every act held meanwhile.
    fn pull_act_button(&self, pull: &Pull, key: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let (act, icon, name, said) = act_of(pull)?;
        let mark = format!("{}#{}", pull.job, pull.number);
        let pending = self.panes.pull_pending.get(&mark).copied();
        let busy = pending.is_some();
        let spinning = pending == Some(act);
        let ink = look.color(if busy { token::FG_SUBTLE } else { token::FG_MUTED });
        let held = pull.clone();
        let row = format!("ledger:{key}");
        let button = div()
            .id(SharedString::from(format!("pull-act:{key}")))
            .flex_none()
            .size(look.length(token::H_CONTROL_SM))
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_MD))
            .tooltip(tip(said))
            .child(glyph(look, if spinning { "loader-circle" } else { icon }, Size::Small, ink))
            .when(!busy, |button| {
                button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.press_pull(&held, act, row.clone(), window, cx);
                }))
            });
        Some(labelled(button, Role::Button, format!("{name} #{}", pull.number)).into_any_element())
    }

    /// Ask Fleet to move the pull request on. Its row reads the change when the Job's record does; a
    /// refusal stands under the row in Fleet's words.
    pub(super) fn press_pull(&mut self, pull: &Pull, act: PullAct, row: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(manifest) = pull.manifest.clone() else {
            self.pull_refused(row, "This Job has no repository".into(), cx);
            return;
        };
        let mark = format!("{}#{}", pull.job, pull.number);
        self.panes.pull_pending.insert(mark.clone(), act);
        cx.notify();
        let path = format!("/pull_requests/{}/{}/{}", super::wire::encoded(&manifest), pull.number, act.route());
        self.post_bare(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                this.panes.pull_pending.remove(&mark);
                if let Err(why) = answer {
                    this.pull_refused(row, why.into(), cx);
                }
                cx.notify();
            }),
        );
    }

    /// Ask Fleet to read the pull request from the forge now (the menu's Refresh).
    fn refresh_pull(&mut self, pull: &Pull, row: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(manifest) = pull.manifest.clone() else {
            self.pull_refused(row, "This Job has no repository".into(), cx);
            return;
        };
        let path = format!("/pull_requests/{}/{}/refresh", super::wire::encoded(&manifest), pull.number);
        self.post_bare(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if let Err(why) = answer {
                    this.pull_refused(row, why.into(), cx);
                }
            }),
        );
    }

    fn pull_refused(&mut self, row: String, why: SharedString, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.clone().and_then(|job| self.pages.get_mut(&job)) {
            page.cards.said.insert(row, why);
        }
        cx.notify();
    }
}

/// The row a pull request is drawn as, among the Ledger's and Pull Requests' rows.
pub(super) fn pull_line(pull: Pull) -> Line {
    Line {
        key: format!("pr:{}", pull.number),
        kind: "pull_request",
        glyph: "git-pull-request",
        said: "Pull request",
        title: format!("#{} {}", pull.number, pull.title).trim_end().to_string(),
        sub: (!pull.branch.is_empty()).then(|| pull.branch.clone()),
        pull: Some(pull),
        ..Line::default()
    }
}

/// Its menu (`usePullRequestMenu`): the heading, Open in browser and Copy link where it has an address,
/// Refresh and Teach.
fn pull_menu(pull: &Pull) -> Vec<Entry> {
    let mut entries = vec![Entry::Label(crate::titlebar::id_of(format!("Pull request #{}", pull.number)))];
    if !pull.address.is_empty() {
        entries.push(Entry::Act { id: "browser", label: "Open in browser".into(), icon: Some("external-link"), danger: false });
        entries.push(Entry::Act { id: "copy", label: "Copy link".into(), icon: Some("link"), danger: false });
    }
    entries.push(Entry::Act { id: "refresh", label: "Refresh".into(), icon: None, danger: false });
    entries.push(Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false });
    entries
}

fn pull_pick(pull: Pull) -> Pick {
    Rc::new(move |this, id, window, cx| match id {
        "browser" => cx.open_url(&pull.address),
        "copy" => cx.write_to_clipboard(ClipboardItem::new_string(pull.address.clone())),
        "refresh" => this.refresh_pull(&pull, format!("ledger:pr:{}", pull.number), window, cx),
        "teach" => {
            let text = if pull.title.is_empty() { format!("#{}", pull.number) } else { format!("#{} {}", pull.number, pull.title) };
            this.teach_open(super::menus::Teach { kind: "pull_request", text, at: None, ids: Vec::new() }, window, cx);
        }
        _ => {}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull(state: PullState, checks: PullChecks, auto: bool, queued: bool) -> Pull {
        Pull { number: 7, title: String::new(), branch: String::new(), address: String::new(), state, checks, failing: String::new(), auto, queued, job: "j".into(), manifest: None, from: None, base: None, root_base: None, stacked_on: None, merge: ipc::PullRequestMergeState::Unknown }
    }

    #[test]
    fn the_act_follows_electrons_act_of() {
        assert_eq!(act_of(&pull(PullState::Draft, PullChecks::Passed, false, false)).map(|one| one.0), Some(PullAct::Ready));
        assert_eq!(act_of(&pull(PullState::Open, PullChecks::Passed, false, false)).map(|one| one.0), Some(PullAct::Merge));
        assert_eq!(act_of(&pull(PullState::Open, PullChecks::Running, false, false)).map(|one| one.0), Some(PullAct::AutoMerge));
        assert_eq!(act_of(&pull(PullState::Open, PullChecks::Running, true, false)).map(|one| one.0), None);
        assert_eq!(act_of(&pull(PullState::Open, PullChecks::Failed, false, false)).map(|one| one.0), None);
        assert_eq!(act_of(&pull(PullState::Open, PullChecks::Passed, false, true)).map(|one| one.0), None);
        assert_eq!(act_of(&pull(PullState::Merged, PullChecks::Passed, false, false)).map(|one| one.0), None);
    }

    #[test]
    fn checks_in_words_name_what_fails_outside_the_gate() {
        let mut one = pull(PullState::Open, PullChecks::Running, false, false);
        assert_eq!(checks_said(&one), "Checks running");
        one.failing = "lint".into();
        assert_eq!(checks_said(&one), "Checks running; failing: lint");
        one.checks = PullChecks::Passed;
        assert_eq!(checks_said(&one), "Checks passed; not required, failing: lint");
        one.checks = PullChecks::Failed;
        assert_eq!(checks_said(&one), "Checks failed: lint");
    }

    #[test]
    fn once_the_page_is_read_its_checks_stand_over_a_stale_ledger_row() {
        let page: ipc::PullRequestPage = ipc::decode(
            "PullRequestPage",
            serde_json::json!({
                "manifest_id": "m", "number": 7, "base": "main", "additions": 0, "deletions": 0, "mergeable": "mergeable", "review": "none", "approvals": 0,
                "conversation": [], "files": [], "commits": [],
                "checks": [{ "name": "lint", "state": "passed" }, { "name": "test", "state": "passed" }, { "name": "docs", "state": "skipped" }],
            })
            .to_string()
            .as_bytes(),
        )
        .expect("a page");
        let mut ledger = pull(PullState::Open, PullChecks::Failed, false, false);
        ledger.failing = "lint".into();
        let read = with_page_read(ledger.clone(), Some(&page));
        assert_eq!((read.checks, read.failing.as_str()), (PullChecks::Passed, ""));
        assert_eq!(act_of(&read).map(|one| one.0), Some(PullAct::Merge), "the act follows the page too");
        let unread = with_page_read(ledger, None);
        assert_eq!((unread.checks, unread.failing.as_str()), (PullChecks::Failed, "lint"), "before the page is read the ledger stands");
    }

    #[test]
    fn the_forges_merge_verdict_picks_the_act_and_without_one_the_checks_do() {
        use ipc::PullRequestMergeState as Merge;
        let act = |merge, checks, auto| {
            let mut one = pull(PullState::Open, checks, auto, false);
            one.merge = merge;
            act_of(&one).map(|one| one.0)
        };
        assert_eq!(act(Merge::Blocked, PullChecks::Passed, false), Some(PullAct::AutoMerge), "every check passed, and a review is still asked for");
        assert_eq!(act(Merge::Blocked, PullChecks::Passed, true), None, "auto-merge is on already");
        assert_eq!(act(Merge::Behind, PullChecks::Passed, false), None);
        assert_eq!(act(Merge::Dirty, PullChecks::Passed, false), None);
        assert_eq!(act(Merge::Clean, PullChecks::Passed, false), Some(PullAct::Merge));
        assert_eq!(act(Merge::HasHooks, PullChecks::Passed, false), Some(PullAct::Merge));
        assert_eq!(act(Merge::Unstable, PullChecks::Failed, false), Some(PullAct::Merge), "the failing checks are not required");
        assert_eq!(act(Merge::Unknown, PullChecks::Passed, false), Some(PullAct::Merge), "an older Fleet: the checks decide");
        assert_eq!(act(Merge::Unknown, PullChecks::Running, false), Some(PullAct::AutoMerge));
        assert_eq!(act(Merge::Unknown, PullChecks::Failed, false), None);
        assert_eq!((merge_named(Some("has_hooks")), merge_named(Some("blocked")), merge_named(None)), (Merge::HasHooks, Merge::Blocked, Merge::Unknown));
    }

    #[test]
    fn once_the_page_is_read_its_merge_verdict_stands_over_the_ledgers() {
        let page = |merge: &str| -> ipc::PullRequestPage {
            ipc::decode(
                "PullRequestPage",
                serde_json::json!({
                    "manifest_id": "m", "number": 7, "base": "main", "additions": 0, "deletions": 0, "mergeable": "mergeable", "merge_state": merge, "review": "required", "approvals": 0,
                    "conversation": [], "files": [], "commits": [], "checks": [],
                })
                .to_string()
                .as_bytes(),
            )
            .expect("a page")
        };
        let mut ledger = pull(PullState::Open, PullChecks::Passed, false, false);
        ledger.merge = ipc::PullRequestMergeState::Clean;
        assert_eq!(with_page_read(ledger.clone(), Some(&page("blocked"))).merge, ipc::PullRequestMergeState::Blocked);
        assert_eq!(with_page_read(ledger.clone(), Some(&page("unknown"))).merge, ipc::PullRequestMergeState::Clean, "a page that cannot say leaves the ledger's");
        assert_eq!(with_page_read(ledger, None).merge, ipc::PullRequestMergeState::Clean);
    }

    #[test]
    fn a_pull_request_address_names_its_repository_and_number() {
        assert_eq!(pull_address("https://forge.example/acme-labs/storefront/pull/418"), Some(("acme-labs/storefront", 418)));
        assert_eq!(pull_address("https://forge.example/acme-labs/storefront/pull/418/"), Some(("acme-labs/storefront", 418)));
        assert_eq!(pull_address("http://git.example/group/sub/storefront/pull/7"), Some(("group/sub/storefront", 7)));
    }

    #[test]
    fn an_address_past_the_number_or_of_an_issue_is_not_a_pull_request() {
        for address in [
            "https://forge.example/acme-labs/storefront/pull/418/files",
            "https://forge.example/acme-labs/storefront/pull/418#discussion_r1",
            "https://forge.example/acme-labs/storefront/pull/418?w=1",
            "https://forge.example/acme-labs/storefront/issues/418",
            "https://forge.example/pull/418",
            "https://forge.example/acme-labs/storefront/pull/",
            "https://forge.example/acme-labs/storefront/pull/41a",
            "ftp://forge.example/acme-labs/storefront/pull/418",
            "armada://pull/acme-labs/storefront/418",
        ] {
            assert_eq!(pull_address(address), None, "{address}");
        }
    }

    #[test]
    fn a_repository_path_finds_the_manifest_named_for_it() {
        let repo = |root: &str, manifest: Option<(&str, &str)>| -> ipc::RepositorySummary {
            let mut one = serde_json::json!({ "root": root, "records_root": "/records" });
            if let Some((id, name)) = manifest {
                one["manifest"] = serde_json::json!({ "id": id, "repository": name, "path": format!("{root}/armada.yml"), "records_root": "/records", "version": 1, "checks": [] });
            }
            ipc::decode("RepositorySummary", one.to_string().as_bytes()).expect("a repository summary")
        };
        let served = [repo("/work/storefront", Some(("m-store", "Storefront"))), repo("/work/ledger", Some(("m-ledger", "ledger"))), repo("/work/loose", None)];
        assert_eq!(manifest_named(&served, "acme-labs/storefront").as_deref(), Some("m-store"));
        assert_eq!(manifest_named(&served, "acme-labs/ledger").as_deref(), Some("m-ledger"));
        assert_eq!(manifest_named(&served, "acme-labs/loose"), None, "no Manifest is read there");
        assert_eq!(manifest_named(&served, "acme-labs/elsewhere"), None, "not the only one there is, as Classic falls back to");
    }
}

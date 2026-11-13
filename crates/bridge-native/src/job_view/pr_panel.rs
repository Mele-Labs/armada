//! A pull request's side panel, opened from its row (`PullRequestPanel`, `shared-panel/pull-request.tsx`):
//! the head, its title, what it is and where it is going, the act that moves it on, the plates beside it,
//! the tabs Conversation, Checks, Files and Commits, and the comment box at its foot. The Review tab comes
//! with the Reviews screen.

use std::collections::HashSet;

use chrono::Local;
use gpui::{
    anchored, deferred, div, point, prelude::*, px, Anchor, Animation, AnimationExt as _, AnyElement, ClipboardItem, Context, Entity, FontWeight, KeyDownEvent, Role, ScrollHandle, SharedString,
    Subscription, Task, Window,
};
use gpui_component::input::{Enter, InputEvent, Textarea, TextareaState};
use ipc::{
    PullRequestCheckState, PullRequestEntry, PullRequestMergeable, PullRequestPage, PullRequestReviewState, PullRequestReviewer, PullRequestReviewerKind, PullRequestVerdict, PullRequestWriteKind,
    PullReviewThread,
};

use super::pulls::{Pull, PullAct, PullChecks, PullState};
use super::rows::{age, glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;

#[path = "pr_draft.rs"]
mod pr_draft;
use crate::look::{token, ActiveLook, Look, Token};

/// The panel's tabs, in Electron's order (`TABS`). Review joins them with the Reviews screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PullTab {
    Conversation,
    Checks,
    Files,
    Commits,
}

impl PullTab {
    const ALL: [PullTab; 4] = [PullTab::Conversation, PullTab::Checks, PullTab::Files, PullTab::Commits];

    fn word(self) -> &'static str {
        match self {
            PullTab::Conversation => "Conversation",
            PullTab::Checks => "Checks",
            PullTab::Files => "Files",
            PullTab::Commits => "Commits",
        }
    }
}

/// The open panel's own state.
pub(super) struct PullPanel {
    /// The pull request as its row read it when pressed. Its live facts come from the Job's record.
    pub pull: Pull,
    pub tab: PullTab,
    /// `GET /pull_request_details`: the tabs and the facts the row does not hold.
    pub page: Option<Result<PullRequestPage, String>>,
    /// The comment box, its words kept under `pull-request:panel:{repository}#{number}`.
    pub say: Entity<TextareaState>,
    pub write: PullRequestWriteKind,
    pub writing: bool,
    pub write_said: Option<SharedString>,
    /// Notify Job was pressed and Fleet took it.
    pub sent_back: bool,
    /// The comments handed to the Job with Evaluate, by the forge's id.
    pub evaluated: HashSet<String>,
    /// Copy link said Copied a moment ago.
    pub copied: bool,
    /// The files whose tests are opened under their row in the list.
    pub reach_open: HashSet<String>,
    /// The file list's one focus, its current row, and the folders folded in it (`FileRows`).
    pub files_focus: gpui::FocusHandle,
    pub files_at: Option<super::pr_files::FileNav>,
    pub folded: HashSet<String>,
    /// Whether the file list held the focus as the panel was drawn, so its current row is ringed.
    pub files_focused: std::cell::Cell<bool>,
    /// The floating list's scroll, so a row the keys reach is scrolled to.
    pub files_scroll: ScrollHandle,
    /// The file the list last pressed, marked current.
    pub current: Option<String>,
    /// The body's scroll, and the Files tab's sections, so a file pressed in the list is scrolled to.
    pub scroll: ScrollHandle,
    pub sections: ScrollHandle,
    /// The boxes notes are written in, each under its line: one for a new note and one per note being changed.
    pub note_boxes: Vec<super::pr_notes::NoteBox>,
    /// His notes a Job resolved are drawn too.
    pub notes_resolved: bool,
    /// A description the Job proposed, drawn in the description's place (`pr_draft`).
    pub draft: pr_draft::DraftPane,
    _typing: Subscription,
    _copied: Option<Task<()>>,
}

impl PullPanel {
    fn kept_key(pull: &Pull) -> String {
        format!("pull-request:panel:{}#{}", pull.manifest.as_deref().unwrap_or(""), pull.number)
    }

    pub(super) fn page(&self) -> Option<&PullRequestPage> {
        self.page.as_ref().and_then(|page| page.as_ref().ok())
    }

    /// The key Fleet's patches are kept under, as the Changes pane keeps them.
    pub(super) fn patches_key(&self) -> Option<String> {
        self.pull.manifest.as_ref().map(|manifest| format!("{manifest}#{}", self.pull.number))
    }
}

/// A plate's hue (`data-tone`), and the frame and ground it gives the plate.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    Neutral,
    Running,
    Success,
    Failed,
}

impl Tone {
    pub(super) fn hue(self) -> Token {
        match self {
            Tone::Neutral => token::BORDER_DEFAULT,
            Tone::Running => token::STATUS_RUNNING,
            Tone::Success => token::STATUS_COMPLETED_SUCCESS,
            Tone::Failed => token::STATUS_COMPLETED_FAILED,
        }
    }
}

/// `1st`, `2nd`, `3rd`, `11th` (`ordinal`).
fn ordinal(place: u32) -> String {
    let suffix = match (place % 100, place % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{place}{suffix}")
}

/// What the status plate says, its glyph and its hue (`standing`). `at` is its place in the merge queue.
fn standing(pull: &Pull, at: Option<u32>) -> (SharedString, &'static str, Tone) {
    let (what, icon, tone): (&'static str, &'static str, Tone) = if pull.state == PullState::Merged {
        ("Merged", "git-merge", Tone::Success)
    } else if pull.state == PullState::Closed {
        ("Closed", "git-pull-request-closed", Tone::Neutral)
    } else if pull.queued {
        let what = at.map_or_else(|| "In merge queue".to_string(), |at| format!("{} in the merge queue", ordinal(at)));
        return (what.into(), "clock", Tone::Running);
    } else if pull.state == PullState::Draft {
        ("Draft", "git-pull-request", Tone::Neutral)
    } else {
        match pull.checks {
            PullChecks::Failed => ("Checks failed", "circle-x", Tone::Failed),
            PullChecks::Passed => ("Ready to merge", "git-merge", Tone::Success),
            PullChecks::Running => ("Checks running", "loader-circle", Tone::Running),
        }
    };
    (what.into(), icon, tone)
}

/// One row of the merge queue a pull request is in (`PlaceRow`).
pub(super) struct PlaceRow {
    place: u32,
    number: u64,
    title: String,
    url: String,
    state: ipc::HubQueueState,
    /// The pull request the panel is for.
    current: bool,
}

/// The forge's merge queue the pull request is in, in place order (`placeOf`): found by its address,
/// else its number and branch. One waiting for ci to join the queue has no place, and neither does
/// one in no queue, so both give None and nothing is drawn.
fn place_of(lines: &[ipc::MergeLineView], pull: &Pull) -> Option<Vec<PlaceRow>> {
    for line in lines {
        let mut placed: Vec<(u32, &ipc::HubPullRequest)> = line.pulls.iter().filter_map(|view| Some((view.pull.queue.as_ref()?.position?, &view.pull))).collect();
        let Some(mine) = placed.iter().find(|(_, row)| row.url == pull.address || (row.number == pull.number && row.branch == pull.branch)).map(|(_, row)| *row) else {
            continue;
        };
        placed.sort_by_key(|(place, _)| *place);
        return Some(
            placed
                .into_iter()
                .filter_map(|(place, row)| {
                    Some(PlaceRow {
                        place,
                        number: row.number,
                        title: if row.title.is_empty() { row.branch.clone() } else { row.title.clone() },
                        url: row.url.clone(),
                        state: row.queue.as_ref()?.state,
                        current: std::ptr::eq(row, mine),
                    })
                })
                .collect(),
        );
    }
    None
}

/// Who a pull request was registered to, as a chip (`RegisteredOwner`): the kind's glyph and the short
/// id in mono, then the machine's mark past a divider. **Marks with tooltips, never a phrase.** `laptop`
/// is this machine; `monitor` is another, and its tooltip names it.
fn owner_chip(owner: &ipc::PullRequestOwner, look: &Look) -> AnyElement {
    let said = match owner.kind {
        ipc::HolderKind::Session => "Job",
    };
    let (icon, machine) = if owner.this_machine { ("laptop", "This machine".to_string()) } else { ("monitor", owner.machine.clone()) };
    let edge = look.color(token::BORDER_DEFAULT);
    labelled(
        div()
            .id("pull-owner")
            .flex_none()
            .flex()
            .items_center()
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .rounded(look.length(token::RADIUS_SM))
            .bg(look.color(token::BG_SUNKEN))
            .text_color(look.color(token::FG_MUTED))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_XS))
            .whitespace_nowrap()
            .child(
                div()
                    .id("pull-owner-who")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_2))
                    .tooltip(tip(said))
                    .child(glyph(look, "square-terminal", Size::Small, look.color(token::FG_MUTED)))
                    .child(SharedString::from(owner.id.clone())),
            )
            .child(
                div()
                    .id("pull-owner-machine")
                    .flex()
                    .items_center()
                    .self_stretch()
                    .px(look.length(token::SPACE_1))
                    .border_l(look.length(token::BORDER_WIDTH))
                    .border_color(edge)
                    .tooltip(tip(machine.clone()))
                    .child(glyph(look, icon, Size::Small, look.color(if owner.this_machine { token::FG_DEFAULT } else { token::FG_SUBTLE }))),
            ),
        Role::Group,
        format!("{said} {}, {machine}", owner.id),
    )
    .into_any_element()
}

/// A queue state's mark (`QUEUE`, read through `LAND_STATE`): its glyph, hue, and whether it pulses.
fn queue_mark(state: ipc::HubQueueState) -> (&'static str, Token, bool) {
    use ipc::HubQueueState as Q;
    match state {
        Q::WaitingForCi => ("git-merge", token::STATUS_RUNNING, true),
        Q::InQueue | Q::Queued => ("clock", token::STATUS_NOT_STARTED, false),
        Q::AwaitingChecks => ("circle-dot", token::STATUS_RUNNING, true),
        Q::Mergeable => ("circle-dot", token::STATUS_RUNNING, true),
        Q::Unmergeable => ("unplug", token::STATUS_ESCALATED, false),
    }
}

/// The checks fact (`CHECKS`).
fn checks_fact(pull: &Pull) -> (&'static str, &'static str, Tone) {
    match pull.checks {
        PullChecks::Running => ("loader-circle", "Checks running", Tone::Running),
        PullChecks::Passed => ("circle-check", "Checks passed", Tone::Success),
        PullChecks::Failed => ("circle-x", "Checks failed", Tone::Failed),
    }
}

/// A review's verdict as a glyph, its word and its hue (`VERDICT`); a reviewer with none is waiting.
fn verdict_look(verdict: Option<PullRequestVerdict>) -> (&'static str, &'static str, Option<Tone>) {
    match verdict {
        None => ("clock", "Waiting", None),
        Some(PullRequestVerdict::Approved) => ("check", "Approved", Some(Tone::Success)),
        Some(PullRequestVerdict::ChangesRequested) => ("ban", "Changes requested", Some(Tone::Failed)),
        Some(PullRequestVerdict::Commented) => ("scroll-text", "Commented", None),
    }
}

/// A check row's state (`CHECK`).
fn check_look(state: PullRequestCheckState) -> (&'static str, &'static str, Option<Tone>) {
    match state {
        PullRequestCheckState::Passed => ("shield-check", "Passed", Some(Tone::Success)),
        PullRequestCheckState::Failed => ("shield-x", "Failed", Some(Tone::Failed)),
        PullRequestCheckState::Running => ("loader-circle", "Running", Some(Tone::Running)),
        PullRequestCheckState::Skipped => ("shield-off", "Skipped", None),
    }
}

/// How long a check ran (`ranFor`).
pub(super) fn ran_for(seconds: u64) -> String {
    let minutes = seconds / 60;
    if seconds < 60 {
        format!("{seconds}s")
    } else if minutes < 60 {
        format!("{minutes}m {:02}s", seconds % 60)
    } else {
        format!("{}h {:02}m", minutes / 60, minutes % 60)
    }
}

/// Where a thread is: its file, and its line where it has one (`whereOfThread`).
pub(super) fn where_of(thread: &PullReviewThread) -> String {
    thread.line.map_or_else(|| thread.path.clone(), |line| format!("{}:{line}", thread.path))
}

/// The message Evaluate hands the Job about one comment (`evaluateMessage`).
pub(super) fn evaluate_message(number: u64, id: &str, author: &str, body: &str, place: Option<&str>) -> String {
    let quoted = body.trim().split('\n').map(|line| if line.trim().is_empty() { ">".to_string() } else { format!("> {line}") }).collect::<Vec<_>>().join("\n");
    let mut lines = vec![format!("Evaluate a comment on #{number}."), format!("Comment: {id}"), format!("Author: {author}")];
    if let Some(place) = place {
        let lined = place.rsplit_once(':').is_some_and(|(_, tail)| !tail.is_empty() && tail.chars().all(|one| one.is_ascii_digit()));
        lines.push(format!("{}: {place}", if lined { "Line" } else { "File" }));
    }
    lines.push(String::new());
    lines.push(quoted);
    lines.push(String::new());
    lines.push("Decide whether the comment is right, and put what you propose in waiting_for as one item for me.".into());
    lines.push("Never reply on the pull request, and never post a comment, review or reaction anywhere.".into());
    lines.join("\n")
}

/// Whether the box may send: a comment and a request for changes need words, an approval need not (`mayWrite`).
fn may_write(kind: PullRequestWriteKind, text: &str) -> bool {
    kind == PullRequestWriteKind::Approve || !text.trim().is_empty()
}

fn write_word(kind: PullRequestWriteKind, writing: bool) -> &'static str {
    match (kind, writing) {
        (PullRequestWriteKind::Comment, false) => "Comment",
        (PullRequestWriteKind::Comment, true) => "Commenting",
        (PullRequestWriteKind::Approve, false) => "Approve",
        (PullRequestWriteKind::Approve, true) => "Approving",
        (PullRequestWriteKind::RequestChanges, false) => "Request changes",
        (PullRequestWriteKind::RequestChanges, true) => "Requesting changes",
    }
}

/// A small button (`Button size="sm"`): primary, or ghost.
pub(super) fn small_button(look: &Look, id: impl Into<gpui::ElementId>, said: &str, primary: bool, on: bool) -> gpui::Stateful<gpui::Div> {
    let base = div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .child(SharedString::from(said.to_string()));
    let styled = if primary {
        let hover = look.color(token::ACCENT_HOVER);
        base.bg(look.color(token::ACCENT)).text_color(look.color(token::TEXT_ON_ACCENT)).when(on, move |button| button.hover(move |style| style.bg(hover)))
    } else {
        let (ground, ink) = (look.color(token::BG_HOVER), look.color(token::FG_DEFAULT));
        base.text_color(look.color(token::FG_MUTED)).when(on, move |button| button.hover(move |style| style.bg(ground).text_color(ink)))
    };
    if on {
        styled.cursor_pointer()
    } else {
        styled.opacity(0.5)
    }
}

/// A ghost icon button (`Button variant="ghost" size="sm" iconOnly`).
pub(super) fn icon_button(look: &Look, id: impl Into<gpui::ElementId>, icon: &'static str, said: SharedString) -> gpui::Stateful<gpui::Div> {
    let ground = look.color(token::BG_HOVER);
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::H_CONTROL_SM))
        .rounded(look.length(token::RADIUS_MD))
        .cursor_pointer()
        .hover(move |style| style.bg(ground))
        .tooltip(tip(said))
        .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
}

/// A glyph that names itself on hover (`Mark`), in its tone's hue.
pub(super) fn mark(look: &Look, id: String, icon: &'static str, said: &'static str, tone: Option<Tone>) -> AnyElement {
    let ink = tone.map_or(look.color(token::FG_MUTED), |tone| look.color(tone.hue()));
    labelled(div().id(SharedString::from(id)).flex_none().tooltip(tip(said)).child(glyph(look, icon, Size::Small, ink)), Role::Image, said).into_any_element()
}

/// A plate (`armada-pr-plate`): a hue-tinted frame round a column.
fn plate(look: &Look, tone: Tone) -> gpui::Div {
    let hue = look.color(tone.hue());
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .p(look.length(token::SPACE_3))
        .border(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4)))
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(token::BG_RAISED).blend(hue.opacity(0.05)))
}

fn eyebrow(look: &Look) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(look.color(token::FG_MUTED))
}

/// Shapes in the places of the lines while the read is out (`SkeletonText`).
pub(super) fn stand_ins(look: &Look, key: &str, widths: &[f32]) -> AnyElement {
    let ground = look.color(token::FG_DEFAULT).opacity(0.07);
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .children(widths.iter().enumerate().map(|(at, width)| {
            div()
                .h(look.length(token::LEADING_SM) - look.length(token::SPACE_1))
                .w(gpui::relative(*width))
                .rounded(look.length(token::RADIUS_SM))
                .bg(ground)
                .with_animation(SharedString::from(format!("{key}-{at}")), Animation::new(std::time::Duration::from_millis(1400)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)), |shape, at| shape.opacity(at))
        }))
        .into_any_element()
}

/// `+a −d` in the diff's hues (`DiffSize`).
pub(super) fn diff_size(look: &Look, id: impl Into<SharedString>, added: u64, removed: u64) -> gpui::Stateful<gpui::Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .flex_none()
        .flex()
        .gap(look.length(token::SPACE_1))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_XS))
        .tooltip(tip("Lines added and removed"))
        .child(div().text_color(look.color(token::DIFF_ADD_FG)).child(SharedString::from(format!("+{added}"))))
        .child(div().text_color(look.color(token::DIFF_DEL_FG)).child(SharedString::from(format!("\u{2212}{removed}"))))
}

/// A card's head band: what it is, who said it, when, and the act at its end (`CommentHead`).
fn card_head(look: &Look, lead: Option<AnyElement>, who: Option<String>, bot: bool, ago: Option<String>, act: Option<AnyElement>) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .bg(look.color(token::BG_OVERLAY))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_MUTED))
        .children(lead)
        .children(who.map(|who| div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(if bot { token::FG_MUTED } else { token::FG_DEFAULT })).child(SharedString::from(who))))
        .children(ago.map(|ago| div().child(SharedString::from(ago))))
        .child(div().flex_1())
        .children(act)
}

/// A description with the HTML comments a template leaves taken out.
fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = rest[start..].find("-->").map_or("", |end| &rest[start + end + 3..]);
    }
    out.push_str(rest);
    out
}

impl JobView {
    /// A pull request's row was pressed: the panel opens over the Job and reads the page.
    pub(super) fn open_pull(&mut self, pull: Pull, window: &mut Window, cx: &mut Context<Self>) {
        let key = PullPanel::kept_key(&pull);
        let kept = self.draft_words(&key);
        let say = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 8));
        say.update(cx, |say, cx| say.set_value(kept, window, cx));
        let typing = cx.subscribe_in(&say, window, move |this, say, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let text = say.read(cx).value().to_string();
                this.keep_words(&key, &text, cx);
                cx.notify();
            }
        });
        let panel = PullPanel {
            pull,
            tab: PullTab::Conversation,
            page: None,
            say,
            write: PullRequestWriteKind::Comment,
            writing: false,
            write_said: None,
            sent_back: false,
            evaluated: HashSet::new(),
            copied: false,
            current: None,
            reach_open: HashSet::new(),
            files_focus: cx.focus_handle().tab_stop(true),
            files_at: None,
            folded: HashSet::new(),
            files_focused: std::cell::Cell::new(false),
            files_scroll: ScrollHandle::new(),
            scroll: ScrollHandle::new(),
            sections: ScrollHandle::new(),
            note_boxes: Vec::new(),
            notes_resolved: false,
            draft: pr_draft::DraftPane::default(),
            _typing: typing,
            _copied: None,
        };
        self.open_pull_panel(panel, window, cx);
        self.read_pull_page(window, cx);
    }

    /// `GET /pull_request_details/:repository/:number`, on mock data the scenario's page.
    fn read_pull_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let pull = panel.pull.clone();
        let Some(manifest) = pull.manifest.clone() else {
            if let Some(panel) = self.pull_panel_mut() {
                panel.page = Some(Err("This Job has no repository".into()));
            }
            return;
        };
        let number = pull.number;
        if self.fleet.is_none() {
            let page = crate::mock::pull_page(&format!("{manifest}#{number}")).ok_or_else(|| "The forge did not answer for this pull request".to_string());
            if let Some(panel) = self.pull_panel_mut() {
                panel.page = Some(page);
            }
            self.want_pull_patches(window, cx);
            self.want_pull_reach(window, cx);
            self.want_pull_notes(window, cx);
            cx.notify();
            return;
        }
        self.get(
            format!("/pull_request_details/{}/{number}", super::wire::encoded(&manifest)),
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<PullRequestPage>("PullRequestPage", &bytes).map_err(|why| why.to_string()));
                if let Some(panel) = this.pull_panel_mut().filter(|panel| panel.pull.number == number) {
                    panel.page = Some(read);
                }
                this.want_pull_patches(window, cx);
                this.want_pull_reach(window, cx);
                this.want_pull_notes(window, cx);
                cx.notify();
            }),
        );
    }

    /// The pull request as the Job's record holds it now, or as it was pressed where the record lets it go.
    /// One no Job holds takes its Checks from the forge's read of its page, once read.
    fn live_pull(&self, pressed: &Pull, cx: &gpui::App) -> Pull {
        if !pressed.held() {
            let mut pull = pressed.clone();
            if let Some(Ok(page)) = self.pull_panel().filter(|panel| panel.pull.number == pressed.number).and_then(|panel| panel.page.as_ref()) {
                (pull.checks, pull.failing) = super::pulls::checks_of_page(page);
            }
            return pull;
        }
        let job = self.shown.clone().map(|id| id.to_string()).unwrap_or_default();
        self.pulls_of(&job, cx).into_iter().find(|one| one.number == pressed.number && one.job == pressed.job).unwrap_or_else(|| pressed.clone())
    }

    fn pull_tab(&mut self, tab: PullTab, window: &mut Window, cx: &mut Context<Self>) {
        let entered = self.pull_panel_mut().map(|panel| {
            let entered = tab == PullTab::Files && panel.tab != PullTab::Files;
            panel.tab = tab;
            panel.scroll.set_offset(point(px(0.), px(0.)));
            entered
        });
        // His notes are read again on every entry to Files, since a Job resolves them while the panel is open.
        if entered == Some(true) {
            self.want_pull_notes(window, cx);
        }
        cx.notify();
    }

    /// The keys the panel takes before its box: `]` and `[` step the tabs where the box does not hold the keys.
    pub(super) fn pull_panel_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let key = event.keystroke.key.as_str();
        if !matches!(key, "[" | "]") || event.keystroke.modifiers.modified() {
            return false;
        }
        let Some(panel) = self.pull_panel() else { return false };
        if gpui::Focusable::focus_handle(panel.say.read(cx), cx).is_focused(window) {
            return false;
        }
        let at = PullTab::ALL.iter().position(|one| *one == panel.tab).unwrap_or(0) as isize;
        let by = if key == "]" { 1 } else { -1 };
        let next = PullTab::ALL[(at + by).rem_euclid(PullTab::ALL.len() as isize) as usize];
        self.pull_tab(next, window, cx);
        true
    }

    /// Notify Job: the Job is told which checks failed (`Checks failed on #N: a, b`).
    fn notify_job(&mut self, pull: &Pull, window: &mut Window, cx: &mut Context<Self>) {
        let text = format!("Checks failed on #{}: {}", pull.number, pull.failing);
        self.send_to_job(pull.job.clone(), text, format!("ledger:pr:{}", pull.number), window, cx, |this, _, cx| {
            if let Some(panel) = this.pull_panel_mut() {
                panel.sent_back = true;
            }
            cx.notify();
        });
    }

    /// Evaluate: one comment handed to the Job that holds the pull request, nothing posted to the forge.
    #[allow(clippy::too_many_arguments)]
    fn evaluate(&mut self, pull: &Pull, id: String, author: String, body: String, place: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let text = evaluate_message(pull.number, &id, &author, &body, place.as_deref());
        self.send_to_job(pull.job.clone(), text, format!("ledger:pr:{}", pull.number), window, cx, move |this, _, cx| {
            if let Some(panel) = this.pull_panel_mut() {
                panel.evaluated.insert(id);
            }
            cx.notify();
        });
    }

    pub(super) fn send_to_job(&mut self, job: String, text: String, row: String, window: &mut Window, cx: &mut Context<Self>, sent: impl FnOnce(&mut JobView, &mut Window, &mut Context<JobView>) + 'static) {
        let body = ipc::encode(&ipc::SendSessionMessageBody {
            message: ipc::SendSessionMessage { session_id: ipc::SessionId::carried(job), text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() },
            aim: None,
        });
        self.post(
            "/sessions/message".into(),
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| match answer {
                Ok(_) => sent(this, window, cx),
                Err(why) => {
                    if let Some(page) = this.shown.clone().and_then(|job| this.pages.get_mut(&job)) {
                        page.cards.said.insert(row, why.into());
                    }
                    cx.notify();
                }
            }),
        );
    }

    /// The comment box sends: a comment, an approval or a request for changes, posted to the forge
    /// (`POST /pull_request_writes/:repository`). Fleet's answer is the page read again; a refusal
    /// stands under the box and the words stay.
    fn write_pull(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let text = panel.say.read(cx).value().to_string();
        if panel.writing || !may_write(panel.write, &text) {
            return;
        }
        let pull = panel.pull.clone();
        let kind = panel.write;
        let Some(manifest) = pull.manifest.clone() else { return };
        if let Some(panel) = self.pull_panel_mut() {
            panel.writing = true;
            panel.write_said = None;
        }
        cx.notify();
        let body = ipc::encode(&ipc::WritePullRequest {
            pull_request: pull.number,
            kind,
            body: (!text.trim().is_empty()).then(|| text.clone()),
            comments: Vec::new(),
            commit: None,
            session_id: pull.held().then(|| ipc::SessionId::carried(pull.job.clone())),
        });
        let number = pull.number;
        let key = PullPanel::kept_key(&pull);
        self.post(
            format!("/pull_request_writes/{}", super::wire::encoded(&manifest)),
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let Some(panel) = this.pull_panel_mut().filter(|panel| panel.pull.number == number) else { return };
                panel.writing = false;
                match answer {
                    Ok(bytes) => {
                        if let Ok(page) = ipc::decode::<PullRequestPage>("PullRequestPage", &bytes) {
                            panel.page = Some(Ok(page));
                        }
                        let say = panel.say.clone();
                        say.update(cx, |say, cx| say.set_value("", window, cx));
                        this.keep_words(&key, "", cx);
                    }
                    Err(why) => panel.write_said = Some(why.into()),
                }
                cx.notify();
            }),
        );
    }

    fn pull_copied(&mut self, cx: &mut Context<Self>) {
        let wait = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(panel) = this.pull_panel_mut() {
                    panel.copied = false;
                }
                cx.notify();
            });
        });
        if let Some(panel) = self.pull_panel_mut() {
            panel.copied = true;
            panel._copied = Some(wait);
        }
        cx.notify();
    }
}

impl JobView {
    /// The panel: the file list beside it on Files; then the sheet with its head, body and comment box.
    /// The plates stand inside the body at every width, where Classic stands them off the leading edge.
    pub(super) fn render_pull_panel(&self, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        let (focus, panel) = self.pull_panel_focus()?;
        let look = cx.look();
        let look = &look;
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let pull = self.live_pull(&panel.pull, cx);
        let title: SharedString = format!("Pull request #{}", pull.number).into();
        let close = labelled(
            icon_button(look, "pull-panel-close", "x", "Close \u{b7} Esc".into()).on_click(cx.listener(|this, _, window, cx| this.close_panel(window, cx))),
            Role::Button,
            "Close",
        );
        let titles = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().truncate().text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title.clone()))
            .children(pull.manifest.clone().map(|name| div().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(name))));
        let head = div()
            .flex_none()
            .flex()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT));
        // Opened from an issue, the way back and the way out share the head's first line, and the title takes the line under them (`Sheet` back).
        let head = match self.panel_came_from().cloned() {
            Some(issue) => {
                let said = format!("Back to {}", issue.label());
                let (ground, ink, muted) = (look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::FG_MUTED));
                let back = labelled(
                    div()
                        .id("pull-panel-back")
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .h(look.length(token::H_CONTROL_SM))
                        .pl(look.length(token::SPACE_2))
                        .pr(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(muted)
                        .cursor_pointer()
                        .hover(move |style| style.bg(ground).text_color(ink))
                        .tooltip(tip(said.clone()))
                        .child(glyph(look, "chevron-left", Size::Small, muted))
                        .child(SharedString::from(said.clone()))
                        .on_click(cx.listener(move |this, _, window, cx| this.open_issue(issue.clone(), window, cx))),
                    Role::Button,
                    said,
                );
                head.flex_col().child(div().flex().items_center().justify_between().child(back).child(close)).child(titles)
            }
            None => head.items_center().child(titles).child(close),
        };
        let margin = look.length(token::SPACE_4);
        let wide = look.length(token::W_DIALOG_WIDE).min(viewport.width - margin * 2.);
        let high = viewport.height - look.length(token::H_TITLE_BAR) - margin * 2.;
        let aside_w = px(336.);
        let room_left = viewport.width - wide - margin * 3.;
        let files_beside = panel.tab == PullTab::Files && panel.page().is_some_and(|page| !page.files.is_empty()) && room_left >= aside_w;
        let body = self.pull_body(panel, &pull, panel.tab != PullTab::Files, !files_beside, look, cx);
        let foot = self.pull_composer(panel, look, cx);
        let sheet = labelled(
            div()
                .id("pull-panel")
                .track_focus(&focus)
                .capture_key_down(cx.listener(Self::panel_key_down))
                .occlude()
                .flex_none()
                .w(wide)
                .h(high)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(head)
                .child(body)
                .child(foot),
            Role::Dialog,
            title,
        );
        let beside: Option<AnyElement> = if files_beside { panel.page().map(|page| self.pull_file_list(panel, page, aside_w, high, look, cx)) } else { None };
        Some(
            deferred(
                anchored().position(room.origin + point(px(0.), look.length(token::H_TITLE_BAR))).anchor(Anchor::TopLeft).child(
                    div()
                        .id("pull-panel-scrim")
                        .occlude()
                        .w(viewport.width)
                        .h(viewport.height - look.length(token::H_TITLE_BAR))
                        .flex()
                        .justify_end()
                        .items_start()
                        .gap(look.length(token::SPACE_3))
                        .pt(margin)
                        .pr(margin)
                        .bg(look.color(token::SCRIM))
                        .on_click(cx.listener(|this, _, window, cx| this.close_panel(window, cx)))
                        .children(beside)
                        .child(sheet),
                ),
            )
            .with_priority(0)
            .into_any_element(),
        )
    }

    /// The column of plates (`PullRequestPlates`): status with its facts, reviewers, and the failing checks.
    fn pull_plates(&self, panel: &PullPanel, pull: &Pull, look: &Look, cx: &Context<Self>) -> AnyElement {
        let page = panel.page();
        let place = self.place_of_open(pull, cx);
        let at = place.as_ref().and_then(|rows| rows.iter().find(|row| row.current)).map(|row| row.place);
        let (what, icon, tone) = standing(pull, at);
        let hue = look.color(tone.hue());
        let failing: Vec<String> = if pull.checks == PullChecks::Failed && !pull.ended() {
            pull.failing.split(',').map(|one| one.trim().to_string()).filter(|one| !one.is_empty()).collect()
        } else {
            Vec::new()
        };
        let ring = div()
            .flex_none()
            .size(look.length(token::SPACE_12) - look.length(token::SPACE_2))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(hue.opacity(0.16))
            .child(glyph(look, icon, Size::Large, hue));
        let why = (!failing.is_empty()).then(|| div().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(failing.join(", "))));
        let status = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(ring)
            .child(div().flex_1().min_w_0().flex().flex_col().child(div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(what.clone())).children(why))
            .children(at.map(|at| {
                labelled(
                    div()
                        .id("pull-place")
                        .flex_none()
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XL))
                        .line_height(gpui::relative(1.))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .tooltip(tip("Place in the merge queue"))
                        .child(SharedString::from(at.to_string())),
                    Role::Image,
                    format!("Place in the merge queue: {at}"),
                )
            }));
        // A fact the status already says is not said twice.
        let mut facts: Vec<(&'static str, &'static str, &'static str, Option<Tone>)> = Vec::new();
        let (check_icon, check_said, check_tone) = checks_fact(pull);
        let said_already = matches!((what.as_ref(), pull.checks), ("Checks failed", _) | ("Checks running", _) | ("Ready to merge", PullChecks::Passed));
        if !said_already {
            facts.push(("checks", check_icon, check_said, Some(check_tone)));
        }
        if pull.auto && !pull.ended() {
            facts.push(("auto", "git-merge", "Auto-merge on", None));
        }
        if let Some(page) = page {
            match (page.review, page.approvals) {
                (PullRequestReviewState::ChangesRequested, _) => facts.push(("review", "ban", "Changes requested", Some(Tone::Failed))),
                (PullRequestReviewState::Approved, _) | (_, 1..) => facts.push(("review", "check", "Approved", Some(Tone::Success))),
                (PullRequestReviewState::Required, _) => facts.push(("review", "user-check", "Review required", None)),
                (PullRequestReviewState::None, _) => {}
            }
            if !pull.ended() {
                match page.mergeable {
                    PullRequestMergeable::Conflicting => facts.push(("conflicts", "git-branch", "Conflicts", Some(Tone::Failed))),
                    PullRequestMergeable::Mergeable => facts.push(("conflicts", "git-branch", "No conflicts", Some(Tone::Success))),
                    PullRequestMergeable::Unknown => {}
                }
            }
        }
        let facts = (!facts.is_empty()).then(|| {
            let cells: Vec<AnyElement> = facts
                .into_iter()
                .map(|(key, icon, said, tone)| {
                    let ink = tone.map_or(look.color(token::FG_MUTED), |tone| look.color(tone.hue()));
                    labelled(
                        div()
                            .id(SharedString::from(format!("pull-fact:{key}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .min_w_0()
                            .p(look.length(token::SPACE_2))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_SUBTLE))
                            .rounded(look.length(token::RADIUS_SM))
                            .bg(look.color(token::BG_RAISED))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .whitespace_nowrap()
                            .child(glyph(look, icon, Size::Small, ink))
                            .child(div().min_w_0().truncate().child(said)),
                        Role::ListItem,
                        said,
                    )
                    .into_any_element()
                })
                .collect();
            let mut rows: Vec<AnyElement> = Vec::new();
            let mut cells = cells.into_iter();
            while let Some(first) = cells.next() {
                let second = cells.next();
                rows.push(div().flex().gap(look.length(token::SPACE_2)).child(div().flex_1().min_w_0().child(first)).child(div().flex_1().min_w_0().children(second)).into_any_element());
            }
            labelled(div().id("pull-facts").flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::List, "Facts")
        });
        let status = labelled(plate(look, tone).id("pull-status").child(status).children(facts), Role::Region, "Status");
        let reviewers = page.map(|page| page.reviewers.as_slice()).filter(|reviewers| !reviewers.is_empty()).map(|reviewers| self.pull_reviewers(reviewers, look, cx));
        let failing_plate = (!failing.is_empty()).then(|| {
            let sent = panel.sent_back;
            let held = pull.clone();
            let word = if sent { "Notified" } else { "Notify Job" };
            let notify = pull.held().then(|| labelled(
                small_button(look, "pull-notify", word, true, !sent)
                    .tooltip(tip("Tells the Job which checks failed"))
                    .when(!sent, |button| button.on_click(cx.listener(move |this, _, window, cx| this.notify_job(&held, window, cx)))),
                Role::Button,
                word,
            ));
            labelled(
                plate(look, Tone::Failed)
                    .id("pull-failing")
                    .child(eyebrow(look).child(glyph(look, "circle-x", Size::Small, look.color(Tone::Failed.hue()))).child("Failing checks"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .children(failing.iter().map(|name| div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).child(SharedString::from(name.clone())))),
                    )
                    .child(div().flex().children(notify)),
                Role::Region,
                "Failing checks",
            )
        });
        let place = place.map(|rows| self.place_plate(rows, look, cx));
        div().flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().child(status).children(reviewers).children(place).children(failing_plate).into_any_element()
    }

    /// The merge lines the open pull request is read against: the mock's in a walk, else Fleet's.
    fn place_of_open(&self, pull: &Pull, cx: &gpui::App) -> Option<Vec<PlaceRow>> {
        if let Some(lines) = crate::mock::merge_lines() {
            return place_of(&lines, pull);
        }
        place_of(self.fleet.as_ref()?.read(cx).held().merge_lines.as_deref()?, pull)
    }

    /// Who the merge line says the pull request was registered to, where its page names nobody.
    fn owner_in_lines(&self, pull: &Pull, cx: &gpui::App) -> Option<ipc::PullRequestOwner> {
        let find = |lines: &[ipc::MergeLineView]| lines.iter().flat_map(|line| &line.pulls).find(|view| view.pull.url == pull.address || (view.pull.number == pull.number && view.pull.branch == pull.branch)).and_then(|view| view.pull.owner.clone());
        if let Some(lines) = crate::mock::merge_lines() {
            return find(&lines);
        }
        find(self.fleet.as_ref()?.read(cx).held().merge_lines.as_deref()?)
    }

    /// The queue the pull request is in (`MergeLinePlace`), on a running plate: place, mark, then number
    /// and title, which open it on the forge. **No count**: the rows are the answer. Its own row takes
    /// the selected fill; the others are set back.
    fn place_plate(&self, rows: Vec<PlaceRow>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let muted = look.color(token::FG_MUTED);
        let items: Vec<AnyElement> = rows
            .into_iter()
            .map(|row| {
                let (icon, hue, pulsing) = queue_mark(row.state);
                let hue = look.color(hue);
                let says = crate::merge_strip::words::queue_reading(row.state).says;
                let mark = div().id(SharedString::from(format!("place-mark:{}", row.number))).flex_none().w(look.length(token::SPACE_4)).flex().justify_center().tooltip(tip(says)).child(glyph(look, icon, Size::Small, hue));
                let mark = if pulsing {
                    mark.with_animation(SharedString::from(format!("place-pulse:{}", row.number)), Animation::new(std::time::Duration::from_millis(1400)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)), |mark, at| mark.opacity(at)).into_any_element()
                } else {
                    mark.into_any_element()
                };
                let (border, ground) = if row.current {
                    (look.color(token::ACCENT), look.color(token::ACCENT_MUTED))
                } else {
                    (look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4)), look.color(token::BG_RAISED).blend(hue.opacity(0.05)))
                };
                let set_back = (!row.current).then_some(muted);
                let strong = look.color(token::FG_DEFAULT);
                let pointed = look.color(token::BG_RAISED).blend(hue.opacity(0.1));
                let url = row.url.clone();
                let name = format!("{}, {says}", row.title);
                labelled(
                    div()
                        .id(SharedString::from(format!("place-row:{}", row.number)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .min_w_0()
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .border(look.length(token::BORDER_WIDTH) * 2.)
                        .border_color(border)
                        .rounded(look.length(token::RADIUS_MD))
                        .bg(ground)
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(set_back.unwrap_or(look.color(token::FG_DEFAULT)))
                        .cursor_pointer()
                        .when(!row.current, |tile| tile.hover(move |style| style.text_color(strong).bg(pointed)))
                        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
                        .child(
                            div()
                                .id(SharedString::from(format!("place-at:{}", row.number)))
                                .flex_none()
                                .w(look.length(token::SPACE_4))
                                .flex()
                                .justify_end()
                                .font_family(mono(look))
                                .text_size(look.length(token::TEXT_XS))
                                .text_color(muted)
                                .tooltip(tip("Place in the merge queue"))
                                .child(SharedString::from(row.place.to_string())),
                        )
                        .child(mark)
                        .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(set_back.unwrap_or(look.color(token::ACCENT))).child(SharedString::from(format!("#{}", row.number))))
                        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(row.title.clone()))),
                    Role::ListItem,
                    name,
                )
                .into_any_element()
            })
            .collect();
        labelled(
            plate(look, Tone::Running)
                .id("pull-place-plate")
                .child(eyebrow(look).child(glyph(look, "git-merge", Size::Small, muted)).child("Merge line"))
                .child(labelled(div().id("pull-place-queue").flex().flex_col().gap(look.length(token::SPACE_2)).children(items), Role::List, "Merge queue")),
            Role::Region,
            "Merge line",
        )
        .into_any_element()
    }

    /// Reviewers (`PullRequestReviewers`): a row each, a requested team's people under its row.
    fn pull_reviewers(&self, reviewers: &[PullRequestReviewer], look: &Look, cx: &Context<Self>) -> AnyElement {
        let teams: Vec<&PullRequestReviewer> = reviewers.iter().filter(|one| one.kind == PullRequestReviewerKind::Team).collect();
        let people: Vec<&PullRequestReviewer> = reviewers.iter().filter(|one| one.kind == PullRequestReviewerKind::Person).collect();
        let row = |one: &PullRequestReviewer| -> AnyElement {
            let (icon, said, tone) = verdict_look(one.verdict);
            let login = SharedString::from(one.login.clone());
            let who = div()
                .id(SharedString::from(format!("reviewer:{}", one.login)))
                .min_w_0()
                .truncate()
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(login.clone());
            let who = who.when_some(one.name.clone(), |who, name| who.tooltip(tip(name)));
            let who = match one.url.clone() {
                Some(url) => labelled(who.cursor_pointer().hover(|style| style.underline()).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))), Role::Link, login.clone()).into_any_element(),
                None => who.into_any_element(),
            };
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .child(who)
                .children((one.kind == PullRequestReviewerKind::Team).then(|| mark(look, format!("reviewer-team:{}", one.login), "users", "Team", None)))
                .child(div().flex_1())
                .child(mark(look, format!("reviewer-verdict:{}", one.login), icon, said, tone))
                .into_any_element()
        };
        let mut out: Vec<AnyElement> = Vec::new();
        for team in &teams {
            out.push(row(team));
            let under: Vec<&PullRequestReviewer> = people.iter().filter(|one| one.team.as_deref() == Some(team.login.as_str())).copied().collect();
            if !under.is_empty() {
                out.push(div().flex().flex_col().gap(look.length(token::SPACE_2)).pl(look.length(token::SPACE_4)).children(under.into_iter().map(row)).into_any_element());
            }
        }
        out.extend(people.iter().filter(|one| one.team.as_deref().is_none_or(|team| !teams.iter().any(|it| it.login == team))).map(|one| row(one)));
        labelled(
            plate(look, Tone::Neutral).id("pull-reviewers").child(eyebrow(look).child("Reviewers")).child(div().flex().flex_col().gap(look.length(token::SPACE_2)).children(out)),
            Role::Region,
            "Reviewers",
        )
        .into_any_element()
    }
}

impl JobView {
    /// The body: what Fleet refused, the title, its meta row, its acts, then the tabs.
    fn pull_body(&self, panel: &PullPanel, pull: &Pull, plates_inside: bool, tree_inside: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let page = panel.page();
        let number = pull.number;
        let refused = self
            .shown
            .as_ref()
            .and_then(|job| self.pages.get(job))
            .and_then(|page| page.cards.said.get(&format!("ledger:pr:{number}")).cloned())
            .map(|why| super::acts::refusal(look, &format!("pull-panel:{number}"), why));
        let (pill_icon, pill_said, pill_hue) = match (pull.state, pull.queued) {
            (PullState::Merged, _) => ("git-merge", "Merged", token::STATUS_COMPLETED_SUCCESS),
            (PullState::Closed, _) => ("git-pull-request-closed", "Closed", token::STATUS_NOT_STARTED),
            (_, true) => ("clock", "In merge queue", token::STATUS_RUNNING),
            (PullState::Draft, _) => ("git-pull-request", "Draft", token::STATUS_NOT_STARTED),
            (PullState::Open, _) => ("git-pull-request", "Open", token::STATUS_COMPLETED_SUCCESS),
        };
        let hue = look.color(pill_hue);
        let pill = labelled(
            div()
                .id("pull-pill")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_2))
                .py(px(1.))
                .rounded_full()
                .bg(hue.opacity(0.14))
                .text_color(hue)
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .child(glyph(look, pill_icon, Size::Small, hue))
                .child(pill_said),
            Role::Status,
            pill_said,
        );
        let branch_words = match page {
            Some(page) if !page.base.is_empty() => format!("{} \u{2192} {}", pull.branch, page.base),
            _ => pull.branch.clone(),
        };
        // One no Job holds has no branch name: the forge's page carries the base and not the head's name.
        let branch = (!pull.branch.is_empty()).then(|| div()
            .id("pull-branch")
            .min_w_0()
            .truncate()
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .bg(look.color(token::BG_SUNKEN))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .tooltip(tip(if page.is_some() { "Branch and the branch it merges into" } else { "Branch" }))
            .child(SharedString::from(branch_words)));
        let now = Local::now();
        let fact = |id: &'static str, said: &'static str, words: String| div().id(id).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip(said)).child(SharedString::from(words));
        let author = page.and_then(|page| page.author.clone()).map(|author| fact("pull-author", "Opened by", author));
        let opened = page.and_then(|page| page.created_at.as_deref()).and_then(|at| age(at, now)).map(|ago| fact("pull-opened", "Opened", ago));
        let size = page.map(|page| diff_size(look, "pull-size", page.additions, page.deletions));
        let owner = page.and_then(|page| page.owner.clone()).or_else(|| self.owner_in_lines(pull, cx)).map(|owner| owner_chip(&owner, look));
        let meta = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).min_w_0().child(pill).children(branch).children(author).children(opened).children(size).children(owner);
        let acts = self.pull_acts(panel, pull, look, cx);
        let plates = plates_inside.then(|| self.pull_plates(panel, pull, look, cx));
        let tabs = match &panel.page {
            None => labelled(div().id("pull-reading").child(stand_ins(look, "pull-reading", &[1.0, 0.85, 0.6, 0.9, 0.4])), Role::Status, format!("Reading pull request #{number}")).into_any_element(),
            Some(Err(said)) => labelled(div().id("pull-page-said").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())), Role::Alert, said.clone()).into_any_element(),
            Some(Ok(page)) => self.pull_tabs(panel, pull, page, tree_inside, look, cx),
        };
        div()
            .id("pull-body")
            .track_scroll(&panel.scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .children(refused)
            .child(div().text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(pull.title.clone())))
            .child(meta)
            .child(acts)
            .children(plates)
            .children(self.notes_bar(panel, pull, look, cx))
            .children(self.notes_resolved_toggle(panel, look, cx))
            .child(tabs)
            .into_any_element()
    }

    /// The acts row (`PullRequestActs`), then Copy link and Open in browser at its end.
    fn pull_acts(&self, panel: &PullPanel, pull: &Pull, look: &Look, cx: &Context<Self>) -> AnyElement {
        let pending = self.panes.pull_pending.get(&format!("{}#{}", pull.job, pull.number)).copied();
        let act: Option<(Option<PullAct>, &'static str, &'static str)> = if pull.ended() {
            None
        } else if pull.queued {
            Some((None, "In merge queue", "It merges when its turn in the queue comes"))
        } else if pull.state == PullState::Draft {
            Some((Some(PullAct::Ready), "Ready for review", "Marks it ready for review"))
        } else {
            match pull.checks {
                PullChecks::Passed => Some((Some(PullAct::Merge), "Merge", "Merges it now")),
                PullChecks::Running if pull.auto => Some((None, "Auto-merge on", "It merges when every check passes")),
                PullChecks::Running => Some((Some(PullAct::AutoMerge), "Enable auto-merge", "Merges it when every check passes")),
                PullChecks::Failed => None,
            }
        };
        let act = act.map(|(which, word, said)| {
            let on = which.is_some() && pending.is_none();
            let spinning = which.is_some() && pending == which;
            let held = pull.clone();
            let button = small_button(look, "pull-act", word, true, on)
                .tooltip(tip(said))
                .children(spinning.then(|| glyph(look, "loader-circle", Size::Small, look.color(token::TEXT_ON_ACCENT))))
                .when_some(which.filter(|_| on), |button, which| button.on_click(cx.listener(move |this, _, window, cx| this.press_pull(&held, which, format!("ledger:pr:{}", held.number), window, cx))));
            labelled(button, Role::Button, word)
        });
        let address = pull.address.clone();
        let copy = (!address.is_empty()).then(|| {
            let said: SharedString = if panel.copied { "Copied".into() } else { "Copy link".into() };
            let address = address.clone();
            labelled(
                icon_button(look, "pull-copy", if panel.copied { "check" } else { "link" }, said.clone()).on_click(cx.listener(move |this, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(address.clone()));
                    this.pull_copied(cx);
                })),
                Role::Button,
                said,
            )
        });
        let open = (!address.is_empty()).then(|| labelled(icon_button(look, "pull-open", "external-link", "Open in browser".into()).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&address))), Role::Button, "Open in browser"));
        labelled(div().id("pull-acts").flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).children(act).child(div().flex_1()).children(copy).children(open), Role::Group, "Acts").into_any_element()
    }

    /// The tab strip and the tab shown (`PullRequestTabs`).
    fn pull_tabs(&self, panel: &PullPanel, pull: &Pull, page: &PullRequestPage, tree_inside: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let accent = look.color(token::ACCENT);
        let strong = look.color(token::FG_DEFAULT);
        let strip = div().id("pull-tabs").flex().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).children(PullTab::ALL.into_iter().map(|one| {
            let front = one == panel.tab;
            labelled(
                div()
                    .id(SharedString::from(format!("pull-tab:{}", one.word())))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                    .text_color(if front { strong } else { look.color(token::FG_MUTED) })
                    .border_b(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(if front { accent } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .hover(move |style| style.text_color(strong))
                    .tooltip(tip(format!("{} \u{b7} [ ]", one.word())))
                    .on_click(cx.listener(move |this, _, window, cx| this.pull_tab(one, window, cx)))
                    .child(one.word()),
                Role::Tab,
                one.word(),
            )
        }));
        let shown: Option<AnyElement> = match panel.tab {
            PullTab::Conversation => self.pull_conversation(panel, pull, page, look, cx),
            PullTab::Checks => pull_checks(page, look, cx),
            PullTab::Files => self.pull_files(panel, page, tree_inside, look, cx),
            PullTab::Commits => pull_commits(page, look),
        };
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(labelled(strip, Role::TabList, "Pull request"))
            .children(shown.map(|shown| labelled(div().id("pull-tab-panel").child(shown), Role::TabPanel, panel.tab.word())))
            .into_any_element()
    }

    /// Conversation: the description, then what was said and the threads on the diff, in the order
    /// they were said (`Conversation`). Nothing said, nothing drawn.
    fn pull_conversation(&self, panel: &PullPanel, pull: &Pull, page: &PullRequestPage, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        enum Said<'a> {
            Entry(&'a PullRequestEntry),
            Thread(&'a PullReviewThread),
        }
        let description = strip_comments(page.body.as_deref().unwrap_or(""));
        let time = |at: &str| chrono::DateTime::parse_from_rfc3339(at).map(|at| at.timestamp_millis()).unwrap_or(0);
        let mut said: Vec<(i64, Said)> = page
            .conversation
            .iter()
            .map(|entry| {
                let at = match entry {
                    PullRequestEntry::Comment { at, .. } | PullRequestEntry::Review { at, .. } => at,
                };
                (time(at), Said::Entry(entry))
            })
            .chain(page.threads.iter().filter_map(|thread| thread.comments.first().map(|first| (time(&first.at), Said::Thread(thread)))))
            .collect();
        said.sort_by_key(|(at, _)| *at);
        if description.trim().is_empty() && said.is_empty() && page.draft.is_none() {
            return None;
        }
        let now = Local::now();
        let mut cards: Vec<AnyElement> = Vec::new();
        if let Some(draft) = &page.draft {
            cards.push(self.pull_draft_card(panel, page, draft, look, cx));
        } else if !description.trim().is_empty() {
            let lead = mark(look, "pull-description-mark".into(), "file-text", "Description", None);
            cards.push(self.pull_card("pull-description", Some(lead), page.author.clone(), false, page.created_at.as_deref(), None, &description, "Description".into(), look, now, cx));
        }
        for (_, one) in said {
            match one {
                Said::Entry(entry) => {
                    let (id, author, bot, at, body, verdict) = match entry {
                        PullRequestEntry::Comment { id, author, bot, at, body, .. } => (id, author, *bot, at, body, None),
                        PullRequestEntry::Review { id, author, bot, at, body, verdict, .. } => (id, author, *bot, at, body, Some(*verdict)),
                    };
                    let lead = verdict.map(|verdict| {
                        let (icon, said, tone) = verdict_look(Some(verdict));
                        mark(look, format!("pull-verdict:{id}"), icon, said, tone)
                    });
                    let act = (!body.trim().is_empty() && pull.held()).then(|| self.evaluate_act(panel, pull, id, author, body, None, look, cx));
                    cards.push(self.pull_card(&format!("pull-entry:{id}"), lead, Some(author.clone()), bot, Some(at), act, body, author.clone(), look, now, cx));
                }
                Said::Thread(thread) => cards.push(self.pull_thread(panel, pull, thread, look, now, cx)),
            }
        }
        Some(labelled(div().id("pull-conversation").flex().flex_col().gap(look.length(token::SPACE_3)).children(cards), Role::List, "Conversation").into_any_element())
    }

    /// Evaluate at the head's end, or the mark that it was sent in the same cell (`Evaluate`).
    #[allow(clippy::too_many_arguments)]
    fn evaluate_act(&self, panel: &PullPanel, pull: &Pull, id: &str, author: &str, body: &str, place: Option<String>, look: &Look, cx: &Context<Self>) -> AnyElement {
        if panel.evaluated.contains(id) {
            return mark(look, format!("pull-evaluated:{id}"), "check", "Sent to the Job", Some(Tone::Success));
        }
        let name = format!("Evaluate: {author}{}", place.as_ref().map(|place| format!(" on {place}")).unwrap_or_default());
        let (held, id, author, body) = (pull.clone(), id.to_string(), author.to_string(), body.to_string());
        labelled(
            small_button(look, SharedString::from(format!("pull-evaluate:{id}")), "Evaluate", false, true)
                .tooltip(tip("Hands this comment to the Job to weigh"))
                .on_click(cx.listener(move |this, _, window, cx| this.evaluate(&held, id.clone(), author.clone(), body.clone(), place.clone(), window, cx))),
            Role::Button,
            name,
        )
        .into_any_element()
    }

    /// One card: its head band (what, who, when, and the act), then its words (`Entry`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn pull_card(&self, id: &str, lead: Option<AnyElement>, who: Option<String>, bot: bool, at: Option<&str>, act: Option<AnyElement>, body: &str, name: String, look: &Look, now: chrono::DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let head = card_head(look, lead, who, bot, at.and_then(|at| age(at, now)), act);
        let words = (!body.trim().is_empty()).then(|| div().min_w_0().p(look.length(token::SPACE_3)).child(self.prose(format!("{id}:body"), body, look, cx)));
        labelled(
            div()
                .id(SharedString::from(id.to_string()))
                .min_w_0()
                .overflow_hidden()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED))
                .child(head)
                .children(words),
            Role::Article,
            name,
        )
        .into_any_element()
    }

    /// A thread on the diff: where it is, the lines it is about, then each comment (`ThreadEntry`).
    pub(super) fn pull_thread(&self, panel: &PullPanel, pull: &Pull, thread: &PullReviewThread, look: &Look, now: chrono::DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let place = where_of(thread);
        let edge = if thread.resolved { look.color(token::FG_SUBTLE) } else { look.color(token::ACCENT) };
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::BG_OVERLAY))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_family(mono(look))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(if thread.resolved { token::FG_MUTED } else { token::FG_DEFAULT }))
                    .child(SharedString::from(place.clone())),
            )
            .child(div().flex_none().w(look.length(token::SPACE_6)).flex().justify_end().children(thread.resolved.then(|| mark(look, format!("pull-thread-resolved:{}", thread.id), "check", "Resolved", Some(Tone::Success)))));
        let excerpt = self.thread_excerpt(panel, thread, look);
        let comments: Vec<AnyElement> = thread
            .comments
            .iter()
            .enumerate()
            .map(|(at, one)| {
                // Evaluate hands a comment to the Job holding the pull request; one no Job holds has none.
                let act = pull.held().then(|| self.evaluate_act(panel, pull, &one.id, &one.author, &one.body, Some(place.clone()), look, cx));
                let head = card_head(look, None, Some(one.author.clone()), one.bot, age(&one.at, now), act);
                let words = (!one.body.trim().is_empty())
                    .then(|| div().min_w_0().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_1)).pb(look.length(token::SPACE_3)).child(self.prose(format!("pull-thread:{}:{}", thread.id, one.id), &one.body, look, cx)));
                labelled(
                    div()
                        .id(SharedString::from(format!("pull-thread-comment:{}", one.id)))
                        .when(at > 0, |comment| comment.ml(look.length(token::SPACE_3)).border_l(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)))
                        .child(head)
                        .children(words),
                    Role::Article,
                    one.author.clone(),
                )
                .into_any_element()
            })
            .collect();
        labelled(
            div()
                .id(SharedString::from(format!("pull-thread:{}", thread.id)))
                .flex()
                .min_w_0()
                .overflow_hidden()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED))
                .child(div().flex_none().w(look.length(token::SPACE_1)).bg(edge))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(head)
                        .children(excerpt)
                        .child(labelled(div().id(SharedString::from(format!("pull-thread-comments:{}", thread.id))).flex().flex_col().children(comments), Role::List, format!("Comments on {place}"))),
                ),
            Role::Article,
            format!("Thread on {place}"),
        )
        .into_any_element()
    }

    /// The comment box at the panel's foot (`PullRequestComposer`): the box, Comment, Approve or Request
    /// changes, and the button that sends it.
    fn pull_composer(&self, panel: &PullPanel, look: &Look, cx: &Context<Self>) -> AnyElement {
        let text = panel.say.read(cx).value().to_string();
        let kind = panel.write;
        let on = !panel.writing && may_write(kind, &text);
        let pick = |which: PullRequestWriteKind| {
            let front = which == kind;
            let word = write_word(which, false);
            labelled(
                div()
                    .id(SharedString::from(format!("pull-write:{word}")))
                    .px(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL_SM) - look.length(token::SPACE_1))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_SM))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                    .text_color(look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED }))
                    .when(front, |tab| tab.bg(look.color(token::BG_RAISED)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(panel) = this.pull_panel_mut() {
                            panel.write = which;
                        }
                        cx.notify();
                    }))
                    .child(word),
                Role::Tab,
                word,
            )
        };
        let kinds = labelled(
            div()
                .id("pull-write-kinds")
                .flex()
                .items_center()
                .gap(px(2.))
                .p(px(2.))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_SUNKEN))
                .child(pick(PullRequestWriteKind::Comment))
                .child(pick(PullRequestWriteKind::Approve))
                .child(pick(PullRequestWriteKind::RequestChanges)),
            Role::TabList,
            "Write",
        );
        let word = write_word(kind, panel.writing);
        let send = labelled(
            small_button(look, "pull-write-send", word, true, on).tooltip(tip(format!("{word} \u{b7} \u{2318}\u{21a9}"))).when(on, |button| button.on_click(cx.listener(|this, _, window, cx| this.write_pull(window, cx)))),
            Role::Button,
            word,
        );
        let said = panel.write_said.clone().map(|why| labelled(div().id("pull-write-said").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why.clone()), Role::Alert, why));
        let field = div()
            .id("pull-comment")
            // ⌘↩ sends. The text area takes the key as its own action before any key listener, so it is
            // caught there, ahead of the line break it would put in.
            .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                if action.secondary {
                    cx.stop_propagation();
                    this.write_pull(window, cx);
                }
            }))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .child(Textarea::new(&panel.say).appearance(false));
        div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(labelled(field, Role::TextInput, "Comment"))
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(kinds).child(div().flex_1()).child(send))
            .children(said)
            .into_any_element()
    }
}

/// Checks: each of the forge's checks, how long it ran, and its log (`Checks`). None, nothing drawn.
fn pull_checks(page: &PullRequestPage, look: &Look, cx: &Context<JobView>) -> Option<AnyElement> {
    if page.checks.is_empty() {
        return None;
    }
    let last = page.checks.len() - 1;
    let rows: Vec<AnyElement> = page
        .checks
        .iter()
        .enumerate()
        .map(|(at, row)| {
            let (icon, said, tone) = check_look(row.state);
            let figure = row.seconds.map(|seconds| div().id(SharedString::from(format!("pull-check-ran:{at}"))).flex_none().font_family(mono(look)).tooltip(tip("Ran for")).child(SharedString::from(ran_for(seconds))));
            let log = row.address.clone().map(|address| {
                labelled(icon_button(look, SharedString::from(format!("pull-check-log:{at}")), "external-link", "Open the log".into()).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&address))), Role::Button, format!("Open the log of {}", row.name))
            });
            labelled(
                div()
                    .id(SharedString::from(format!("pull-check:{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .when(at < last, |row| row.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(mark(look, format!("pull-check-mark:{at}"), icon, said, tone))
                    .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.name.clone())))
                    .children(figure)
                    .child(div().flex_none().w(look.length(token::H_CONTROL_SM)).children(log)),
                Role::ListItem,
                format!("{}, {said}", row.name),
            )
            .into_any_element()
        })
        .collect();
    Some(labelled(div().id("pull-checks").flex().flex_col().children(rows), Role::List, "Checks").into_any_element())
}

/// Commits: each one's short sha, headline and author (`Commits`). None, nothing drawn.
fn pull_commits(page: &PullRequestPage, look: &Look) -> Option<AnyElement> {
    if page.commits.is_empty() {
        return None;
    }
    let last = page.commits.len() - 1;
    let rows: Vec<AnyElement> = page
        .commits
        .iter()
        .enumerate()
        .map(|(at, commit)| {
            labelled(
                div()
                    .id(SharedString::from(format!("pull-commit-row:{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .when(at < last, |row| row.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(div().id(SharedString::from(format!("pull-commit:{at}"))).flex_none().font_family(mono(look)).tooltip(tip("Commit")).child(SharedString::from(commit.sha.chars().take(7).collect::<String>())))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(commit.headline.clone())))
                    .children(commit.author.clone().map(|author| div().flex_none().child(SharedString::from(author)))),
                Role::ListItem,
                commit.headline.clone(),
            )
            .into_any_element()
        })
        .collect();
    Some(labelled(div().id("pull-commits").flex().flex_col().children(rows), Role::List, "Commits").into_any_element())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_check_ran_for_reads_as_electrons() {
        assert_eq!(ran_for(42), "42s");
        assert_eq!(ran_for(125), "2m 05s");
        assert_eq!(ran_for(3725), "1h 02m");
    }

    #[test]
    fn evaluate_names_a_line_or_a_file_and_quotes_the_comment() {
        let said = evaluate_message(12, "c1", "ana", "Too slow\n\nTry a map", Some("src/a.rs:40"));
        assert!(said.starts_with("Evaluate a comment on #12.\nComment: c1\nAuthor: ana\nLine: src/a.rs:40\n\n> Too slow\n>\n> Try a map\n"));
        assert!(evaluate_message(12, "c1", "ana", "x", Some("src/a.rs")).contains("\nFile: src/a.rs\n"));
        assert!(!evaluate_message(12, "c1", "ana", "x", None).contains("Line:"));
    }

    #[test]
    fn an_approval_may_go_without_words() {
        assert!(may_write(PullRequestWriteKind::Approve, ""));
        assert!(!may_write(PullRequestWriteKind::Comment, "  "));
        assert!(!may_write(PullRequestWriteKind::RequestChanges, ""));
    }

    #[test]
    fn a_template_comment_leaves_the_description() {
        assert_eq!(strip_comments("Fixes it.<!-- fill in --> Done."), "Fixes it. Done.");
    }
}

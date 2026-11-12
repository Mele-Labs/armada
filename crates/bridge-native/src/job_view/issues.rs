//! A Job's Issues and Jobs tabs (`IssuesBody`, `JobsBody`), and an issue's side panel (`IssuePanel`).
//!
//! **An issue is keyed on its repository and its number.** Armada Classic keys it on the number
//! alone, so two repositories' #7 fold into one row. A row's menu teaches which Ticket state the
//! tracker's status reads as (`ticket-word-menu.tsx`); the running Ticket lifecycle is read once for
//! each repository as the tab shows, and again after a teach.

use std::collections::HashSet;
use std::rc::Rc;

use chrono::Local;
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, ScrollHandle, SharedString, Window};
use ipc::{AppliedLifecycle, IssueRead, IssueState, LifecycleStateDefinition, LinkedPullRequestState, ManifestId, RepositoryLifecycles, SessionRecord, StateReadFrom, TeachTicketWord};

use super::menus::Pick;
use super::pr_panel::{icon_button, mark, stand_ins, Tone};
use super::rows::{age, glyph, mono, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// The states `teach_ticket_word` takes: the menu's id, Armada's label, and the registry's glyph.
const TICKET_STATES: [(&str, &str, &str, &str); 4] = [
    ("ticket-state:todo", "todo", "To do", "circle-dashed"),
    ("ticket-state:in_progress", "in_progress", "In progress", "circle-dot"),
    ("ticket-state:in_review", "in_review", "In review", "eye"),
    ("ticket-state:closed", "closed", "Closed", "circle-check"),
];

/// An issue a Job works, claimed or cited, with the repository it is read from.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct IssueItem {
    /// The forge's number as digits, or a tracker's key.
    pub reference: String,
    pub title: String,
    pub address: String,
    pub closed: bool,
    /// The tracker's status as it said it, such as `Ready for QA`.
    pub status: Option<String>,
    pub manifest: Option<ManifestId>,
}

impl IssueItem {
    /// `#7`, or a tracker's key as it is (`issueLabel`).
    pub(super) fn label(&self) -> String {
        if !self.reference.is_empty() && self.reference.bytes().all(|byte| byte.is_ascii_digit()) {
            format!("#{}", self.reference)
        } else {
            self.reference.clone()
        }
    }

    /// What the tracker calls its state: its own status, else Open or Closed (`ticketWordOf`).
    fn word(&self) -> String {
        match self.status.as_deref().map(str::trim).filter(|said| !said.is_empty()) {
            Some(said) => said.to_string(),
            None if self.closed => "Closed".into(),
            None => "Open".into(),
        }
    }

    fn key(&self) -> String {
        format!("{}#{}", self.manifest.as_ref().map_or("", |id| id.as_str()), self.reference)
    }
}

/// An issue attachment as its row reads it.
pub(super) fn issue_of(record: &SessionRecord, one: &ipc::Attachment) -> IssueItem {
    let detail = |key: &str| one.detail.get(key).cloned();
    IssueItem {
        reference: one.target.clone(),
        title: detail("title").unwrap_or_default(),
        address: detail("address").unwrap_or_default(),
        closed: detail("state").as_deref() == Some("closed"),
        status: detail("status").filter(|said| !said.is_empty()),
        manifest: one.manifest_id.clone().or_else(|| record.manifest_id.clone()),
    }
}

/// The Ticket lifecycle that runs for a repository: of those not replaced, the one kept nearest it.
fn running_ticket(applied: &[AppliedLifecycle]) -> Vec<LifecycleStateDefinition> {
    let reach = |one: &AppliedLifecycle| match one.kept.rank.reach.as_str() {
        "workspace" => 1,
        "repository" => 2,
        _ => 0,
    };
    let mut best: Option<&AppliedLifecycle> = None;
    for one in applied.iter().filter(|one| one.lifecycle.id == "ticket" && one.kept.overridden_by.is_none()) {
        if best.is_none_or(|held| reach(one) > reach(held)) {
            best = Some(one);
        }
    }
    best.map(|one| one.lifecycle.states.clone()).unwrap_or_default()
}

/// The state whose tracker words hold `word` exactly.
fn read_as<'a>(states: &'a [LifecycleStateDefinition], word: &str) -> Option<&'a LifecycleStateDefinition> {
    states.iter().find(|one| matches!(&one.read, StateReadFrom::Tracker { words } if words.iter().any(|one| one == word)))
}

/// The glyph and words of the Ticket state a status reads as, where the registry draws one (`ticketMarkOf`).
fn ticket_mark(states: &[LifecycleStateDefinition], word: &str) -> Option<(&'static str, String)> {
    let state = read_as(states, word)?;
    let glyph = TICKET_STATES.iter().find(|(_, id, ..)| *id == state.id)?.3;
    Some((glyph, format!("Ticket state: {}", state.label)))
}

/// The menu: its heading, then each state, the one the word reads as now ticked (`ticketWordEntries`).
fn ticket_entries(label: &str, word: &str, states: Option<&Vec<LifecycleStateDefinition>>) -> Vec<Entry> {
    let ticked = states.and_then(|states| read_as(states, word)).map(|one| one.id.clone());
    let mut entries = vec![Entry::Label(crate::titlebar::id_of(format!("{label} is {word}. In Armada that is")))];
    entries.extend(TICKET_STATES.iter().map(|(pick, id, default, _)| Entry::Choice {
        id: pick,
        label: states.and_then(|states| states.iter().find(|one| one.id == *id)).map_or(*default, |one| one.label.as_str()).to_string().into(),
        on: ticked.as_deref() == Some(*id),
    }));
    entries
}

/// The hue a Job's card wears, by its state's bucket (`jobToneOf`), and whether it is calm.
fn card_hue(state: Option<ipc::JobState>) -> (Token, bool) {
    use ipc::JobState as S;
    match state {
        Some(S::Waiting | S::Done | S::Unstarted | S::StoppedAsks | S::Stuck | S::Failing) => (token::STATUS_AWAITING_REVIEW, false),
        Some(S::Working | S::Running | S::Subagent) => (token::STATUS_RUNNING, false),
        Some(S::Held | S::Limited | S::After | S::Stopped) => (token::STATUS_HANDED_IN, false),
        _ => (token::FG_SUBTLE, true),
    }
}

/// The issue panel's tab.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum IssueTab {
    Conversation,
    Linked,
}

impl IssueTab {
    fn word(self) -> &'static str {
        match self {
            IssueTab::Conversation => "Conversation",
            IssueTab::Linked => "Pull requests",
        }
    }
}

/// An issue's panel: the row pressed, its tab, and Fleet's read of it.
pub(super) struct IssuePanel {
    pub item: IssueItem,
    pub tab: IssueTab,
    pub read: Option<Result<IssueRead, String>>,
    pub scroll: ScrollHandle,
}

impl JobView {
    /// The Job's issues, then those of each Job it started, each once by its repository and number.
    pub(super) fn issues_of(&self, id: &str, cx: &gpui::App) -> Vec<IssueItem> {
        let Some(job) = self.record(id, cx) else { return Vec::new() };
        let mut holders = vec![job];
        holders.extend(job.attachments.iter().filter(|one| one.kind == "started_to").filter_map(|one| self.record(&one.target, cx)));
        let mut seen = HashSet::new();
        holders
            .into_iter()
            .flat_map(|holder| holder.attachments.iter().filter(|one| one.kind == "issue").map(move |one| issue_of(holder, one)))
            .filter(|item| seen.insert(item.key()))
            .collect()
    }

    /// Issues: one row each. None, and nothing is drawn.
    pub(super) fn pane_issues(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let rows: Vec<AnyElement> = self.issues_of(id, cx).into_iter().map(|item| self.issue_row(item, look, cx)).collect();
        if rows.is_empty() {
            return None;
        }
        Some(labelled(div().id("pane-issues").flex().flex_col().children(rows), Role::List, "Issues").into_any_element())
    }

    /// The row (`IssueRow`): the mark of the Ticket state its status reads as, else the ticket in its
    /// Open or Closed hue; its number and title. A press opens its panel; a right press its menu.
    pub(super) fn issue_row(&self, item: IssueItem, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.key();
        // Another repository's issue names it before its number, so two repositories' #7 tell apart.
        let own = self.shown.as_ref().and_then(|id| self.record(id, cx)).and_then(|job| job.manifest_id.clone());
        let elsewhere = item.manifest.clone().filter(|manifest| Some(manifest) != own.as_ref()).map(|manifest| manifest.as_str().to_string());
        let label = format!("{}{}", elsewhere.as_deref().unwrap_or_default(), item.label());
        let word = item.word();
        let states = item.manifest.as_ref().and_then(|id| self.panes.tickets.get(id.as_str()));
        let read = states.and_then(|states| ticket_mark(states, &word));
        let (icon, said) = read.unwrap_or_else(|| ("ticket", if item.closed { "Closed".into() } else { "Open".into() }));
        let ink = look.color(if item.closed { token::STEP_ADVANCED } else { token::ACCENT });
        let (ground, strong) = (look.color(token::BG_HOVER), look.color(token::FG_DEFAULT));
        let lead = labelled(div().id(SharedString::from(format!("issue-mark:{key}"))).flex_none().flex().items_center().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, ink)), Role::Image, said);
        let row = div()
            .id(SharedString::from(format!("issue-row:{key}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .w_full()
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .cursor_pointer()
            .hover(move |style| style.bg(ground).text_color(strong))
            .child(lead)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .children(elsewhere.map(|repository| div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(repository))))
                    .child(div().text_color(look.color(token::ACCENT)).child(SharedString::from(item.label()))),
            )
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(item.title.clone())));
        let pressed = item.clone();
        let row = row.on_click(cx.listener(move |this, _, window, cx| this.open_issue(pressed.clone(), window, cx)));
        let row = match self.shown.as_ref().and_then(|job| self.pages.get(job)) {
            Some(page) => {
                let spot = self.spot(page, &format!("issue-menu:{key}"), cx);
                let entries_of = self.panes.tickets.clone();
                let (manifest, menu_label, menu_word) = (item.manifest.as_ref().map(|id| id.as_str().to_string()), label.clone(), word.clone());
                let entries = Rc::new(move || ticket_entries(&menu_label, &menu_word, manifest.as_ref().and_then(|id| entries_of.get(id))));
                self.menu_on(row.track_focus(&spot.focus), &spot, format!("Issue {label}"), spot.focus.clone(), entries, ticket_pick(item.clone()), cx)
            }
            None => row,
        };
        labelled(row, Role::Button, format!("Issue {label} {}", item.title)).into_any_element()
    }

    /// The Ticket lifecycle of each repository the tab's issues come from, read as the tab shows.
    pub(super) fn want_tickets(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let mut manifests: Vec<String> = self.issues_of(id, cx).into_iter().filter_map(|item| item.manifest.map(|id| id.as_str().to_string())).collect();
        manifests.dedup();
        for manifest in manifests {
            self.read_tickets(manifest, window, cx);
        }
    }

    /// `GET /lifecycles?manifest_id=`: the running Ticket lifecycle's states. A refusal reads as none,
    /// so the rows keep their Open or Closed.
    fn read_tickets(&mut self, manifest: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            if !self.panes.tickets.contains_key(&manifest) {
                let states = crate::mock::lifecycles(&manifest).map(|read| running_ticket(&read.lifecycles)).unwrap_or_default();
                self.panes.tickets.insert(manifest, states);
            }
            return;
        }
        let path = format!("/lifecycles?manifest_id={}", super::wire::encoded(&manifest));
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let states = answer.ok().and_then(|bytes| ipc::decode::<RepositoryLifecycles>("RepositoryLifecycles", &bytes).ok()).map(|read| running_ticket(&read.lifecycles)).unwrap_or_default();
                this.panes.tickets.insert(manifest, states);
                cx.notify();
            }),
        );
    }

    /// Teach Armada that the tracker's status reads as `state`, then read the lifecycle again. A
    /// refusal is a toast in Fleet's words.
    fn teach_ticket(&mut self, item: &IssueItem, state: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(manifest) = item.manifest.clone() else {
            cx.defer(|cx| crate::toasts::tell("This Job has no repository", cx));
            return;
        };
        let word = item.word();
        let key = manifest.as_str().to_string();
        if self.fleet.is_none() {
            // The mock's Fleet takes it as Fleet does: the word moves to the state chosen.
            if let Some(states) = self.panes.tickets.get_mut(&key) {
                for one in states.iter_mut() {
                    if let StateReadFrom::Tracker { words } = &mut one.read {
                        words.retain(|one| *one != word);
                        if one.id == state {
                            words.push(word.clone());
                        }
                    }
                }
            }
            cx.notify();
            return;
        }
        let body = ipc::encode(&TeachTicketWord { manifest_id: manifest, word, state: state.to_string() });
        self.post(
            "/lifecycles/ticket/teach".into(),
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| match answer {
                Ok(_) => this.read_tickets(key, window, cx),
                Err(why) => cx.defer(move |cx| crate::toasts::tell(why, cx)),
            }),
        );
    }

    /// The Jobs linked to this one: those it started, those that started it, and the one it was forked
    /// from (`linkedSessionsOf`).
    fn linked_jobs<'a>(&self, id: &str, cx: &'a gpui::App) -> Vec<&'a SessionRecord> {
        let Some(job) = self.record(id, cx) else { return Vec::new() };
        let all = self.list.read(cx).records();
        let started = job.attachments.iter().filter(|one| one.kind == "started_to").filter_map(|one| all.iter().find(|other| other.id.as_str() == one.target));
        let starters = all.iter().filter(|one| one.attachments.iter().any(|to| to.kind == "started_to" && to.target == id));
        let forked = job.attachments.iter().filter(|one| one.kind == "forked_from").filter_map(|one| all.iter().find(|other| other.id.as_str() == one.target));
        started.chain(starters).chain(forked).collect()
    }

    /// Jobs: each a card. None, and nothing is drawn.
    pub(super) fn pane_jobs(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let cards: Vec<AnyElement> = self.linked_jobs(id, cx).into_iter().map(|job| job_card(job, look, cx)).collect();
        if cards.is_empty() {
            return None;
        }
        Some(labelled(div().id("pane-jobs").flex().flex_col().gap(look.length(token::SPACE_2)).children(cards), Role::List, "Jobs").into_any_element())
    }
}

/// A Job as a card (`JobCard`): its state's hue on the frame and the left edge, the bot glyph, its
/// title and state, the last line it said and its age. A press opens it.
pub(super) fn job_card(job: &SessionRecord, look: &Look, cx: &Context<JobView>) -> AnyElement {
    let state = job.mark.map(|mark| mark.state);
    let (hue_token, calm) = card_hue(state);
    let hue = look.color(hue_token);
    let title = job.title.clone().unwrap_or_else(|| job.id.as_str().to_string());
    let says = job.activity.as_ref().and_then(|activity| activity.now.clone().or_else(|| activity.said.clone())).or_else(|| job.so_far.clone());
    let ago = job.last_turn_at.as_ref().and_then(|at| age(at.as_str(), Local::now()));
    let id = job.id.as_str().to_string();
    let mark = state.map(|state| {
        let (icon, said) = crate::jobs::glyph_of(state, None);
        let ink = if calm { look.color(token::FG_MUTED) } else { hue };
        labelled(div().id(SharedString::from(format!("job-card-state:{id}"))).flex_none().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, ink)), Role::Image, said)
    });
    let edge = look.length(token::BORDER_WIDTH);
    let (fill, hover) = (look.color(token::BG_RAISED).blend(hue.opacity(0.08)), look.color(token::BG_RAISED).blend(hue.opacity(0.14)));
    let line = |child: AnyElement| div().flex().items_center().gap(look.length(token::SPACE_3)).min_w_0().child(child);
    let card = div()
        .id(SharedString::from(format!("job-card:{id}")))
        .relative()
        .overflow_hidden()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .pl(look.length(token::SPACE_3) + edge * 2.)
        .pr(look.length(token::SPACE_3))
        .border(edge)
        .border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4)))
        .rounded(look.length(token::RADIUS_MD))
        .bg(fill)
        .shadow(look.shadows(token::SHADOW_CARD))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_DEFAULT))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.).bg(hue))
        .child(labelled(div().id(SharedString::from(format!("job-card-kind:{id}"))).flex_none().tooltip(tip("Job")).child(glyph(look, "bot", Size::Large, look.color(token::FG_MUTED))), Role::Image, "Job"))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(gpui::px(2.))
                .child(line(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight(look.number(if calm { token::WEIGHT_MEDIUM } else { token::WEIGHT_HEADING }))).child(SharedString::from(title.clone())))
                        .children(mark)
                        .into_any_element(),
                ))
                .child(line(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .min_h(look.length(token::LEADING_XS))
                        .text_size(look.length(token::TEXT_2XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(says.unwrap_or_default())))
                        .children(ago.map(|ago| div().flex_none().child(SharedString::from(ago))))
                        .into_any_element(),
                )),
        )
        .on_click(cx.listener(move |this, _, _, cx| this.open_job(id.clone(), cx)));
    labelled(card, Role::Button, format!("Job {title}")).into_any_element()
}

/// A row's menu pick: the Ticket state chosen.
fn ticket_pick(item: IssueItem) -> Pick {
    Rc::new(move |this, id, window, cx| {
        if let Some((_, state, ..)) = TICKET_STATES.iter().find(|(pick, ..)| *pick == id) {
            this.teach_ticket(&item, state, window, cx);
        }
    })
}

impl JobView {
    /// An issue's row was pressed: its panel opens over the Job and reads the issue.
    pub(super) fn open_issue(&mut self, item: IssueItem, window: &mut Window, cx: &mut Context<Self>) {
        let reading = item.clone();
        self.open_issue_panel(IssuePanel { item, tab: IssueTab::Conversation, read: None, scroll: ScrollHandle::new() }, window, cx);
        let Some(manifest) = reading.manifest.clone() else {
            self.issue_read(&reading, Err("This Job has no repository".into()), cx);
            return;
        };
        if self.fleet.is_none() {
            let read = crate::mock::issue(&reading.key()).ok_or_else(|| "The tracker did not answer for this issue".to_string());
            self.issue_read(&reading, read, cx);
            return;
        }
        let path = format!("/issues/{}/{}", super::wire::encoded(manifest.as_str()), super::wire::encoded(&reading.reference));
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<IssueRead>("IssueRead", &bytes).map_err(|why| why.to_string()));
                this.issue_read(&reading, read, cx);
            }),
        );
    }

    fn issue_read(&mut self, item: &IssueItem, read: Result<IssueRead, String>, cx: &mut Context<Self>) {
        if let Some(panel) = self.issue_panel_mut().filter(|panel| panel.item == *item) {
            panel.read = Some(read);
        }
        cx.notify();
    }

    pub(super) fn issue_tab(&mut self, tab: IssueTab, cx: &mut Context<Self>) {
        if let Some(panel) = self.issue_panel_mut() {
            panel.tab = tab;
            panel.scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
        }
        cx.notify();
    }

    /// `]` and `[` step the panel's tabs.
    pub(super) fn issue_panel_key(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.issue_panel().map(|panel| panel.tab) else { return };
        self.issue_tab(if tab == IssueTab::Conversation { IssueTab::Linked } else { IssueTab::Conversation }, cx);
    }

    /// A pull request that names the issue: its panel where this Job holds it, else the forge's page.
    fn open_linked(&mut self, manifest: Option<ManifestId>, number: u64, url: String, window: &mut Window, cx: &mut Context<Self>) {
        let job = self.shown.clone().map(|id| id.to_string()).unwrap_or_default();
        let held = self.pulls_of(&job, cx).into_iter().find(|pull| pull.number == number && pull.manifest.as_deref() == manifest.as_ref().map(|id| id.as_str()));
        match held {
            Some(pull) => self.open_pull(pull, window, cx),
            None => cx.open_url(&url),
        }
    }

    /// The panel's body (`IssuePanel`): the title, its state, labels, who opened it and when and who it
    /// is assigned to, Open in browser, then Conversation and Pull requests.
    pub(super) fn issue_body(&self, panel: &IssuePanel, look: &Look, cx: &Context<Self>) -> AnyElement {
        let item = &panel.item;
        let read = panel.read.as_ref().and_then(|read| read.as_ref().ok());
        let title = read.map_or(item.title.clone(), |read| read.title.clone());
        let body = div().id("issue-body").track_scroll(&panel.scroll).flex_1().min_h_0().overflow_y_scroll().px(look.length(token::SPACE_6)).py(look.length(token::SPACE_4)).flex().flex_col().gap(look.length(token::SPACE_3));
        let heading = div().text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(title));
        let read = match &panel.read {
            None => return body.child(heading).child(labelled(div().id("issue-reading").child(stand_ins(look, "issue-reading", &[0.5, 1.0, 0.85, 0.6])), Role::Status, format!("Reading issue {}", item.label()))).into_any_element(),
            Some(Err(said)) => {
                return body.child(heading).child(labelled(div().id("issue-said").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())), Role::Alert, said.clone())).into_any_element();
            }
            Some(Ok(read)) => read,
        };
        let open = read.state == IssueState::Open;
        let (pill_icon, pill_said, pill_hue) = if open { ("circle-dot", "Open", token::STATUS_RUNNING) } else { ("circle-check", "Closed", token::STATUS_COMPLETED_SUCCESS) };
        let hue = look.color(pill_hue);
        let pill = labelled(
            div().id("issue-pill").flex_none().flex().items_center().gap(look.length(token::SPACE_1)).px(look.length(token::SPACE_2)).py(gpui::px(1.)).rounded_full().bg(hue.opacity(0.14)).text_color(hue).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(glyph(look, pill_icon, Size::Small, hue)).child(pill_said),
            Role::Status,
            pill_said,
        );
        let fact = |id: String, said: &'static str, words: String| div().id(SharedString::from(id)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip(said)).child(SharedString::from(words));
        let labels = read.labels.iter().enumerate().map(|(at, one)| {
            div().id(SharedString::from(format!("issue-label:{at}"))).flex_none().px(look.length(token::SPACE_2)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_SUNKEN)).font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip("Label")).child(SharedString::from(one.clone()))
        });
        let author = (!read.author.is_empty()).then(|| fact("issue-author".into(), "Opened by", read.author.clone()));
        let opened = age(read.opened_at.as_str(), Local::now()).map(|ago| fact("issue-opened".into(), "Opened", ago));
        let assignees = read.assignees.iter().enumerate().map(|(at, one)| fact(format!("issue-assignee:{at}"), "Assigned to", one.clone()));
        let meta = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).min_w_0().child(pill).children(labels).children(author).children(opened).children(assignees);
        let address = read.url.clone();
        let browser = (!address.is_empty()).then(|| labelled(icon_button(look, "issue-open", "external-link", "Open in browser".into()).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&address))), Role::Button, "Open in browser"));
        let acts = labelled(div().id("issue-acts").flex().items_center().gap(look.length(token::SPACE_2)).child(div().flex_1()).children(browser), Role::Group, "Acts");
        body.child(heading).child(meta).child(acts).child(self.issue_tabs(panel, read, look, cx)).into_any_element()
    }

    fn issue_tabs(&self, panel: &IssuePanel, read: &IssueRead, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (accent, strong) = (look.color(token::ACCENT), look.color(token::FG_DEFAULT));
        let strip = div().id("issue-tabs").flex().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).children([IssueTab::Conversation, IssueTab::Linked].into_iter().map(|one| {
            let front = one == panel.tab;
            labelled(
                div()
                    .id(SharedString::from(format!("issue-tab:{}", one.word())))
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
                    .on_click(cx.listener(move |this, _, _, cx| this.issue_tab(one, cx)))
                    .child(one.word()),
                Role::Tab,
                one.word(),
            )
        }));
        let shown: Option<AnyElement> = match panel.tab {
            IssueTab::Conversation => self.issue_conversation(read, look, cx),
            IssueTab::Linked => self.issue_linked(&panel.item, read, look, cx),
        };
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(labelled(strip, Role::TabList, "Issue"))
            .children(shown.map(|shown| labelled(div().id("issue-tab-panel").child(shown), Role::TabPanel, panel.tab.word())))
            .into_any_element()
    }

    /// Conversation: the description, then each comment, oldest first. Nothing said, nothing drawn.
    fn issue_conversation(&self, read: &IssueRead, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let now = Local::now();
        let author = (!read.author.is_empty()).then(|| read.author.clone());
        let mut cards: Vec<AnyElement> = Vec::new();
        if !read.body.trim().is_empty() {
            let lead = mark(look, "issue-description-mark".into(), "file-text", "Description", None);
            cards.push(self.pull_card("issue-description", Some(lead), author, false, Some(read.opened_at.as_str()), None, &read.body, "Description".into(), look, now, cx));
        }
        for (at, one) in read.comments.iter().enumerate() {
            let who = (!one.author.is_empty()).then(|| one.author.clone());
            cards.push(self.pull_card(&format!("issue-comment-{at}"), None, who, false, Some(one.at.as_str()), None, &one.body, one.author.clone(), look, now, cx));
        }
        (!cards.is_empty()).then(|| labelled(div().id("issue-conversation").flex().flex_col().gap(look.length(token::SPACE_3)).children(cards), Role::List, "Conversation").into_any_element())
    }

    /// Pull requests: those that close or name the issue, each a row that opens it. None, nothing drawn.
    fn issue_linked(&self, item: &IssueItem, read: &IssueRead, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if read.linked_pull_requests.is_empty() {
            return None;
        }
        let ground = look.color(token::BG_HOVER);
        let rows = read.linked_pull_requests.iter().map(|one| {
            let (icon, said, tone) = match one.state {
                LinkedPullRequestState::Open => ("git-pull-request", "Open", None),
                LinkedPullRequestState::Merged => ("git-merge", "Merged", Some(Tone::Success)),
                LinkedPullRequestState::Closed => ("git-pull-request-closed", "Closed", None),
            };
            let (manifest, number, url) = (item.manifest.clone(), one.number, one.url.clone());
            labelled(
                div()
                    .id(SharedString::from(format!("issue-linked:{number}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_SM))
                    .cursor_pointer()
                    .hover(move |style| style.bg(ground))
                    .child(mark(look, format!("issue-linked-mark:{number}"), icon, said, tone))
                    .child(div().flex_1().min_w_0().truncate().child(SharedString::from(format!("#{number} {}", one.title))))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_linked(manifest.clone(), number, url.clone(), window, cx))),
                Role::Button,
                format!("Pull request #{number} {}", one.title),
            )
        });
        Some(labelled(div().id("issue-linked").flex().flex_col().children(rows), Role::List, "Pull requests").into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(reference: &str, status: Option<&str>, closed: bool) -> IssueItem {
        IssueItem { reference: reference.into(), title: String::new(), address: String::new(), closed, status: status.map(Into::into), manifest: None }
    }

    fn state(id: &str, label: &str, words: &[&str]) -> LifecycleStateDefinition {
        LifecycleStateDefinition { id: id.into(), label: label.into(), read: StateReadFrom::Tracker { words: words.iter().map(|one| one.to_string()).collect() } }
    }

    fn states() -> Vec<LifecycleStateDefinition> {
        vec![state("todo", "To do", &["To Do"]), state("in_progress", "In progress", &["In Progress"]), state("in_review", "In review", &["In Review"]), state("closed", "Closed", &["Done"])]
    }

    fn ticket(reach: &str, states: Vec<LifecycleStateDefinition>) -> AppliedLifecycle {
        AppliedLifecycle {
            lifecycle: ipc::LifecycleDefinition { id: "ticket".into(), name: "Ticket".into(), scope: ipc::PieceScope::Everywhere, states, transitions: Vec::new(), ends: Vec::new() },
            kept: ipc::PieceKept { rank: ipc::PieceRank { reach: reach.into(), source: "armada".into(), home: false }, path: None, overridden_by: None, hides: None },
        }
    }

    #[test]
    fn a_forge_number_reads_with_a_hash_and_a_tracker_key_as_it_is() {
        assert_eq!(item("7", None, false).label(), "#7");
        assert_eq!(item("ABC-12", None, false).label(), "ABC-12");
    }

    #[test]
    fn the_word_is_the_trackers_status_else_open_or_closed() {
        assert_eq!(item("7", Some("Ready for QA"), false).word(), "Ready for QA");
        assert_eq!(item("7", Some("  "), true).word(), "Closed");
        assert_eq!(item("7", None, false).word(), "Open");
    }

    #[test]
    fn a_status_the_lifecycle_lists_wears_its_states_glyph() {
        let states = states();
        assert_eq!(ticket_mark(&states, "In Progress"), Some(("circle-dot", "Ticket state: In progress".to_string())));
        assert_eq!(ticket_mark(&states, "Ready for QA"), None);
    }

    #[test]
    fn the_menu_ticks_the_state_the_word_reads_as() {
        let states = states();
        let ticked: Vec<&str> = ticket_entries("#3290", "In Progress", Some(&states))
            .into_iter()
            .filter_map(|entry| match entry {
                Entry::Choice { label, on: true, .. } => Some(if label.as_ref() == "In progress" { "in_progress" } else { "other" }),
                _ => None,
            })
            .collect();
        assert_eq!(ticked, ["in_progress"]);
    }

    #[test]
    fn the_ticket_lifecycle_kept_nearest_the_repository_runs() {
        let far = ticket("everywhere", states());
        let mut near = ticket("repository", states()[..1].to_vec());
        assert_eq!(running_ticket(&[far.clone(), near.clone()]).len(), 1);
        near.kept.overridden_by = Some(far.kept.rank.clone());
        assert_eq!(running_ticket(&[far, near]).len(), 4);
    }
}

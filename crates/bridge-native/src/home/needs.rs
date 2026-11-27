//! The Needs you block of the Cockpit's home: a card for each Job that waits on him, over every project or the one selected. It
//! reads the Jobs the Jobs list holds, so what it shows is the list's own Needs you, and a press on a card opens that Job.
//! The scope is the block's own, a small value a later step saves with the layout.

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, AnyElement, Context, Entity, EventEmitter, FontWeight, MouseButton, Render, Role, SharedString, Subscription, Toggled, Window};
use ipc::{JobLamp, Projects, SessionRecord, SessionState, WaitingSource};

use crate::grid::View;
use crate::jobs::JobsList;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::{bone, ring_colour};
use crate::titlebar::parts::{glyph, sans, tip};

/// The most of a Job's open items its card says; a Job with more says so and the Job holds the rest.
const LINES: usize = 3;

/// Which Jobs the block draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NeedsScope {
    /// Those of the project selected in the title bar; where none is, every project's.
    #[default]
    Selected,
    All,
}

impl NeedsScope {
    pub const ALL: [NeedsScope; 2] = [NeedsScope::Selected, NeedsScope::All];

    /// The word the layout keeps it as.
    pub fn slug(self) -> &'static str {
        match self {
            NeedsScope::Selected => "selected",
            NeedsScope::All => "all",
        }
    }

    pub fn from_slug(slug: &str) -> Option<NeedsScope> {
        NeedsScope::ALL.into_iter().find(|scope| scope.slug() == slug)
    }
}

/// The scope was changed by a press: what the host keeps with the layout.
pub struct ScopeChanged(pub NeedsScope);

/// A Job that waits on him, as its card says it.
#[derive(Clone, Debug, PartialEq)]
pub struct NeedCard {
    /// The Job's id, which is what a press opens.
    pub id: String,
    pub title: String,
    /// The project's name, where the Job is in one.
    pub project: Option<String>,
    /// What it waits for, oldest first, at most [`LINES`].
    pub lines: Vec<String>,
    /// More is owed than the lines say.
    pub more: bool,
    /// When it began to wait, as RFC 3339.
    pub since: Option<String>,
}

impl NeedCard {
    /// The name VoiceOver reads: the Job, and what it waits for first.
    pub fn name(&self) -> String {
        match self.lines.first() {
            Some(line) => format!("{}, {line}", self.title),
            None => self.title.clone(),
        }
    }
}

/// The project a Job is in: the one Fleet says it was started for, else the first that holds its repository.
fn project_of<'a>(job: &SessionRecord, projects: Option<&'a Projects>) -> Option<&'a ipc::Project> {
    let all = projects?.projects.as_slice();
    match job.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()) {
        Some(id) => all.iter().find(|one| one.id == id),
        None => job.manifest_id.as_ref().and_then(|manifest| all.iter().find(|one| one.members.iter().any(|member| member.manifest_id.as_str() == manifest.as_str()))),
    }
}

/// The Jobs waiting on him, longest first: the Jobs list's Needs you, which leaves out a Job he snoozed. `selected` narrows to
/// one project under [`NeedsScope::Selected`]; none selected is every project.
pub fn needing(jobs: &[SessionRecord], projects: Option<&Projects>, scope: NeedsScope, selected: Option<&str>) -> Vec<NeedCard> {
    let only = match scope {
        NeedsScope::Selected => selected,
        NeedsScope::All => None,
    };
    let mut cards: Vec<NeedCard> = jobs
        .iter()
        .filter(|job| !job.operator && job.state == SessionState::Live && job.snoozed_until.is_none() && job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs))
        .filter_map(|job| {
            let project = project_of(job, projects);
            if only.is_some_and(|id| project.map(|one| one.id.as_str()) != Some(id)) {
                return None;
            }
            let open: Vec<&ipc::WaitingItem> = job.waiting_for.iter().filter(|item| item.source != WaitingSource::Done).collect();
            Some(NeedCard {
                id: job.id.as_str().to_string(),
                title: crate::jobs::titled(job.title.as_deref()),
                project: project.map(|one| one.name.clone()),
                lines: open.iter().take(LINES).map(|item| item.text.clone()).collect(),
                more: open.len() > LINES,
                since: open.iter().map(|item| item.since.as_str()).min().map(str::to_string),
            })
        })
        .collect();
    // RFC 3339 in one zone sorts as the instants do; a Job with nothing dated stands last.
    cards.sort_by(|a, b| match (&a.since, &b.since) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    cards
}

/// `4m`, `3h`, `2d`: how long since `at`, as RFC 3339.
fn age(at: &str, now: DateTime<Utc>) -> Option<String> {
    let then = DateTime::parse_from_rfc3339(at).ok()?;
    let minutes = (now - then.with_timezone(&Utc)).num_minutes().max(0);
    Some(match minutes {
        0 => "now".into(),
        1..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h", minutes / 60),
        _ => format!("{}d", minutes / 1440),
    })
}

pub struct NeedsCards {
    list: Entity<JobsList>,
    scope: NeedsScope,
    /// The project the title bar is on, by Fleet's id.
    selected: Option<String>,
    cards: Vec<NeedCard>,
    /// Fleet has not yet said which Jobs there are.
    reading: bool,
    /// Cards, or a line to a Job.
    view: View,
    /// The block's header is drawn by the grid it stands in, and carries the scope control: the block draws none of its own.
    hosted: bool,
    _subs: Vec<Subscription>,
}

impl EventEmitter<ScopeChanged> for NeedsCards {}

impl NeedsCards {
    pub fn new(list: Entity<JobsList>, scope: NeedsScope, cx: &mut Context<Self>) -> Self {
        let mut subs = vec![cx.observe(&list, |this, _, cx| this.refresh(cx))];
        if let Some(bar) = crate::titlebar::live(cx) {
            subs.push(cx.observe(&bar, |this, _, cx| this.refresh(cx)));
        }
        let mut this = NeedsCards { list, scope, selected: None, cards: Vec::new(), reading: false, view: View::Cards, hosted: false, _subs: subs };
        this.refresh(cx);
        this
    }

    pub fn scope(&self) -> NeedsScope {
        self.scope
    }

    /// The scope a saved layout names, set without the host being told.
    pub fn set_scope(&mut self, scope: NeedsScope, cx: &mut Context<Self>) {
        if scope != self.scope {
            self.scope = scope;
            self.refresh(cx);
        }
    }

    /// Cards or lines, as the layout keeps it. Anything but a list is cards.
    pub fn set_view(&mut self, view: View, cx: &mut Context<Self>) {
        let view = if view == View::List { View::List } else { View::Cards };
        if view != self.view {
            self.view = view;
            cx.notify();
        }
    }

    /// The host draws the header, scope control and all: the block is its rows alone.
    pub fn set_hosted(&mut self, hosted: bool, cx: &mut Context<Self>) {
        if hosted != self.hosted {
            self.hosted = hosted;
            cx.notify();
        }
    }

    /// The project the title bar is on, where there is no bar to say it.
    pub fn select_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if project != self.selected {
            self.selected = project;
            self.refresh(cx);
        }
    }

    /// The cards as drawn now.
    pub fn cards(&self) -> &[NeedCard] {
        &self.cards
    }

    /// Work the cards out again and redraw only if they moved: the Jobs list changes far more often than who waits on him.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if let Some(bar) = crate::titlebar::live(cx) {
            self.selected = bar.read(cx).face.scoped_project().map(str::to_string);
        }
        let list = self.list.read(cx);
        let (projects, reading) = match list.fleet() {
            Some(fleet) => {
                let held = fleet.read(cx).held();
                (held.projects.clone(), held.cursor.is_none() && held.jobs.is_empty())
            }
            None => (Some(crate::mock::projects()), false),
        };
        let cards = needing(list.records(), projects.as_ref(), self.scope, self.selected.as_deref());
        if cards != self.cards || reading != self.reading {
            self.cards = cards;
            self.reading = reading;
            cx.notify();
        }
    }

    /// The name the first segment of the scope control wears: the project selected, or where none is, the words that say so.
    fn selected_name(&self, cx: &gpui::App) -> String {
        let projects = match self.list.read(cx).fleet() {
            Some(fleet) => fleet.read(cx).held().projects.clone(),
            None => Some(crate::mock::projects()),
        };
        self.selected
            .as_deref()
            .and_then(|id| projects.as_ref().and_then(|all| all.projects.iter().find(|one| one.id == id)).map(|one| one.name.clone()))
            .unwrap_or_else(|| "Selected project".to_string())
    }

    fn pick(&mut self, scope: NeedsScope, cx: &mut Context<Self>) {
        if scope == self.scope {
            return;
        }
        self.set_scope(scope, cx);
        cx.emit(ScopeChanged(scope));
    }

    fn open(&mut self, id: &str, cx: &mut Context<Self>) {
        self.list.update(cx, |list, cx| {
            list.open_job(id, cx);
        });
    }

    // -- drawn ---------------------------------------------------------------------------------

    /// The two segments of the scope: the project selected, and all of them. One height, one baseline.
    pub(crate) fn scope_control(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let selected = self.selected_name(cx);
        let mut group = div().id("needs-scope").flex_none().flex().items_center().gap(look.length(token::SPACE_1));
        for scope in NeedsScope::ALL {
            let on = scope == self.scope;
            let (words, said) = match scope {
                NeedsScope::Selected => (selected.clone(), "Needs you in the selected project".to_string()),
                NeedsScope::All => ("All projects".to_string(), "Needs you in all projects".to_string()),
            };
            let button = div()
                .id(SharedString::from(format!("needs-scope-{}", scope.slug())))
                .flex_none()
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_2))
                .flex()
                .items_center()
                .rounded(look.length(token::RADIUS_SM))
                .text_size(look.length(token::TEXT_2XS))
                .when(on, |button| button.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                .when(!on, |button| button.text_color(look.color(token::FG_SUBTLE)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.bg(look.color(token::BG_HOVER)).border_color(ring))
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .tooltip(tip(said.clone()))
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    cx.stop_propagation();
                    window.prevent_default();
                })
                .on_click(cx.listener(move |this, _, _, cx| this.pick(scope, cx)))
                .child(SharedString::from(words));
            group = group.child(labelled(button, Role::Button, said));
        }
        labelled(group, Role::Group, "Scope of Needs you").into_any_element()
    }

    /// Grey shapes where the cards will be, while Fleet has not said which Jobs there are.
    fn bones(&self, look: &Look) -> AnyElement {
        let card = |n: usize| {
            div()
                .id(SharedString::from(format!("needs-bone-{n}")))
                .flex_grow(1.)
                .flex_shrink(1.)
                .flex_basis(look.length(token::SPACE_12) * 5.)
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_OVERLAY))
                .child(bone(look, look.length(token::SPACE_12) * 2.0, look.length(token::SPACE_3)))
                .child(bone(look, look.length(token::SPACE_12) * 3.0, look.length(token::SPACE_3)))
        };
        labelled(div().id("needs-loading").flex().flex_wrap().gap(look.length(token::SPACE_2)).child(card(0)).child(card(1)), Role::Image, "Loading").into_any_element()
    }

    fn empty(&self, look: &Look, cx: &gpui::App) -> AnyElement {
        let said = match (self.scope, self.selected.is_some()) {
            (NeedsScope::Selected, true) => format!("Nothing needs you in {}", self.selected_name(cx)),
            _ => "Nothing needs you".to_string(),
        };
        labelled(
            div()
                .id("needs-empty")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph("circle-check", look.length(token::SPACE_4), look.color(token::FG_SUBTLE)))
                .child(SharedString::from(said.clone())),
            Role::Status,
            said,
        )
        .into_any_element()
    }

    /// A Job as one line: its mark, its title, the first thing it waits for, its age and, where the block spans projects, its project.
    fn line(&self, card: &NeedCard, now: DateTime<Utc>, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let id = card.id.clone();
        let age = card.since.as_deref().and_then(|at| age(at, now));
        let project = card.project.as_ref().filter(|_| self.scope == NeedsScope::All);
        let element = div()
            .id(SharedString::from(format!("needs-line-{}", card.id)))
            .w_full()
            .min_w_0()
            .h(look.length(token::H_ROW_ACTION))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .text_size(look.length(token::TEXT_XS))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .on_click(cx.listener(move |this, _, _, cx| this.open(&id, cx)))
            .child(glyph("bell-ring", look.length(token::SPACE_4), look.color(token::YOU)))
            .child(div().flex_none().max_w(look.length(token::SPACE_12) * 4.).truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(card.title.clone())))
            .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(card.lines.first().cloned().unwrap_or_default())))
            .children(project.map(|name| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(name.clone()))))
            .children(age.map(|age| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(age))));
        labelled(element, Role::Button, card.name()).into_any_element()
    }

    /// The Dashboard's card: a title with its mark and its age, what the Job waits for under them, and the project in a chip where
    /// the block spans every project.
    fn card(&self, card: &NeedCard, now: DateTime<Utc>, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let hue: Token = token::YOU;
        let id = card.id.clone();
        let age = card.since.as_deref().and_then(|at| age(at, now));
        let project = card.project.as_ref().filter(|_| self.scope == NeedsScope::All);
        let mark = labelled(
            div()
                .id(SharedString::from(format!("needs-card-{}-mark", card.id)))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::SPACE_6))
                .tooltip(tip("Needs you"))
                .child(glyph("bell-ring", look.length(token::SPACE_4), look.color(hue))),
            Role::Image,
            "Needs you",
        );
        let lines = card.lines.iter().map(|line| div().line_clamp(2).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(line.clone())));
        let more = card.more.then(|| div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("More waiting in the Job"));
        let element = div()
            .id(SharedString::from(format!("needs-card-{}", card.id)))
            .flex_grow(1.)
            .flex_shrink(1.)
            .flex_basis(look.length(token::SPACE_12) * 5.)
            .max_w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            .text_size(look.length(token::TEXT_XS))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .on_click(cx.listener(move |this, _, _, cx| this.open(&id, cx)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(mark)
                    .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(card.title.clone())))
                    .children(age.map(|age| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(age)))),
            )
            .children(lines)
            .children(more)
            .children(project.map(|name| {
                div().flex().child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(look.length(token::H_BADGE))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::BG_HOVER))
                        .text_size(look.length(token::TEXT_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(look.color(token::FG_MUTED))
                        .whitespace_nowrap()
                        .child(SharedString::from(name.clone())),
                )
            }));
        labelled(element, Role::Button, card.name()).into_any_element()
    }
}

impl Render for NeedsCards {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let look = &look;
        let now = Utc::now();
        let body = if self.reading {
            self.bones(look)
        } else if self.cards.is_empty() {
            self.empty(look, cx)
        } else if self.view == View::List {
            let lines: Vec<AnyElement> = self.cards.clone().iter().map(|card| self.line(card, now, look, cx)).collect();
            labelled(div().id("needs-cards-list").flex().flex_col().children(lines), Role::List, "Jobs that need you").into_any_element()
        } else {
            let cards: Vec<AnyElement> = self.cards.clone().iter().map(|card| self.card(card, now, look, cx)).collect();
            labelled(div().id("needs-cards-list").flex().flex_wrap().gap(look.length(token::SPACE_2)).children(cards), Role::List, "Jobs that need you").into_any_element()
        };
        let toolbar = (!self.hosted).then(|| div().flex_none().flex().items_center().h(look.length(token::H_ROW_ACTION)).px(look.length(token::SPACE_2)).child(self.scope_control(look, cx)));
        div()
            .id("home-needs-cards")
            .size_full()
            .flex()
            .flex_col()
            .font_family(sans(look))
            .text_color(look.color(token::FG_DEFAULT))
            .children(toolbar)
            .child(div().id("needs-body").flex_1().min_h_0().overflow_y_scroll().px(look.length(token::SPACE_2)).pb(look.length(token::SPACE_2)).when(self.hosted, |body| body.pt(look.length(token::SPACE_2))).child(body))
    }
}

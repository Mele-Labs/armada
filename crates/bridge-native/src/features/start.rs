//! The start panel beside the Features list while the project holds no confirmed or seeded feature: the modules its build files
//! name with Draft as Pending, the docs at the top of its repositories with Read them, and the librarian's Ask box. An entity
//! of its own, as a project page is, so the screen draws it in the page's place.
//!
//! What is read is Fleet's `get_library_start`, and Draft as Pending is `draft_module_features`: Fleet reads the modules again and
//! keeps each as a pending feature, and the screen reads the library again for the rows. Read them is an ask of the librarian
//! through the box, with the docs as its files, and the librarian drafts each feature it finds as its own pending row.
//! `docs/concepts/feature-catalog.md`, *Starting a library*.

use gpui::{div, prelude::*, px, AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, IntoElement, KeyDownEvent, Render, Role, SharedString, Subscription, Task, Window};
use ipc::{LibraryStart, ModulesDrafted};

use super::ask_box::{self, AskBox, AskBoxEvent, AskSource};
use super::dossier::{button, tint};
use super::page::Load;
use super::page_view::{head, inner, shell};
use super::wire;
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::settings::view::{bone, refusal, ring_colour, tip};
use crate::settings::wire::Refused;
use crate::titlebar::parts::{glyph, mono, sans, spinner};

/// The panel's heading.
pub const HEADING: &str = "No features yet";
/// Draft as Pending's words, and what its glyph says once every module is a pending feature.
pub const DRAFT: &str = "Draft as Pending";
pub const DRAFTED: &str = "Each module is a Pending feature";
/// Read them's words, and the words its ask goes with.
pub const READ: &str = "Read them";
pub const READ_ASK: &str = "Draft each feature these docs describe";
/// The element the librarian's ask box mounts in.
pub const ASK_SLOT: &str = "features-start-ask";

/// What the panel tells the screen that holds it.
pub enum StartEvent {
    /// Fleet kept the drafts: the library is read again for the rows.
    Drafted,
    /// An ask drafted pending features, or Undo took them out: the library is read again.
    Asked,
    /// Esc or Tab: the keys go back to the list.
    Escape,
}

/// Where Draft as Pending stands, kept apart from what Fleet read.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Drafting {
    #[default]
    Idle,
    Working,
    /// Fleet refused it, in its own words.
    Said(String),
}

/// A ring of `colour` drawn `spread` outside an element's edge.
fn halo(colour: gpui::Hsla, spread: gpui::Pixels) -> gpui::BoxShadow {
    gpui::BoxShadow { color: colour, offset: gpui::point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: spread, inset: false }
}

/// Whether a press would draft anything: a module the library does not hold yet.
pub fn to_draft(start: &LibraryStart) -> bool {
    start.modules.iter().any(|one| !one.held)
}

/// The modules marked held that a press kept or passed over, so the button goes without another read.
pub fn after_draft(start: &mut LibraryStart, drafted: &ModulesDrafted) {
    for one in &mut start.modules {
        if drafted.drafted.contains(&one.slug) || drafted.held.contains(&one.slug) {
            one.held = true;
        }
    }
}

pub struct StartPanel {
    pub within: FocusHandle,
    /// The keys are on the panel: Enter presses Draft as Pending.
    pub(super) body: FocusHandle,
    fleet: Option<Entity<FleetConnection>>,
    pub(super) project: String,
    pub(super) start: Load<LibraryStart>,
    pub(super) drafting: Drafting,
    /// What the mock's press is refused with, where a test wants it refused.
    pub(super) mock_refusal: Option<String>,
    /// The keys on Read them.
    pub(super) read_focus: FocusHandle,
    /// The librarian's box, made on the first draw, which has the window it needs.
    pub(super) ask: Option<Entity<AskBox>>,
    work: Option<Task<()>>,
    draft_work: Option<Task<()>>,
    _ask_sub: Option<Subscription>,
}

impl EventEmitter<StartEvent> for StartPanel {}

impl Focusable for StartPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.body.clone()
    }
}

impl StartPanel {
    /// On Fleet: the start of `project`, by Fleet's id, read now.
    pub fn live(fleet: &Entity<FleetConnection>, project: String, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Some(fleet.clone()), project, cx);
        this.read(cx);
        this
    }

    /// On the mock start in `mock.rs`.
    pub fn mock(project: String, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(None, project, cx);
        this.start = Load::Read(super::mock::start(&this.project));
        this
    }

    fn new(fleet: Option<Entity<FleetConnection>>, project: String, cx: &mut Context<Self>) -> Self {
        StartPanel {
            within: cx.focus_handle(),
            body: cx.focus_handle(),
            fleet,
            project,
            start: Load::Reading,
            drafting: Drafting::Idle,
            mock_refusal: None,
            read_focus: cx.focus_handle(),
            ask: None,
            work: None,
            draft_work: None,
            _ask_sub: None,
        }
    }

    fn port(&self, cx: &App) -> Option<u16> {
        self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port())
    }

    fn read(&mut self, cx: &mut Context<Self>) {
        let Some(port) = self.port(cx) else {
            self.start = Load::Failed("Fleet is not connected".into());
            return;
        };
        let ask = wire::library_start(port, &self.project, cx);
        self.work = Some(cx.spawn(async move |this, cx| {
            let answer = ask.await;
            let _ = this.update(cx, |this, cx| {
                this.start = match answer {
                    Ok(start) => Load::Read(start),
                    Err(why) => Load::Failed(why.said),
                };
                cx.notify();
            });
        }));
    }

    /// Whether Draft as Pending is drawn: the modules are read and one is not a pending feature yet.
    pub(super) fn can_draft(&self) -> bool {
        matches!(&self.start, Load::Read(start) if to_draft(start))
    }

    /// Draft as Pending: Fleet keeps each module the library does not hold as a pending feature.
    pub fn draft(&mut self, cx: &mut Context<Self>) {
        if self.drafting == Drafting::Working || !self.can_draft() {
            return;
        }
        let ask = match (&self.fleet, self.port(cx)) {
            (Some(_), Some(port)) => wire::draft_modules(port, &self.project, cx),
            (Some(_), None) => Task::ready(Err(Refused::said("Fleet is not connected"))),
            (None, _) => Task::ready(match (self.mock_refusal.clone(), &self.start) {
                (Some(said), _) => Err(Refused::said(said)),
                (None, Load::Read(start)) => Ok(ModulesDrafted { project: self.project.clone(), drafted: start.modules.iter().filter(|one| !one.held).map(|one| one.slug.clone()).collect(), held: Vec::new() }),
                (None, _) => Err(Refused::said("Not read")),
            }),
        };
        self.drafting = Drafting::Working;
        self.draft_work = Some(cx.spawn(async move |this, cx| {
            let answer = ask.await;
            let _ = this.update(cx, |this, cx| this.drafted(answer, cx));
        }));
        cx.notify();
    }

    fn drafted(&mut self, answer: Result<ModulesDrafted, Refused>, cx: &mut Context<Self>) {
        match answer {
            Ok(drafted) => {
                self.drafting = Drafting::Idle;
                if let Load::Read(start) = &mut self.start {
                    after_draft(start, &drafted);
                }
                cx.emit(StartEvent::Drafted);
            }
            Err(why) => self.drafting = Drafting::Said(why.said),
        }
        cx.notify();
    }

    pub fn focus_body(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.body, cx);
        cx.notify();
    }

    /// The box, made once with the panel's project and the source the panel reads from.
    fn ask_box(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Entity<AskBox> {
        if let Some(ask) = &self.ask {
            return ask.clone();
        }
        let source = match &self.fleet {
            Some(fleet) => AskSource::Live(fleet.clone()),
            None => AskSource::Mock(Box::default()),
        };
        let project = Some(self.project.clone());
        let ask = cx.new(|cx| AskBox::new(source, project, ask_box::FEATURES_EMPTY, window, cx));
        self._ask_sub = Some(cx.subscribe(&ask, |_, _, event: &AskBoxEvent, cx| {
            if matches!(event, AskBoxEvent::FeaturesAgain) {
                cx.emit(StartEvent::Asked);
            }
        }));
        self.ask = Some(ask.clone());
        ask
    }

    /// Whether Read them is drawn: the docs are read and there is one.
    pub(super) fn can_read(&self) -> bool {
        matches!(&self.start, Load::Read(start) if !start.docs.is_empty())
    }

    /// Read them: the docs go to the librarian through the box, which shows what it drafted with Undo.
    pub fn read_docs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Load::Read(start) = &self.start else { return };
        let docs = start.docs.clone();
        if docs.is_empty() {
            return;
        }
        let ask = self.ask_box(window, cx);
        ask.update(cx, |ask, cx| ask.ask_with(READ_ASK, docs, window, cx));
    }

    // ---- keys --------------------------------------------------------------------------------

    /// Where the keys are along the panel: on the panel itself, which is Draft as Pending, on Read them, or in the box.
    fn stop_at(&self, window: &Window) -> usize {
        if self.body.is_focused(window) {
            0
        } else if self.read_focus.is_focused(window) {
            1
        } else {
            2
        }
    }

    /// Tab goes on along the panel, ⇧Tab back, and past either end the keys go back to the list.
    fn tab(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let at = self.stop_at(window);
        let along: Vec<usize> = [0, 1, 2].into_iter().filter(|stop| *stop != 1 || self.can_read()).filter(|stop| *stop != 2 || self.ask.is_some()).collect();
        let here = along.iter().position(|stop| *stop >= at).unwrap_or(0);
        let next = if back { here.checked_sub(1) } else { Some(here + 1).filter(|next| *next < along.len()) };
        match next.map(|next| along[next]) {
            Some(0) => self.focus_body(window, cx),
            Some(1) => window.focus(&self.read_focus, cx),
            Some(_) => {
                if let Some(ask) = &self.ask {
                    let input = ask.read(cx).input.clone();
                    input.update(cx, |state, cx| state.focus(window, cx));
                }
            }
            None => cx.emit(StartEvent::Escape),
        }
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => cx.emit(StartEvent::Escape),
            // The box sits under the text box context, so Tab in it is no act of the registry's, and is taken here.
            "tab" if self.stop_at(window) == 2 => self.tab(event.keystroke.modifiers.shift, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// The registry's `features` acts while the keys are on the panel. Enter presses Draft as Pending or Read them, and Tab goes
    /// along the panel and on to the list, as a project page's does; the filter, the project chips and Catch up are the list's.
    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if !act.id.starts_with("features_") || ["features_filter", "features_project", "features_catch_up"].contains(&act.id) {
            cx.propagate();
            return;
        }
        match act.id {
            "features_open" if self.read_focus.is_focused(window) => self.read_docs(window, cx),
            "features_open" => self.draft(cx),
            "features_pane" => self.tab(act.slot % 2 == 1, window, cx),
            _ => {}
        }
    }

    // ---- drawing -----------------------------------------------------------------------------

    /// A chip with a fill (`.chip`), with a glyph before its words where it has one.
    fn chip(look: &Look, id: SharedString, icon: Option<&str>, words: &str, code: bool, pending: bool) -> gpui::Stateful<gpui::Div> {
        let (ground, ink) = if pending { (look.color(token::OPERATOR).opacity(0.16), look.color(token::OPERATOR)) } else { (look.color(token::BG_HOVER), look.color(token::FG_MUTED)) };
        div()
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1) + px(1.))
            .h(look.length(token::H_BADGE))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::H_BADGE) / 2.)
            .bg(ground)
            .text_color(ink)
            .text_size(look.length(token::TEXT_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .whitespace_nowrap()
            .when(code, |chip| chip.font_family(mono(look)))
            .children(icon.map(|icon| glyph(icon, look.length(token::SPACE_3) - px(1.), ink)))
            .child(SharedString::from(words.to_string()))
    }

    fn chips(look: &Look) -> gpui::Div {
        div().flex().flex_wrap().gap(look.length(token::SPACE_1) + px(2.))
    }

    /// The Modules card's title band: the title, Fleet's refusal where there is one, and Draft as Pending, which reads Drafting
    /// beside a spinner while Fleet keeps them, or the glyph that every module is a pending feature.
    fn modules_head(&self, look: &Look, focused: bool, cx: &mut Context<Self>) -> AnyElement {
        let title = "Modules in the build files";
        let working = self.drafting == Drafting::Working;
        let said = match &self.drafting {
            Drafting::Said(said) => Some(labelled(div().id("features-start-said").flex_1().min_w_0().child(refusal(look, said.clone()).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS))), Role::Status, said.clone())),
            _ => None,
        };
        let end = if self.can_draft() {
            let words = if working { "Drafting" } else { DRAFT };
            let (ring, gap) = (ring_colour(look), look.color(token::BG_BASE));
            let press = button(look, "features-start-draft", words, true, false, working, None)
                .tooltip(tip("Keep each module as a Pending feature; no model"))
                // The button's own edge is the accent, so the keys' ring stands outside it with a gap of the ground between.
                .when(focused && !working, move |press| press.shadow(vec![halo(ring, px(4.)), halo(gap, px(2.))]))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus_body(window, cx);
                    this.draft(cx);
                }));
            Some(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1) + px(2.))
                    .children(working.then(|| spinner("features-start-draft-spin", look.length(token::SPACE_3), look.color(token::STATUS_RUNNING), cx)))
                    .child(labelled(press, Role::Button, words))
                    .into_any_element(),
            )
        } else if matches!(&self.start, Load::Read(_)) {
            let done = div().id("features-start-drafted").flex_none().tooltip(tip(DRAFTED)).child(glyph("check", look.length(token::SPACE_3) + px(2.), look.color(token::STATUS_COMPLETED_SUCCESS)));
            Some(labelled(done, Role::Image, DRAFTED).into_any_element())
        } else {
            None
        };
        div()
            .id("feature-card-title-start-modules")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2) - px(1.))
            .min_h(look.length(token::LEADING_XS) + (look.length(token::SPACE_2) + px(2.)) * 2.)
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .rounded_t(crate::reviews::card_inner_radius(look))
            .bg(tint(look, look.color(token::BG_SUNKEN), token::BG_BASE, 0.60))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(labelled(div().id("features-start-modules-title").flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title), Role::Heading, title))
            .children(said)
            .when(!matches!(self.drafting, Drafting::Said(_)), |head| head.child(div().flex_1()))
            .children(end)
            .into_any_element()
    }

    fn render_modules(&self, look: &Look, start: &LibraryStart, focused: bool, cx: &mut Context<Self>) -> AnyElement {
        let mut chips = Self::chips(look);
        for one in &start.modules {
            let where_ = if one.path.is_empty() { "The repository's root".to_string() } else { one.path.clone() };
            let said = if one.held { format!("{where_}, a Pending feature") } else { where_ };
            let chip = Self::chip(look, SharedString::from(format!("features-start-module-{}", one.slug)), None, &one.name, true, one.held).tooltip(tip(said.clone()));
            chips = chips.child(labelled(chip, Role::Label, one.name.clone()));
        }
        shell(look, "features-start-modules", false, false).child(self.modules_head(look, focused, cx)).child(inner(look).child(chips)).into_any_element()
    }

    /// The Docs card's title band, with Read them, which is off while the box waits on an answer.
    fn docs_head(&self, look: &Look, focused: bool, waiting: bool, cx: &mut Context<Self>) -> AnyElement {
        let title = "Docs in the repo";
        let press = button(look, "features-start-read", READ, false, false, waiting, Some(&self.read_focus))
            .tooltip(tip("Ask the librarian to draft each feature these docs describe, each as a Pending feature"))
            .when(focused, |press| press.border_color(ring_colour(look)))
            .when(!waiting, |press| {
                press.on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.read_focus, cx);
                    this.read_docs(window, cx);
                }))
            });
        div()
            .id("feature-card-title-start-docs")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2) - px(1.))
            .min_h(look.length(token::LEADING_XS) + (look.length(token::SPACE_2) + px(2.)) * 2.)
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .rounded_t(crate::reviews::card_inner_radius(look))
            .bg(tint(look, look.color(token::BG_SUNKEN), token::BG_BASE, 0.60))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(labelled(div().id("features-start-docs-title").flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title), Role::Heading, title))
            .child(div().flex_1())
            .child(labelled(press, Role::Button, READ))
            .into_any_element()
    }

    fn render_docs(&self, look: &Look, start: &LibraryStart, focused: bool, waiting: bool, cx: &mut Context<Self>) -> AnyElement {
        let mut chips = Self::chips(look);
        for doc in &start.docs {
            let icon = if doc.ends_with('/') { "folder" } else { "file-text" };
            let chip = Self::chip(look, SharedString::from(format!("features-start-doc-{doc}")), Some(icon), doc, false, false);
            chips = chips.child(labelled(chip, Role::Label, doc.clone()));
        }
        shell(look, "features-start-docs", false, false).child(self.docs_head(look, focused, waiting, cx)).child(inner(look).child(chips)).into_any_element()
    }

    /// The librarian's box in its slot under the title. **It sits under the text box context**, as its input does, so Enter, the
    /// arrows and Tab are the box's and its card's, and not the list's.
    fn render_feed(&self, look: &Look, ask: Entity<AskBox>) -> AnyElement {
        shell(look, "features-start-feed", false, false)
            .child(head(look, "start-feed", "Or feed the librarian", false, true))
            .child(inner(look).child(div().id(ASK_SLOT).key_context(crate::keymap::TEXT_BOX).child(ask)))
            .into_any_element()
    }

    fn render_bones(&self, look: &Look) -> AnyElement {
        let mut cards = div().w_full().flex().flex_col().gap(look.length(token::SPACE_4));
        for at in 0..2 {
            cards = cards.child(
                shell(look, SharedString::from(format!("features-start-bone-{at}")), false, false)
                    .child(div().px(look.length(token::SPACE_3) + px(2.)).py(look.length(token::SPACE_3)).child(bone(look, px(140. + 40. * at as f32), look.length(token::LEADING_XS) - px(4.)))),
            );
        }
        labelled(cards.id("features-start-loading"), Role::Image, "Loading").into_any_element()
    }
}

impl Render for StartPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let focused = self.body.is_focused(window);
        let on_read = self.read_focus.is_focused(window);
        let ask = self.ask_box(window, cx);
        let waiting = ask.read(cx).waiting();
        let content = match &self.start {
            Load::Reading => self.render_bones(&look),
            Load::Failed(why) => labelled(div().id("features-start-not-read").child(refusal(&look, why.clone())), Role::Status, why.clone()).into_any_element(),
            Load::Read(start) => {
                let start = start.clone();
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_4))
                    // A card with nothing to list is not drawn: a label left over nothing goes with it.
                    .when(!start.modules.is_empty(), |cards| cards.child(self.render_modules(&look, &start, focused, cx)))
                    .when(!start.docs.is_empty(), |cards| cards.child(self.render_docs(&look, &start, on_read, waiting, cx)))
                    .child(self.render_feed(&look, ask))
                    .into_any_element()
            }
        };
        let heading = labelled(
            div().id("features-start-title").text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(HEADING),
            Role::Heading,
            HEADING,
        );
        let side = look.length(token::SPACE_6) + look.length(token::SPACE_1);
        div()
            .id("features-start")
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .size_full()
            .flex()
            .flex_col()
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().flex_none().px(side).pt(look.length(token::SPACE_4) + px(2.)).child(heading))
            .child(div().id("features-start-body").track_focus(&self.body).flex_1().min_h_0().overflow_y_scroll().px(side).pt(look.length(token::SPACE_4)).pb(look.length(token::SPACE_6)).child(content))
    }
}

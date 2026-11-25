//! A live prototype: a scene an agent writes in HTML-shaped markup naming the app's own parts,
//! drawn natively in a Prototype tab of the Job and redrawn each time the agent saves.
//!
//! The agent saves a scene file and fixture JSON; Fleet sends them as an artifact part and
//! [`PrototypeView::set_source`] reads them again. A press on a part that says `goes="id"` swaps
//! the state drawn, and the state Fleet names moves it too, so the agent can step him through.
//! Each `data-part` is a labelled element, so a note he writes with ⌥⌘A names the part.
//!
//! `markup` reads the file, `fixtures` its data through `ipc`, `style` the attributes any part
//! takes, `draw` the tree as `crate::parts`, `window` the pop-out. Nothing here is drawn from a
//! colour, spacing or font literal: a value the scene names is a token, or it is trouble.

pub mod draw;
pub mod fixtures;
pub mod markup;
pub mod shots;
pub mod style;
#[cfg(test)]
mod tests;
pub mod window;

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    canvas, div, prelude::*, App, Bounds, Context, EventEmitter, FocusHandle, Focusable, Global, IntoElement, Pixels, Point, Render, Role,
    SharedString, WeakEntity, Window, WindowId,
};

use self::fixtures::Fixtures;
use self::markup::Scene;
use crate::labels::{labelled, LabelBook};
use crate::look::{token, ActiveLook};
use crate::parts::{self, button::Kind, chip::chip, text::TextStyle, Tone};
use crate::settings::view::tip;

/// What a saved prototype carries: the scene, its markup, the fixture files as `(name, text)`, the
/// state the agent set and the trouble Fleet found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Source {
    pub scene: String,
    pub markup: String,
    pub fixtures: Vec<(String, String)>,
    pub state: Option<String>,
    pub trouble: Option<String>,
}

impl From<&ipc::PrototypePart> for Source {
    fn from(part: &ipc::PrototypePart) -> Source {
        Source {
            scene: part.scene.clone(),
            markup: part.markup.clone(),
            fixtures: part.fixtures.iter().map(|fixture| (fixture.name.clone(), fixture.text.clone())).collect(),
            state: part.scene_state.clone(),
            trouble: part.trouble.clone(),
        }
    }
}

/// What the view asks of whoever holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrototypeEvent {
    PopOut,
    /// The Job's Prototype tab for this scene, in front.
    OpenTab,
}

impl EventEmitter<PrototypeEvent> for PrototypeView {}

/// Whether the view is a tab of a Job, with Pop out, a part in the review sheet, with Open tab as well, or is the pop-out window itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Home {
    Tab,
    Sheet,
    Window,
    /// The scene alone, with no head and no trouble line: what the shot check draws to set against a built screen.
    Shot,
}

pub struct PrototypeView {
    source: Source,
    scene: Result<Scene, String>,
    fixtures: Fixtures,
    /// What reading and checking the scene found, then what Fleet said.
    trouble: Vec<String>,
    /// The state drawn.
    state: Option<String>,
    home: Home,
    /// The Job whose scene this is: a note made over it goes there (`prototype_at`).
    job: Option<SharedString>,
    /// Where the scene was drawn last, in its window.
    drawn: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub(crate) focus: FocusHandle,
}

/// The name the scene's whole drawing is labelled with: what a note made on it says it is in, and its band's chip.
const LABEL: &str = "Prototype";

/// What the band says where the scene has views to press.
const HINT: &str = "Views. Press one to see it below";

/// Every scene drawn for a Job, so a note made over one goes to its Job.
#[derive(Default)]
struct Drawn(Vec<WeakEntity<PrototypeView>>);

impl Global for Drawn {}

/// The Job and scene of the prototype drawn at `at` in `window` this frame, if one is. A view drawn
/// last frame and not this one (a tab behind another, a sheet closed) is not in the window's label
/// book, so it answers nothing.
pub fn prototype_at(window: WindowId, at: Point<Pixels>, cx: &App) -> Option<(SharedString, String)> {
    let drawn = cx.try_global::<Drawn>()?;
    let book = cx.try_global::<LabelBook>()?;
    let shown = book.drawn(window);
    drawn.0.iter().filter_map(|view| view.upgrade()).find_map(|view| {
        let view = view.read(cx);
        let job = view.job.clone()?;
        let bounds = view.drawn.get()?;
        shown
            .iter()
            .any(|one| one.role == Role::Group && one.name.as_ref() == LABEL && one.bounds.contains(&bounds.center()) && one.shown.contains(&at) && bounds.contains(&at))
            .then(|| (job, view.source.scene.clone()))
    })
}

impl PrototypeView {
    pub fn new(source: Source, home: Home, cx: &mut Context<Self>) -> PrototypeView {
        let mut view = PrototypeView {
            source: Source::default(),
            scene: Err(String::new()),
            fixtures: Fixtures::default(),
            trouble: Vec::new(),
            state: None,
            home,
            job: None,
            drawn: Rc::default(),
            focus: cx.focus_handle(),
        };
        view.read(source, cx);
        let me = cx.entity().downgrade();
        let drawn = cx.default_global::<Drawn>();
        drawn.0.retain(|one| one.upgrade().is_some());
        drawn.0.push(me);
        view
    }

    /// The Job the scene belongs to, which a note made on it goes to.
    pub fn set_job(&mut self, job: SharedString) {
        self.job = Some(job);
    }

    /// The prototype saved again: the scene is read from its markup, and the state drawn is the
    /// person's own unless the agent set a different one this time.
    pub fn set_source(&mut self, source: Source, cx: &mut Context<Self>) {
        if self.source == source {
            return;
        }
        self.read(source, cx);
        cx.notify();
    }

    fn read(&mut self, source: Source, cx: &mut Context<Self>) {
        let look = cx.look();
        let mut trouble = Vec::new();
        self.scene = match markup::parse(&source.markup) {
            Ok(scene) => {
                let (fixtures, bad) = Fixtures::read(&scene.fixtures, &source.fixtures);
                trouble.extend(bad);
                trouble.extend(draw::check(look, &scene, &fixtures).into_lines());
                self.fixtures = fixtures;
                Ok(scene)
            }
            Err(why) => {
                self.fixtures = Fixtures::default();
                trouble.push(why.clone());
                Err(why)
            }
        };
        trouble.extend(source.trouble.clone());
        let declared = |state: &str| self.scene.as_ref().is_ok_and(|scene| scene.states.iter().any(|one| one == state));
        let moved = source.state.is_some() && source.state != self.source.state;
        // The agent's state moves the drawing when it is new; else what he pressed stays, where the scene still has it.
        let next = match source.state.as_deref() {
            Some(set) if moved && declared(set) => Some(set.to_string()),
            _ => self
                .state
                .clone()
                .filter(|state| declared(state))
                .or_else(|| source.state.clone().filter(|set| declared(set)))
                .or_else(|| self.scene.as_ref().ok().and_then(|scene| scene.opening()).map(str::to_string)),
        };
        if let (Some(set), true) = (source.state.as_deref(), moved) {
            if self.scene.is_ok() && !declared(set) {
                trouble.push(format!("The state {set} the agent set is not one the scene declares"));
            }
        }
        self.state = next;
        self.trouble = trouble;
        self.source = source;
    }

    /// A press on a part that says `goes`: that state is drawn.
    pub fn go(&mut self, state: &str, cx: &mut Context<Self>) {
        if self.scene.as_ref().is_ok_and(|scene| scene.states.iter().any(|one| one == state)) && self.state.as_deref() != Some(state) {
            self.state = Some(state.to_string());
            cx.notify();
        }
    }

    /// The ids of the states the scene declares, in the order it writes them.
    pub fn declared(&self) -> Vec<String> {
        self.scene.as_ref().map(|scene| scene.states.clone()).unwrap_or_default()
    }

    pub fn title(&self) -> String {
        self.scene.as_ref().map(|scene| scene.title.clone()).ok().filter(|title| !title.is_empty()).unwrap_or_else(|| self.source.scene.clone())
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }

    pub fn trouble(&self) -> &[String] {
        &self.trouble
    }

    /// The head: a band tinted in the accent naming it a Prototype, with its title, the hint where it has views to press, and Open
    /// tab and Pop out; under it a tile for each view, its name over what it shows, with the one drawn marked.
    fn head(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let accent = look.color(token::ACCENT);
        let views: Vec<markup::View> = self.scene.as_ref().map(|scene| scene.views.clone()).unwrap_or_default();
        let several = views.len() > 1;
        let tiles: Vec<_> = views.into_iter().map(|view| self.tile(view, cx)).collect();
        let open = (self.home == Home::Sheet).then(|| {
            labelled(
                parts::button::button(look, "prototype-open-tab", Kind::Secondary, Some("app-window"), "Open tab", false)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PrototypeEvent::OpenTab))),
                Role::Button,
                "Open tab",
            )
        });
        let pop = (self.home != Home::Window).then(|| {
            labelled(
                parts::button::button(look, "prototype-pop-out", Kind::Secondary, Some("maximize-2"), "Pop out", false)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PrototypeEvent::PopOut))),
                Role::Button,
                "Pop out",
            )
        });
        let hint = several.then(|| {
            div().flex_none().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(HINT)
        });
        let band = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .bg(accent.opacity(0.1))
            .child(labelled(chip(look, "prototype-chip", accent, Some("app-window"), LABEL), Role::Label, LABEL))
            .child(div().flex_1().min_w_0().truncate().child(labelled(parts::text::text(look, "prototype-title", TextStyle::Heading, self.title()), Role::Heading, self.title())))
            .children(hint)
            .children(open)
            .children(pop);
        let row = several.then(|| {
            labelled(
                div()
                    .id("prototype-views")
                    .flex_none()
                    .flex()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_2))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .children(tiles),
                Role::Group,
                "Views",
            )
        });
        div().flex_none().flex().flex_col().child(band).children(row)
    }

    /// One view's tile: a press, Enter or Space draws it.
    fn tile(&self, view: markup::View, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let current = self.state.as_deref() == Some(view.id.as_str());
        let accent = look.color(token::ACCENT);
        let (target, keyed) = (view.id.clone(), view.id.clone());
        let says = view.says.clone().map(|says| {
            div().truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(says)
        });
        let named = match (&view.says, current) {
            (Some(says), true) => format!("{}, shown. {says}", view.title),
            (Some(says), false) => format!("{}. {says}", view.title),
            (None, true) => format!("{}, shown", view.title),
            (None, false) => view.title.clone(),
        };
        labelled(
            div()
                .id(SharedString::from(format!("state-{}", view.id)))
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1) / 2.)
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if current { accent } else { look.color(token::BORDER_DEFAULT) })
                .when(current, |tile| tile.bg(accent.opacity(0.1)))
                .when(!current, |tile| tile.hover(|tile| tile.bg(look.color(token::BG_HOVER))))
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .focus_visible(|tile| tile.border_color(look.color(token::EDGE_FOCUS)))
                .when_some(view.says.clone(), |tile, says| tile.tooltip(tip(says)))
                .on_click(cx.listener(move |this, _, _, cx| this.go(&target, cx)))
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                        this.go(&keyed, cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .truncate()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(view.title.clone()),
                )
                .children(says),
            Role::Button,
            named,
        )
    }

    /// What went wrong, a line each, in the danger tint with its hairline.
    fn trouble_line(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        if self.trouble.is_empty() {
            return None;
        }
        let look = cx.look();
        let ink = Tone::Danger.ink(look);
        let lines: Vec<_> = self
            .trouble
            .iter()
            .map(|line| {
                div()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(parts::glyph("triangle-alert", look.length(token::SPACE_4), ink))
                    .child(div().min_w_0().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(line.clone()))
            })
            .collect();
        Some(labelled(
            div()
                .id("prototype-trouble")
                .flex_none()
                .max_h(look.length(token::SPACE_12) * 4.)
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .mx(look.length(token::SPACE_4))
                .mt(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(ink.opacity(0.4))
                .bg(ink.opacity(0.1))
                .children(lines),
            Role::Alert,
            self.trouble.join(". "),
        ))
    }
}

impl Focusable for PrototypeView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PrototypeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let body = match &self.scene {
            Ok(scene) => draw::draw(look, scene, &self.fixtures, self.state.as_deref().unwrap_or_default(), cx.entity().downgrade()).0,
            Err(_) => div().into_any_element(),
        };
        let shot = self.home == Home::Shot;
        let head = (!shot).then(|| self.head(cx));
        let trouble = if shot { None } else { self.trouble_line(cx) };
        let look = cx.look();
        let drawn = Rc::clone(&self.drawn);
        let root = div()
            .id("prototype")
            .track_focus(&self.focus)
            .when(self.home != Home::Window, |view| {
                view.key_context("prototype").on_action(cx.listener(|_, act: &crate::keymap::Act, _, cx| {
                    if act.is("prototype_pop_out") {
                        cx.emit(PrototypeEvent::PopOut);
                    } else {
                        cx.propagate();
                    }
                }))
            })
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            // The frame: a full hairline in the accent marks where the build he approves ends and the Job's own words begin.
            .when(!shot, |root| root.overflow_hidden().rounded(look.length(token::RADIUS_MD)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::ACCENT)))
            .children(head)
            .children(trouble)
            .child(div().id("prototype-scene").flex_1().min_h_0().overflow_y_scroll().child(body))
            .child(canvas(move |bounds, _, _| drawn.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full());
        if shot {
            return root.into_any_element();
        }
        labelled(root, Role::Group, LABEL).into_any_element()
    }
}

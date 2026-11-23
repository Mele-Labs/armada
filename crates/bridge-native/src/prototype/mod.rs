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
pub mod style;
pub mod window;

use gpui::{div, prelude::*, Context, EventEmitter, FocusHandle, Focusable, IntoElement, Render, Role, SharedString, Window};

use self::fixtures::Fixtures;
use self::markup::Scene;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::parts::{self, button::Kind, chip::chip, text::TextStyle, Tone};

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
}

impl EventEmitter<PrototypeEvent> for PrototypeView {}

/// Whether the view is a tab of a Job, with Pop out, or is the pop-out window itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Home {
    Tab,
    Window,
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
    pub(crate) focus: FocusHandle,
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
            focus: cx.focus_handle(),
        };
        view.read(source, cx);
        view
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

    /// The head: the scene's title, a chip for each state with the one drawn marked, and Pop out in a tab.
    fn head(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let states: Vec<String> = self.scene.as_ref().map(|scene| scene.states.clone()).unwrap_or_default();
        let chips: Vec<_> = states
            .into_iter()
            .map(|id| {
                let current = self.state.as_deref() == Some(id.as_str());
                let (ink, glyph) = if current { (Tone::Accent.ink(look), "circle-dot") } else { (Tone::Neutral.ink(look), "circle") };
                let target = id.clone();
                let keyed = id.clone();
                labelled(
                    chip(look, SharedString::from(format!("state-{id}")), ink, Some(glyph), id.clone())
                        .cursor_pointer()
                        .focusable()
                        .tab_stop(true)
                        .on_click(cx.listener(move |this, _, _, cx| this.go(&target, cx)))
                        .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                                this.go(&keyed, cx);
                                cx.stop_propagation();
                            }
                        })),
                    Role::Button,
                    if current { format!("{id}, shown") } else { id },
                )
            })
            .collect();
        let pop = (self.home == Home::Tab).then(|| {
            labelled(
                parts::button::button(look, "prototype-pop-out", Kind::Secondary, Some("square-arrow-out-up-right"), "Pop out", false)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PrototypeEvent::PopOut))),
                Role::Button,
                "Pop out",
            )
        });
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(parts::text::text(look, "prototype-title", TextStyle::Heading, self.title()), Role::Heading, self.title()))
            .child(div().flex_1().min_w_0().flex().items_center().gap(look.length(token::SPACE_2)).children(chips))
            .children(pop)
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
        let head = self.head(cx);
        let trouble = self.trouble_line(cx);
        let look = cx.look();
        div()
            .id("prototype")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .child(head)
            .children(trouble)
            .child(div().id("prototype-scene").flex_1().min_h_0().overflow_y_scroll().child(body))
    }
}

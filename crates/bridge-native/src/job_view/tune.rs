//! The model and effort picker on the message box's bar (`TunePicker.tsx`, section 4.1): a trigger
//! that names the model and draws the effort as a meter, and a panel that opens upward from it.
//!
//! ↑ and ↓ move between the models and pick the one they land on, ← and → slide the effort, Enter
//! and Esc put the panel away. A model list Fleet has not answered for draws its rows as shapes.

use gpui::{
    anchored, deferred, div, prelude::*, svg, Anchor, Animation, AnimationExt, App, Context, DismissEvent, EventEmitter,
    FocusHandle, Focusable, FontWeight, KeyDownEvent, MouseButton, Pixels, Point, Role, SharedString, Window,
};
use ipc::OfferedModel;

use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What was picked: a model and an effort, each absent for the machine's own.
pub(super) struct Tuned {
    pub model: Option<String>,
    pub effort: Option<String>,
}

pub(super) struct TunePicker {
    models: Option<Vec<OfferedModel>>,
    default: String,
    model: Option<String>,
    effort: Option<String>,
    at: Point<Pixels>,
    focus: FocusHandle,
}

impl EventEmitter<Tuned> for TunePicker {}
impl EventEmitter<DismissEvent> for TunePicker {}

impl Focusable for TunePicker {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// The efforts a model takes, weakest first, as the slider's steps after Auto.
fn efforts_of<'a>(models: &'a [OfferedModel], model: Option<&str>, default: &str) -> &'a [String] {
    let name = model.unwrap_or(default);
    models.iter().find(|one| one.name == name).map_or(&[], |one| one.efforts.as_slice())
}

/// The effort meter: one bar for each step, as tall as its place, lit up to the one picked.
pub(super) fn effort_meter(look: &Look, steps: usize, lit: Option<usize>) -> impl IntoElement {
    let height = look.length(token::SPACE_3) - gpui::px(2.);
    labelled(
        div()
            .id("effort-meter")
            .flex_none()
            .flex()
            .items_end()
            .gap(gpui::px(2.))
            .h(height)
            .children((0..steps).map(move |at| {
                let on = lit.is_some_and(|lit| at <= lit);
                div()
                    .w(gpui::px(3.))
                    .h(height * ((at + 1) as f32 / steps as f32))
                    .rounded(gpui::px(1.))
                    .bg(if on { look.color(token::ACCENT) } else { look.color(token::FG_DEFAULT).opacity(0.16) })
            })),
        Role::Image,
        match lit {
            Some(_) => "Effort",
            None => "Effort: auto",
        },
    )
}

impl TunePicker {
    pub fn new(models: Option<Vec<OfferedModel>>, default: String, model: Option<String>, effort: Option<String>, at: Point<Pixels>, cx: &mut Context<Self>) -> Self {
        TunePicker { models, default, model, effort, at, focus: cx.focus_handle() }
    }

    /// The models Jobs may use, with the default's row first.
    fn rows(&self) -> Vec<Option<&OfferedModel>> {
        let mut rows = vec![None];
        rows.extend(self.models.iter().flatten().filter(|one| one.on).map(Some));
        rows
    }

    fn row_at(&self) -> usize {
        self.rows().iter().position(|row| row.map(|one| one.name.as_str()) == self.model.as_deref()).unwrap_or(0)
    }

    fn pick_model(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(model) = self.rows().get(at).map(|row| row.map(|one| one.name.clone())) else { return };
        self.model = model;
        // An effort the new model does not take is the machine's own.
        let steps = self.models.as_deref().map(|models| efforts_of(models, self.model.as_deref(), &self.default).to_vec()).unwrap_or_default();
        if self.effort.as_ref().is_some_and(|effort| !steps.contains(effort)) {
            self.effort = None;
        }
        self.tuned(cx);
    }

    fn slide(&mut self, by: isize, cx: &mut Context<Self>) {
        let steps = self.models.as_deref().map(|models| efforts_of(models, self.model.as_deref(), &self.default).to_vec()).unwrap_or_default();
        let now = self.effort.as_ref().and_then(|effort| steps.iter().position(|one| one == effort)).map_or(0, |at| at + 1);
        let next = (now as isize + by).clamp(0, steps.len() as isize) as usize;
        self.effort = (next > 0).then(|| steps[next - 1].clone());
        self.tuned(cx);
    }

    fn pick_effort(&mut self, effort: Option<String>, cx: &mut Context<Self>) {
        self.effort = effort;
        self.tuned(cx);
    }

    fn tuned(&mut self, cx: &mut Context<Self>) {
        cx.emit(Tuned { model: self.model.clone(), effort: self.effort.clone() });
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "down" => {
                let next = (self.row_at() + 1).min(self.rows().len() - 1);
                self.pick_model(next, cx);
            }
            "up" => {
                let next = self.row_at().saturating_sub(1);
                self.pick_model(next, cx);
            }
            "right" => self.slide(1, cx),
            "left" => self.slide(-1, cx),
            "enter" | "escape" => cx.emit(DismissEvent),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Render for TunePicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let selected = self.row_at();
        let row = |id: SharedString, label: SharedString, name: SharedString, ticked: bool, at: usize, cx: &Context<Self>| {
            labelled(
                div()
                    .id(id)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(look.length(token::H_CONTROL_LG))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(label)
                    .when(ticked, |row| row.child(svg().path(icon_path("check")).size(look.length(token::SPACE_4)).text_color(look.color(token::ACCENT))))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_model(at, cx))),
                Role::MenuItemCheckBox,
                name,
            )
        };
        let rows: Vec<gpui::AnyElement> = match &self.models {
            None => (0..3)
                .map(|at| {
                    div()
                        .h(look.length(token::H_CONTROL_LG))
                        .flex()
                        .items_center()
                        .px(look.length(token::SPACE_3))
                        .child(
                            div()
                                .h(look.length(token::SPACE_2))
                                .w(gpui::relative(0.35 + 0.15 * at as f32))
                                .rounded(look.length(token::RADIUS_SM))
                                .bg(look.color(token::FG_DEFAULT).opacity(0.07)),
                        )
                        .with_animation(
                            SharedString::from(format!("tune-skeleton-{at}")),
                            Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                            |shape, at| shape.opacity(at),
                        )
                        .into_any_element()
                })
                .collect(),
            Some(models) => self
                .rows()
                .into_iter()
                .enumerate()
                .map(|(at, one)| {
                    let (label, name): (SharedString, SharedString) = match one {
                        None => {
                            let named = models.iter().find(|model| model.name == self.default).map_or(self.default.clone(), |model| display_of(model));
                            (format!("Default \u{b7} {named}").into(), format!("Default, {named}").into())
                        }
                        Some(model) => (display_of(model).into(), display_of(model).into()),
                    };
                    row(SharedString::from(format!("tune-model-{at}")), label, name, at == selected, at, cx).into_any_element()
                })
                .collect(),
        };
        let steps: Vec<String> = self.models.as_deref().map(|models| efforts_of(models, self.model.as_deref(), &self.default).to_vec()).unwrap_or_default();
        let effort_row = (!steps.is_empty()).then(|| {
            let chosen = self.effort.clone();
            let step = |id: SharedString, label: SharedString, on: bool, value: Option<String>, cx: &Context<Self>| {
                labelled(
                    div()
                        .id(id)
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(look.length(token::H_CONTROL_SM))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(if on { look.color(token::TEXT_ON_ACCENT) } else { look.color(token::FG_MUTED) })
                        .when(on, |step| step.bg(look.color(token::ACCENT)))
                        .when(!on, |step| step.hover(|style| style.bg(look.color(token::BG_HOVER))))
                        .cursor_pointer()
                        .child(label.clone())
                        .on_click(cx.listener(move |this, _, _, cx| this.pick_effort(value.clone(), cx))),
                    Role::MenuItemCheckBox,
                    label,
                )
            };
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_3))
                .pb(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child("Effort"))
                .child(
                    labelled(
                        div()
                            .id("tune-effort")
                            .flex()
                            .gap(look.length(token::SPACE_1))
                            .child(step("effort-auto".into(), "Auto".into(), chosen.is_none(), None, cx))
                            .children(steps.iter().map(|effort| {
                                step(SharedString::from(format!("effort-{effort}")), effort.clone().into(), chosen.as_ref() == Some(effort), Some(effort.clone()), cx)
                            })),
                        Role::Toolbar,
                        "Effort",
                    ),
                )
        });
        deferred(
            anchored().position(self.at).anchor(Anchor::BottomLeft).snap_to_window_with_margin(look.length(token::SPACE_1)).child(labelled(
                div()
                    .id("tune")
                    .track_focus(&self.focus)
                    .capture_key_down(cx.listener(Self::on_key))
                    .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .occlude()
                    .mb(look.length(token::SPACE_2))
                    .w(look.length(token::SPACE_12) * 6.)
                    .flex()
                    .flex_col()
                    .p(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                    .child(labelled(div().id("tune-models").flex().flex_col().children(rows), Role::Menu, "Model"))
                    .children(effort_row),
                Role::Dialog,
                "Model and effort",
            )),
        )
        .with_priority(1)
    }
}

/// A model as the picker names it: the harness's own label, or the alias where it gave none.
pub(super) fn display_of(model: &OfferedModel) -> String {
    if model.display.is_empty() { model.name.clone() } else { model.display.clone() }
}

/// What the trigger says for a model and effort (`Model <name>, effort <level or auto>`).
pub(super) fn trigger_name(models: Option<&[OfferedModel]>, default: &str, model: Option<&str>, effort: Option<&str>) -> (String, String) {
    let name = model.unwrap_or(default);
    let shown = models.and_then(|models| models.iter().find(|one| one.name == name)).map_or_else(|| name.to_string(), display_of);
    let said = format!("Model {shown}, effort {}", effort.unwrap_or("auto"));
    (shown, said)
}

/// Where the effort sits among a model's steps: its place, or `None` for Auto.
pub(super) fn effort_place(models: Option<&[OfferedModel]>, default: &str, model: Option<&str>, effort: Option<&str>) -> (usize, Option<usize>) {
    let steps = models.map_or(&[][..], |models| efforts_of(models, model, default));
    let at = effort.and_then(|effort| steps.iter().position(|one| one == effort));
    (steps.len(), at)
}

//! The plan edited in place (Electron's `PlanEdit.tsx`), as the review sheet draws it: press a title to
//! rename it, the glyphs on hover to suggest on or remove a group or task, a task's mark to drag it to
//! another group, a group's subagent type to hand it to another, the last line of a group to add a
//! task, and Add group. **None of it changes the plan.** Each edit is a mark (`plan_marks`) held on the
//! Job's page, and they go to the agent together with the sheet's Approve or Send back.
//!
//! One field is open at a time, as one box takes the keys: a rename, a suggestion, a new task or a new
//! group. Enter saves it and Esc leaves it; a new task's line stays open for the next.

use gpui::{div, prelude::*, AnyElement, Context, Entity, FontWeight, KeyDownEvent, MouseButton, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{PlanMark, SessionPlan};

use super::acts::{button, Tone};
use super::plan::task_look;
use super::plan_marks::{self, Since, Target, ViewGroup, ViewTask};
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// The subagent types a group can be handed to.
const AGENTS: [&str; 6] = ["rust-engineer", "bridge-engineer", "protocol-engineer", "fixture-author", "judge", "general-purpose"];

/// What the one open field edits.
#[derive(Clone, PartialEq)]
pub(super) enum What {
    Rename(Target),
    Note(Target),
    NewTask(String),
    NewGroup,
}

struct Field {
    what: What,
    input: Entity<InputState>,
    _subs: Vec<Subscription>,
}

/// A Job's marks on its plan, and the field open on it.
#[derive(Default)]
pub(super) struct EditState {
    pub marks: Vec<PlanMark>,
    /// Edit plan: the rows are drawn in place of the diff against the version before.
    pub editing: bool,
    field: Option<Field>,
}

/// A task held by the pointer on its way to another group.
#[derive(Clone)]
struct Dragged {
    task: String,
    text: SharedString,
}

/// What the pointer carries: the task's words.
struct Ghost(SharedString);

impl Render for Ghost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        div()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(self.0.clone())
    }
}

impl JobView {
    pub(super) fn plan_of_shown(&self, cx: &gpui::App) -> Option<(SharedString, SessionPlan)> {
        let id = self.shown.clone()?;
        let plan = self.record(&id, cx)?.plan.clone()?;
        Some((id, plan))
    }

    /// The marks on the shown Job's plan.
    pub(super) fn plan_marks(&self, id: &str) -> Vec<PlanMark> {
        self.pages.get(id).map(|page| page.plan_edit.marks.clone()).unwrap_or_default()
    }

    fn set_marks(&mut self, marks: Vec<PlanMark>, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            page.plan_edit.marks = marks;
        }
        cx.notify();
    }

    /// The marks go with an answer: the page forgets them.
    pub(super) fn take_marks(&mut self, id: &str) -> Vec<PlanMark> {
        self.pages.get_mut(id).map(|page| std::mem::take(&mut page.plan_edit.marks)).unwrap_or_default()
    }

    fn edit(&mut self, change: impl FnOnce(&SessionPlan, &[PlanMark]) -> Vec<PlanMark>, cx: &mut Context<Self>) {
        let Some((id, plan)) = self.plan_of_shown(cx) else { return };
        let next = change(&plan, &self.plan_marks(&id));
        self.set_marks(next, cx);
    }

    fn open_field(&mut self, what: What, value: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |input, cx| {
            input.set_value(value, window, cx);
            input.focus(window, cx);
        });
        let blur_shuts = matches!(what, What::Rename(_) | What::NewGroup);
        let subs = vec![cx.subscribe_in(&input, window, move |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.save_field(window, cx),
            InputEvent::Blur if blur_shuts => this.shut_field(cx),
            _ => {}
        })];
        if let Some(page) = self.pages.get_mut(&id) {
            page.plan_edit.field = Some(Field { what, input, _subs: subs });
        }
        cx.notify();
    }

    fn shut_field(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            page.plan_edit.field = None;
        }
        cx.notify();
    }

    pub(super) fn plan_field_open(&self) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id.as_str())).is_some_and(|page| page.plan_edit.field.is_some())
    }

    /// Shut by a key: the keys go back to the sheet's list, not to a field no longer drawn.
    pub(super) fn field_done(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shut_field(cx);
        if self.review_open() {
            window.focus(&self.review_focus.list, cx);
        } else {
            self.to_composer(window, cx);
        }
    }

    fn save_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some((what, input)) = self.pages.get(&id).and_then(|page| page.plan_edit.field.as_ref()).map(|field| (field.what.clone(), field.input.clone())) else { return };
        let text = input.read(cx).value().to_string();
        match &what {
            What::Rename(target) => self.edit(|plan, marks| plan_marks::rename(plan, marks, target, &text), cx),
            What::Note(target) => self.edit(|_, marks| plan_marks::add_note(marks, target, &text), cx),
            What::NewTask(group) => {
                // The line stays for the next task, empty.
                if !text.trim().is_empty() {
                    self.edit(|_, marks| plan_marks::add_task(marks, group, &text), cx);
                    input.update(cx, |input, cx| input.set_value(String::new(), window, cx));
                }
                return;
            }
            What::NewGroup => self.edit(|_, marks| plan_marks::add_group(marks, &text), cx),
        }
        self.field_done(window, cx);
    }

    fn agent_menu(&mut self, group: String, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, plan)) = self.plan_of_shown(cx) else { return };
        let now = plan_marks::view_of(&plan, &self.plan_marks(self.shown.as_deref().unwrap_or_default()))
            .into_iter()
            .find(|one| one.id == group)
            .map(|one| one.agent)
            .unwrap_or_default();
        // The group's own type first, as Electron's select lists it, then the rest.
        let own = (!AGENTS.contains(&now.as_str())).then(|| Entry::Choice { id: "own", label: now.clone().into(), on: true });
        let entries = own.into_iter().chain(AGENTS.iter().map(|agent| Entry::Choice { id: agent, label: (*agent).into(), on: *agent == now })).collect();
        self.open_pop(
            "Subagent type",
            entries,
            at,
            from_keys,
            Some(if self.review_open() { self.review_focus.list.clone() } else { gpui::Focusable::focus_handle(self.composer.input.read(cx), cx) }),
            move |this, picked, _, cx| {
                if picked != "own" {
                    this.edit(|plan, marks| plan_marks::set_agent(plan, marks, &group, picked), cx);
                }
            },
            window,
            cx,
        );
    }

    // ---- drawing -------------------------------------------------------------------------

    /// The plan in the review sheet: its summary, then each group edited in place and Add group. A plan
    /// recorded again and not marked reads as the diff against the version before, with Edit plan.
    /// `compact` is Now's form, as Electron's `PlanGroup` draws it there: no card, and the subagent type
    /// behind a chevron.
    pub(super) fn plan_editor(&self, id: &SharedString, plan: &SessionPlan, diff: Option<AnyElement>, compact: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let state = self.pages.get(id.as_str()).map(|page| &page.plan_edit);
        let marks = state.map(|state| state.marks.as_slice()).unwrap_or_default();
        let field = state.and_then(|state| state.field.as_ref());
        let editing = state.is_some_and(|state| state.editing);
        let summary = (!plan.summary.trim().is_empty()).then(|| {
            div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(plan.summary.clone()))
        });
        let switch = (diff.is_some() && marks.is_empty()).then(|| {
            let said = if editing { "Show changes" } else { "Edit plan" };
            div().flex().child(labelled(
                button(look, "plan-edit-switch", said, Tone::Ghost, false, None, true).on_click(cx.listener(|this, _, _, cx| {
                    if let Some(page) = this.shown.clone().and_then(|id| this.pages.get_mut(&id)) {
                        page.plan_edit.editing = !page.plan_edit.editing;
                    }
                    cx.notify();
                })),
                Role::Button,
                said,
            ))
        });
        let body: AnyElement = match diff.filter(|_| !editing && marks.is_empty()) {
            Some(diff) => diff,
            None => {
                let groups = plan_marks::view_of(plan, marks);
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .children(groups.iter().map(|group| self.group_editor(group, field, compact, look, cx)))
                    .child(self.add_group(field, look, cx))
                    .into_any_element()
            }
        };
        div().flex().flex_col().gap(look.length(token::SPACE_3)).children(summary).children(switch).child(body).into_any_element()
    }

    /// The one field, if it is open on `what`.
    fn field_on<'a>(field: Option<&'a Field>, what: &What) -> Option<&'a Field> {
        field.filter(|field| &field.what == what)
    }

    fn field_box(&self, field: &Field, label: SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(format!("plan-field:{label}")))
                .key_context(crate::keymap::TEXT_BOX)
                .flex_1()
                .min_w_0()
                .h(look.length(token::H_CONTROL))
                .flex()
                .items_center()
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::ACCENT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.field_done(window, cx);
                    }
                }))
                .child(Input::new(&field.input).appearance(false)),
            Role::TextInput,
            label,
        )
        .into_any_element()
    }

    /// A name that is a button until pressed, then a field: Enter saves it, Esc or leaving keeps the old one.
    #[allow(clippy::too_many_arguments)]
    fn edit_text(&self, key: String, kind: &'static str, value: &str, was: Option<&str>, target: Target, strong: bool, field: Option<&Field>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let what = What::Rename(target);
        if let Some(open) = Self::field_on(field, &what) {
            return self.field_box(open, kind.into(), look, cx);
        }
        let start = value.to_string();
        let name = labelled(
            div()
                .id(SharedString::from(format!("plan-name:{key}")))
                .min_w_0()
                .truncate()
                .cursor_text()
                .text_color(look.color(if strong { token::FG_DEFAULT } else { token::FG_MUTED }))
                .when(strong, |name| name.font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                .hover(|style| style.underline())
                .child(SharedString::from(value.to_string()))
                .on_click(cx.listener(move |this, _, window, cx| this.open_field(what.clone(), start.clone(), window, cx))),
            Role::Button,
            format!("{kind}: {value}"),
        );
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .children(was.map(|was| div().flex_none().line_through().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(was.to_string()))))
            .child(name)
            .into_any_element()
    }

    /// Suggest, and Remove or Keep: the glyphs a row shows on hover.
    fn tools(&self, key: &str, hover: &'static str, what: String, target: Target, removed: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let tool = |id: String, icon: &'static str, tooltip: &'static str| {
            div()
                .id(SharedString::from(id))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip(tooltip))
                .child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED)))
        };
        let (note, gone) = (target.clone(), target);
        let keep = if removed { "Keep" } else { "Remove" };
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .opacity(0.)
            .group_hover(hover, |style| style.opacity(1.))
            .child(labelled(
                tool(format!("suggest:{key}"), "message-square-plus", "Suggest").on_click(cx.listener(move |this, _, window, cx| this.open_field(What::Note(note.clone()), String::new(), window, cx))),
                Role::Button,
                format!("Suggest on {what}"),
            ))
            .child(labelled(
                tool(format!("remove:{key}"), if removed { "undo-2" } else { "trash" }, keep).on_click(cx.listener(move |this, _, _, cx| this.edit(|_, marks| plan_marks::remove(marks, &gone), cx))),
                Role::Button,
                format!("{keep} {what}"),
            ))
            .into_any_element()
    }

    /// What he wrote, pinned to what it is about, and the box a new one is written in.
    fn notes(&self, key: &str, notes: &[plan_marks::Note], target: &Target, about: &str, field: Option<&Field>, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let open = Self::field_on(field, &What::Note(target.clone()));
        if notes.is_empty() && open.is_none() {
            return None;
        }
        let pins = notes.iter().map(|note| {
            let id = note.id.clone();
            div()
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .child(labelled(
                    div().id(SharedString::from(format!("pin:{}", note.id))).flex_none().pt(look.length(token::SPACE_1) / 2.).tooltip(tip("Suggestion, not sent yet")).child(glyph(look, "message-square", Size::Small, look.color(token::ACCENT))),
                    Role::Image,
                    "Suggestion",
                ))
                .child(div().flex_1().min_w_0().child(SharedString::from(note.text.clone())))
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("unpin:{}", note.id)))
                        .flex_none()
                        .cursor_pointer()
                        .tooltip(tip("Take back"))
                        .child(glyph(look, "x", Size::Small, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, _, cx| this.edit(|_, marks| plan_marks::drop_note(marks, &id), cx))),
                    Role::Button,
                    "Take back suggestion",
                ))
        });
        let typing = open.map(|open| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .child(self.field_box(open, format!("Suggest on {about}").into(), look, cx))
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("close-note:{key}")))
                        .flex_none()
                        .cursor_pointer()
                        .tooltip(tip("Close"))
                        .child(glyph(look, "x", Size::Mark, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(|this, _, _, cx| this.shut_field(cx))),
                    Role::Button,
                    "Close suggestion",
                ))
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(labelled(div().id(SharedString::from(format!("pins:{key}"))).flex().flex_col().gap(look.length(token::SPACE_1)).children(pins), Role::List, "Suggestions"))
                .children(typing)
                .into_any_element(),
        )
    }

    fn since_mark(key: &str, since: &Since, look: &Look) -> AnyElement {
        let (icon, said) = match since {
            Since::Added => ("plus", "Added by the agent".to_string()),
            Since::Renamed(from) => ("pencil", format!("Renamed by the agent, was {from}")),
        };
        labelled(div().id(SharedString::from(format!("since:{key}"))).flex_none().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, look.color(token::STEP_RUNNING))), Role::Image, said)
            .into_any_element()
    }

    fn added_mark(key: &str, look: &Look) -> AnyElement {
        labelled(div().id(SharedString::from(format!("added:{key}"))).flex_none().tooltip(tip("Added, not sent yet")).child(glyph(look, "plus", Size::Small, look.color(token::ACCENT))), Role::Image, "Added")
            .into_any_element()
    }

    fn task_row(&self, task: &ViewTask, group: &ViewGroup, field: Option<&Field>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let target = Target::task(&task.home, &task.id);
        let hover: &'static str = "plan-task";
        let (icon, ink, said) = task_look(task.state);
        let key = format!("{}:{}", group.id, task.id);
        let dragged = Dragged { task: task.id.clone(), text: task.text.clone().into() };
        let line = div()
            .id(SharedString::from(format!("plan-task:{key}")))
            .group(hover)
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_CONTROL_SM))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .when(task.removed, |line| line.line_through().opacity(0.6))
            .child(
                div()
                    .id(SharedString::from(format!("plan-task-mark:{key}")))
                    .flex_none()
                    .cursor_grab()
                    .tooltip(tip(said))
                    .child(glyph(look, icon, Size::Small, look.color(ink)))
                    .on_drag(dragged, |dragged, _, _, cx| cx.new(|_| Ghost(dragged.text.clone()))),
            )
            .child(div().flex_1().min_w_0().child(self.edit_text(key.clone(), "Task", &task.text, task.was.as_deref(), target.clone(), false, field, look, cx)))
            .children(task.added.then(|| Self::added_mark(&key, look)))
            .children(task.moved_from.as_ref().map(|from| div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(format!("from {from}")))))
            .children(task.since.as_ref().map(|since| Self::since_mark(&key, since, look)))
            .child(self.tools(&key, hover, format!("task {}", task.text), target.clone(), task.removed, look, cx));
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(line)
            .children(self.notes(&key, &task.notes, &target, &task.text, field, look, cx).map(|notes| div().pl(look.length(token::SPACE_6)).child(notes)))
            .into_any_element()
    }

    /// The last task line: an empty one to type into, as Electron's `AddTask` draws it.
    fn add_task(&self, group: &ViewGroup, field: Option<&Field>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let what = What::NewTask(group.id.clone());
        let label: SharedString = format!("New task in {}", group.title).into();
        let mark = glyph(look, "plus", Size::Small, look.color(token::FG_SUBTLE));
        match Self::field_on(field, &what) {
            Some(open) => div().flex().items_center().gap(look.length(token::SPACE_2)).child(mark).child(self.field_box(open, label, look, cx)).into_any_element(),
            None => labelled(
                div()
                    .id(SharedString::from(format!("plan-new-task:{}", group.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL_SM))
                    .cursor_text()
                    .child(mark)
                    .on_click(cx.listener(move |this, _, window, cx| this.open_field(what.clone(), String::new(), window, cx))),
                Role::TextInput,
                label,
            )
            .into_any_element(),
        }
    }

    fn group_editor(&self, group: &ViewGroup, field: Option<&Field>, compact: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let target = Target::group(&group.id);
        let hover: &'static str = "plan-group";
        let key = group.id.clone();
        let agent_group = group.id.clone();
        let unfolded = compact && self.side_unfolded(&format!("plan:{key}"));
        let fold = compact.then(|| {
            let fold_key = format!("plan:{key}");
            labelled(
                div()
                    .id(SharedString::from(format!("plan-fold:{key}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL_SM))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(glyph(look, if unfolded { "chevron-up" } else { "chevron-down" }, Size::Small, look.color(token::FG_MUTED)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.side_fold(&fold_key);
                        cx.notify();
                    })),
                Role::Button,
                format!("Subagent type and paths of {}", group.title),
            )
        });
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(if compact { token::TEXT_XS } else { token::TEXT_BASE }))
            .line_height(look.length(if compact { token::LEADING_XS } else { token::LEADING_BASE }))
            .when(compact, |head| head.child(glyph(look, "dot", Size::Small, look.color(token::FG_SUBTLE))))
            .child(div().flex_1().min_w_0().child(self.edit_text(key.clone(), "Group", &group.title, group.was.as_deref(), target.clone(), true, field, look, cx)))
            .children(group.added.then(|| Self::added_mark(&key, look)))
            .children(group.since.as_ref().map(|since| Self::since_mark(&key, since, look)))
            .child(self.tools(&key, hover, format!("group {}", group.title), target.clone(), group.removed, look, cx))
            .children(fold);
        let paths: Vec<String> = self.plan_of_shown(cx).and_then(|(_, plan)| plan.groups.into_iter().find(|one| one.id == group.id)).map(|one| one.paths).unwrap_or_default();
        let paths = (unfolded && !paths.is_empty()).then(|| {
            div().flex().flex_col().pl(look.length(token::SPACE_6)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).children(paths.into_iter().map(|path| div().truncate().child(SharedString::from(path))))
        });
        let agent = labelled(
            div()
                .id(SharedString::from(format!("plan-agent:{key}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .pl(look.length(token::SPACE_2))
                .pr(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .cursor_pointer()
                .hover(|style| style.border_color(look.color(token::BORDER_STRONG)))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .tooltip(tip("Subagent type"))
                .child(SharedString::from(group.agent.clone()))
                .child(div().flex_1())
                .child(glyph(look, "chevron-down", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    let at = event.position();
                    this.agent_menu(agent_group.clone(), at, event.is_keyboard(), window, cx);
                })),
            Role::ComboBox,
            format!("Subagent type of {}", group.title),
        );
        let meta = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(glyph(look, "bot", Size::Small, look.color(token::FG_MUTED)))
            .children(group.agent_was.as_ref().map(|was| div().line_through().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(was.clone()))))
            .child(div().w(look.length(token::SPACE_12) * 3.).child(agent));
        let gone = group.gone.iter().map(|text| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_SUBTLE))
                .child(labelled(
                    div().id(SharedString::from(format!("gone:{key}:{text}"))).flex_none().tooltip(tip("Removed by the agent")).child(glyph(look, "trash", Size::Small, look.color(token::STEP_FAILED))),
                    Role::Image,
                    "Removed by the agent",
                ))
                .child(div().line_through().child(SharedString::from(text.clone())))
        });
        let home = group.id.clone();
        labelled(
            div()
                .id(SharedString::from(format!("plan-group:{key}")))
                .group(hover)
                .flex()
                .flex_col()
                .gap(look.length(if compact { token::SPACE_1 } else { token::SPACE_2 }))
                .when(compact, |card| card.text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)))
                .when(!compact, |card| card.p(look.length(token::SPACE_3)).rounded(look.length(token::RADIUS_MD)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).bg(look.color(token::BG_RAISED)))
                .when(group.removed, |card| card.opacity(0.6))
                .drag_over::<Dragged>(|style, _, _, cx| style.border_color(cx.look().color(token::ACCENT)))
                .on_drop(cx.listener(move |this, dragged: &Dragged, _, cx| {
                    let task = dragged.task.clone();
                    this.edit(|plan, marks| plan_marks::move_task(plan, marks, &task, &home), cx);
                }))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(head)
                .when(!compact || unfolded, |card| card.child(meta))
                .children(paths)
                .children(self.notes(&key, &group.notes, &target, &group.title, field, look, cx))
                .child(
                    labelled(
                        div()
                            .id(SharedString::from(format!("plan-tasks:{key}")))
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .children(group.tasks.iter().map(|task| self.task_row(task, group, field, look, cx)))
                            .children(gone)
                            .child(self.add_task(group, field, look, cx)),
                        Role::List,
                        "Tasks",
                    ),
                ),
            Role::Region,
            group.title.clone(),
        )
        .into_any_element()
    }

    /// Add group: a button until pressed, then a field for its title.
    fn add_group(&self, field: Option<&Field>, look: &Look, cx: &Context<Self>) -> AnyElement {
        match Self::field_on(field, &What::NewGroup) {
            Some(open) => self.field_box(open, "New group".into(), look, cx),
            None => div()
                .flex()
                .justify_center()
                .child(labelled(
                    button(look, "plan-add-group", "Add group", Tone::Ghost, false, None, true)
                        .child(glyph(look, "plus", Size::Mark, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(|this, _, window, cx| this.open_field(What::NewGroup, String::new(), window, cx))),
                    Role::Button,
                    "Add group",
                ))
                .into_any_element(),
        }
    }
}

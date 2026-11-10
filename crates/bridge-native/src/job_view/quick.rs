//! The quick bar on the message box's top edge (`QuickBar.tsx`, section 4.7): the answers Fleet
//! offers to the agent's last message, then the lines the person saved, then the ones Fleet learnt.
//!
//! The saved lines live in the `thread.look` setting. Every act on them is a plain write of that
//! setting, drawn as done the same frame and put back with Fleet's reason if it refuses. What
//! Electron works out when a learnt line is pinned, reworded or deleted is not ported: a learnt
//! line is pressed and read, and has no menu.

use std::collections::{BTreeMap, HashMap};

use gpui::{
    div, point, prelude::*, svg, Anchor, AnyElement, ClickEvent, Context, Entity, Focusable, KeyDownEvent, MouseButton,
    Role, SharedString, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{SaveSettings, SessionQuick, SettingValue, SettingsList, ThreadItem};

use super::head::{below_left, drawn, icon_button, tip};
use super::wire::Done;
use super::JobView;
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, Look};

const KEY: &str = "thread.look";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Kind {
    Asked,
    Saved,
    Learnt,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Line {
    pub id: Option<String>,
    pub text: String,
    pub kind: Kind,
    pub pinned: bool,
}

struct Edit {
    id: String,
    input: Entity<InputState>,
    _subs: Vec<Subscription>,
}

#[derive(Default)]
pub(super) struct Quick {
    /// `thread.look` as Fleet holds it, once read.
    look: Option<BTreeMap<String, SettingValue>>,
    looked: bool,
    asked: HashMap<SharedString, Vec<String>>,
    /// The message each Job's answers were asked for.
    asking: HashMap<SharedString, String>,
    learnt: Vec<String>,
    edit: Option<Edit>,
    /// What a mock scenario answers for every Job.
    mock: Option<SessionQuick>,
}

impl Quick {
    /// A mock scenario's look and answers, where Fleet has none to give.
    pub fn mock(look: SettingValue, quick: SessionQuick) -> Self {
        let look = match look {
            SettingValue::Map(map) => Some(map),
            _ => None,
        };
        Quick { look, looked: true, learnt: quick.learnt.clone(), mock: Some(quick), ..Quick::default() }
    }

    fn flag(&self, key: &str) -> bool {
        matches!(self.look.as_ref().and_then(|look| look.get(key)), Some(SettingValue::Bool(true)))
    }

    fn texts(&self, key: &str) -> Vec<String> {
        match self.look.as_ref().and_then(|look| look.get(key)) {
            Some(SettingValue::List(items)) => items.iter().filter_map(|one| if let SettingValue::Text(text) = one { Some(text.clone()) } else { None }).collect(),
            _ => Vec::new(),
        }
    }

    /// The lines he saved, in the order the setting keeps them.
    fn kept(&self) -> Vec<Line> {
        let Some(SettingValue::List(items)) = self.look.as_ref().and_then(|look| look.get("quick")) else { return Vec::new() };
        items
            .iter()
            .filter_map(|one| {
                let SettingValue::Map(map) = one else { return None };
                let text = match map.get("text") {
                    Some(SettingValue::Text(text)) => text.clone(),
                    _ => return None,
                };
                let id = match map.get("id") {
                    Some(SettingValue::Text(id)) => Some(id.clone()),
                    _ => None,
                };
                let kind = match map.get("kind") {
                    Some(SettingValue::Text(kind)) if kind == "learnt" => Kind::Learnt,
                    _ => Kind::Saved,
                };
                Some(Line { id, text, kind, pinned: matches!(map.get("pinned"), Some(SettingValue::Bool(true))) })
            })
            .collect()
    }

    /// Whether the bar is drawn in this Job (`quickShownIn`).
    fn shown_in(&self, job: &str) -> bool {
        !self.flag("quickOff") && !self.texts("quickOffFor").iter().any(|one| one == job)
    }

    /// The lines as the bar draws them: Fleet's answers, then his own with pinned first, then learnt.
    fn lines(&self, job: &str) -> Vec<Line> {
        let mut lines: Vec<Line> = self
            .asked
            .get(job)
            .into_iter()
            .flatten()
            .map(|text| Line { id: None, text: text.clone(), kind: Kind::Asked, pinned: false })
            .collect();
        let kept = self.kept();
        lines.extend(kept.iter().filter(|one| one.pinned).cloned());
        lines.extend(kept.iter().filter(|one| !one.pinned).cloned());
        if self.flag("learn") {
            let forgotten = self.texts("forgotten");
            for text in &self.learnt {
                if !forgotten.contains(text) && !lines.iter().any(|one| one.text == *text) {
                    lines.push(Line { id: None, text: text.clone(), kind: Kind::Learnt, pinned: false });
                }
            }
        }
        lines
    }

    fn set_kept(&mut self, lines: Vec<Line>) {
        let items = lines
            .into_iter()
            .map(|one| {
                let mut map = BTreeMap::new();
                map.insert("id".to_string(), SettingValue::Text(one.id.unwrap_or_default()));
                map.insert("text".to_string(), SettingValue::Text(one.text));
                map.insert("kind".to_string(), SettingValue::Text(if one.kind == Kind::Learnt { "learnt" } else { "saved" }.into()));
                if one.pinned {
                    map.insert("pinned".to_string(), SettingValue::Bool(true));
                }
                SettingValue::Map(map)
            })
            .collect();
        self.look.get_or_insert_with(BTreeMap::new).insert("quick".into(), SettingValue::List(items));
    }
}

/// What a line's menu offers, by its id.
const EDIT: &str = "edit";
const PIN: &str = "pin";
const DELETE: &str = "delete";
const SAVE_ASKED: &str = "save-asked";

impl JobView {
    /// Read `thread.look` once from Fleet.
    pub(super) fn want_look(&mut self, cx: &mut Context<Self>) {
        if self.composer.quick.looked || self.fleet.is_none() {
            return;
        }
        let Some(port) = self.port(cx) else { return };
        self.composer.quick.looked = true;
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, "/settings"))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<SettingsList>("settings", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(list) => {
                        let value = list.settings.into_iter().find(|one| one.key == KEY).map(|one| one.value);
                        if let Some(SettingValue::Map(map)) = value {
                            this.composer.quick.look = Some(map);
                        }
                    }
                    Err(_) => this.composer.quick.looked = false,
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The message Fleet's answers are for: the thread's last row that is a message.
    fn last_message(&self, id: &str, cx: &gpui::App) -> String {
        self.thread(id, cx)
            .and_then(|held| {
                held.items.iter().rev().find_map(|item| match item {
                    ThreadItem::Row { row: ipc::SessionRow::Message { id, .. } } => Some(id.clone()),
                    _ => None,
                })
            })
            .unwrap_or_default()
    }

    /// Ask Fleet for the answers to the Job's last message, once for each message.
    pub(super) fn quick_follow(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let message = self.last_message(&id, cx);
        if self.composer.quick.asking.get(&id) == Some(&message) {
            return;
        }
        self.composer.quick.asking.insert(id.clone(), message.clone());
        if let Some(mock) = &self.composer.quick.mock {
            let answers = mock.asked.clone();
            self.composer.quick.asked.insert(id, answers);
            return;
        }
        let Some(port) = self.port(cx) else { return };
        let path = format!("/sessions/quick?session_id={}", super::wire::encoded(&id));
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<SessionQuick>("SessionQuick", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(read) => {
                        this.composer.quick.asked.insert(id, read.asked);
                        this.composer.quick.learnt = read.learnt;
                    }
                    // A read that failed is asked again the next time the last message moves.
                    Err(_) => {
                        this.composer.quick.asking.remove(&id);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Whether a card in the thread waits on the person: the bar steps aside (`waits`).
    fn waits(&self, id: &str, cx: &gpui::App) -> bool {
        self.thread(id, cx).is_some_and(|held| {
            held.head.asked.is_some()
                || held.head.lapsed.is_some()
                || held.head.plan.as_ref().is_some_and(|plan| plan.state == ipc::PlanCardState::Waiting)
        })
    }

    /// Write the look. The bar already shows it; a refusal puts the old one back with Fleet's reason.
    fn write_look(&mut self, was: Option<BTreeMap<String, SettingValue>>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(map) = self.composer.quick.look.clone() else { return };
        let mut changes = BTreeMap::new();
        changes.insert(KEY.to_string(), Some(SettingValue::Map(map)));
        let job = self.shown.clone();
        let done: Done = Box::new(move |this, answer, _, cx| {
            if let Err(why) = answer {
                this.composer.quick.look = was;
                if let Some(page) = job.as_ref().and_then(|job| this.pages.get_mut(job)) {
                    page.refused = Some(why.into());
                }
            }
            cx.notify();
        });
        self.post("/settings/save".into(), ipc::encode(&SaveSettings { changes }), window, cx, done);
        cx.notify();
    }

    /// Save what is typed as a quick line (skips a line already kept).
    fn save_typed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.kept(cx).text.trim().to_string();
        self.save_line(text, window, cx);
    }

    fn save_line(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if text.is_empty() || self.composer.quick.kept().iter().any(|one| one.text == text) {
            return;
        }
        let was = self.composer.quick.look.clone();
        let mut lines = self.composer.quick.kept();
        lines.push(Line { id: Some(format!("saved:{}", chrono::Utc::now().timestamp_millis())), text, kind: Kind::Saved, pinned: false });
        self.composer.quick.set_kept(lines);
        self.write_look(was, window, cx);
    }

    /// Press line `n`: send it as it is, or with ⇧ put it after the draft. False where there is no line.
    pub(super) fn press_quick(&mut self, n: usize, shift: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.shown.clone() else { return false };
        if !self.composer.quick.shown_in(&id) || self.waits(&id, cx) {
            return false;
        }
        let Some(line) = self.composer.quick.lines(&id).get(n).cloned() else { return false };
        self.press_line(line.text, shift, window, cx);
        true
    }

    fn press_line(&mut self, text: String, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !shift {
            self.send_line(text, window, cx);
            return;
        }
        let held = self.kept(cx);
        let joined = if held.text.trim().is_empty() { text } else { format!("{} {text}", held.text.trim_end()) };
        let kept = super::composer::Kept { text: joined, tags: held.tags };
        self.composer.setting = true;
        let content = kept.clone();
        self.composer.input.update(cx, |input, cx| {
            input.set_value(content.text.clone(), window, cx);
            let end = input.value().len();
            input.set_selected_range(end..end, cx);
            input.focus(window, cx);
        });
        self.typed_now(cx);
    }

    fn line_by(&self, id: &str) -> Option<Line> {
        self.composer.quick.kept().into_iter().find(|one| one.id.as_deref() == Some(id))
    }

    fn act_on_line(&mut self, id: String, act: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let was = self.composer.quick.look.clone();
        let mut lines = self.composer.quick.kept();
        match act {
            PIN => {
                let Some(at) = lines.iter().position(|one| one.id.as_deref() == Some(&id)) else { return };
                let mut one = lines.remove(at);
                one.pinned = !one.pinned;
                one.kind = Kind::Saved;
                // Pinning puts the line last among the pinned, the order they were pinned in.
                let place = lines.iter().filter(|other| other.pinned).count();
                lines.insert(if one.pinned { place } else { lines.len() }, one);
            }
            DELETE => {
                let Some(gone) = lines.iter().find(|one| one.id.as_deref() == Some(&id)).map(|one| one.text.clone()) else { return };
                lines.retain(|one| one.id.as_deref() != Some(&id));
                let mut forgotten = self.composer.quick.texts("forgotten");
                forgotten.push(gone);
                if let Some(look) = self.composer.quick.look.as_mut() {
                    look.insert("forgotten".into(), SettingValue::List(forgotten.into_iter().map(SettingValue::Text).collect()));
                }
            }
            EDIT => {
                self.start_edit(id, window, cx);
                return;
            }
            _ => return,
        }
        self.composer.quick.set_kept(lines);
        self.write_look(was, window, cx);
    }

    fn start_edit(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(line) = self.line_by(&id) else { return };
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |input, cx| {
            input.set_value(line.text.clone(), window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        let entered = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_edit(true, window, cx),
            _ => {}
        });
        self.composer.quick.edit = Some(Edit { id, input, _subs: vec![entered] });
        cx.notify();
    }

    /// Enter and leaving save, Esc cancels, and a line left empty or unchanged is a cancel.
    fn finish_edit(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.composer.quick.edit.take() else { return };
        let said = edit.input.read(cx).value().trim().to_string();
        let box_keys = self.composer.input.read(cx).focus_handle(cx);
        window.focus(&box_keys, cx);
        let was = self.composer.quick.look.clone();
        let mut lines = self.composer.quick.kept();
        let Some(one) = lines.iter_mut().find(|one| one.id.as_deref() == Some(&edit.id)) else { return };
        if keep && !said.is_empty() && said != one.text {
            one.text = said;
            one.kind = Kind::Saved;
            self.composer.quick.set_kept(lines);
            self.write_look(was, window, cx);
        }
        cx.notify();
    }

    fn hide(&mut self, everywhere: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let was = self.composer.quick.look.clone();
        let look = self.composer.quick.look.get_or_insert_with(BTreeMap::new);
        if everywhere {
            look.insert("quickOff".into(), SettingValue::Bool(true));
        } else {
            let mut off = self.composer.quick.texts("quickOffFor");
            off.push(job.to_string());
            if let Some(look) = self.composer.quick.look.as_mut() {
                look.insert("quickOffFor".into(), SettingValue::List(off.into_iter().map(SettingValue::Text).collect()));
            }
        }
        self.write_look(was, window, cx);
    }

    fn open_hide_menu(&mut self, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = vec![
            Entry::Act { id: "hide-job", label: "Hide for this Job".into(), icon: Some("eye-off"), danger: false },
            Entry::Act { id: "hide-all", label: "Hide for all Jobs".into(), icon: Some("eye-off"), danger: false },
        ];
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Hide quick replies", entries, at, Anchor::BottomRight, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.hide(*id == "hide-all", window, cx);
        });
        self.open_popup(menu, back, vec![picked], window, cx);
    }

    /// A line's menu: Edit, Pin, Delete for a kept line; Save for one of Fleet's answers.
    fn open_line_menu(&mut self, line: Line, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = match (&line.id, line.kind) {
            (Some(_), _) => vec![
                Entry::Act { id: EDIT, label: "Edit".into(), icon: Some("square-pen"), danger: false },
                Entry::Act { id: PIN, label: if line.pinned { "Unpin" } else { "Pin" }.into(), icon: Some("arrow-up-to-line"), danger: false },
                Entry::Rule,
                Entry::Act { id: DELETE, label: "Delete".into(), icon: Some("trash-2"), danger: true },
            ],
            (None, Kind::Asked) => {
                let kept = self.composer.quick.kept().iter().any(|one| one.text == line.text);
                vec![Entry::Item {
                    id: SAVE_ASKED,
                    label: "Save".into(),
                    icon: Some("message-square-quote"),
                    danger: false,
                    key: None,
                    off: kept.then(|| "Already a quick reply".into()),
                }]
            }
            _ => return,
        };
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Quick reply", entries, at, Anchor::TopLeft, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            match (*id, line.id.clone()) {
                (SAVE_ASKED, _) => this.save_line(line.text.clone(), window, cx),
                (act, Some(job)) => this.act_on_line(job, act, window, cx),
                _ => {}
            }
        });
        self.open_popup(menu, back, vec![picked], window, cx);
    }

    /// The bar (4.7): the chips, then Save and Hide. Not drawn where it is hidden or a card waits.
    pub(super) fn render_quick(&self, id: &SharedString, look: &Look, _window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.composer.quick.shown_in(id) || self.waits(id, cx) {
            return None;
        }
        let lines = self.composer.quick.lines(id);
        let asked = lines.iter().filter(|one| one.kind == Kind::Asked).count();
        let mut chips: Vec<AnyElement> = Vec::new();
        for (at, line) in lines.iter().enumerate() {
            if asked > 0 && at == asked {
                chips.push(div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::SPACE_4)).bg(look.color(token::BORDER_DEFAULT)).into_any_element());
            }
            if let (Some(edit), Some(line_id)) = (&self.composer.quick.edit, &line.id) {
                if edit.id == *line_id {
                    chips.push(
                        labelled(
                            div()
                                .id("quick-edit")
                                .flex_none()
                                .w(look.length(token::SPACE_12) * 3.)
                                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key == "escape" {
                                        this.finish_edit(false, window, cx);
                                        cx.stop_propagation();
                                    }
                                }))
                                .text_size(look.length(token::TEXT_XS))
                                .child(Input::new(&edit.input).appearance(false).cleanable(false)),
                            Role::TextInput,
                            format!("Edit {}", line.text),
                        )
                        .into_any_element(),
                    );
                    continue;
                }
            }
            let (glyph, word) = match line.kind {
                Kind::Asked => ("message-circle-question", "Asked"),
                Kind::Learnt => ("repeat", "Learnt"),
                Kind::Saved => ("message-square-quote", "Saved"),
            };
            let text = line.text.clone();
            let name: SharedString = text.clone().into();
            let menu_line = line.clone();
            let key_line = line.clone();
            let press = text.clone();
            chips.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("quick-{at}")))
                        .tab_index(0)
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .h(look.length(token::H_CONTROL))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .bg(look.color(token::BG_RAISED))
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .hover(|style| style.border_color(look.color(token::BORDER_DEFAULT)).bg(look.color(token::BG_HOVER)))
                        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                        .child(
                            div().id(SharedString::from(format!("quick-kind-{at}"))).flex_none().tooltip(tip(word.into())).child(
                                svg().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)),
                            ),
                        )
                        .child(text.clone())
                        .when(line.pinned, |chip| {
                            chip.child(
                                div().id(SharedString::from(format!("quick-pin-{at}"))).flex_none().tooltip(tip("Pinned".into())).child(
                                    svg().path(icon_path("arrow-up-to-line")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)),
                                ),
                            )
                        })
                        .when(at < 9, |chip| {
                            chip.child(div().ml(look.length(token::SPACE_1)).text_color(look.color(token::FG_SUBTLE)).child(format!("\u{2318}{}", at + 1)))
                        })
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            this.press_line(press.clone(), event.modifiers().shift, window, cx);
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                this.open_line_menu(menu_line.clone(), event.position, false, window, cx);
                            }),
                        )
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            let keys = &event.keystroke;
                            let menu_key = keys.key == "menu" || (keys.key == "f10" && keys.modifiers.shift);
                            if menu_key {
                                if let Some(at) = drawn(window, cx, Role::Button, &key_line.text).map(below_left) {
                                    this.open_line_menu(key_line.clone(), at, true, window, cx);
                                    cx.stop_propagation();
                                }
                            } else if keys.key == "enter" || keys.key == "space" {
                                this.press_line(key_line.text.clone(), keys.modifiers.shift, window, cx);
                                cx.stop_propagation();
                            }
                        })),
                    Role::Button,
                    name,
                )
                .into_any_element(),
            );
        }
        let empty = self.kept(cx).text.trim().is_empty();
        let save = labelled(
            icon_button(look, "quick-save", "message-square-quote", "Save what is typed as a quick reply".into(), false, empty)
                .when(empty, |button| button.opacity(0.4))
                .when(!empty, |button| button.on_click(cx.listener(|this, _, window, cx| this.save_typed(window, cx)))),
            Role::Button,
            "Save as quick reply",
        );
        let hide = labelled(
            icon_button(look, "quick-hide", "eye-off", "Hide quick replies".into(), false, false).on_click(cx.listener(
                |this, event: &ClickEvent, window, cx| {
                    let at = drawn(window, cx, Role::Button, "Hide quick replies").map(|bounds| point(bounds.origin.x + bounds.size.width, bounds.origin.y)).unwrap_or(event.position());
                    this.open_hide_menu(at, event.is_keyboard(), window, cx);
                },
            )),
            Role::Button,
            "Hide quick replies",
        );
        Some(
            labelled(
                div()
                    .id("quick")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .pl(look.length(token::SPACE_4))
                    .pr(look.length(token::SPACE_2))
                    .pt(look.length(token::SPACE_2))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| this.open_hide_menu(event.position, false, window, cx)),
                    )
                    .child(div().id("quick-chips").flex_1().min_w_0().flex().items_center().gap(look.length(token::SPACE_1)).overflow_x_scroll().children(chips))
                    .child(save)
                    .child(hide),
                Role::Group,
                "Quick replies",
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick() -> Quick {
        let look = ipc::decode::<SettingValue>(
            "look",
            br#"{"learn":true,"quickOff":false,"quickOffFor":["gone"],"forgotten":["old"],"quick":[
              {"id":"a","text":"Later","kind":"saved"},{"id":"b","text":"First","kind":"saved","pinned":true}]}"#,
        )
        .expect("a look");
        let mut quick = Quick::mock(look, SessionQuick { asked: vec!["Yes".into()], learnt: vec!["old".into(), "Often".into(), "Later".into()] });
        quick.asked.insert("job".into(), vec!["Yes".into()]);
        quick
    }

    #[test]
    fn the_bar_draws_fleets_answers_then_pinned_lines_then_the_rest_then_learnt_ones_he_did_not_drop() {
        let texts: Vec<_> = quick().lines("job").into_iter().map(|one| (one.text, one.kind)).collect();
        assert_eq!(
            texts,
            [("Yes".to_string(), Kind::Asked), ("First".into(), Kind::Saved), ("Later".into(), Kind::Saved), ("Often".into(), Kind::Learnt)]
        );
    }

    #[test]
    fn a_job_the_bar_is_hidden_for_draws_none() {
        assert!(quick().shown_in("job"));
        assert!(!quick().shown_in("gone"));
    }

    #[test]
    fn lines_written_back_keep_their_order_and_their_pin() {
        let mut quick = quick();
        let mut lines = quick.kept();
        lines.swap(0, 1);
        quick.set_kept(lines);
        let kept = quick.kept();
        assert_eq!((kept[0].text.as_str(), kept[0].pinned), ("First", true));
        assert_eq!((kept[1].text.as_str(), kept[1].pinned), ("Later", false));
    }
}

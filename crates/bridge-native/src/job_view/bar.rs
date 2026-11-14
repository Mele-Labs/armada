//! The bar under an agent's message: Copy, Reply and the quick responses (A2.3a), and the editor a
//! response's menu opens.

use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, ClipboardItem, Context, Entity, FocusHandle, FontWeight, Role, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};

use super::acts::{button, Tone};
use super::composer::Kept;
use super::quick::{Response, MAX_RESPONSES};
use super::rows::{glyph, tip, Size};
use super::{JobView, Page};
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// How long Copy reads "Copied" after a press.
const COPIED_FOR: Duration = Duration::from_millis(1500);
/// The words of a message as plain text: formatting dropped, a line's words joined by spaces.
pub(super) fn plain_of(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let mut line = line.trim();
        line = line.trim_start_matches('#').trim_start_matches('>').trim();
        for marker in ["- ", "* ", "+ "] {
            if let Some(rest) = line.strip_prefix(marker) {
                line = rest.trim();
            }
        }
        if !out.is_empty() && !line.is_empty() {
            out.push(' ');
        }
        out.push_str(line);
    }
    let mut plain = String::new();
    let mut chars = out.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '`' | '*' => {}
            '[' => {
                let label: String = chars.by_ref().take_while(|&c| c != ']').collect();
                if chars.peek() == Some(&'(') {
                    chars.by_ref().take_while(|&c| c != ')').for_each(drop);
                }
                plain.push_str(&label);
            }
            _ => plain.push(ch),
        }
    }
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The line and the emoji a sent message answers with, where it is a response (`readResponse`): the line
/// quoted, a blank line, then one emoji.
pub(super) fn read_response(message: &str) -> Option<(&str, &str)> {
    let (line, emoji) = message.strip_prefix("> ")?.split_once("\n\n")?;
    let mut chars = emoji.chars();
    let first = chars.next()?;
    let one = !line.is_empty() && !line.contains('\n') && !first.is_ascii() && chars.all(|c| matches!(c, '\u{200d}' | '\u{fe0f}' | '\u{20e3}') || !c.is_ascii());
    one.then_some((line, emoji))
}

/// The emoji the person already answered the agent message `id` with, from their own messages: a mark
/// outlives the thread being drawn again. An answer is credited to the latest agent message before it whose
/// words hold the quoted line, else to the latest agent message.
pub(super) fn marked_in(items: &[ipc::ThreadItem], id: &str, plain: &str) -> Option<String> {
    let mut agents: Vec<(&str, String)> = Vec::new();
    let mut marked = None;
    for item in items {
        let ipc::ThreadItem::Row { row: ipc::SessionRow::Message { id: row_id, from, text, .. } } = item else { continue };
        match from {
            ipc::SessionVoice::You => {
                let Some((line, emoji)) = read_response(text) else { continue };
                let holder = agents.iter().rev().find(|(_, said)| said.contains(line)).or(agents.last());
                if holder.is_some_and(|(held, _)| *held == id) && line == plain {
                    marked = Some(emoji.to_string());
                }
            }
            ipc::SessionVoice::Agent => agents.push((row_id.as_str(), plain_of(text))),
            _ => {}
        }
    }
    marked
}

/// The message a response sends: the words quoted, a blank line, then the emoji.
pub(super) fn response_message(line: &str, emoji: &str) -> String {
    format!("> {line}\n\n{emoji}")
}

/// The editor of one response: its emoji and its name.
pub(super) struct Editor {
    id: Option<String>,
    from: String,
    emoji: Entity<TextareaState>,
    name: Entity<TextareaState>,
    back: Option<FocusHandle>,
    /// Remove, Cancel and Save, which Tab goes round with the two fields.
    stops: [FocusHandle; 3],
    _enter: Vec<Subscription>,
}

/// What the bars of the thread share: which message was just copied and which holds a menu.
pub(super) struct Bar {
    copied: Option<String>,
    held: Option<String>,
    pub(super) editor: Option<Editor>,
    next: u32,
}

impl Bar {
    pub fn new() -> Self {
        Bar { copied: None, held: None, editor: None, next: 0 }
    }
}

impl JobView {
    /// Copy, Reply and the responses under the agent's message `id`, then its time.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn message_bar(&self, id: &str, text: &str, at: &ipc::Instant, page: &Page, look: &Look, now: DateTime<Local>, window: &Window, cx: &Context<Self>) -> AnyElement {
        let group = SharedString::from(format!("agent:{id}"));
        let live = self.shown.as_ref().is_some_and(|job| !self.closed(job, cx));
        let copied = self.bar.copied.as_deref() == Some(id);
        let mut focused = self.bar.held.as_deref() == Some(id);
        let mut reach = |key: &str| {
            let spot = page.keys.spot(key, cx);
            focused |= spot.focus.is_focused(window);
            spot
        };
        let ghost = |key: String, spot: &super::keys::Spot, said: SharedString, mark: AnyElement| {
            div()
                .id(SharedString::from(key))
                .track_focus(&spot.focus)
                .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded_full()
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip(said))
                .child(mark)
        };
        let said = text.to_string();
        let copy_spot = reach(&format!("copy:{id}"));
        let copy_name: SharedString = if copied { "Copied" } else { "Copy message" }.into();
        let copy = labelled(
            ghost(format!("copy:{id}"), &copy_spot, copy_name.clone(), glyph(look, if copied { "check" } else { "copy" }, Size::Large, look.color(token::FG_SUBTLE)).into_any_element()).on_click(cx.listener({
                let (said, id) = (said.clone(), id.to_string());
                move |this, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(said.clone()));
                    this.bar.copied = Some(id.clone());
                    cx.notify();
                    let id = id.clone();
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(COPIED_FOR).await;
                        this.update(cx, |this, cx| {
                            if this.bar.copied.as_deref() == Some(id.as_str()) {
                                this.bar.copied = None;
                                cx.notify();
                            }
                        })
                        .ok();
                    })
                    .detach();
                }
            })),
            Role::Button,
            copy_name,
        );
        let reply = live.then(|| {
            let spot = reach(&format!("reply:{id}"));
            labelled(
                ghost(format!("reply:{id}"), &spot, "Reply".into(), glyph(look, "reply", Size::Large, look.color(token::FG_SUBTLE)).into_any_element()).on_click(cx.listener({
                    let said = said.clone();
                    move |this, _, window, cx| this.quote(&said, window, cx)
                })),
                Role::Button,
                "Reply",
            )
        });
        let plain = plain_of(text);
        let marked = self.shown.as_ref().and_then(|job| self.thread(job, cx)).and_then(|held| marked_in(&held.items, id, &plain));
        let responses: Vec<AnyElement> = if live {
            self.composer
                .quick
                .responses()
                .iter()
                .map(|response| {
                    let key = format!("response:{id}:{}", response.id);
                    let spot = reach(&key);
                    let pressed = marked.as_deref() == Some(response.emoji.as_str());
                    let mark = div().text_size(look.length(token::TEXT_SM)).opacity(if pressed { 1. } else { 0.7 }).hover(|style| style.opacity(1.)).child(SharedString::from(response.emoji.clone())).into_any_element();
                    let press = {
                        let (plain, response) = (plain.clone(), response.clone());
                        cx.listener(move |this, _, window, cx| this.respond(&plain, &response, window, cx))
                    };
                    let button = ghost(key, &spot, response.name.clone().into(), mark).when(pressed, |button| button.bg(look.color(token::ACCENT_FAINT))).aria_toggled(if pressed { gpui::Toggled::True } else { gpui::Toggled::False }).on_click(press);
                    let (count, chosen, row) = (self.composer.quick.responses().len(), response.clone(), id.to_string());
                    let button = self.menu_on(
                        button,
                        &spot,
                        format!("Quick response {}", response.name),
                        spot.focus.clone(),
                        Rc::new(move || response_entries(count)),
                        Rc::new(move |this, pick, window, cx| this.response_act(&row, &chosen, pick, window, cx)),
                        cx,
                    );
                    labelled(button, Role::Button, response.name.clone()).into_any_element()
                })
                .collect()
        } else {
            Vec::new()
        };
        labelled(
            div()
                .id(SharedString::from(format!("message-bar:{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .ml(-look.length(token::SPACE_2))
                .opacity(if focused { 1. } else { 0. })
                .group_hover(group, |style| style.opacity(1.))
                .child(copy)
                .children(reply)
                .children(responses)
                .child(div().ml(look.length(token::SPACE_2)).child(self.stamp(at, look, now, false, cx))),
            Role::Toolbar,
            "Message",
        )
        .into_any_element()
    }

    /// A response pressed: it sends the quoted words and its emoji, or puts the quote in the box.
    fn respond(&mut self, plain: &str, response: &Response, window: &mut Window, cx: &mut Context<Self>) {
        if response.ask {
            self.quote(plain, window, cx);
            return;
        }
        let message = response_message(plain, &response.emoji);
        let held = self.kept(cx);
        self.set_box(&Kept { text: message, tags: Vec::new() }, window, cx);
        self.send(window, cx);
        if !held.text.trim().is_empty() {
            self.set_box(&held, window, cx);
            self.typed_now(cx);
        }
    }

    /// The responses changed: they are kept in the thread's look beside the Aa panel's own, one copy and one write.
    fn responses_set(&mut self, list: Vec<Response>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.composer.quick.loaded() {
            return;
        }
        let was = self.composer.quick.look.clone();
        self.composer.quick.set_responses(list);
        self.write_look(was, window, cx);
    }

    /// A pick from a response's menu: edit it, add another, or take it away.
    fn response_act(&mut self, row: &str, response: &Response, pick: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        match pick {
            "edit" => self.open_editor(row, Some(response.clone()), window, cx),
            "add" => self.open_editor(row, None, window, cx),
            "remove" => {
                let mut list = self.composer.quick.responses();
                if list.len() > 1 {
                    list.retain(|one| one.id != response.id);
                    self.responses_set(list, window, cx);
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn open_editor(&mut self, row: &str, edit: Option<Response>, window: &mut Window, cx: &mut Context<Self>) {
        let back = window.focused(cx);
        let line = |text: String, window: &mut Window, cx: &mut Context<Self>| {
            let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 1).submit_on_enter(true));
            area.update(cx, |area, cx| area.set_value(text, window, cx));
            area
        };
        let emoji = line(edit.as_ref().map(|one| one.emoji.clone()).unwrap_or_default(), window, cx);
        let name = line(edit.as_ref().map(|one| one.name.clone()).unwrap_or_default(), window, cx);
        let enter = [&emoji, &name].map(|area| {
            cx.subscribe_in(area, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.save_editor(window, cx);
                }
            })
        });
        let from = edit.as_ref().map_or_else(|| "Add response".to_string(), |one| format!("Edit {}", one.name));
        let first = emoji.clone();
        self.bar.held = Some(row.to_string());
        self.bar.editor = Some(Editor { id: edit.map(|one| one.id), from, emoji, name, back, stops: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle()], _enter: enter.into() });
        first.update(cx, |area, cx| area.focus(window, cx));
        cx.notify();
    }

    fn close_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.bar.held = None;
        if let Some(back) = self.bar.editor.take().and_then(|editor| editor.back) {
            window.focus(&back, cx);
        }
        cx.notify();
    }

    fn save_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.bar.editor.as_ref() else { return };
        let (emoji, name) = (editor.emoji.read(cx).value().trim().to_string(), editor.name.read(cx).value().trim().to_string());
        if emoji.is_empty() || name.is_empty() {
            return;
        }
        let mut list = self.composer.quick.responses();
        match editor.id.clone() {
            Some(id) => {
                if let Some(one) = list.iter_mut().find(|one| one.id == id) {
                    one.emoji = emoji;
                    one.name = name;
                }
            }
            None => {
                if list.len() < MAX_RESPONSES {
                    // A number no response holds, so a saved one is never taken twice.
                    let id = loop {
                        self.bar.next += 1;
                        let id = format!("response:{}", self.bar.next);
                        if !list.iter().any(|one| one.id == id) {
                            break id;
                        }
                    };
                    list.push(Response { id, emoji, name, ask: false });
                }
            }
        }
        self.responses_set(list, window, cx);
        self.close_editor(window, cx);
    }

    fn remove_in_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.bar.editor.as_ref().and_then(|editor| editor.id.clone()) {
            let mut list = self.composer.quick.responses();
            if list.len() > 1 {
                list.retain(|one| one.id != id);
                self.responses_set(list, window, cx);
            }
        }
        self.close_editor(window, cx);
    }

    /// The editor a response's menu opens, over the Job's area.
    pub(super) fn render_editor(&self, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let editor = self.bar.editor.as_ref()?;
        let look = cx.look();
        let field = |name: &'static str, area: &Entity<TextareaState>| {
            labelled(
                div()
                    .id(SharedString::from(format!("response-field:{name}")))
                    .key_context(crate::keymap::TEXT_BOX)
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(Textarea::new(area).appearance(false).aria_label(name)),
                Role::TextInput,
                name,
            )
        };
        let removable = editor.id.is_some() && self.composer.quick.responses().len() > 1;
        let title = editor.from.clone();
        // The scrim is the whole window's, as Electron's body portal is.
        Some(gpui::deferred(gpui::anchored().anchor(gpui::Anchor::TopLeft).position(crate::room::of(window, cx).origin).child(
            div()
                .id("response-editor-scrim")
                .w(crate::room::of(window, cx).size.width)
                .h(crate::room::of(window, cx).size.height)
                .flex()
                .items_center()
                .justify_center()
                .bg(look.color(token::BG_BASE).opacity(0.6))
                .occlude()
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.close_editor(window, cx);
                    } else if event.keystroke.key == "tab" {
                        // Tab goes round the dialog: its two fields, then Remove where drawn, Cancel and Save.
                        if let Some(editor) = this.bar.editor.as_ref() {
                            let removable = editor.id.is_some() && this.composer.quick.responses().len() > 1;
                            let mut order = vec![gpui::Focusable::focus_handle(editor.emoji.read(cx), cx), gpui::Focusable::focus_handle(editor.name.read(cx), cx)];
                            order.extend(removable.then(|| editor.stops[0].clone()));
                            order.extend([editor.stops[1].clone(), editor.stops[2].clone()]);
                            super::pop::trap_tab(&order, event.keystroke.modifiers.shift, window, cx);
                        }
                    }
                }))
                .child(labelled(
                    div()
                        .id("response-editor")
                        .w(look.length(token::SPACE_12) * 6.)
                        .max_w_full()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_3))
                        .p(look.length(token::SPACE_6))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_OVERLAY))
                        .shadow(look.shadows(token::SHADOW_OVERLAY))
                        .child(div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title.clone())))
                        .child(field("Emoji", &editor.emoji))
                        .child(field("Name", &editor.name))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap(look.length(token::SPACE_2))
                                .children(removable.then(|| {
                                    labelled(
                                        button(&look, "response-remove", "Remove", Tone::Ghost, false, None, true).track_focus(&editor.stops[0]).on_click(cx.listener(|this, _, window, cx| this.remove_in_editor(window, cx))),
                                        Role::Button,
                                        "Remove",
                                    )
                                }))
                                .child(labelled(button(&look, "response-cancel", "Cancel", Tone::Ghost, false, None, true).track_focus(&editor.stops[1]).on_click(cx.listener(|this, _, window, cx| this.close_editor(window, cx))), Role::Button, "Cancel"))
                                .child(labelled(button(&look, "response-save", "Save", Tone::Primary, false, None, true).track_focus(&editor.stops[2]).on_click(cx.listener(|this, _, window, cx| this.save_editor(window, cx))), Role::Button, "Save")),
                        ),
                    Role::Dialog,
                    title,
                ))
        )).with_priority(3).into_any_element())
    }
}

/// A response's menu: Edit, Add response, then Remove.
fn response_entries(count: usize) -> Vec<Entry> {
    let add = if count >= MAX_RESPONSES {
        Entry::Off { label: "Add response".into(), hint: "Six at most" }
    } else {
        Entry::Act { id: "add", label: "Add response".into(), icon: None, danger: false }
    };
    let remove = if count <= 1 {
        Entry::Off { label: "Remove".into(), hint: "One at least" }
    } else {
        Entry::Act { id: "remove", label: "Remove".into(), icon: None, danger: true }
    };
    vec![Entry::Act { id: "edit", label: "Edit".into(), icon: None, danger: false }, add, Entry::Rule, remove]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_reads_as_plain_words_on_one_line() {
        assert_eq!(plain_of("## Plan\n\n- **Read** the `chip` stories\n- see [the docs](https://x.y)"), "Plan Read the chip stories see the docs");
    }

    #[test]
    fn a_response_quotes_the_words_then_the_emoji() {
        assert_eq!(response_message("Go ahead", "\u{1F44D}"), "> Go ahead\n\n\u{1F44D}");
    }

    #[test]
    fn the_menu_disables_what_the_list_cannot_take() {
        assert!(matches!(response_entries(6)[1], Entry::Off { .. }));
        assert!(matches!(response_entries(1)[3], Entry::Off { .. }));
        assert!(matches!(response_entries(3)[1], Entry::Act { id: "add", .. }));
    }

    #[test]
    fn the_defaults_are_the_four_the_owner_asked_for() {
        let names: Vec<_> = Response::defaults().into_iter().map(|one| one.name).collect();
        assert_eq!(names, ["Pick this", "Yes to this", "No to this", "Ask about this"]);
    }

    #[test]
    fn a_sent_response_reads_back_as_its_line_and_emoji() {
        assert_eq!(read_response(&response_message("Rename the chip", "\u{1F44D}")), Some(("Rename the chip", "\u{1F44D}")));
        assert_eq!(read_response("> Rename the chip\n\nyes"), None);
        assert_eq!(read_response("Rename the chip"), None);
    }

    #[test]
    fn responses_survive_a_round_trip_through_the_saved_look() {
        let mut kept = Response::defaults();
        kept[1].name = "Looks right".into();
        let mut quick = super::super::quick::Quick::default();
        quick.set_responses(kept.clone());
        assert_eq!(quick.responses(), kept);
    }
}

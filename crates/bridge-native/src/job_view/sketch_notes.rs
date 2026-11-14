//! His notes on a sketch read on its own (`ViewerNotes` on `SketchScene`, in `SketchReading`). A right press on a part, or
//! Shift+F10 on the one picked, offers Add note. The note is held with a number standing on the part, and ⌘↵ sends every held
//! note to the Job as one message, numbered, each naming its sketch and its parts: **a note is a message to the Job, never part
//! of an answer**, as Classic sends it. A part is a node, or an element inside a mock frame.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, MouseButton, Pixels, Point, Role, SharedString, Window};
use gpui_component::input::Enter;

use super::acts::{button, Mode, Tone, SHEET};
use super::rows::tip;
use super::sketch_read::Reading;
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::scene::{Mark, Marks};

pub(super) use crate::scene::Part;

/// A note held: what it is about, and what he wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Held {
    pub parts: Vec<Part>,
    pub text: String,
}

/// The note being written: the box it is in, and what it is about.
#[derive(Clone, Debug)]
pub(super) struct Writing {
    pub key: String,
    pub parts: Vec<Part>,
}

/// The notes as the Job reads them, one numbered block each (`notesMessage`): what it is about on the number's line, then the
/// note under it, indented past the number.
pub(super) fn notes_message(title: &str, notes: &[Held]) -> String {
    notes
        .iter()
        .enumerate()
        .map(|(at, note)| {
            let n = at + 1;
            let parts = note.parts.iter().map(|part| format!("{} ({})", part.id, part.label)).collect::<Vec<_>>().join(", ");
            let heading = format!("{n}. Sketch \"{title}\", {} {parts}", if note.parts.len() == 1 { "part" } else { "parts" });
            let indent = " ".repeat(n.to_string().len() + 2);
            let body = note.text.trim().lines().map(|line| if line.is_empty() { String::new() } else { format!("{indent}{line}") });
            std::iter::once(heading).chain(body).collect::<Vec<_>>().join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// What a note is about, in words: `Note on <part>`, or the parts together.
fn said(parts: &[Part]) -> String {
    parts.iter().map(|part| part.label.as_str()).collect::<Vec<_>>().join(", ")
}

impl JobView {
    /// Where his notes on the sketch read are held, `sketch:<job>:<card>` (`SketchBody`'s scope).
    fn notes_scope(&self) -> Option<String> {
        let reading = self.sketch_reading.as_ref()?;
        Some(format!("sketch:{}:{}", self.shown.as_ref()?, reading.card))
    }

    pub(super) fn held_on_reading(&self) -> &[Held] {
        self.notes_scope().and_then(|scope| self.sketch_notes.get(&scope)).map_or(&[], Vec::as_slice)
    }

    /// The key the drawing of this reading is kept by, which `scene::part_at` and `nodes` ask for.
    pub(super) fn drawn_key(reading: &Reading) -> String {
        if reading.changes && reading.has_against() {
            format!("{}:changes", reading.key)
        } else {
            reading.key.clone()
        }
    }

    /// The numbers standing on the sketch: each held note's, the one being written, and the part picked ringed.
    pub(super) fn marks_of(&self, reading: &Reading) -> Rc<Marks> {
        let held = self.held_on_reading();
        let mut notes: Vec<Mark> = held.iter().enumerate().map(|(at, one)| Mark { n: at + 1, writing: false, ids: one.parts.iter().map(|part| part.id.clone()).collect() }).collect();
        if let Some(writing) = &reading.writing {
            notes.push(Mark { n: held.len() + 1, writing: true, ids: writing.parts.iter().map(|part| part.id.clone()).collect() });
        }
        Rc::new(Marks { notes, picked: reading.target.iter().map(|part| part.id.clone()).collect() })
    }

    /// A right press, or Shift+F10: the menu at `at`, about `part` where there is one. A part is picked as it is pressed.
    pub(super) fn note_menu(&mut self, at: Point<Pixels>, part: Option<Part>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(reading) = self.sketch_reading.as_mut() else { return };
        if reading.earlier {
            return;
        }
        if part.is_some() {
            reading.target = part;
        }
        let about = reading.target.is_some();
        let back = reading.focus.clone();
        let mut entries = Vec::new();
        if about {
            entries.push(Entry::Act { id: "note", label: "Add note".into(), icon: Some("message-square-plus"), danger: false });
        }
        if !self.held_on_reading().is_empty() {
            entries.push(Entry::Item { id: "send-notes", label: "Send notes".into(), icon: Some("send"), danger: false, key: Some("\u{2318}\u{21b5}".into()), off: None });
        }
        if entries.is_empty() {
            cx.notify();
            return;
        }
        self.open_button_menu(
            "Note",
            entries,
            at,
            Some(back),
            |this, id, window, cx| match id {
                "note" => this.begin_note(window, cx),
                "send-notes" => this.send_sketch_notes(window, cx),
                _ => {}
            },
            window,
            cx,
        );
    }

    /// A press that did not travel: the part under it is picked.
    pub(super) fn pick_part(&mut self, part: Option<Part>, cx: &mut Context<Self>) {
        if let Some(reading) = self.sketch_reading.as_mut() {
            reading.target = part;
        }
        cx.notify();
    }

    /// Tab and Shift+Tab walk the nodes of the sketch: the next one is picked, and a note is about it.
    pub(super) fn walk_parts(&mut self, back: bool, cx: &mut Context<Self>) {
        let Some(reading) = self.sketch_reading.as_mut() else { return };
        let all = crate::scene::nodes(&Self::drawn_key(reading));
        if all.is_empty() {
            return;
        }
        let at = reading.target.as_ref().and_then(|part| all.iter().position(|(one, _)| one.id == part.id));
        let next = match (at, back) {
            (None, false) => 0,
            (None, true) => all.len() - 1,
            (Some(at), false) => (at + 1) % all.len(),
            (Some(at), true) => (at + all.len() - 1) % all.len(),
        };
        reading.target = Some(all[next].0.clone());
        cx.notify();
    }

    /// Shift+F10: the menu at the foot of the part picked.
    pub(super) fn note_menu_by_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(reading) = self.sketch_reading.as_ref() else { return };
        let at = reading
            .target
            .as_ref()
            .and_then(|part| crate::scene::bounds_of(&Self::drawn_key(reading), &part.id))
            .map_or_else(|| window.mouse_position(), |bounds| gpui::point(bounds.left(), bounds.bottom()));
        self.note_menu(at, None, window, cx);
    }

    fn note_box_key(reading: &Reading) -> String {
        format!("note:{}", reading.answer)
    }

    /// Add note: the box opens under the part, with the words he left in it before.
    fn begin_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(reading) = self.sketch_reading.as_mut() else { return };
        let Some(part) = reading.target.clone() else { return };
        let key = Self::note_box_key(reading);
        reading.writing = Some(Writing { key: key.clone(), parts: vec![part] });
        self.open_box(SHEET, &key, Mode::Note, "", window, cx);
    }

    /// Hold (Enter): the note is kept with its number standing on the part, and the box shuts.
    pub(super) fn hold_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(scope), Some(reading)) = (self.notes_scope(), self.sketch_reading.as_ref()) else { return };
        let Some(writing) = reading.writing.clone() else { return };
        let text = self.typed_in(&writing.key, Mode::Note, cx).trim().to_string();
        if text.is_empty() {
            return;
        }
        self.sketch_notes.entry(scope).or_default().push(Held { parts: writing.parts, text });
        self.shut_note(window, cx);
    }

    /// The box shuts and the keys go back to the sketch; the words stay kept for the next open unless they were held.
    pub(super) fn shut_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(reading) = self.sketch_reading.as_mut() else { return };
        let Some(writing) = reading.writing.take() else { return };
        let focus = reading.focus.clone();
        // Held, the words go; shut with Esc they are kept for the next time (`keep`), as Classic's box keeps them.
        let held = self.held_on_reading().last().is_some_and(|one| one.parts == writing.parts && self.typed_in(&writing.key, Mode::Note, cx).trim() == one.text);
        if held {
            self.close_box(SHEET, &writing.key, cx);
        } else if let Some(cards) = self.cards_mut() {
            cards.modes.remove(&writing.key);
        }
        window.focus(&focus, cx);
        cx.notify();
    }

    fn drop_sketch_note(&mut self, at: usize, cx: &mut Context<Self>) {
        if let Some(scope) = self.notes_scope() {
            if let Some(held) = self.sketch_notes.get_mut(&scope).filter(|held| at < held.len()) {
                held.remove(at);
            }
        }
        cx.notify();
    }

    /// ⌘↵: every held note, and the one being written if it has words, goes to the Job as one message.
    pub(super) fn send_sketch_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(scope), Some(reading), Some(job)) = (self.notes_scope(), self.sketch_reading.as_ref(), self.shown.clone()) else { return };
        let title = reading.title.to_string();
        let writing = reading.writing.clone();
        let mut all = self.held_on_reading().to_vec();
        if let Some(writing) = &writing {
            let text = self.typed_in(&writing.key, Mode::Note, cx).trim().to_string();
            if !text.is_empty() {
                all.push(Held { parts: writing.parts.clone(), text });
            }
        }
        if all.is_empty() {
            return;
        }
        let text = notes_message(&title, &all);
        let row = Self::note_box_key(reading);
        self.send_to_job(job.to_string(), text, row, window, cx, move |this, window, cx| {
            this.sketch_notes.remove(&scope);
            if let Some(writing) = this.sketch_reading.as_mut().and_then(|reading| reading.writing.take()) {
                this.close_box(SHEET, &writing.key, cx);
            }
            if let Some(focus) = this.sketch_reading.as_ref().map(|reading| reading.focus.clone()) {
                window.focus(&focus, cx);
            }
            cx.notify();
        });
    }

    /// The box under the part it is about: its number and the part, the notes held before it, the words, and Hold and Send.
    pub(super) fn render_note_box(&self, reading: &Reading, room: gpui::Bounds<Pixels>, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let writing = reading.writing.as_ref()?;
        let held = self.held_on_reading();
        let about = said(&writing.parts);
        let anchor = writing.parts.first().and_then(|part| crate::scene::bounds_of(&Self::drawn_key(reading), &part.id));
        let key = writing.key.clone();
        let blank = self.typed_in(&key, Mode::Note, cx).trim().is_empty();
        let width = look.length(token::SPACE_12) * 6.;
        let gap = look.length(token::SPACE_2);
        // Under what it is about, else over it, else at the sheet's foot: always inside the sheet.
        let tall = look.length(token::SPACE_12) * 4.5 + look.length(token::SPACE_6) * held.len() as f32;
        let left = anchor.map_or(gap, |at| (at.left() - room.left()).max(gap).min((room.size.width - width - gap).max(gap)));
        let (below, above) = match anchor {
            Some(at) => (at.bottom() + gap + tall < room.bottom(), at.top() - gap - tall > room.top()),
            None => (true, false),
        };
        // `Some(top)` stands the box at that distance from the sheet's top, `None` at the foot.
        let place = match anchor {
            Some(at) if below => Some(at.bottom() - room.top() + gap),
            Some(at) if above => Some(at.top() - room.top() - gap - tall),
            Some(_) => None,
            None => Some(gap),
        };
        let number = held.len() + 1;
        let pin = |n: usize, writing: bool| {
            let (fill, ink) = if writing { (look.color(token::BG_OVERLAY), look.color(token::ACCENT)) } else { (look.color(token::ACCENT), look.color(token::FG_INVERSE)) };
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .min_w(look.length(token::LEADING_2XS))
                .h(look.length(token::LEADING_2XS))
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::ACCENT))
                .bg(fill)
                .text_color(ink)
                .font_family(super::rows::mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .child(n.to_string())
        };
        let list = (!held.is_empty()).then(|| {
            labelled(
                div()
                    .id("note-held")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .children(held.iter().enumerate().map(|(at, one)| {
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .child(pin(at + 1, false))
                            .child(div().flex_none().max_w(gpui::relative(0.35)).truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(said(&one.parts))))
                            .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one.text.clone())))
                            .child(labelled(
                                div()
                                    .id(SharedString::from(format!("note-remove:{at}")))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(look.length(token::H_CONTROL_SM))
                                    .rounded(look.length(token::RADIUS_MD))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                                    .tooltip(tip("Remove"))
                                    .child(super::rows::glyph(look, "x", super::rows::Size::Small, look.color(token::FG_MUTED)))
                                    .on_click(cx.listener(move |this, _, _, cx| this.drop_sketch_note(at, cx))),
                                Role::Button,
                                format!("Remove note {}", at + 1),
                            ))
                    })),
                Role::List,
                "Held notes",
            )
        });
        let hold = labelled(
            button(look, "note-hold", "Hold", Tone::Secondary, false, Some("\u{21b5}"), !blank).when(!blank, |it| it.on_click(cx.listener(|this, _, window, cx| this.hold_note(window, cx)))),
            Role::Button,
            "Hold",
        );
        let send = labelled(
            button(look, "note-send", "Send", Tone::Primary, false, Some("\u{2318}\u{21b5}"), !blank || !held.is_empty())
                .when(!blank || !held.is_empty(), |it| it.on_click(cx.listener(|this, _, window, cx| this.send_sketch_notes(window, cx)))),
            Role::Button,
            "Send",
        );
        // ⌘↵ in the box sends, taken before the text area puts a line break in.
        if let Some(page) = self.shown.as_ref().and_then(|job| self.pages.get(job)) {
            page.keys.commit(&key, Some(Rc::new(|this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.send_sketch_notes(window, cx))));
        }
        let refused = self.shown.as_ref().and_then(|job| self.pages.get(job)).and_then(|page| page.cards.said.get(&key).cloned()).map(|why| super::acts::refusal(look, &key, why));
        let card = div()
            .id("note-box")
            .occlude()
            .absolute()
            .left(left)
            .when_some(place, |it, top| it.top(top))
            .when(place.is_none(), |it| it.bottom(gap))
            .w(width)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_STRONG))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            // Enter holds the note and Esc shuts the box, before the text area or the sheet takes them.
            .capture_action(cx.listener(move |this, action: &Enter, window, cx| {
                if !action.secondary && !action.shift {
                    cx.stop_propagation();
                    this.hold_note(window, cx);
                }
            }))
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    cx.stop_propagation();
                    this.shut_note(window, cx);
                }
            }))
            .children(list)
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(pin(number, true)).child(div().min_w_0().truncate().child(SharedString::from(about.clone()))))
            .children(self.box_of(&key, Mode::Note, "Note", look, cx))
            .children(refused)
            .child(div().flex().items_center().justify_end().gap(look.length(token::SPACE_2)).child(hold).child(send));
        Some(labelled(card, Role::Dialog, format!("Note on {about}")).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(parts: &[(&str, &str)], text: &str) -> Held {
        Held { parts: parts.iter().map(|(id, label)| Part { id: id.to_string(), label: label.to_string() }).collect(), text: text.into() }
    }

    #[test]
    fn a_note_names_its_sketch_and_the_part_by_id_and_label() {
        let one = held(&[("hook", "Pre-push hook")], "Keep the failed tone this red");
        assert_eq!(notes_message("Freshness chip", &[one]), "1. Sketch \"Freshness chip\", part hook (Pre-push hook)\n   Keep the failed tone this red");
    }

    #[test]
    fn notes_are_numbered_and_an_element_of_a_mock_is_its_node_and_key() {
        let all = [held(&[("mock-chip#body > span:nth-child(1)", "2 behind")], "Say how many"), held(&[("a", "A"), ("b", "B")], "One line\n\nand more")];
        assert_eq!(
            notes_message("Chip", &all),
            "1. Sketch \"Chip\", part mock-chip#body > span:nth-child(1) (2 behind)\n   Say how many\n\n2. Sketch \"Chip\", parts a (A), b (B)\n   One line\n\n   and more"
        );
    }

    #[test]
    fn a_note_is_held_on_an_element_id_and_the_numbers_stand_on_it() {
        let held_one = held(&[("m#chip", "chip")], "Round it");
        let mark = Mark { n: 1, writing: false, ids: held_one.parts.iter().map(|part| part.id.clone()).collect() };
        assert_eq!(mark.ids, ["m#chip"]);
    }
}

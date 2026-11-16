//! The In this build sheet drawn as the approved mock draws it: a header band, the notes down the left, the chosen
//! note's look, steps and feedback box on the right, and a band of keys under them. Every size and colour is a token.

use gpui::{div, prelude::*, AnyElement, Context, Div, Focusable, FontWeight, Role, SharedString, Window};
use gpui_component::input::Textarea;

use ipc::NoteVerdict;

use super::{looked, mark, send_label, short, step_lines, verdict, BuildNotesSheet, STARTS_WITH};
use crate::generated::actions::ActionScope;
use crate::keymap::{self, Act, TEXT_BOX};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, kbd, mono, sans, tip, Btn, Weight};

impl Render for BuildNotesSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let look = cx.look();
        let view = crate::room::of(window, cx).size;
        let width = (look.length(token::PALETTE_WIDTH) * 1.75).min(view.width - look.length(token::SPACE_6) * 2.);
        let top = (view.height * 0.1).max(look.length(token::H_TITLE_BAR));
        let height = view.height - top - look.length(token::SPACE_6) * 2.;
        let frame = div()
            .id("build-notes-frame")
            .key_context(ActionScope::ReviewSheet.key_context().unwrap_or_default())
            .track_focus(&self.frame)
            .on_action(cx.listener(|this, key: &Act, window, cx| this.on_act(key, window, cx)))
            // A press on the sheet is not a press on the scrim.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(width)
            .h(height)
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .child(self.head(look))
            .child(div().flex_1().min_h_0().flex().child(self.notes_column(window, cx)).child(self.pane(window, cx)))
            .child(self.keys(look));
        let scrim = div()
            .id("build-notes-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .bg(look.color(token::SCRIM))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)));
        let column = div().absolute().top(top).left((view.width - width) / 2.).child(frame);
        labelled(div().id("build-notes").absolute().inset_0().child(scrim).child(column), Role::Dialog, "In this build").into_any_element()
    }
}

fn band(look: &Look) -> Div {
    div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).px(look.length(token::SPACE_4)).py(look.length(token::SPACE_3)).bg(look.color(token::BG_RAISED))
}

impl BuildNotesSheet {
    fn head(&self, look: &Look) -> Div {
        let commit = self.held.as_ref().and_then(|held| held.commit.as_deref()).map(short).map(str::to_string);
        band(look)
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(glyph("eye", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)))
            .child(
                div()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child("In this build"),
            )
            .children(commit.map(|commit| {
                div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(commit)
            }))
            .child(div().flex_1())
            .child(kbd("Esc", look))
    }

    /// One row for each note: its state's glyph first, then the Job's title, then a mark where the Job has ended.
    fn notes_column(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let at = self.at();
        let focused = self.list.is_focused(window);
        let rows: Vec<AnyElement> = self
            .notes()
            .iter()
            .enumerate()
            .map(|(i, note)| {
                let seen = looked(note, &self.marked);
                let state = mark(seen, verdict(note, &self.marked));
                let chosen = at == Some(i);
                let ink = look.color(state.hue);
                let title: SharedString = note.title.clone().into();
                let row = div()
                    .id(("build-note", i))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    // The state's hue on the row's edge; the chosen row's edge is the accent.
                    .border_l(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(if chosen { look.color(token::ACCENT) } else { ink })
                    .when(chosen, |row| row.bg(look.color(token::ACCENT_MUTED)))
                    .when(!chosen, |row| row.hover(|style| style.bg(look.color(token::BG_HOVER))))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| this.select(i, window, cx)))
                    .child(labelled(
                        div()
                            .id(("build-note-state", i))
                            .flex_none()
                            .h(look.length(token::LEADING_SM))
                            .flex()
                            .items_center()
                            .tooltip(tip(state.said))
                            .child(glyph(state.glyph, look.length(token::SPACE_4), ink)),
                        Role::Image,
                        state.said,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .text_color(look.color(if seen { token::FG_SUBTLE } else { token::FG_DEFAULT }))
                            .when(chosen && focused, |text| text.font_weight(FontWeight(look.number(token::WEIGHT_HEADING))))
                            .child(title.clone()),
                    )
                    .children(note.job_ended.then(|| {
                        labelled(
                            div()
                                .id(("build-note-ended", i))
                                .flex_none()
                                .h(look.length(token::LEADING_SM))
                                .flex()
                                .items_center()
                                .tooltip(tip("Ended"))
                                .child(glyph("circle-x", look.length(token::SPACE_4), look.color(token::STATUS_KILLED))),
                            Role::Image,
                            "Ended",
                        )
                    }));
                labelled(row, Role::ListItem, title).into_any_element()
            })
            .collect();
        div()
            .id("build-notes-list")
            .key_context("list")
            .track_focus(&self.list)
            .flex_none()
            .w(look.length(token::SPACE_12) * 5. + look.length(token::SPACE_2))
            .overflow_y_scroll()
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::BG_SUNKEN))
            .border_r(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(div().id("build-notes-rows").flex().flex_col().children(rows), Role::List, "Notes"))
            .into_any_element()
    }

    /// The chosen note: its title, what to look at, its steps, and the box that answers it.
    fn pane(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let Some(note) = self.note() else { return div().id("build-note-pane").flex_1().into_any_element() };
        let ended = note.job_ended;
        let good = verdict(note, &self.marked) == Some(NoteVerdict::Good);
        let title: SharedString = note.title.clone().into();
        let heading = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(labelled(
                div()
                    .id("build-note-title")
                    .min_w_0()
                    .text_size(look.length(token::TEXT_LG))
                    .line_height(look.length(token::LEADING_LG))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(title.clone()),
                Role::Heading,
                title,
            ))
            .children(ended.then(|| {
                labelled(
                    div()
                        .id("build-note-ended-chip")
                        .flex_none()
                        .px(look.length(token::SPACE_2))
                        .rounded_full()
                        .bg(look.color(token::STATUS_KILLED_BG))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::STATUS_KILLED))
                        .child("Ended"),
                    Role::Status,
                    "Ended",
                )
            }));
        let look_line = note.look.clone().filter(|words| !words.trim().is_empty()).map(|words| {
            div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(words)
        });
        let steps: Vec<AnyElement> = step_lines(&note.steps)
            .into_iter()
            .map(|line| {
                let number = line.number;
                let go = line.go.map(|go| {
                    let name = format!("Go to step {number}");
                    let button =
                        Btn::new(("build-go", number), "Go", Weight::Primary).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.go(number, window, cx)));
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .children((!go.keycap.is_empty()).then(|| kbd(go.keycap.clone(), look)))
                        .child(labelled(button, Role::Button, name))
                });
                div()
                    .id(("build-step", number))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .child(
                        div()
                            .flex_none()
                            .size(look.length(token::SPACE_6))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(look.color(token::ACCENT_MUTED))
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_XS))
                            .child(number.to_string()),
                    )
                    .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(line.say))
                    .children(go)
                    .into_any_element()
            })
            .collect();
        let box_focused = self.feedback.focus_handle(cx).is_focused(window);
        let ring = look.color(token::ACCENT);
        let field = labelled(
            div()
                .id("build-note-feedback")
                .key_context(TEXT_BOX)
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if box_focused { ring } else { look.color(token::BORDER_DEFAULT) })
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .child(Textarea::new(&self.feedback).appearance(false).bordered(false)),
            Role::TextInput,
            "Feedback for this Job",
        );
        let worded = !self.feedback.read(cx).value().trim().is_empty();
        let label = send_label(ended);
        let send = Btn::new("build-note-send", label, Weight::Primary)
            .icon(if ended { "plus" } else { "play" })
            .pending(self.sending)
            .disabled(!worded)
            .draw(look, cx)
            .shadow(look.shadows(token::SHADOW_PRIMARY))
            .track_focus(&self.send_focus)
            .when(self.send_focus.is_focused(window), |button| button.border_color(look.color(token::FG_DEFAULT)))
            .when(worded && !self.sending, |button| button.on_click(cx.listener(|this, _, window, cx| this.send(window, cx))))
            .child(kbd(keymap::shown("send_message"), look));
        let marked = Btn::new("build-note-good", "Good", Weight::Secondary)
            .icon("check")
            .disabled(good)
            .draw(look, cx)
            .track_focus(&self.good_focus)
            .when(self.good_focus.is_focused(window), |button| button.border_color(ring))
            .when(!good, |button| button.on_click(cx.listener(|this, _, window, cx| this.judge(NoteVerdict::Good, window, cx))))
            .child(kbd(keymap::shown("build_note_looked"), look));
        // Never off: on a note already marked so, it still puts the keys in the box.
        let needs_work = Btn::new("build-note-needs-work", "Needs work", Weight::Secondary)
            .icon("circle-alert")
            .draw(look, cx)
            .track_focus(&self.needs_work_focus)
            .when(self.needs_work_focus.is_focused(window), |button| button.border_color(ring))
            .on_click(cx.listener(|this, _, window, cx| this.judge(NoteVerdict::NeedsWork, window, cx)))
            .child(kbd(keymap::shown("build_note_needs_work"), look));
        let open_job = Btn::new("build-note-open-job", "Open Job", Weight::Secondary)
            .icon("message-square")
            .draw(look, cx)
            .tooltip(tip(format!("Opens the Job's thread  {}", keymap::shown("build_note_open_job"))))
            .track_focus(&self.open_job_focus)
            .when(self.open_job_focus.is_focused(window), |button| button.border_color(ring))
            .on_click(cx.listener(|this, _, window, cx| this.open_job(window, cx)))
            .child(kbd(keymap::shown("build_note_open_job"), look));
        let refusal = self
            .refused
            .clone()
            .map(|why| labelled(div().id("build-note-refused").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why));
        let subtle = |words: &'static str| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(words);
        div()
            .id("build-note-pane")
            .flex_1()
            .min_w_0()
            .overflow_y_scroll()
            .p(look.length(token::SPACE_4))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .child(heading)
            .children(look_line)
            .children((!steps.is_empty()).then(|| labelled(div().id("build-steps").flex().flex_col().gap(look.length(token::SPACE_2)).children(steps), Role::List, "Steps")))
            .child(field)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(send, Role::Button, label)).child(labelled(marked, Role::Button, "Good")).child(labelled(needs_work, Role::Button, "Needs work")).child(labelled(open_job, Role::Button, "Open Job")))
                    .children(ended.then(|| subtle(STARTS_WITH)))
                    .children(refusal),
            )
            .into_any_element()
    }

    /// The band of keys under the sheet.
    fn keys(&self, look: &Look) -> Div {
        let one = |keys: String, said: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .child(kbd(keys, look))
                .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(said))
        };
        band(look)
            .gap(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(one("↑↓".into(), "Note"))
            .child(one(keymap::shown("build_note_step"), "Go to step"))
            .child(one(keymap::shown("send_message"), "Send"))
            .child(one(keymap::shown("build_note_looked"), "Good"))
            .child(one(keymap::shown("build_note_needs_work"), "Needs work"))
            .child(one(keymap::shown("build_note_open_job"), "Open Job"))
            .child(one("Esc".into(), "Close"))
    }
}

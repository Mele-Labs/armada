//! The dispatch sheet drawn (`DP/DispatchPanel.tsx`, `DP/dispatch-panel.css`). Sizes follow the
//! inventory's section 1.5, with the owner's 9 Nov tweak of more room: taller controls, wider
//! gaps and the larger radius, all from the same tokens the Jobs list uses.

use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, px, App, Bounds, Context, FocusHandle, KeyDownEvent, Pixels, Role, SharedString, Window,
};
use gpui_component::input::Textarea;

use super::face::{Glyph, Kind, Mentions, Part, Reading, Row};
use super::sheet::DispatchSheet;
use crate::job_view::ask_tools::{plus_button, ASK_FOR};
use crate::job_view::composer::{attach_button, tray};
use crate::job_view::sketch_pad::{drawn_tiles, sketch_button};
use crate::keymap::{self, Act};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::titlebar::parts::{glyph, kbd, mono, sans, spinner, tip, Btn, Weight};

impl Render for DispatchSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let view = crate::room::of(window, cx).size;
        let width = (look.length(token::PALETTE_WIDTH) * 1.75).min(view.width - look.length(token::SPACE_6));
        let top = view.height * 0.12;
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let focused = self.frame.contains_focused(window, cx);
        let edge = if self.asking {
            hue
        } else if focused {
            look.color(token::ACCENT)
        } else {
            look.color(token::BORDER_SUBTLE)
        };
        let ring = gpui::BoxShadow {
            color: edge.opacity(0.2),
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: look.length(token::BORDER_WIDTH) * 3.,
            inset: false,
        };
        let mut shadows = look.shadows(token::SHADOW_OVERLAY);
        if focused || self.asking {
            shadows.push(ring);
        }
        let here = self.here_card(window, cx);
        let here_room = if here.is_some() { look.length(token::H_CONTROL_LG) + look.length(token::SPACE_6) } else { px(0.) };
        let frame = div()
            .id("dispatch-frame")
            .key_context("dispatch")
            .track_focus(&self.frame)
            .on_action(cx.listener(|this, key: &Act, window, cx| this.on_act(key, window, cx)))
            // A press on the sheet is not a press on the scrim.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w_full()
            .max_h(view.height - top - look.length(token::SPACE_6) - here_room)
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(look.color(token::BG_SUNKEN))
            .shadow(shadows)
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .on_modifiers_changed(cx.listener(|this, event: &gpui::ModifiersChangedEvent, _, cx| crate::job_view::talk::modifiers(this, event, cx)))
            .children(self.wait_chip(window, cx))
            .child(
                // The request box: what is attached over the words, the words, and its tools under them. Files dropped anywhere on it attach.
                div()
                    .id("dispatch-box")
                    .flex()
                    .flex_col()
                    .on_drop(cx.listener(|this, paths: &gpui::ExternalPaths, _, cx| this.take_paths(paths.paths().to_vec(), cx)))
                    .children(self.tray(window, cx))
                    .child(self.line(window, cx))
                    .children(self.menu_view(cx))
                    .children(self.tool_list(cx))
                    .child(self.tools(window, cx)),
            )
            .child(self.drawer(window, cx));
        // A press on the scrim shuts the sheet.
        let scrim = div()
            .id("dispatch-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .bg(look.color(token::SCRIM))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)));
        // The card under the sheet stands apart from it, the same width, and moves with it.
        let column = div()
            .id("dispatch-column")
            .absolute()
            .top(top)
            .left((view.width - width) / 2.)
            .w(width)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(frame)
            .children(here);
        labelled(
            div()
                .id("dispatch")
                .absolute()
                .inset_0()
                .child(scrim)
                .child(column)
                .children(self.popup.as_ref().map(|popup| popup.dropdown.clone()))
                // Outside the frame, so the sheet's own keys stand aside while the pad has them.
                .children(self.pad.read(cx).is_open().then(|| self.pad.clone())),
            Role::Dialog,
            "Dispatch",
        )
    }
}

/// The focus ring: an outer ring set off from the control, as the notes layer draws it, in the control's own radius
/// plus the offset. It takes no press, and the control's own border stays as its state left it.
fn ringed(look: &Look, control: gpui::Stateful<gpui::Div>, on: bool, radius: Pixels) -> gpui::Stateful<gpui::Div> {
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let out = -(look.length(token::FOCUS_RING_WIDTH) + px(2.));
    control.relative().when(on, |control| {
        control.child(div().absolute().top(out).bottom(out).left(out).right(out).rounded(radius - out).border(look.length(token::FOCUS_RING_WIDTH)).border_color(ring))
    })
}

impl DispatchSheet {
    /// New Side Job in <project>: a card of its own under the sheet that starts what ⌘T starts. Tab from the
    /// sheet reaches it, and Enter or Space starts it. It names the repository the picker names, else the title bar's. None where the title bar names nowhere.
    fn here_card(&self, window: &Window, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let name = self.here_in()?;
        let look = cx.look();
        let focused = self.here_key.is_focused(window);
        let said = format!("New Side Job in {name}");
        let hover = look.color(token::BG_HOVER);
        let card = div()
            .id("dispatch-here")
            .track_focus(&self.here_key)
            .tab_stop(true)
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, window, cx| this.job_here(window, cx)))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let key = &event.keystroke;
                let plain = !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt && !key.modifiers.shift;
                match key.key.as_str() {
                    "enter" | "space" if plain => this.job_here(window, cx),
                    "escape" => this.close(window, cx),
                    // Tab goes on round the sheet: the card is its last stop.
                    "tab" if !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt => this.step_focus(key.modifiers.shift, window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .tooltip(tip("Starts a Side Job with no workflow and opens its thread"))
            .h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_4))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .cursor_pointer()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .hover(move |style| style.bg(hover))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .font_family(sans(look))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(glyph("square-terminal", look.length(token::SPACE_4), look.color(token::ACCENT)))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(said.clone())))
            .child(kbd("⌘T", look));
        Some(labelled(ringed(look, card, focused, look.length(token::RADIUS_LG)), Role::Button, said).into_any_element())
    }

    /// The sentence over the request: `Starts when [Job]'s [PR lands] · from [main] · needs [..]`.
    fn wait_chip(&self, window: &Window, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let wait = self.wait.as_ref()?;
        let look = cx.look();
        let off = !self.face.connected;
        let part = |id: &'static str, which: Part, at: usize, label: SharedString, name: String, icon: Option<&'static str>, cx: &Context<Self>| {
            let handle = self.parts[at].clone();
            let on = handle.is_focused(window);
            let by_hand = wait.his(which);
            let body = div()
                .id(id)
                .track_focus(&handle)
                .tab_stop(true)
                .h(look.length(token::H_CONTROL_SM) + look.length(token::SPACE_1))
                .max_w(look.length(token::PALETTE_WIDTH) * 0.6)
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if by_hand { token::ACCENT } else { token::BORDER_DEFAULT }))
                .bg(look.color(token::BG_SUNKEN))
                .text_color(look.color(token::FG_DEFAULT))
                .when(!off, |body| body.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                .when(off, |body| body.opacity(0.5))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.part_keys(which, event, window, cx)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if !this.face.connected {
                        return;
                    }
                    if which == Part::On {
                        this.wait_menu = if this.wait_menu.is_some() { None } else { Some(0) };
                    } else {
                        this.cycle(which, false, cx);
                    }
                    let focus = this.parts[at].clone();
                    window.focus(&focus, cx);
                    cx.notify();
                }))
                .tooltip(tip(wait.tip(which)))
                .children(icon.map(|name| glyph(name, look.length(token::SPACE_3), look.color(token::ACCENT))))
                .child(div().min_w_0().truncate().child(label));
            labelled(ringed(look, body, on, look.length(token::RADIUS_MD)), Role::Button, name)
        };
        let word = |text: &'static str| div().flex_none().child(text);
        let mut line = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(look.color(token::BG_RAISED))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(word("Starts when"))
            .child(part("wait-on", Part::On, 0, wait.on.title.clone(), format!("Job {}", wait.on.title), Some("square-terminal"), cx));
        if wait.when.value != "ends" {
            // The possessive sits against the title: it takes back the gap before it.
            line = line.child(word("'s").ml(-look.length(token::SPACE_3)));
        }
        line = line
            .child(part("wait-when", Part::When, 1, wait.when.label.clone(), format!("Condition {}", wait.when.label), None, cx))
            .child(word("·"))
            .child(word("from"))
            .child(part("wait-base", Part::Base, 2, wait.base.label.clone(), format!("Starts from {}", wait.base.label), None, cx));
        if let Some(needs) = &wait.needs {
            line = line
                .child(word("·"))
                .child(word("needs"))
                .child(part("wait-needs", Part::Needs, 3, needs.label.clone(), format!("Needs {}", needs.label), None, cx));
        }
        let menu = self.wait_menu.map(|active| {
            let rows: Vec<Row> = wait
                .others
                .iter()
                .map(|one| Row {
                    glyph: Glyph::Job,
                    name: one.title.clone(),
                    says: None,
                    standing: None,
                    insert: one.title.clone(),
                    feature: None,
                })
                .collect();
            listbox("Jobs", &rows, active, look, cx.listener(|this, at: &usize, _, cx| this.wait_on(*at, cx)))
        });
        Some(labelled(div().id("wait").flex().flex_col().child(line).children(menu), Role::Group, "Start condition").into_any_element())
    }

    /// The line: the kind's glyph, the words, `⇥` and the key that opens the sheet.
    fn line(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let mark = look.length(token::SPACE_4) + look.length(token::SPACE_1);
        let words = self.words(cx);
        let tag_range = self.tagged.as_ref().and_then(|(_, title)| {
            let needle = format!("@{title}");
            words.find(&needle).map(|at| at..at + needle.len())
        });
        let line_h = look.length(token::LEADING_LG);
        let begun = self.openings(cx);
        let plain = div()
            .relative()
            .flex_1()
            .min_w_0()
            .h(line_h * 5.)
            // The tag in the request is lit under the field, as Electron's mirror lights it.
            .child(
                div()
                    .absolute()
                    .inset_0()
                    // The field's own padding, which the text sits inside.
                    .px(gpui_component::Size::Medium.input_px())
                    .py(gpui_component::Size::Medium.input_py())
                    .text_size(look.length(token::TEXT_LG))
                    .line_height(line_h)
                    .text_color(gpui::transparent_black())
                    .child(match begun.is_empty() {
                        true => highlighted(&words, tag_range, look.color(token::BG_RAISED), look.color(token::BORDER_DEFAULT)),
                        false => self.ghost(&words, &begun, cx),
                    }),
            )
            .child(labelled(
                // The text-box key context comes with the Jobs screens' `TEXT_BOX` in keymap.rs: add it to
                // this element when the two branches meet.
                div()
                    .id("dispatch-request")
                    .size_full()
                    .text_size(look.length(token::TEXT_LG))
                    .line_height(line_h)
                    .on_key_up(cx.listener(|this, event: &gpui::KeyUpEvent, _, cx| crate::job_view::talk::key_up(this, event, cx)))
                    .child(
                        Textarea::new(&self.request)
                            .appearance(false)
                            .bordered(false)
                            .h(line_h * 5.)
                            .p_0()
                            .text_size(look.length(token::TEXT_LG))
                            .on_paste({
                                let view = cx.entity().downgrade();
                                move |item, _, cx| view.update(cx, |this, cx| this.paste(item, cx)).unwrap_or(false)
                            }),
                    ),
                Role::TextInput,
                "Request",
            ))
            .children(self.openings_line(cx));
        // A sentence being written has the request's place, and its kind's glyph the Job's.
        let kind = self.story.as_ref().map(|story| story.kind());
        let field = match kind {
            Some(_) => self.story_field(window, cx),
            None => plain.into_any_element(),
        };
        let (mark_glyph, mark_ink, mark_name) = match kind {
            Some(kind) => (kind.glyph, look.color(Token(kind.hue)), kind.name),
            None => (Kind::Job.glyph(), look.color(token::ACCENT), Kind::Job.word()),
        };
        div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_4))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .child(
                div().id("dispatch-prompt").flex_none().h(line_h).flex().items_center().tooltip(tip(mark_name)).child(labelled(
                    div().id("dispatch-kind").child(glyph(mark_glyph, mark, mark_ink)),
                    Role::Image,
                    mark_name,
                )),
            )
            .child(field)
            .child(
                div()
                    .id("dispatch-key")
                    .h(line_h)
                    .flex()
                    .items_center()
                    .tooltip(tip(format!("Dispatch  {}", keymap::shown("new_job"))))
                    // Electron's sheet draws the bare letter.
                    .child(kbd("N", look)),
            )
    }

    /// What goes with the request, as tiles over the words: files as a Job's box shows them, then sketches.
    fn tray(&self, window: &Window, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        // While a sentence stands its tools are in its own footer.
        if !self.holds() || (self.story.is_some() && self.files.is_empty() && self.drawn.is_empty()) {
            return None;
        }
        let look = cx.look();
        let remove = |key: u64| -> Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)> {
            let this = cx.entity().downgrade();
            Box::new(move |_, window, cx| {
                let _ = this.update(cx, |this, cx| this.remove_file(key, window, cx));
            })
        };
        let mut drawn = drawn_tiles(&self.drawn, look, cx, |this: &mut Self, gone, cx| this.remove_drawn(gone, cx));
        if self.story.is_none() {
            drawn.extend(self.tool_chips(window, cx));
        }
        // A Job that waits starts with no message, so what is held is said not to go.
        let unsent = (!self.takes_files()).then(|| {
            labelled(
                div()
                    .id("dispatch-unsent")
                    .px(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child("Not sent: a Job that waits starts with no message."),
                Role::Status,
                "Not sent",
            )
        });
        // The tray's own inset, and this much more, lines its tiles up with the words.
        Some(div().px(look.length(token::SPACE_2)).child(tray(&self.files, drawn, look, remove)).children(unsent).into_any_element())
    }

    /// The request box's tools under the words, over a hairline as a Job's box has them: the + that lists Armada's
    /// tools, Attach and Draw sketch, and Talk at the right.
    fn tools(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let radius = look.length(token::RADIUS_MD);
        let dress = |key: &FocusHandle| {
            let key = key.clone();
            let on = key.is_focused(window);
            move |button: gpui::Stateful<gpui::Div>| ringed(look, button.track_focus(&key), on, radius)
        };
        div()
            .id("dispatch-tools")
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .when(self.takes_files(), |row| {
                // ⌘/ is heard in a Job's thread and not here, so the tooltip shows no key.
                row.child(plus_button(look, self.tools_open.is_some(), false, dress(&self.tools_key), cx, |this: &mut Self, window, cx| this.open_tools(window, cx)))
                    .child(attach_button(look, None, dress(&self.attach_key), cx, |this: &mut Self, window, cx| this.attach_files(window, cx)))
                    .child(sketch_button(look, None, dress(&self.sketch_key), cx, |this: &mut Self, window, cx| this.open_sketch(window, cx)))
            })
            .child(div().flex_1())
            .child(crate::job_view::talk::render_with(self, look, dress(&self.talk_key), cx))
    }

    /// The tools the + lists, under the words as the `@` menu hangs: the heading, then each tool's glyph in its hue,
    /// its name and what it gives.
    fn tool_list(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let active = self.tools_open.filter(|_| self.takes_files())?;
        let look = cx.look();
        let mut items: Vec<gpui::AnyElement> = Vec::new();
        for (at, row) in crate::job_view::ask_tools::rows().into_iter().enumerate() {
            if let Some(group) = row.group {
                items.push(
                    div()
                        .px(look.length(token::SPACE_4))
                        .py(look.length(token::SPACE_1))
                        .text_size(look.length(token::TEXT_2XS))
                        .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(group.to_uppercase())
                        .into_any_element(),
                );
            }
            let hue = match &row.pick {
                crate::job_view::offer::Pick::Tag { id, .. } => crate::job_view::ask_tools::hue(id),
                _ => None,
            };
            let name: SharedString = format!("{} {}", row.name, row.says).into();
            items.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("tool-{at}")))
                        .h(look.length(token::H_MENU_ITEM))
                        .px(look.length(token::SPACE_4))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .bg(if at == active { look.color(token::BG_HOVER) } else { gpui::transparent_black() })
                        .cursor_pointer()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                                this.pick_tool(at, window, cx);
                            }),
                        )
                        .child(glyph(row.glyph, look.length(token::SPACE_3), look.color(hue.unwrap_or(token::FG_MUTED))))
                        .child(div().flex_none().text_color(look.color(token::FG_DEFAULT)).child(row.name))
                        .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(row.says)),
                    Role::ListBoxOption,
                    name,
                )
                .into_any_element(),
            );
        }
        Some(
            labelled(
                div()
                    .id("dispatch-tool-list")
                    .flex()
                    .flex_col()
                    .py(look.length(token::SPACE_1))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_OVERLAY))
                    .children(items),
                Role::ListBox,
                ASK_FOR,
            )
            .into_any_element(),
        )
    }

    /// Each tool held, as a chip of its hue beside the tiles: a press, or Enter or Space on it, removes it.
    pub(super) fn tool_chips(&self, window: &Window, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let look = cx.look();
        let painted = crate::job_view::ask_tools::painted(look);
        self.tools
            .iter()
            .map(|one| {
                let (icon, hue) = painted.iter().find(|(id, ..)| *id == one.tag.id).map_or(("plus", look.color(token::FG_MUTED)), |(_, icon, hue)| (*icon, *hue));
                let id = one.tag.id.clone();
                let said = format!("Remove {}", one.tag.title);
                let chip = div()
                    .id(SharedString::from(format!("tool-chip-{}", one.tag.id)))
                    .track_focus(&one.key)
                    .flex_none()
                    .self_center()
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(hue.opacity(0.6))
                    .bg(hue.opacity(0.16))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .cursor_pointer()
                    .tooltip(tip(said.clone()))
                    .on_click(cx.listener(move |this, _, window, cx| this.remove_tool(&id, window, cx)))
                    .child(glyph(icon, look.length(token::SPACE_3), hue))
                    .child(one.tag.title.clone())
                    .child(glyph("x", look.length(token::SPACE_3), look.color(token::FG_MUTED)));
                labelled(ringed(look, chip, one.key.is_focused(window), look.length(token::RADIUS_MD)), Role::Button, said).into_any_element()
            })
            .collect()
    }

    fn menu_view(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.menu.as_ref()?;
        let look = cx.look();
        let label = Mentions::label(menu.trigger, &menu.query);
        Some(listbox(label, &menu.rows, menu.active, look, cx.listener(|this, at: &usize, window, cx| this.choose_row(*at, window, cx))))
    }

    /// The drawer: the repository row, the band with the act, and the settings.
    fn drawer(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let asking = self.asking;
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let off = !self.face.connected;
        let chosen_name = self.face.chosen().map(|chip| chip.label().to_string());
        // A project's row names its repository after it, quiet, as its line in the list does.
        let chosen_repository = self.face.chosen().filter(|chip| chip.project.is_some()).map(|chip| chip.name.clone());
        let reading = self.face.reading;
        let mark = {
            let ink = if asking {
                hue
            } else if reading.found() || reading == Reading::Detecting {
                look.color(token::ACCENT)
            } else {
                look.color(token::FG_MUTED)
            };
            let size = look.length(token::SPACE_3) + look.length(token::SPACE_1);
            let icon: gpui::AnyElement = if reading == Reading::Detecting {
                spinner("dispatch-reading-spin", size, ink, cx).into_any_element()
            } else {
                // The switcher's glyphs: layers for a project, a branch for a repository in none.
                let icon = match (&chosen_name, &chosen_repository) {
                    (Some(_), None) => "git-branch",
                    _ => "layers",
                };
                glyph(icon, size, ink).into_any_element()
            };
            labelled(
                div().id("dispatch-reading").h(look.length(token::H_CONTROL)).px(look.length(token::SPACE_1)).flex().items_center().tooltip(tip(reading.tip())).child(icon),
                Role::Image,
                reading.name(chosen_name.as_deref()),
            )
        };
        // The project is one picker at the start of the band, as Model and Effort are: its list holds his
        // projects and every repository with its key, and ⌥1 to ⌥9 still pick without opening it.
        let chosen = chosen_name.is_some();
        let edge = if asking {
            hue
        } else if chosen {
            look.color(token::ACCENT)
        } else {
            look.color(token::BORDER_DEFAULT)
        };
        let at = Rc::clone(&self.at.repository);
        let picker = labelled(
            ringed(look, div()
                .id("dispatch-repository")
                .track_focus(&self.row)
                .tab_stop(true)
                .relative()
                .flex_none()
                .h(look.length(token::H_CONTROL))
                .min_w(look.length(token::SPACE_12) * 2.5)
                .max_w(look.length(token::SPACE_12) * 5.)
                .pl(look.length(token::SPACE_2))
                .pr(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(edge)
                .bg(if asking { hue.opacity(0.16) } else { look.color(token::BG_SUNKEN) })
                // Sans, as Electron's repository menu draws its trigger and its rows (`DropdownMenu.css`).
                .font_family(sans(look))
                .text_color(look.color(if chosen { token::FG_DEFAULT } else { token::FG_MUTED }))
                .when(!off, |pick| {
                    pick.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(|this, _, window, cx| this.open_repositories(window, cx)))
                })
                .when(off, |pick| pick.opacity(0.5))
                .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(mark)
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(chosen_name.clone().unwrap_or_else(|| "Project".into()))))
                .children(chosen_repository.clone().map(|name| div().flex_none().font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(name)))
                .child(glyph("chevron-down", look.length(token::SPACE_3), look.color(token::FG_MUTED))),
                self.row.is_focused(window),
                look.length(token::RADIUS_MD),
            ),
            Role::ComboBox,
            match (&chosen_name, &chosen_repository) {
                (Some(name), Some(repository)) => format!("Project {name}, {repository}"),
                (Some(name), None) => format!("Repository {name}"),
                _ => "Project".to_string(),
            },
        );
        // A start Fleet refused, in Fleet's words, on a line of its own over the band.
        let refusal = self.face.refused.clone().map(|why| {
            div()
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .child(labelled(div().id("dispatch-refused").text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why))
        });
        let tuning = {
            let model = self.model.clone().unwrap_or_else(|| "Auto".into());
            let effort = self.effort.clone().unwrap_or_else(|| "Auto".into());
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(self.pick_button("dispatch-model", "cpu", "Model", model, Rc::clone(&self.at.model), (&self.model_key, self.model_key.is_focused(window)), off, cx.listener(|this, _, window, cx| this.open_model(window, cx)), cx))
                .child(self.pick_button("dispatch-effort", "zap", "Effort", effort, Rc::clone(&self.at.effort), (&self.effort_key, self.effort_key.is_focused(window)), off, cx.listener(|this, _, window, cx| this.open_effort(window, cx)), cx))
        };
        let queue = (self.face.queue && !self.holds()).then(|| {
            let live = self.queue_live(cx);
            let button = labelled(
                ringed(look, Btn::new("dispatch-queue", "Queue for later", Weight::Secondary)
                    .disabled(!live)
                    .pending(self.queueing)
                    .draw(look, cx)
                    .track_focus(&self.queue_key)
                    .focus_visible(|style| style)
                    .when(live, |button| button.on_click(cx.listener(|this, _, window, cx| this.queue(window, cx)))),
                    self.queue_key.is_focused(window),
                    look.length(token::RADIUS_LG),
                ),
                Role::Button,
                "Queue for later",
            );
            button
        });
        let starting = self.sending && !self.drafting;
        let said = if starting { "Starting" } else { "Start" };
        let live = self.act_live(cx);
        // A draft is a start held on its start card; a wait has no card.
        let draft = self.wait.is_none().then(|| {
            let said = if self.drafting { "Drafting" } else { "Draft" };
            labelled(
                ringed(look, Btn::new("dispatch-draft", said, Weight::Secondary)
                    .disabled(!live)
                    .pending(self.drafting)
                    .draw(look, cx)
                    .track_focus(&self.draft_key)
                    .focus_visible(|style| style)
                    .tooltip(tip("Nothing runs until you press Start on its card"))
                    .when(live, |button| button.on_click(cx.listener(|this, _, window, cx| this.draft(window, cx)))),
                    self.draft_key.is_focused(window),
                    look.length(token::RADIUS_LG),
                ),
                Role::Button,
                said,
            )
        });
        // No key beside Queue for later, Draft or Start: Electron's sheet shows none.
        let act = div()
            .flex()
            .items_center()
            .child(labelled(
                ringed(look, Btn::new("dispatch-act", said, Weight::Primary)
                    .disabled(!live)
                    .pending(starting)
                    .draw(look, cx)
                    .track_focus(&self.act_key)
                    .focus_visible(|style| style)
                    .when(live, |button| button.on_click(cx.listener(|this, _, window, cx| this.act(window, cx)))),
                    self.act_key.is_focused(window),
                    look.length(token::RADIUS_LG),
                ),
                Role::Button,
                said,
            ));
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::H_ROW) * 1.75)
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .text_color(look.color(token::FG_MUTED))
            // The acts are in the UI face; the chips and the selects stay mono.
            .font_family(sans(look))
            .child(picker)
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).ml_auto().child(tuning).children(queue).children(draft).child(act));
        div()
            .id("dispatch-drawer")
            .flex()
            .flex_col()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .children(refusal)
            .child(band)
    }

    /// A Model or Effort pick: a glyph and the value, opening its list.
    #[allow(clippy::too_many_arguments)]
    fn pick_button(
        &self,
        id: &'static str,
        icon: &'static str,
        name: &'static str,
        value: SharedString,
        at: Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
        key: (&FocusHandle, bool),
        off: bool,
        press: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let look = cx.look();
        labelled(
            ringed(look, div()
                .id(id)
                .track_focus(key.0)
                .relative()
                .h(look.length(token::H_CONTROL))
                .min_w(look.length(token::SPACE_12) * 1.5)
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .font_family(mono(look))
                .text_color(look.color(token::FG_DEFAULT))
                .tooltip(tip(name))
                .when(!off, |pick| pick.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(press))
                .when(off, |pick| pick.opacity(0.5))
                .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(glyph(icon, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(div().flex_1().min_w_0().truncate().child(value))
                .child(glyph("chevron-down", look.length(token::SPACE_3), look.color(token::FG_MUTED))),
                key.1,
                look.length(token::RADIUS_MD),
            ),
            Role::ComboBox,
            name,
        )
    }
}

/// The words with the tagged part lit: the mirror under the field.
fn highlighted(words: &str, tag: Option<std::ops::Range<usize>>, ground: gpui::Hsla, edge: gpui::Hsla) -> gpui::AnyElement {
    let text: SharedString = if words.is_empty() { " ".into() } else { words.to_string().into() };
    let styled = gpui::StyledText::new(text);
    match tag {
        Some(range) => styled
            .with_highlights([(
                range,
                gpui::HighlightStyle {
                    background_color: Some(ground),
                    underline: Some(gpui::UnderlineStyle { thickness: px(1.), color: Some(edge), wavy: false }),
                    ..Default::default()
                },
            )])
            .into_any_element(),
        None => styled.into_any_element(),
    }
}

/// A listbox of rows, as `SessionMenu` draws it: a glyph, a name, what it says and its standing. A
/// press chooses without taking the keys off the field.
fn listbox(label: &'static str, rows: &[Row], active: usize, look: &Look, choose: impl Fn(&usize, &mut Window, &mut App) + 'static) -> gpui::AnyElement {
    let choose = Rc::new(choose);
    let rows = rows.iter().enumerate().map(|(at, row)| {
        let on = at == active;
        let pick = Rc::clone(&choose);
        labelled(
            div()
                .id(SharedString::from(format!("option-{at}")))
                .h(look.length(token::H_MENU_ITEM))
                .px(look.length(token::SPACE_4))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .bg(if on { look.color(token::BG_HOVER) } else { gpui::transparent_black() })
                .cursor_pointer()
                .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    pick(&at, window, cx);
                })
                .child(glyph(row.glyph.icon(), look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(div().flex_none().text_color(look.color(token::FG_DEFAULT)).child(row.name.clone()))
                .children(row.says.clone().map(|says| div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(says)))
                .children(row.standing.clone().map(|standing| div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(standing))),
            Role::ListBoxOption,
            row.name.clone(),
        )
    });
    labelled(
        div()
            .id(SharedString::from(format!("listbox-{label}")))
            .max_h(look.length(token::H_MENU_ITEM) * 5.)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .py(look.length(token::SPACE_1))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .children(rows),
        Role::ListBox,
        label,
    )
    .into_any_element()
}

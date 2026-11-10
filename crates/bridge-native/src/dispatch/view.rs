//! The dispatch sheet drawn (`DP/DispatchPanel.tsx`, `DP/dispatch-panel.css`). Sizes follow the
//! inventory's section 1.5, with the owner's 9 Nov tweak of more room: taller controls, wider
//! gaps and the larger radius, all from the same tokens the Jobs list uses.

use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, px, App, Bounds, Context, Entity, FontWeight, KeyDownEvent, Pixels, Role, SharedString, Window,
};
use gpui_component::input::{Input, InputState, Textarea};

use super::face::{Chip, Glyph, Kind, Mark, Mentions, Part, Reading, Row};
use super::sheet::DispatchSheet;
use crate::keymap::{self, Act};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, kbd, mono, sans, spinner, tip, Btn, Weight};

impl Render for DispatchSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let view = window.viewport_size();
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
        let frame = div()
            .id("dispatch-frame")
            .key_context("dispatch")
            .track_focus(&self.frame)
            .on_action(cx.listener(|this, key: &Act, window, cx| this.on_act(key, window, cx)))
            // A press on the sheet is not a press on the scrim.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .absolute()
            .top(top)
            .left((view.width - width) / 2.)
            .w(width)
            .max_h(view.height - top - look.length(token::SPACE_6))
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
            .children(self.wait_chip(cx))
            .child(self.line(window, cx))
            .children(self.snippets(cx))
            .children(self.menu_view(cx))
            .child(self.drawer(window, cx));
        // A press on the scrim shuts the sheet.
        let scrim = div()
            .id("dispatch-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .bg(look.color(token::SCRIM))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)));
        labelled(
            div().id("dispatch").absolute().inset_0().child(scrim).child(frame).children(self.popup.as_ref().map(|popup| popup.dropdown.clone())),
            Role::Dialog,
            "Dispatch",
        )
    }
}

impl DispatchSheet {
    /// The sentence over the request: `Starts when [Job]'s [PR lands] · from [main] · needs [..]`.
    fn wait_chip(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let wait = self.wait.as_ref().filter(|_| self.kind == Kind::Job)?;
        let look = cx.look();
        let off = !self.face.connected;
        let part = |id: &'static str, which: Part, at: usize, label: SharedString, name: String, icon: Option<&'static str>, cx: &Context<Self>| {
            let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
            let handle = self.parts[at].clone();
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
                .focus_visible(move |style| style.border_color(ring))
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
            labelled(body, Role::Button, name)
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
                })
                .collect();
            listbox("Jobs", &rows, active, look, cx.listener(|this, at: &usize, _, cx| this.wait_on(*at, cx)))
        });
        Some(labelled(div().id("wait").flex().flex_col().child(line).children(menu), Role::Group, "Start condition").into_any_element())
    }

    /// The line: the kind's glyph, the words, `⇥` and the key that opens the sheet.
    fn line(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let kinds = self.face.kinds.clone();
        let mark = look.length(token::SPACE_4) + look.length(token::SPACE_1);
        let tab_tip = kinds.iter().map(|kind| kind.word()).collect::<Vec<_>>().join(" / ");
        let in_frame = self.frame.contains_focused(window, cx);
        let words = self.words(cx);
        let tag_range = self.tagged.as_ref().and_then(|(_, title)| {
            let needle = format!("@{title}");
            words.find(&needle).map(|at| at..at + needle.len())
        });
        let line_h = look.length(token::LEADING_LG);
        let field = div()
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
                    .child(highlighted(&words, tag_range, look.color(token::BG_RAISED), look.color(token::BORDER_DEFAULT))),
            )
            .child(labelled(
                // The text-box key context comes with the Jobs screens' `TEXT_BOX` in keymap.rs: add it to
                // this element when the two branches meet.
                div().id("dispatch-request").size_full().text_size(look.length(token::TEXT_LG)).line_height(line_h).child(
                    Textarea::new(&self.request).appearance(false).bordered(false).h(line_h * 5.).p_0().text_size(look.length(token::TEXT_LG)),
                ),
                Role::TextInput,
                "Request",
            ));
        div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_4))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .child(
                div().id("dispatch-prompt").flex_none().h(line_h).flex().items_center().tooltip(tip(self.kind.word())).child(labelled(
                    div().id("dispatch-kind").child(glyph(self.kind.glyph(), mark, look.color(token::ACCENT))),
                    Role::Image,
                    self.kind.word(),
                )),
            )
            .child(field)
            .when(kinds.len() > 1 && in_frame, |line| {
                line.child(div().id("dispatch-tab-key").h(line_h).flex().items_center().tooltip(tip(tab_tip)).child(kbd("⇥", look)))
            })
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

    fn snippets(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        if self.attached.is_empty() {
            return None;
        }
        let look = cx.look();
        let chips = self.attached.iter().enumerate().map(|(at, name)| {
            let name = name.clone();
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .pl(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .child(glyph("file", look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(name.clone())
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("attached-{at}")))
                        .size(look.length(token::H_CONTROL))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(glyph("x", look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if at < this.attached.len() {
                                this.attached.remove(at);
                            }
                            cx.notify();
                        })),
                    Role::Button,
                    format!("Remove {name}"),
                ))
        });
        Some(
            labelled(
                div()
                    .id("dispatch-attached")
                    .flex()
                    .flex_wrap()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_6))
                    .pb(look.length(token::SPACE_3))
                    .children(chips),
                Role::Group,
                "Attached",
            )
            .into_any_element(),
        )
    }

    fn menu_view(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.menu.as_ref()?;
        let look = cx.look();
        let label = Mentions::label(menu.trigger, &menu.query);
        Some(listbox(label, &menu.rows, menu.active, look, cx.listener(|this, at: &usize, window, cx| this.choose_row(*at, window, cx))))
    }

    /// The drawer: the repository row, the band with the kinds and the act, and the settings.
    fn drawer(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let asking = self.asking;
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let off = !self.face.connected;
        let chosen_name = self.face.chosen().map(|chip| chip.name.to_string());
        let reading = self.face.reading;
        let mark = {
            let ink = if asking {
                look.color(token::FG_INVERSE)
            } else if reading.found() || reading == Reading::Detecting {
                look.color(token::ACCENT)
            } else {
                look.color(token::FG_MUTED)
            };
            let size = look.length(token::SPACE_3) + look.length(token::SPACE_1);
            let icon: gpui::AnyElement = if reading == Reading::Detecting {
                spinner("dispatch-reading-spin", size, ink, cx).into_any_element()
            } else {
                glyph("folder-git-2", size, ink).into_any_element()
            };
            labelled(
                div().id("dispatch-reading").h(look.length(token::H_CONTROL)).px(look.length(token::SPACE_1)).flex().items_center().tooltip(tip(reading.tip())).child(icon),
                Role::Image,
                reading.name(chosen_name.as_deref()),
            )
        };
        let on_row = self.row.is_focused(window);
        let chips: Vec<gpui::AnyElement> = self
            .face
            .chips
            .iter()
            .enumerate()
            .map(|(at, chip)| self.chip(at, chip, on_row && self.rover == at, off, asking, cx).into_any_element())
            .collect();
        let more = (!self.face.more.is_empty()).then(|| self.more_menu(off, cx));
        let repos = labelled(
            div()
                .id("dispatch-repos")
                .track_focus(&self.row)
                .tab_stop(true)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(if asking { hue } else { look.color(token::BORDER_DEFAULT) })
                .bg(if asking { hue } else { look.color(token::BG_SUNKEN) })
                .child(mark)
                .children(chips)
                .children(more),
            Role::RadioGroup,
            "Repository",
        );
        let kinds = self.face.kinds.clone();
        let tabs = (kinds.len() > 1).then(|| {
            labelled(
                div().id("dispatch-tabs").flex().items_center().gap(look.length(token::SPACE_1)).children(kinds.iter().map(|kind| {
                    let kind = *kind;
                    let on = self.kind == kind;
                    labelled(
                        div()
                            .id(SharedString::from(format!("tab-{}", kind.word())))
                            .h(look.length(token::H_CONTROL))
                            .px(look.length(token::SPACE_4))
                            .flex()
                            .items_center()
                            .rounded(look.length(token::RADIUS_MD))
                            .text_size(look.length(token::TEXT_XS))
                            .font_weight(FontWeight(look.number(if on { token::WEIGHT_HEADING } else { token::WEIGHT_BODY })))
                            .text_color(look.color(if on { token::FG_DEFAULT } else { token::FG_MUTED }))
                            .bg(if on { look.color(token::BG_SUNKEN) } else { gpui::transparent_black() })
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .child(kind.word())
                            .on_click(cx.listener(move |this, _, window, cx| this.set_kind(kind, window, cx))),
                        Role::Tab,
                        kind.word(),
                    )
                })),
                Role::TabList,
                "Kind",
            )
        });
        let worded = self.worded(cx);
        let tuning = (self.kind == Kind::Job).then(|| {
            let model = self.model.clone().or_else(|| self.face.tuning.default_model.clone()).unwrap_or_else(|| "Auto".into());
            let effort = self.effort.clone().unwrap_or_else(|| "Auto".into());
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(self.pick_button("dispatch-model", "cpu", "Model", model, Rc::clone(&self.at.model), off, cx.listener(|this, _, window, cx| this.open_model(window, cx)), cx))
                .child(self.pick_button("dispatch-effort", "zap", "Effort", effort, Rc::clone(&self.at.effort), off, cx.listener(|this, _, window, cx| this.open_effort(window, cx)), cx))
        });
        let queue = (self.kind == Kind::Job && self.face.queue && self.attached.is_empty()).then(|| {
            let live = !off && !self.sending && !self.queueing && !self.words(cx).trim().is_empty();
            let button = labelled(
                Btn::new("dispatch-queue", "Queue for later", Weight::Secondary)
                    .disabled(!live)
                    .pending(self.queueing)
                    .draw(look, cx)
                    .when(live, |button| button.on_click(cx.listener(|this, _, window, cx| this.queue(window, cx)))),
                Role::Button,
                "Queue for later",
            );
            button
        });
        let word = self.face.act(self.kind);
        let said = if self.sending { pending_word(self.kind, word) } else { word };
        let live = !off && !self.queueing && worded && !self.sending;
        // No key beside Queue for later or Start: Electron's sheet shows neither.
        let act = div()
            .flex()
            .items_center()
            .child(labelled(
                Btn::new("dispatch-act", said, Weight::Primary)
                    .disabled(!live)
                    .pending(self.sending)
                    .draw(look, cx)
                    .when(live, |button| button.on_click(cx.listener(|this, _, window, cx| this.act(window, cx)))),
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
            // The kinds and the acts are in the UI face; the chips and the selects stay mono.
            .font_family(sans(look))
            .children(tabs)
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).ml_auto().children(tuning).children(queue).child(act));
        div()
            .id("dispatch-drawer")
            .flex()
            .flex_col()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(repos)
            .child(band)
            .children(self.settings(cx))
    }

    fn chip(&self, at: usize, chip: &Chip, rover: bool, off: bool, asking: bool, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let chosen = chip.mark == Mark::Chosen;
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let (ground, ink, edge) = if asking {
            (gpui::transparent_black(), look.color(token::FG_INVERSE), look.color(token::FG_INVERSE))
        } else if chosen {
            (look.color(token::ACCENT), look.color(token::FG_INVERSE), look.color(token::ACCENT))
        } else if chip.mark == Mark::Read {
            (look.color(token::BG_RAISED), look.color(token::FG_DEFAULT), look.color(token::ACCENT))
        } else {
            (look.color(token::BG_RAISED), look.color(token::FG_DEFAULT), look.color(token::BORDER_DEFAULT))
        };
        let name = chip.name.clone();
        let picked = chip.name.clone();
        labelled(
            div()
                .id(SharedString::from(format!("repo-{at}")))
                .h(look.length(token::H_CONTROL))
                .max_w_full()
                .pl(look.length(token::SPACE_3))
                .pr(look.length(token::SPACE_1))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if rover { ring } else { edge })
                .bg(ground)
                .text_color(ink)
                .font_weight(FontWeight(look.number(if chosen { token::WEIGHT_HEADING } else { token::WEIGHT_BODY })))
                .when(chosen && !asking, |chip| chip.shadow(look.shadows(token::SHADOW_PRIMARY)))
                .when(!off, |chip| {
                    chip.cursor_pointer().hover(|style| style.bg(if chosen { look.color(token::ACCENT_HOVER) } else { look.color(token::BG_HOVER) }))
                })
                .when(off, |chip| chip.opacity(0.5))
                .child(div().min_w_0().truncate().child(name.clone()))
                .children(chip.key.and_then(|n| keymap::shown_digit("dispatch_repository", n)).map(|key| kbd(key, look)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.face.connected {
                        this.pick(picked.clone(), window, cx);
                    }
                })),
            Role::RadioButton,
            format!("Repository {name}"),
        )
    }

    fn more_menu(&self, off: bool, cx: &Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let hidden = self.face.more.iter().find(|chip| chip.mark == Mark::Chosen);
        let at = Rc::clone(&self.at.more);
        let label: SharedString = hidden.map_or("More".into(), |chip| chip.name.clone());
        let name = hidden.map_or("More repositories".to_string(), |chip| format!("Repository {}", chip.name));
        let lit = hidden.is_some();
        labelled(
            div()
                .id("dispatch-more")
                .relative()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if lit { token::ACCENT } else { token::BORDER_DEFAULT }))
                .bg(look.color(if lit { token::ACCENT } else { token::BG_RAISED }))
                .text_color(look.color(if lit { token::FG_INVERSE } else { token::FG_DEFAULT }))
                .when(!off, |more| more.cursor_pointer().hover(|style| style.bg(look.color(if lit { token::ACCENT_HOVER } else { token::BG_HOVER }))))
                .when(off, |more| more.opacity(0.5))
                .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(label)
                .child(glyph("chevron-down", look.length(token::SPACE_3), look.color(if lit { token::FG_INVERSE } else { token::FG_MUTED })))
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.face.connected {
                        this.open_more(window, cx);
                    }
                })),
            Role::ComboBox,
            name,
        )
        .into_any_element()
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
        off: bool,
        press: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let look = cx.look();
        labelled(
            div()
                .id(id)
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
            Role::ComboBox,
            name,
        )
    }

    /// The Autopilot body: the instruction and the fields, or what was refused.
    fn settings(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        if self.kind != Kind::Autopilot || (self.instruction.is_none() && self.face.refused.is_none()) {
            return None;
        }
        let look = cx.look();
        let field = |label: &'static str, input: &Entity<InputState>, said: Option<Option<SharedString>>| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(label))
                .child(labelled(div().id(SharedString::from(format!("field-{label}"))).child(Input::new(input)), Role::TextInput, label))
                .children(said.map(|said| {
                    div()
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(match said {
                            Some(words) => format!("“{words}”"),
                            None => "not said".to_string(),
                        })
                }))
        };
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .children(self.instruction.as_ref().map(|input| field("Instruction", input, None)))
            .child(div().flex().flex_wrap().gap(look.length(token::SPACE_3)).children(self.fields.iter().map(|(label, input, said)| {
                div().w(look.length(token::SPACE_12) * 4.).child(field(label, input, Some(said.clone())))
            })))
            .children(
                self.face
                    .refused
                    .clone()
                    .map(|why| labelled(div().id("autopilot-refused").text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why)),
            );
        Some(body.into_any_element())
    }
}

/// The act's word while its act is out.
fn pending_word(kind: Kind, act: &'static str) -> &'static str {
    match (kind, act) {
        (Kind::Job, _) => "Starting",
        (Kind::Autopilot, "Read it") => "Reading",
        (Kind::Autopilot, _) => "Starting Autopilot",
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

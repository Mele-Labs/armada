//! The dispatch sheet drawn (`DP/DispatchPanel.tsx`, `DP/dispatch-panel.css`). Sizes follow the
//! inventory's section 1.5, with the owner's 9 Nov tweak of more room: taller controls, wider
//! gaps and the larger radius, all from the same tokens the Jobs list uses.

use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, px, App, Bounds, Context, KeyDownEvent, Pixels, Role, SharedString, Window,
};
use gpui_component::input::Textarea;

use super::face::{Glyph, Kind, Mentions, Part, Reading, Row};
use super::sheet::DispatchSheet;
use crate::keymap::{self, Act};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
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
            .child(self.line(cx))
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
        let wait = self.wait.as_ref()?;
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
    fn line(&self, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let mark = look.length(token::SPACE_4) + look.length(token::SPACE_1);
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

    /// The drawer: the repository row, the band with the act, and the settings.
    fn drawer(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let asking = self.asking;
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let off = !self.face.connected;
        // A folder chosen for a terminal Job stands in the picker in place of a repository.
        let folder = self.folder.as_ref().filter(|_| self.kind == Kind::Terminal).map(|path| path.to_string());
        let chosen_name = folder.as_deref().map(folder_name).or_else(|| self.face.chosen().map(|chip| chip.name.to_string()));
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
            let icon: gpui::AnyElement = if folder.is_some() {
                glyph("folder", size, look.color(token::ACCENT)).into_any_element()
            } else if reading == Reading::Detecting {
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
        // The repository is one picker at the start of the band, as Model and Effort are: its list holds
        // every repository with its key, and ⌥1 to ⌥9 still pick without opening it.
        let chosen = chosen_name.is_some();
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let edge = if self.row.is_focused(window) {
            ring
        } else if asking {
            hue
        } else if chosen {
            look.color(token::ACCENT)
        } else {
            look.color(token::BORDER_DEFAULT)
        };
        let at = Rc::clone(&self.at.repository);
        let picker = labelled(
            div()
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
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(chosen_name.clone().unwrap_or_else(|| "Repository".into()))))
                .child(glyph("chevron-down", look.length(token::SPACE_3), look.color(token::FG_MUTED))),
            Role::ComboBox,
            match (&folder, &chosen_name) {
                (Some(path), _) => format!("Folder {path}"),
                (None, Some(name)) => format!("Repository {name}"),
                (None, None) => "Repository".to_string(),
            },
        );
        let picker = div().id("dispatch-repository-tip").when_some(folder.clone(), |wrap, path| wrap.tooltip(tip(path))).child(picker);
        // One kind while Fleet cannot start a terminal Job, and the band draws no track for one.
        let kinds = crate::terminal::shells::starts(cx).then(|| self.kinds(cx));
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
        let worded = self.worded(cx);
        let tuning = {
            let model = self.model.clone().or_else(|| self.face.tuning.default_model.clone()).unwrap_or_else(|| "Auto".into());
            let effort = self.effort.clone().unwrap_or_else(|| "Auto".into());
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(self.pick_button("dispatch-model", "cpu", "Model", model, Rc::clone(&self.at.model), off, cx.listener(|this, _, window, cx| this.open_model(window, cx)), cx))
                .child(self.pick_button("dispatch-effort", "zap", "Effort", effort, Rc::clone(&self.at.effort), off, cx.listener(|this, _, window, cx| this.open_effort(window, cx)), cx))
        };
        let queue = (self.face.queue && self.attached.is_empty() && self.kind == Kind::Job).then(|| {
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
        let said = if self.sending { "Starting" } else { "Start" };
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
            // The acts are in the UI face; the chips and the selects stay mono.
            .font_family(sans(look))
            .child(picker)
            .children(kinds)
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).ml_auto().child(tuning).children(queue).child(act));
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

    /// The kinds, as Electron's band draws them once there is more than one (`DispatchPanel.tsx:592`): a
    /// raised track, the chosen one filled. Tab on the line steps through them too.
    fn kinds(&self, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let inset = look.length(token::SPACE_1) / 2.;
        let tabs = Kind::ALL.into_iter().map(|kind| {
            let on = kind == self.kind;
            let hover = look.color(token::FG_DEFAULT);
            labelled(
                div()
                    .id(SharedString::from(format!("dispatch-kind-{}", kind.word())))
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_3))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_SM))
                    .text_size(look.length(token::TEXT_SM))
                    .cursor_pointer()
                    .when(on, |tab| tab.bg(look.color(token::ACCENT_MUTED)).text_color(look.color(token::FG_DEFAULT)).font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .when(!on, |tab| tab.text_color(look.color(token::FG_MUTED)).hover(move |style| style.text_color(hover)))
                    .on_click(cx.listener(move |this, _, window, cx| this.choose_kind(kind, window, cx)))
                    .child(kind.word()),
                Role::Tab,
                kind.word(),
            )
        });
        labelled(
            div()
                .id("dispatch-kinds")
                .flex()
                .items_center()
                .p(inset)
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .children(tabs),
            Role::TabList,
            "Kind",
        )
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

/// A folder's last part, as the picker names it; the whole path is its tooltip.
fn folder_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or_else(|| path.to_string(), |name| name.to_string_lossy().into_owned())
}

//! A question asked in pictures (`WaitingOption.picture`), in the Needs you panel when he looks: the options side by
//! side as stills, the Job's pick ringed in accent, parts outlined to take across the pictures, and the Your mix bar
//! under them. A press on a picture outside its parts takes the option whole, as `1` to `9` do; Send mix (↵) posts the
//! parts taken to `answer_waiting_parts`. With `look_questions` at `pictures_whole` a press takes the whole option and
//! no part is outlined and no mix is built.
//!
//! **Stills for now**: each option is the image Fleet hands over, with its part boxes drawn over it. Live drawing
//! waits on the browser embedding in the documents reader talk.
//!
//! **Parts are outlined with gpui's own dashed border.** A pointer press lands on the picture, which finds the part under
//! it by position (`look_mix::part_at`); each part is also a stop of its own for the keyboard, where Enter or Space takes it.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, img, prelude::*, relative, AnyElement, Bounds, ClickEvent, Context, FontWeight, ObjectFit, Pixels, Role, SharedString, Window};
use ipc::{AnswerWaiting, NeedsYouRow, SessionId, WaitingOption};

use super::acts::{button, kbd, Tone};
use super::look_dock::Still;
use super::look_mix::{body, fractions, is_taken, part_at, pill, toggled, Taken};
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::ways;

/// The narrowest an option's card is drawn, in `--space-12`s: a narrower panel stands the options in fewer columns.
const NARROWEST: f32 = 4.;

impl JobView {
    /// The question, its hint, the options as cards and the Your mix bar.
    pub(super) fn look_question(&self, id: &SharedString, row: &NeedsYouRow, options: &[WaitingOption], look: &Look, cx: &Context<Self>) -> AnyElement {
        let mixes = ways::mixes_parts();
        let taken: Vec<Taken> = self.dock_of(id).and_then(|dock| dock.looked.taken.get(&row.id).cloned()).unwrap_or_default();
        let pick = ways::pick_of(options);
        let hint = if mixes { "Press a picture to take it whole, or press labelled parts across pictures." } else { "Press a picture to take it whole." };
        let across = self.look_across(id, options.len(), look);
        let rows = options.chunks(across).enumerate().map(|(line, chunk)| {
            let cards = chunk.iter().enumerate().map(|(at, option)| {
                let at = line * across + at;
                self.look_option(id, row, at, option, pick == Some(at), &taken, mixes, look, cx)
            });
            // A short last row keeps the width of the others.
            let blanks = (0..across - chunk.len()).map(|_| div().flex_1().min_w_0().into_any_element());
            div().w_full().flex().items_start().gap(look.length(token::SPACE_3)).children(cards).children(blanks).into_any_element()
        });
        div()
            .id("look-question")
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(self.look_title(&row.text, look))
            .child(div().text_size(look.length(token::TEXT_META)).text_color(look.color(token::FG_MUTED)).child(hint))
            .children(rows)
            .children(mixes.then(|| self.look_mix_bar(row, options, &taken, look, cx)))
            .into_any_element()
    }

    /// How many options stand side by side: one each while the panel holds them at their narrowest, else as many as it does.
    fn look_across(&self, id: &SharedString, options: usize, look: &Look) -> usize {
        let panel = self.pages.get(id).map(|page| f32::from(page.list.viewport_bounds().size.width)).unwrap_or_default();
        let gap = f32::from(look.length(token::SPACE_3));
        // The panel's own sides and its card's padding come off what the thread holds.
        let inside = panel - f32::from(look.length(token::SPACE_4)) * 2. - f32::from(look.length(token::BORDER_WIDTH)) * 2.;
        let narrowest = f32::from(look.length(token::SPACE_12)) * NARROWEST;
        if panel <= 0. {
            return options.max(1);
        }
        (((inside + gap) / (narrowest + gap)).floor() as usize).clamp(1, options.max(1))
    }

    /// One option: its head, and its still in a well with its parts over it.
    #[allow(clippy::too_many_arguments)]
    fn look_option(
        &self,
        id: &SharedString,
        row: &NeedsYouRow,
        at: usize,
        option: &WaitingOption,
        pick: bool,
        taken: &[Taken],
        mixes: bool,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let name = ways::name_of(&option.label).to_string();
        let edge = look.length(token::BORDER_WIDTH);
        let accent = look.color(token::ACCENT);
        let still = option.picture.as_ref().map(|picture| (picture, self.dock_of(id).and_then(|dock| dock.looked.stills.get(&picture.image))));
        let key = at + 1;
        let whole_item = row.id.clone();
        let cap = div()
            .id(SharedString::from(format!("look-key:{}:{at}", row.id)))
            .flex_none()
            .tooltip(tip(format!("Takes {name} whole")))
            .child(kbd(look, &key.to_string()));
        let head = div()
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_META))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(name.clone())),
            )
            .children(pick.then(|| {
                div()
                    .id(SharedString::from(format!("look-pick:{}:{at}", row.id)))
                    .flex_none()
                    .px(look.length(token::SPACE_2))
                    .rounded_full()
                    .bg(look.color(token::ACCENT_MUTED))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(accent)
                    .child("My pick")
            }))
            .child(cap);
        let picture = still.map(|(picture, read)| {
            let ratio = picture.width.max(1) as f32 / picture.height.max(1) as f32;
            let bounds: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
            let measured = bounds.clone();
            let pressed = (picture.clone(), bounds, whole_item.clone());
            let body = match read {
                Some(Still::Ready(image)) => img(image.clone()).absolute().size_full().object_fit(ObjectFit::Fill).into_any_element(),
                Some(Still::Unread) => div().absolute().size_full().bg(look.color(token::BG_HOVER)).into_any_element(),
                _ => div().absolute().size_full().rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_DEFAULT).opacity(0.07)).into_any_element(),
            };
            // Each part is a box and a label tab: the tabs are laid over every box, so a later part's edge never cuts an earlier part's label.
            let mut tabs = Vec::new();
            let parts: Vec<AnyElement> = picture.parts.iter().filter(|_| mixes).map(|part| {
                let [x, y, w, h] = fractions(picture, part);
                let on = is_taken(taken, at as u32, &part.name);
                let said = format!("{} of {name}", part.label);
                let ink = if on { look.color(token::SKETCH_TEAL) } else { look.color(token::BORDER_STRONG) };
                let (item, taking) = (row.id.clone(), part.name.clone());
                let tab = div()
                    .absolute()
                    .top(-look.length(token::SPACE_2))
                    .left(look.length(token::SPACE_1) * 1.5)
                    .px(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(edge)
                    .border_color(if on { look.color(token::SKETCH_TEAL) } else { look.color(token::BORDER_DEFAULT) })
                    .bg(if on { look.color(token::SKETCH_TEAL) } else { look.color(token::SURFACE_CARD) })
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(if on { look.color(token::FG_INVERSE) } else { look.color(token::FG_MUTED) })
                    .whitespace_nowrap()
                    .child(SharedString::from(part.label.clone()));
                tabs.push(div().absolute().left(relative(x)).top(relative(y)).w(relative(w)).child(tab).into_any_element());
                labelled(
                    div()
                        .id(SharedString::from(format!("look-part:{}:{at}:{}", row.id, part.name)))
                        .absolute()
                        .left(relative(x))
                        .top(relative(y))
                        .w(relative(w))
                        .h(relative(h))
                        .rounded(look.length(token::RADIUS_SM))
                        .border(if on { edge * 2. } else { edge })
                        .border_color(ink)
                        .when(!on, |part| part.border_dashed())
                        .when(on, |part| part.bg(look.color(token::ACCENT_FAINT)))
                        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                        .tab_index(0)
                        // A pointer press lands on the picture, which finds this part by where it fell; a key press is the part's own.
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.mouse_position().is_none() {
                                this.look_take_part(&item, at as u32, &taking, window, cx);
                            }
                        })),
                    Role::Button,
                    said,
                )
                .into_any_element()
            }).collect();
            let whole_at = at;
            let taking_whole = format!("Take {name} whole");
            let drawn = div()
                .id(SharedString::from(format!("look-picture:{}:{at}", row.id)))
                .relative()
                .w_full()
                .aspect_ratio(ratio)
                .cursor_pointer()
                .child(body)
                .children(parts)
                .children(tabs)
                .child(canvas(move |area, _, _| measured.set(area), |_, _, _, _| {}).absolute().size_full())
                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                    let (picture, bounds, item) = (&pressed.0, &pressed.1, &pressed.2);
                    let area = bounds.get();
                    let part = event.mouse_position().filter(|_| ways::mixes_parts()).and_then(|at| {
                        let local = at - area.origin;
                        part_at(picture, f32::from(area.size.width), f32::from(local.x), f32::from(local.y))
                    });
                    match part.and_then(|part| picture.parts.get(part)) {
                        Some(part) => {
                            // The keys stay with the panel, so Return sends the mix.
                            window.focus(&this.dock_focus, cx);
                            this.look_take_part(item, whole_at as u32, &part.name.clone(), window, cx);
                        }
                        None => this.look_take_whole(item, whole_at, window, cx),
                    }
                }));
            labelled(drawn, Role::Button, taking_whole).into_any_element()
        });
        let well = div()
            .w_full()
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .border(edge)
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::SURFACE_WELL))
            .children(picture);
        let card = div()
            .id(SharedString::from(format!("look-option:{}:{at}", row.id)))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::SURFACE_CARD))
            .border(if pick { edge * 2. } else { edge })
            .border_color(if pick { accent } else { look.color(token::BORDER_DEFAULT) })
            .shadow(look.shadows(token::SHADOW_CARD))
            .child(head)
            .child(well);
        labelled(card, Role::Group, if pick { format!("{name}, my pick") } else { name }).into_any_element()
    }

    /// Your mix: what is taken, as pills, with Clear and Send mix at the end.
    fn look_mix_bar(&self, row: &NeedsYouRow, options: &[WaitingOption], taken: &[Taken], look: &Look, cx: &Context<Self>) -> AnyElement {
        let teal = look.color(token::SKETCH_TEAL);
        let pills = taken.iter().filter_map(|one| pill(options, one)).enumerate().map(|(at, said)| {
            labelled(
                div()
                    .id(SharedString::from(format!("look-pill:{at}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .h(look.length(token::H_CONTROL_SM))
                    .pl(look.length(token::SPACE_2))
                    .pr(look.length(token::SPACE_3))
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(teal)
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(glyph(look, "check", Size::Mark, teal))
                    .child(SharedString::from(said.clone())),
                Role::ListItem,
                said,
            )
            .into_any_element()
        });
        let some = !taken.is_empty();
        let (clear_item, send_item) = (row.id.clone(), row.id.clone());
        let clear = labelled(
            button(look, "look-clear", "Clear", Tone::Ghost, false, None, some).when(some, |press| press.on_click(cx.listener(move |this, _, _, cx| this.look_clear(&clear_item, cx)))),
            Role::Button,
            "Clear",
        );
        let send = labelled(
            button(look, "look-send-mix", "Send mix", Tone::Primary, false, Some("\u{21b5}"), some)
                .when(some, |press| press.on_click(cx.listener(move |this, _, window, cx| this.look_send_mix(&send_item, window, cx)))),
            Role::Button,
            "Send mix",
        );
        labelled(
            div()
                .id("look-mix")
                .w_full()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .child(
                    div()
                        .flex_grow(1.)
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Your mix"))
                        .children(pills),
                )
                .child(div().flex_none().ml_auto().flex().items_center().gap(look.length(token::SPACE_2)).child(clear).child(send)),
            Role::Group,
            "Your mix",
        )
        .into_any_element()
    }

    // ---- What a press does -------------------------------------------------------------------------------------

    /// A part taken, or let go where it was taken.
    pub(super) fn look_take_part(&mut self, item: &str, option: u32, part: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if !ways::mixes_parts() {
            return;
        }
        if let Some(dock) = self.dock_mut() {
            toggled(dock.looked.taken.entry(item.to_string()).or_default(), option, part);
        }
        cx.notify();
    }

    /// Clear lets every part go.
    pub(super) fn look_clear(&mut self, item: &str, cx: &mut Context<Self>) {
        if let Some(dock) = self.dock_mut() {
            dock.looked.taken.remove(item);
        }
        cx.notify();
    }

    /// An option taken whole: the normal answer, its choice the option's index.
    pub(super) fn look_take_whole(&mut self, item: &str, option: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let body = ipc::encode(&AnswerWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.to_string(), choice: Some(option as u32), text: None, mode: None, fixed: None });
        self.look_settle(item, "/sessions/waiting/answer", body.map_err(|why| format!("{why:?}")), Vec::new(), window, cx);
    }

    /// Send mix: the parts taken, as `answer_waiting_parts` takes them. With none taken nothing goes.
    pub(super) fn look_send_mix(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let taken = self.dock_of(&job).and_then(|dock| dock.looked.taken.get(item).cloned()).unwrap_or_default();
        if taken.is_empty() {
            return;
        }
        let body = ipc::encode(&body(&job, item, &taken));
        self.look_settle(item, "/sessions/waiting/answer_parts", body.map_err(|why| format!("{why:?}")), taken, window, cx);
    }

    /// `1` to `9` in an open picture question take that option whole. Whether the key was this panel's.
    pub(super) fn look_digit(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.shown.clone() else { return false };
        let Some(key) = self.dock_of(&id).and_then(|dock| dock.detail.clone()) else { return false };
        let Some(item) = key.strip_prefix("item:") else { return false };
        let options = self
            .needs_of(&id, cx)
            .into_iter()
            .flat_map(|view| view.sections.iter().flat_map(|section| section.rows.iter()))
            .find(|row| row.id == item)
            .map(|row| super::dock::row_options(row).to_vec())
            .unwrap_or_default();
        if n == 0 || n > options.len() || !options.iter().any(|option| option.picture.is_some()) {
            return false;
        }
        let item = item.to_string();
        self.look_take_whole(&item, n - 1, window, cx);
        true
    }

    /// The item goes at once; Fleet's refusal brings it back, with its reason said as a toast and the parts as they were.
    fn look_settle(&mut self, item: &str, route: &'static str, body: Result<String, String>, taken: Vec<Taken>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if let Some(dock) = self.dock_mut() {
            dock.hidden.insert(item.to_string());
            dock.detail = None;
            dock.looked.taken.remove(item);
        }
        // On mock data nothing answers, so the view is grouped again without the item.
        self.want_needs(&job, cx);
        let (back, item) = (job.clone(), item.to_string());
        self.post_with(
            job,
            route,
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.dock.hidden.remove(&item);
                    page.dock.detail = Some(format!("item:{item}"));
                    page.dock.open = Some(true);
                    if !taken.is_empty() {
                        page.dock.looked.taken.insert(item, taken);
                    }
                }
                this.want_needs(&back, cx);
                cx.defer(move |cx| crate::toasts::tell(why, cx));
                cx.notify();
            },
            window,
            cx,
        );
        window.focus(&self.dock_focus, cx);
        cx.notify();
    }
}

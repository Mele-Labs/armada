//! The Needs you panel as he reads it when he takes a Job's work in as pictures (`take_in` = look). A press on a row
//! swaps the list for that one item, under a head that reads Needs you and where it stands among the items; Esc or ←
//! goes back and ↑ ↓ move between items. A sketch to review is drawn large with its answers, a walk is a strip of its
//! stops, a command is its steps as a row of chips with the command under them, and a question whose options carry
//! pictures is `look_question.rs`'s. At the default way, `read`, none of this is drawn and the panel is as it was.
//!
//! The keys here are the panel's own (`on_panel_key`), as the keys of its other items are.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Image, ImageFormat, Role, SharedString, Window};
use ipc::{NeedsYouAct, NeedsYouRow, SessionRow, WaitingOption};

use super::acts::{button, Tone};
use super::cards::{settled_of, shown_scene, Subject};
use super::dock::{row_options, Thing};
use super::look_mix::Taken;
use super::rows::{glyph, tip, Size};
use super::wire::{encoded, LeadPicture};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::scene::{Clock, Fit, Lens};
use crate::ways;

/// A still of a question's option, as far as it has been read.
pub(super) enum Still {
    Reading,
    Ready(Arc<Image>),
    /// Fleet would not give it: the option is drawn without one.
    Unread,
}

/// What a finished Job's last picture reads as.
#[derive(Default, Clone)]
pub(super) enum Lead {
    #[default]
    Unasked,
    /// Asked of Fleet for the `done_since` it holds, and not answered yet.
    Asked,
    /// The Job showed none before it said it was done: the card leads with its summary.
    Absent,
    Is(LeadPicture),
}

/// What the panel remembers of one Job's Look: the stills it read, the parts taken into each question's mix, and the
/// picture the done card leads with.
#[derive(Default)]
pub(super) struct Looked {
    pub stills: HashMap<String, Still>,
    pub taken: HashMap<String, Vec<Taken>>,
    pub lead: Lead,
    /// The `done_since` the lead was asked for.
    pub lead_for: Option<String>,
}

/// What an item is when it is opened whole in Look.
pub(super) enum Kind<'a> {
    Pictures(&'a [WaitingOption]),
    Sketch,
    Walk(&'a str),
    Command { command: &'a str, steps: &'a [String] },
}

/// What a row opens as, or `None` where it keeps the press it had.
pub(super) fn kind_of(row: &NeedsYouRow) -> Option<Kind<'_>> {
    match &row.act {
        NeedsYouAct::Answer { options, .. } if options.iter().any(|option| option.picture.is_some()) => Some(Kind::Pictures(options)),
        NeedsYouAct::OpenSketch { .. } => Some(Kind::Sketch),
        NeedsYouAct::Walk { url } => Some(Kind::Walk(url)),
        NeedsYouAct::Run { command } => Some(Kind::Command { command, steps: &[] }),
        NeedsYouAct::Steps { steps, command: Some(command) } => Some(Kind::Command { command, steps }),
        _ => None,
    }
}

/// Whether Look opens the row whole where it is pressed, which every question already did.
pub(super) fn opens_whole(act: &NeedsYouAct) -> bool {
    matches!(act, NeedsYouAct::OpenSketch { .. } | NeedsYouAct::Walk { .. } | NeedsYouAct::Run { .. } | NeedsYouAct::Steps { command: Some(_), .. })
}

/// Whether a thing of the list has a page of its own to open: the items ↑ ↓ move between.
fn has_page(thing: &Thing) -> bool {
    match thing {
        Thing::Ask(_) | Thing::Lapsed(_) => true,
        Thing::Row(row) => !row_options(row).is_empty() || kind_of(row).is_some(),
    }
}

/// The format a still is in, read from its first bytes, for gpui to draw.
fn format_of(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        Some(ImageFormat::Jpeg)
    } else if bytes.starts_with(b"GIF8") {
        Some(ImageFormat::Gif)
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some(ImageFormat::Webp)
    } else {
        None
    }
}

impl JobView {
    /// The things the panel can open whole, in the list's order, by their keys.
    fn look_order(&self, id: &str, cx: &gpui::App) -> Vec<String> {
        self.things(id, cx).0.iter().filter(|thing| has_page(thing)).map(Self::thing_key).collect()
    }

    /// Where Fleet serves a picture an option or the Job carries: an address of its own, or a file the Job showed.
    pub(super) fn still_path(job: &str, image: &str) -> String {
        if image.starts_with("/sessions/") || image.starts_with("/files/") {
            image.to_string()
        } else {
            format!("/sessions/shown?session_id={}&path={}", encoded(job), encoded(image))
        }
    }

    /// Read what Look draws that is not on the wire already: the stills of the Job's questions, and the picture its done
    /// card leads with. Called as the Job's views change; a still is read once.
    pub(super) fn want_looks(&mut self, id: &gpui::SharedString, cx: &mut Context<Self>) {
        if !ways::looks() || self.shown.as_ref() != Some(id) {
            return;
        }
        let wanted: Vec<String> = self
            .needs_of(id, cx)
            .into_iter()
            .flat_map(|view| view.sections.iter().flat_map(|section| section.rows.iter()))
            .flat_map(|row| row_options(row).iter().filter_map(|option| option.picture.as_ref().map(|picture| picture.image.clone())))
            .collect();
        for image in wanted {
            if self.pages.get(id).is_none_or(|page| page.dock.looked.stills.contains_key(&image)) {
                continue;
            }
            if let Some(page) = self.pages.get_mut(id) {
                page.dock.looked.stills.insert(image.clone(), Still::Reading);
            }
            let path = Self::still_path(id, &image);
            self.read_still(id.clone(), image, path, cx);
        }
        self.want_lead(id, cx);
    }

    /// One still, kept under `key`, off the main thread; with no Fleet, the mock's own.
    pub(super) fn read_still(&mut self, job: gpui::SharedString, key: String, path: String, cx: &mut Context<Self>) {
        self.fetch(
            path,
            cx,
            move |this, answer, cx| {
                let bytes = answer.ok().filter(|(status, _)| (200..300).contains(status)).map(|(_, bytes)| bytes);
                this.keep_still(&job, key, bytes, cx);
            },
            |name| crate::mock::still(name).map(<[u8]>::to_vec),
        );
    }

    /// Ask Fleet for `path` off the main thread and run `done` with its answer on it. With no Fleet, as on mock data,
    /// `mocked` answers for the whole path, and the answer lands after the draw that asked.
    pub(super) fn fetch(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut JobView, Result<(u16, Vec<u8>), String>, &mut Context<JobView>) + 'static,
        mocked: impl FnOnce(&str) -> Option<Vec<u8>> + 'static,
    ) {
        let Some(port) = self.port(cx) else {
            let bytes = mocked(&path);
            cx.spawn(async move |this, cx| {
                let _ = this.update(cx, |this, cx| match bytes {
                    Some(bytes) => done(this, Ok((200, bytes)), cx),
                    None => done(this, Ok((404, Vec::new())), cx),
                });
            })
            .detach();
            return;
        };
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    runtime.block_on(crate::transport::get(port, &path))
                })
                .await;
            let _ = this.update(cx, |this, cx| done(this, answer, cx));
        })
        .detach();
    }

    fn keep_still(&mut self, job: &str, image: String, bytes: Option<Vec<u8>>, cx: &mut Context<Self>) {
        let still = match bytes.and_then(|bytes| format_of(&bytes).map(|format| Image::from_bytes(format, bytes))) {
            Some(image) => Still::Ready(Arc::new(image)),
            None => Still::Unread,
        };
        if let Some(page) = self.pages.get_mut(job) {
            page.dock.looked.stills.insert(image, still);
        }
        cx.notify();
    }

    // ---- The panel ---------------------------------------------------------------------------------------------

    /// ↑ ↓ in an open item: the previous or the next item, where there is one.
    pub(super) fn look_step(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let order = self.look_order(&id, cx);
        let here = self.dock_of(&id).and_then(|dock| dock.detail.as_ref()).and_then(|key| order.iter().position(|one| one == key));
        let Some(here) = here else { return };
        let to = here as isize + by;
        if to < 0 || to as usize >= order.len() {
            return;
        }
        self.set_needs(true, Some(order[to as usize].clone()), window, cx);
    }

    /// The head of an open item: Back, the Needs you band with where this item stands, and the way to the next.
    pub(super) fn look_head(&self, id: &SharedString, key: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let order = self.look_order(id, cx);
        let at = order.iter().position(|one| one == key);
        let (before, after) = (at.is_some_and(|at| at > 0), at.is_some_and(|at| at + 1 < order.len()));
        let hue = look.color(token::YOU);
        let step = |name: &'static str, said: &'static str, icon: &'static str, by: isize, on: bool| {
            let press = div()
                .id(name)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .tooltip(tip(said))
                .child(glyph(look, icon, Size::Small, if on { look.color(token::FG_MUTED) } else { look.color(token::FG_SUBTLE) }));
            let press = if on {
                press.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| this.look_step(by, window, cx)))
            } else {
                press.opacity(0.4)
            };
            labelled(press, Role::Button, said)
        };
        let back = labelled(
            div()
                .id("needs-back")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Back"))
                .child(glyph(look, "chevron-left", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| this.set_needs(true, None, window, cx))),
            Role::Button,
            "Back",
        );
        let band = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .pl(look.length(token::SPACE_3))
            .pr(look.length(token::SPACE_1))
            .border_l(look.length(token::BORDER_WIDTH) * 3.)
            .border_color(hue)
            .rounded_r(look.length(token::RADIUS_SM))
            .bg(hue.opacity(0.12))
            .child(glyph(look, "bell-ring", Size::Small, hue))
            .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(hue).child("Needs you"))
            .children(at.map(|at| {
                let place = format!("{} of {}", at + 1, order.len());
                labelled(
                    div().id("needs-place").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(place.clone())),
                    Role::Status,
                    place,
                )
            }))
            .child(div().flex_1())
            .child(step("needs-previous", "Previous item", "chevron-up", -1, before))
            .child(step("needs-next", "Next item", "chevron-down", 1, after));
        div().w_full().flex().items_center().gap(look.length(token::SPACE_2)).child(back).child(band).into_any_element()
    }

    /// The title of an open item, as a question is titled.
    pub(super) fn look_title(&self, text: &str, look: &Look) -> AnyElement {
        div()
            .id("needs-title")
            .text_size(look.length(token::TEXT_LG))
            .line_height(look.length(token::LEADING_LG))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(SharedString::from(text.to_string()))
            .into_any_element()
    }

    /// What Look draws for an open row that has a page of its own, under the head: `None` for a row the deck draws.
    pub(super) fn look_body(&self, id: &SharedString, row: &NeedsYouRow, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let page = match kind_of(row)? {
            Kind::Pictures(options) => self.look_question(id, row, options, look, cx),
            Kind::Sketch => self.look_sketch(id, row, look, cx),
            Kind::Walk(url) => self.look_walk(id, row, url, look, cx),
            Kind::Command { command, steps } => self.look_command(id, row, command, steps, look, cx),
        };
        Some(page)
    }

    // ---- A sketch to review ------------------------------------------------------------------------------------

    /// The sketch drawn large, with the answers it has under it and Open large beside its title.
    fn look_sketch(&self, id: &SharedString, row: &NeedsYouRow, look: &Look, cx: &Context<Self>) -> AnyElement {
        let title = self.look_title(&row.text, look);
        let held = self.thread(id, cx);
        let card = held.and_then(|held| super::review::sketch_card(&held.head, &held.items, &row.id));
        let (Some(card), Some(page)) = (card, self.pages.get(id)) else {
            return div().flex().flex_col().gap(look.length(token::SPACE_3)).child(title).into_any_element();
        };
        let pinned = held.is_some_and(|held| held.head.sketch.as_ref().is_some_and(|pin| pin.id == card.id));
        let key = if pinned { format!("pin:{}", card.id) } else { card.id.clone() };
        let settled = settled_of(card.settled.as_ref(), page.cards.answers.get(&key));
        let open = settled.is_none() && !card.replaced;
        let (scene_key, scene) = shown_scene(card, None);
        let drawn = self.drawn_of(&card.sketch, cx);
        let (read, shown) = (card.id.clone(), key.clone());
        let large = labelled(
            button(look, SharedString::from(format!("look-open-large:{key}")), "Open large", Tone::Ghost, false, None, true)
                .on_click(cx.listener(move |this, _, window, cx| this.read_sketch(&read, &shown, window, cx))),
            Role::Button,
            "Open large",
        );
        let glass = div()
            .id("look-sketch")
            .w_full()
            .h(self.look_sketch_height(id, look))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::SURFACE_WELL))
            .overflow_hidden()
            .children(scene.map(|scene| crate::scene::view(scene_key.into(), scene, Fit::Reading, Clock::default(), Lens::default())));
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).child(div().flex_1().min_w_0().child(title)).child(large))
            .child(labelled(glass, Role::Group, format!("Sketch: {}", card.title)))
            .child(self.approval(super::acts::SHEET, &key, Subject::Sketch { sketch: card.sketch.clone(), drawn }, open, false, settled, false, page, look, cx))
            .into_any_element()
    }

    /// The sketch's height: large, but short enough that its answers stand under it in the panel.
    fn look_sketch_height(&self, id: &SharedString, look: &Look) -> gpui::Pixels {
        let unit = look.length(token::SPACE_12);
        let thread = self.pages.get(id).map(|page| page.list.viewport_bounds().size.height).unwrap_or_default();
        if thread <= gpui::px(0.) {
            return unit * 6.;
        }
        // Up to the panel's 95% of the thread, less the band, the head, the title and the answers.
        (thread * 0.95 - unit * 4.5).max(unit * 3.).min(unit * 8.)
    }

    // ---- A walk ------------------------------------------------------------------------------------------------

    /// The stops of the walk as numbered frames in a strip, and the press that starts it.
    fn look_walk(&self, id: &SharedString, row: &NeedsYouRow, url: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let stops: Vec<String> = self
            .thread_rows(id, cx)
            .into_iter()
            .rev()
            .find_map(|one| match one {
                SessionRow::Window { url: shown, steps, .. } if shown == url => Some(steps.into_iter().map(|step| step.say).collect()),
                _ => None,
            })
            .unwrap_or_default();
        let frames = stops.iter().enumerate().map(|(at, said)| {
            let name = format!("Stop {}: {said}", at + 1);
            labelled(
                div()
                    .id(SharedString::from(format!("look-stop:{at}")))
                    .flex_none()
                    .w(look.length(token::SPACE_12) * 4.)
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::SURFACE_CARD))
                    .shadow(look.shadows(token::SHADOW_CARD))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(look.length(token::H_CONTROL_SM))
                            .rounded_full()
                            .bg(look.color(token::ACCENT_MUTED))
                            .text_size(look.length(token::TEXT_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::ACCENT))
                            .child(SharedString::from((at + 1).to_string())),
                    )
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::FG_DEFAULT))
                            .line_clamp(3)
                            .child(SharedString::from(said.clone())),
                    ),
                Role::ListItem,
                name,
            )
            .into_any_element()
        });
        let url = url.to_string();
        let start = labelled(
            button(look, "look-start-walk", "Start the walk", Tone::Primary, false, None, true).on_click(cx.listener(move |this, _, window, cx| {
                this.set_needs(false, None, window, cx);
                this.press_walk(&url, window, cx);
            })),
            Role::Button,
            "Start the walk",
        );
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).child(div().flex_1().min_w_0().child(self.look_title(&row.text, look))).child(start))
            .child(labelled(
                div().id("look-stops").w_full().flex().gap(look.length(token::SPACE_3)).pb(look.length(token::SPACE_2)).overflow_x_scroll().children(frames),
                Role::List,
                "Stops of the walk",
            ))
            .into_any_element()
    }

    // ---- A command ---------------------------------------------------------------------------------------------

    /// The steps as chips joined by chevrons, the command in a mono well under them, and Run.
    fn look_command(&self, id: &SharedString, row: &NeedsYouRow, command: &str, steps: &[String], look: &Look, cx: &Context<Self>) -> AnyElement {
        let mark = self.dock_of(id).and_then(|dock| dock.runs.get(command).cloned());
        let held = matches!(mark, Some(super::dock::RunMark::Running | super::dock::RunMark::Passed));
        let last = steps.len().saturating_sub(1);
        let chips = steps.iter().enumerate().flat_map(|(at, step)| {
            let chip = labelled(
                div()
                    .id(SharedString::from(format!("look-step:{at}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_3))
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::ACCENT).opacity(0.45))
                    .bg(look.color(token::ACCENT_FAINT))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(step.clone())),
                Role::ListItem,
                format!("Step {}: {step}", at + 1),
            )
            .into_any_element();
            let join = (at < last).then(|| div().flex_none().child(glyph(look, "chevron-right", Size::Small, look.color(token::FG_SUBTLE))).into_any_element());
            std::iter::once(chip).chain(join)
        });
        let command_text = command.to_string();
        let run_text = command.to_string();
        let run = labelled(
            button(look, "look-run", "Run", Tone::Primary, false, None, !held).when(!held, |press| {
                press.on_click(cx.listener(move |this, _, window, cx| this.run_item(run_text.clone(), window, cx)))
            }),
            Role::Button,
            "Run",
        );
        let ran = mark.as_ref().map(|mark| {
            let (icon, said) = mark.look();
            div().id("look-run-mark").flex_none().tooltip(tip(said)).child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
        });
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).child(div().flex_1().min_w_0().child(self.look_title(&row.text, look))).children(ran).child(run))
            .children((!steps.is_empty()).then(|| {
                labelled(div().id("look-steps").w_full().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).children(chips), Role::List, "Steps")
            }))
            .child(labelled(
                div()
                    .id("look-command")
                    .w_full()
                    .p(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::SURFACE_WELL))
                    .font_family(super::rows::mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(command_text)),
                Role::Group,
                "Command",
            ))
            .children(crate::terminal::run::under_item(id, command, cx))
            .into_any_element()
    }
}

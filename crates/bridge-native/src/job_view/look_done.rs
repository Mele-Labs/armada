//! A Job's result as he reads it when he looks (`take_in` = look): the done card leads with the picture the Job last
//! showed before it said it was done, then two lines of what it did, then how the picture is opened, and an Also line
//! with a chip for each other form the Job made. A Job that showed no picture leads with its summary alone.
//!
//! **Listen** is a chip that is drawn off: it has nothing to speak with until Listen is built.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{LeadPicture, ThreadItem, ThreadSketch};

use super::look_dock::{Lead, Still};
use super::look_mix::{also, Form, Produced};
use super::pictures::natural_size;
use super::acts::{button, Tone};
use super::rows::{glyph, tip, Size};
use super::wire::encoded;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::scene::{Clock, Fit, Lens};

/// The sketch card the Job drew under this title: the pinned one, else the newest in the thread.
fn card_titled<'a>(head: &'a ipc::ThreadHead, items: &'a [ThreadItem], title: &str) -> Option<&'a ThreadSketch> {
    head.sketch.as_ref().filter(|card| card.title == title).or_else(|| {
        items.iter().rev().find_map(|item| match item {
            ThreadItem::Sketch(card) if card.title == title => Some(card),
            _ => None,
        })
    })
}

impl JobView {
    /// Ask Fleet which picture a Job that said it is done last showed. Once for each time it said so.
    pub(super) fn want_lead(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let Some(since) = self.record(id, cx).and_then(|record| record.done_since.as_ref().map(|since| since.as_str().to_string())) else { return };
        if !self.bands_of(id, cx).end || self.pages.get(id).is_none_or(|page| page.dock.looked.lead_for.as_deref() == Some(since.as_str())) {
            return;
        }
        if let Some(page) = self.pages.get_mut(id) {
            page.dock.looked.lead_for = Some(since);
            page.dock.looked.lead = Lead::Asked;
        }
        let (job, mocked) = (id.clone(), id.to_string());
        self.fetch(
            format!("/sessions/lead_picture?session_id={}", encoded(id)),
            cx,
            move |this, answer, cx| {
                // A 404 is a Job that showed none, and so is an answer that does not read.
                let lead = answer.ok().filter(|(status, _)| (200..300).contains(status)).and_then(|(_, bytes)| ipc::decode::<LeadPicture>("LeadPicture", &bytes).ok());
                if let Some(LeadPicture::Image { path }) = &lead {
                    this.read_still(job.clone(), format!("lead:{path}"), Self::still_path(&job, path), cx);
                }
                if let Some(page) = this.pages.get_mut(&job) {
                    page.dock.looked.lead = lead.map_or(Lead::Absent, Lead::Is);
                }
                cx.notify();
            },
            move |_| crate::mock::lead_picture(&mocked),
        );
    }

    /// What the done card carries in Look: the lead, the summary and how the picture opens, and the Also line.
    /// `lead` goes above the card's head and `rest` under it, where the summary stood.
    pub(super) fn look_result(&self, id: &SharedString, summary: Option<&str>, look: &Look, cx: &Context<Self>) -> (Option<AnyElement>, Vec<AnyElement>) {
        // A picture of a kind native cannot draw (svg, heic) reads as none: no lead and no Picture chip.
        let lead = self.dock_of(id).and_then(|dock| match &dock.looked.lead {
            Lead::Is(LeadPicture::Image { path }) if matches!(dock.looked.stills.get(&format!("lead:{path}")), Some(Still::Unread)) => None,
            Lead::Is(picture) => Some(picture.clone()),
            _ => None,
        });
        let page = self.thread(id, cx).and_then(|held| held.items.iter().rev().find_map(|item| match item {
            ThreadItem::Document(card) => Some(card.page.clone()),
            _ => None,
        }));
        let mut rest = Vec::new();
        if let Some(text) = summary {
            rest.push(
                div()
                    .id("done-summary")
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .line_clamp(2)
                    .text_ellipsis()
                    .child(SharedString::from(text.to_string()))
                    .into_any_element(),
            );
        }
        let still = matches!(lead, Some(LeadPicture::Sketch { .. } | LeadPicture::Image { .. }));
        if still {
            rest.push(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child("Shown as a still. Press to open it large.").into_any_element());
        }
        let forms = also(Produced { page: page.is_some(), picture: still, summary: summary.is_some() });
        if !forms.is_empty() {
            rest.push(self.look_also(&forms, page, look, cx));
        }
        (lead.map(|lead| self.look_lead(id, &lead, look, cx)), rest)
    }

    /// The picture the Job last showed, drawn at the head of the card; a press opens it large, or for a window starts the walk.
    fn look_lead(&self, id: &SharedString, lead: &LeadPicture, look: &Look, cx: &Context<Self>) -> AnyElement {
        // Large, yet short enough that the summary under it shows in the same view of the thread. The thread's own height
        // cannot be read here: the card is drawn inside the list that holds it.
        let height = look.length(token::SPACE_12) * 4.;
        let frame = || {
            div()
                .id("done-lead")
                .w_full()
                .h(height)
                .p(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::SURFACE_WELL))
                .overflow_hidden()
                .cursor_pointer()
        };
        match lead {
            LeadPicture::Sketch { title } => {
                let card = self.thread(id, cx).and_then(|held| card_titled(&held.head, &held.items, title));
                let drawn = card
                    .map(|card| super::cards::shown_scene(card, None))
                    .and_then(|(key, scene)| scene.map(|scene| crate::scene::view(key.into(), scene, Fit::Reading, Clock::default(), Lens::default())));
                labelled(frame().children(drawn).on_click(cx.listener(|this, _, window, cx| this.open_lead(window, cx))), Role::Button, format!("Open {title} large")).into_any_element()
            }
            LeadPicture::Image { path } => {
                let name = path.rsplit('/').next().unwrap_or(path).to_string();
                let image = match self.dock_of(id).and_then(|dock| dock.looked.stills.get(&format!("lead:{path}"))) {
                    Some(Still::Ready(image)) => Some(image.clone()),
                    _ => None,
                };
                labelled(
                    frame()
                        .children(image.map(|image| gpui::img(image).size_full().object_fit(gpui::ObjectFit::Contain)))
                        .on_click(cx.listener(|this, _, window, cx| this.open_lead(window, cx))),
                    Role::Button,
                    format!("Open {name} large"),
                )
                .into_any_element()
            }
            LeadPicture::Window { url } => {
                let url = url.clone();
                labelled(
                    div()
                        .id("done-lead")
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .p(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .bg(look.color(token::SURFACE_WELL))
                        .child(glyph(look, "footprints", Size::Small, look.color(token::FG_MUTED)))
                        .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(url.clone())))
                        .child(labelled(
                            button(look, "done-walk-it", "Walk it", Tone::Secondary, false, None, true).on_click(cx.listener(move |this, _, window, cx| this.press_walk(&url, window, cx))),
                            Role::Button,
                            "Walk it",
                        )),
                    Role::Group,
                    "The window the Job showed",
                )
                .into_any_element()
            }
        }
    }

    /// The lead picture opened large: a sketch in the sketch reader, a file over the window as a sent picture is.
    fn open_lead(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(Lead::Is(lead)) = self.dock_of(&id).map(|dock| dock.looked.lead.clone()) else { return };
        match lead {
            LeadPicture::Sketch { title } => {
                let card = self.thread(&id, cx).and_then(|held| card_titled(&held.head, &held.items, &title)).map(|card| card.id.clone());
                if let Some(card) = card {
                    self.read_sketch(&card, &card, window, cx);
                }
            }
            LeadPicture::Image { path } => {
                let image = match self.dock_of(&id).and_then(|dock| dock.looked.stills.get(&format!("lead:{path}"))) {
                    Some(Still::Ready(image)) => Some(image.clone()),
                    _ => None,
                };
                if let Some(image) = image {
                    let natural = natural_size(&image.bytes).map_or(gpui::size(960., 600.), |(w, h)| gpui::size(w as f32, h as f32));
                    let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                    self.open_picture(image, natural, name.into(), window, cx);
                }
            }
            LeadPicture::Window { url } => self.press_walk(&url, window, cx),
        }
    }

    /// Also: a chip for each other form the Job produced.
    fn look_also(&self, forms: &[Form], page: Option<String>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let chip = |id: &'static str, icon: &'static str, said: &'static str, on: bool| {
            let press = div()
                .id(id)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::SURFACE_CARD))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED)))
                .child(said);
            if on {
                press.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER)))
            } else {
                press.opacity(0.4)
            }
        };
        let chips = forms.iter().map(|form| match form {
            Form::Page => {
                let page = page.clone();
                labelled(
                    chip("done-also-page", "file-text", "Page", true).on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(page) = &page {
                            this.open_document_tab(page, window, cx);
                        }
                    })),
                    Role::Button,
                    "Page",
                )
                .into_any_element()
            }
            Form::Picture => labelled(chip("done-also-picture", "image", "Picture", true).on_click(cx.listener(|this, _, window, cx| this.open_lead(window, cx))), Role::Button, "Picture").into_any_element(),
            // Nothing speaks yet: the chip is drawn off, with the reason where the pointer rests.
            Form::Listen => labelled(chip("done-also-listen", "volume-2", "Listen", false).tooltip(tip("Listen is not available yet")), Role::Button, "Listen").into_any_element(),
        });
        labelled(
            div()
                .id("done-also")
                .flex()
                .flex_wrap()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Also"))
                .children(chips),
            Role::Group,
            "Also",
        )
        .into_any_element()
    }
}

//! The Waits for card: what he sent a Job on another Mac while that Mac could not be reached, pinned at the
//! thread's end with a Take back on each (`crate::waiting_sends`).

use chrono::{Local, TimeZone};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{MacLink, Queued};

use super::acts::{button, Tone};
use super::rows::{glyph, tip, Size};
use super::wire::Done;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::waiting_sends::{said_of, TAKE_BACK};

impl JobView {
    /// The sends waiting for the Job's Mac, oldest first: Fleet's list, or on mock data the mock's.
    pub(super) fn sends_waiting(&self, id: &str, cx: &gpui::App) -> Vec<Queued> {
        let list = match &self.fleet {
            Some(fleet) => fleet.read(cx).held().queued.as_ref(),
            None => Some(&self.mock_queued),
        };
        list.map(|list| crate::waiting_sends::of_job(list, id).into_iter().cloned().collect()).unwrap_or_default()
    }

    /// The card: its head names the Mac and, in its tooltip, since when it sleeps; a row for each send says what
    /// it is, when he made it, and takes it back. A send that Mac refused says why under it, in the warning hue.
    pub(super) fn waits_for_card(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let sends = self.sends_waiting(id, cx);
        let first = sends.first()?;
        let mac = crate::pairing::macs(cx).iter().find(|one| one.id == first.mac);
        let name = mac.map_or_else(|| "another Mac".to_string(), |one| one.name.clone());
        let asleep = mac.and_then(|one| match one.link {
            Some(MacLink::Asleep { since }) => Some(crate::pairing::asleep_said(since)),
            _ => None,
        });
        let mac_hue = mac.map_or_else(|| look.color(token::FG_MUTED), |one| crate::settings::macs::colour_of(look, one.colour));
        let refused = sends.iter().any(|one| one.refused.is_some());
        // The card's state: grey while each send only waits, the warning hue once that Mac refused one.
        let hue = look.color(if refused { token::STATUS_AWAITING_REVIEW } else { token::STATUS_NOT_STARTED });
        let state_said = if refused { "Refused" } else { "Waiting" };
        let waiting = self.record(id, cx).map(|job| job.waiting_for.as_slice()).unwrap_or_default();
        // Wide, each row is one line with the kinds in a column; narrow, as the message box narrows (under 400px), the
        // time goes under the words so Take back keeps its place at the end.
        let narrow = self.pane.is_some_and(|pane| (pane - look.length(token::SPACE_4) * 2.).min(self.column(look)) < gpui::px(400.));
        let kind_w = if narrow { gpui::px(0.) } else { look.length(token::SPACE_12) * 2. };
        let rows: Vec<AnyElement> = sends
            .iter()
            .map(|one| {
                let said = said_of(&one.act, waiting);
                let at = Local.timestamp_millis_opt(one.since).single().map(crate::settings::time::stamp).unwrap_or_default();
                let (chip_hue, chip_ground) = match one.refused {
                    Some(_) => (look.color(token::STATUS_AWAITING_REVIEW), look.color(token::STATUS_AWAITING_REVIEW_BG)),
                    None => (mac_hue, mac_hue.opacity(0.16)),
                };
                let sent = one.id.clone();
                let goes: SharedString = format!("Take it back before it goes to {name}").into();
                let name: SharedString = match &said.detail {
                    Some(detail) => format!("{} {detail}", said.kind).into(),
                    None => said.kind.clone().into(),
                };
                let time = div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(at));
                let (inline, under) = if narrow { (None, Some(time)) } else { (Some(time), None) };
                let line = div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_h(look.length(token::H_ROW))
                    .child(div().flex_none().min_w(kind_w).flex().child(crate::operator::view::chip(said.kind.clone(), chip_hue, chip_ground, look)))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).children(said.detail.clone().map(SharedString::from)))
                    .children(inline)
                    .child(labelled(
                        button(look, SharedString::from(format!("take-back:{}", one.id)), "Take back", Tone::Secondary, false, None, true)
                            .tooltip(tip(goes))
                            .on_click(cx.listener(move |this, _, window, cx| this.take_back(sent.clone(), window, cx))),
                        Role::Button,
                        format!("Take back {}", said.kind),
                    ));
                let why = one.refused.clone().map(|why| {
                    labelled(
                        div()
                            .id(SharedString::from(format!("send-refused:{}", one.id)))
                            .pl(kind_w + look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_SM))
                            .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                            .child(SharedString::from(why)),
                        Role::Status,
                        "Refused",
                    )
                });
                labelled(
                    div()
                        .id(SharedString::from(format!("send:{}", one.id)))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .py(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .child(line)
                        .children(under)
                        .children(why),
                    Role::Row,
                    name,
                )
                .into_any_element()
            })
            .collect();
        let title: SharedString = format!("Waits for {name}").into();
        let head = div()
            .id("waits-for-head")
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .h(look.length(token::H_ROW))
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(look.color(token::FG_MUTED))
            .child(div().id("waits-for-state").flex_none().child(glyph(look, "clock", Size::Small, hue)).tooltip(tip(state_said)))
            .child("Waits for")
            .child(div().text_color(mac_hue).child(SharedString::from(name.clone())))
            .when_some(asleep, |head, said| head.tooltip(tip(said)));
        Some(
            labelled(
                div()
                    .id("waits-for")
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .p(look.length(token::SPACE_2))
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(hue)
                    .bg(look.color(token::BG_RAISED))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .child(labelled(head, Role::Heading, title.clone()))
                    .children(rows),
                Role::Region,
                title,
            )
            .into_any_element(),
        )
    }

    /// Take back: Fleet drops the send and answers with what still waits. A refusal is a toast in Fleet's words.
    pub(super) fn take_back(&mut self, sent: String, window: &mut Window, cx: &mut Context<Self>) {
        let body = crate::waiting_sends::take_back_body(&sent);
        let done: Done = Box::new(move |this, answer, _, cx| {
            match answer {
                Ok(bytes) => match this.fleet.clone() {
                    Some(fleet) => {
                        if let Ok(list) = ipc::decode::<ipc::QueuedList>("QueuedList", &bytes) {
                            fleet.update(cx, |fleet, cx| fleet.hold_queued(list, cx));
                        }
                    }
                    None => this.mock_queued.queued.retain(|one| one.id != sent),
                },
                Err(why) => cx.defer(move |cx| crate::toasts::tell(why, cx)),
            }
            if let Some(id) = this.shown.clone() {
                this.sync(&id, cx);
            }
            cx.notify();
        });
        self.post(TAKE_BACK.into(), body, window, cx, done);
    }

    /// Mock data: a send to a Job on a Mac asleep is kept, as Fleet keeps it, and stands in the card. Whether it was.
    pub(super) fn mock_keep(&mut self, id: &SharedString, path: &str, body: &str, cx: &mut Context<Self>) -> bool {
        let Some(mac) = self.record(id, cx).and_then(|job| job.mac.clone()) else { return false };
        if crate::pairing::chip(Some(&mac), cx).and_then(|chip| chip.asleep_since).is_none() {
            return false;
        }
        let Some(act) = crate::waiting_sends::act_of(path, body) else { return false };
        let since = chrono::Utc::now().timestamp_millis();
        self.mock_queued.queued.push(Queued { id: format!("mock-kept-{since}"), mac, act, since, refused: None });
        self.sync(id, cx);
        cx.notify();
        true
    }
}

//! Another Mac waiting for his Confirm to pair: a blocked card atop Your turn, one for each `mac_pairing`
//! call Fleet holds. **The acts are the Phone Gateway's** (`StandingCallKind::MacPairing`): Confirm posts
//! `/admin/macs/pair/confirm` and Decline `/admin/macs/forget`, each naming the call's `mac`. The call leaves
//! when Fleet next publishes, or when the ask's five minutes run out; a refusal's sentence stays on the card.
//! Settings › Macs keeps its own Confirm on the Mac's row.

use gpui::{div, prelude::*, AnyElement, App, Context, FontWeight, Role, SharedString, Task, Window};
use ipc::{StandingCall, StandingCallKind};

use super::face::Kind;
use super::page::OperatorPageView;
use super::view::{kind_chip, press, pressed, round, Weight};
use super::wire::Wire;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, tip};

const PAIRING: Kind = Kind { word: "Mac pairing", glyph: "monitor", hue: token::STATUS_AWAITING_REVIEW, ground: token::STATUS_AWAITING_REVIEW_BG };

impl OperatorPageView {
    /// The Macs waiting for his Confirm to pair, oldest first, as Fleet's calls hold them.
    pub fn pairings(&self, cx: &App) -> Vec<StandingCall> {
        let calls = match &self.wire {
            Wire::Live(fleet) => fleet.read(cx).held().calls.as_ref().map(|calls| calls.calls.clone()).unwrap_or_default(),
            Wire::Mock(mock) => mock.pairings.clone(),
        };
        calls.into_iter().filter(|call| call.kind == StandingCallKind::MacPairing).collect()
    }

    /// A banner's press, or a walk: that Mac's card at the top of the page, and the keys on its Confirm at the
    /// next frame, after the screen has brought the page to the front and given it the keys.
    pub fn show_pairing(&mut self, key: &str, cx: &mut Context<Self>) {
        self.pairing_front = Some(key.to_string());
        self.pairing_reveal = true;
        cx.notify();
    }

    /// The front card's reveal, at the frame after `show_pairing`.
    pub(super) fn reveal_pairing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.pairing_reveal) {
            return;
        }
        self.main_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
        let keys = self.pairing_focus.clone();
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
    }

    /// Confirm (`yes`) or Decline the Mac waiting under `key`. Once at a time; a refusal stays on its card.
    pub fn answer_pairing(&mut self, key: &str, yes: bool, cx: &mut Context<Self>) {
        let Some(mac) = self.pairings(cx).into_iter().find(|call| call.key == key).and_then(|call| call.mac) else { return };
        if !self.pairing_pending.insert(key.to_string()) {
            return;
        }
        if self.pairing_refused.as_ref().is_some_and(|(on, _)| on == key) {
            self.pairing_refused = None;
        }
        cx.notify();
        let key = key.to_string();
        if let Wire::Mock(mock) = &mut self.wire {
            let refused = mock.refuse_pairing.take();
            if refused.is_none() {
                mock.pairings.retain(|call| call.key != key);
            }
            self.pairing_settled(&key, refused, cx);
            return;
        }
        let asked: Task<Result<(), String>> = if yes {
            let confirm = crate::settings::gateway::pair_confirm(mac, cx);
            cx.background_spawn(async move { confirm.await.map(|_| ()) })
        } else {
            crate::settings::gateway::forget(mac, cx)
        };
        cx.spawn(async move |this, cx| {
            let refused = asked.await.err();
            let _ = this.update(cx, |this, cx| this.pairing_settled(&key, refused, cx));
        })
        .detach();
    }

    /// The Gateway answered. A yes stays pressed until Fleet's calls drop the Mac; a no frees the presses.
    fn pairing_settled(&mut self, key: &str, refused: Option<String>, cx: &mut Context<Self>) {
        if let Some(why) = refused {
            Note::warn("a Mac's pairing was refused").field("why", why.as_str()).emit();
            self.pairing_pending.remove(key);
            self.pairing_refused = Some((key.to_string(), why.into()));
        } else if matches!(self.wire, Wire::Mock(_)) {
            self.pairing_pending.remove(key);
        }
        cx.notify();
    }

    /// Each Mac waiting to pair, a card atop Your turn.
    pub(super) fn pairing_cards(&self, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let calls = self.pairings(cx);
        let front = self.pairing_front.as_deref().filter(|key| calls.iter().any(|call| call.key == *key)).or(calls.first().map(|call| call.key.as_str())).map(str::to_string);
        calls.iter().map(|call| self.pairing_card(call, front.as_deref() == Some(call.key.as_str()), look, cx)).collect()
    }

    fn pairing_card(&self, call: &StandingCall, front: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = call.key.clone();
        let sent = self.pairing_pending.contains(&key);
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(kind_chip(PAIRING, look))
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(call.title.clone()))
            .child(self.stamp(call.since.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx));
        let refused = self.pairing_refused.as_ref().filter(|(on, _)| *on == key).map(|(_, why)| {
            labelled(
                div()
                    .id(SharedString::from(format!("pairing-refused:{key}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .child(glyph("circle-alert", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_FAILED)))
                    .child(div().flex_1().min_w_0().child(why.clone())),
                Role::Alert,
                why.clone(),
            )
        });
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .child(div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(call.text.clone()))
            .children(refused);
        let act = |id: &str, label: &'static str, icon: &'static str, weight: Weight, said: &'static str, yes: bool| {
            let button = press(format!("{id}:{key}"), label.into(), Some(icon), None, weight, look).tooltip(tip(said));
            let button = if front && yes { button.track_focus(&self.pairing_focus) } else { button };
            let button = if sent {
                button.opacity(0.5).cursor_default()
            } else {
                let key = key.clone();
                pressed(button, cx, move |this, _, cx| this.answer_pairing(&key, yes, cx))
            };
            labelled(button, Role::Button, label).into_any_element()
        };
        let acts = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(act("pairing-confirm", "Confirm", "check", Weight::Pick, "Pairs the two Macs", true))
            .child(act("pairing-decline", "Decline", "x", Weight::Plain, "Forgets the ask here and on the other Mac", false));
        let card = div()
            .id(SharedString::from(format!("pairing:{key}")))
            .flex_none()
            .rounded(round(look))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if front { token::STATUS_AWAITING_REVIEW } else { token::OPERATOR_EDGE }))
            .bg(look.color(token::BG_RAISED))
            .overflow_hidden()
            .child(head)
            .child(body)
            .child(acts);
        labelled(card, Role::Article, call.title.clone()).into_any_element()
    }
}

/// A Mac waiting to pair, as Fleet's calls carry it: for the walk's scenes and the tests.
pub fn waiting(mac: &str, name: &str, code: &str, since: &str) -> StandingCall {
    let json = format!(
        r#"{{"key":"mac:{mac}","owner":"mac:{mac}","kind":"mac_pairing","title":"Confirm {name} pairing","text":"{name} wants to pair. Confirm if it shows {code}.","since":"{since}","tone":"blocked","opened":false,"mac":"{mac}"}}"#
    );
    ipc::decode::<StandingCall>("StandingCall", json.as_bytes()).unwrap_or_else(|why| panic!("a pairing call does not decode: {why}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Entity, TestAppContext, VisualTestContext};

    fn open(cx: &mut TestAppContext) -> (Entity<OperatorPageView>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (page, cx) = cx.add_window_view(|window, cx| OperatorPageView::mock(false, window, cx));
        page.update(cx, |page, _| {
            let mock = page.mock_fleet().expect("the walk's Fleet");
            mock.pairings = vec![waiting("0a0b", "Work", "4821", "2025-11-01T12:26:40.000Z")];
        });
        (page, cx)
    }

    #[gpui::test]
    fn a_confirmed_mac_leaves_its_card(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| {
            assert_eq!(page.pairings(cx).len(), 1);
            page.answer_pairing("mac:0a0b", true, cx);
            assert!(page.pairings(cx).is_empty());
            assert!(page.pairing_refused.is_none());
        });
    }

    #[gpui::test]
    fn a_refusal_keeps_the_card_with_the_gateways_sentence_and_frees_its_presses(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| {
            page.mock_fleet().expect("the walk's Fleet").refuse_pairing = Some("Work stopped asking.".into());
            page.answer_pairing("mac:0a0b", false, cx);
            assert_eq!(page.pairings(cx).len(), 1);
            assert_eq!(page.pairing_refused.as_ref().map(|(key, why)| (key.as_str(), why.as_ref())), Some(("mac:0a0b", "Work stopped asking.")));
            assert!(page.pairing_pending.is_empty());
        });
    }

    #[gpui::test]
    fn a_job_call_draws_no_pairing_card(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| {
            let mut job = waiting("0c0d", "Air", "1234", "2025-11-01T12:26:40.000Z");
            job.kind = StandingCallKind::Job;
            page.mock_fleet().expect("the walk's Fleet").pairings.push(job);
            assert_eq!(page.pairings(cx).iter().map(|call| call.key.as_str()).collect::<Vec<_>>(), ["mac:0a0b"]);
        });
    }
}

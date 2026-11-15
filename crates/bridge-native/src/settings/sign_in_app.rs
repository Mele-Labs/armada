//! Settings › Account: the press that registers Armada's sign-in app on GitHub. While `account.clientId` is blank the
//! page draws a row with "Register on GitHub", which asks Fleet for the page that carries the app's manifest to the
//! forge (`crates/ipc/operations/register_sign_in_app.toml`) and opens it in his browser. Fleet writes the client id
//! when the forge sends the browser back, `settings.changed` reaches the page, and the row goes.
//! **A refusal is said in Fleet's own words** under the press, and the press is back, as the Claims store's are.

use std::time::Duration;

use gpui::{div, prelude::*, svg, Animation, AnimationExt, AnyElement, App, Context, FocusHandle, FontWeight, KeyDownEvent, Role, SharedString, Task, Window};
use ipc::{SettingValue, SignInAppRegistration};

use super::overlay::SettingsOverlay;
use super::rows::{says, split_row, Row, Spread};
use super::store::refused_line;
use super::view::ringed;
use super::wire::Refused;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

const KEY: &str = "account.clientId";
const REGISTER: &str = "Register on GitHub";
const OPENING: &str = "Opening";
const TITLE: &str = "Sign-in app";
const WHAT: &str = "GitHub makes the app on your account and opens its settings page, where you tick Enable Device Flow.";

/// The press's own state: whether Fleet has been asked and not yet answered, what it refused with, and the press's key.
pub struct SignInApp {
    pub(super) out: bool,
    pub(super) refused: Option<String>,
    pub(super) focus: FocusHandle,
    _act: Option<Task<()>>,
}

impl SignInApp {
    pub fn new(cx: &mut App) -> SignInApp {
        SignInApp { out: false, refused: None, focus: cx.focus_handle().tab_stop(true), _act: None }
    }
}

/// What the mock Fleet answers the press with, and the presses it has had.
pub struct MockSignInApp {
    pub calls: usize,
    /// A sentence the press is refused with, as Fleet would say it.
    pub refusal: Option<String>,
    /// How long the answer takes, so a walk can draw the press while it is out.
    pub wait: Option<Duration>,
}

impl MockSignInApp {
    pub fn new() -> MockSignInApp {
        MockSignInApp { calls: 0, refusal: None, wait: None }
    }

    pub fn register(&mut self) -> (Result<SignInAppRegistration, Refused>, Option<Duration>) {
        self.calls += 1;
        let answer = match &self.refusal {
            Some(said) => Err(Refused::said(said.clone())),
            None => Ok(SignInAppRegistration { url: "http://127.0.0.1:7777/accounts/sign_in_app/form?state=mock".into() }),
        };
        (answer, self.wait)
    }
}

impl SettingsOverlay {
    /// Whether the app has no client id yet, as the last read of Fleet's settings says.
    pub(super) fn sign_in_app_owed(&self) -> bool {
        self.setting(KEY).is_some_and(|one| matches!(super::limits::current(one), SettingValue::Text(id) if id.trim().is_empty()))
    }

    /// The press has the keys: it is a stop of the page, between settings.json and the fields.
    pub(super) fn sign_in_app_stops(&self) -> Vec<FocusHandle> {
        if self.sign_in_app_owed() { vec![self.sign_in_app.focus.clone()] } else { Vec::new() }
    }

    /// `POST /accounts/sign_in_app`, then the page it answers with opens in his browser. One at a time.
    pub(super) fn register_sign_in_app(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sign_in_app.out || !self.live {
            return;
        }
        self.sign_in_app.out = true;
        self.sign_in_app.refused = None;
        let press = self.wire.register_sign_in_app(cx);
        self.sign_in_app._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.sign_in_app.out = false;
                match answer {
                    Ok(registration) => this.wire.open_link(&registration.url, cx),
                    Err(refused) => this.sign_in_app.refused = Some(refused.said),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// The row, or none once the client id is filled.
    pub(super) fn sign_in_app_row(&self, spread: Spread, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.sign_in_app_owed() {
            return None;
        }
        let busy = self.sign_in_app.out;
        let off = busy || !self.live;
        let said = if busy { OPENING } else { REGISTER };
        let ink = look.color(token::FG_INVERSE);
        let mark = if busy {
            svg()
                .flex_none()
                .path(icon_path("loader-circle"))
                .size(look.length(token::SPACE_4))
                .text_color(ink)
                .with_animation("sign-in-app-busy", Animation::new(look.duration(token::DURATION_SPIN)).repeat(), |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))))
                .into_any_element()
        } else {
            svg().flex_none().path(icon_path("external-link")).size(look.length(token::SPACE_4)).text_color(ink).into_any_element()
        };
        let focus = &self.sign_in_app.focus;
        let button = div()
            .id("sign-in-app-register")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::ACCENT))
            .bg(look.color(token::ACCENT))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(ink)
            .when(off && !busy, |button| button.opacity(0.5))
            .when(!off, |button| {
                let hover = look.color(token::ACCENT_HOVER);
                button
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .on_click(cx.listener(|this, _, window, cx| this.register_sign_in_app(window, cx)))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.register_sign_in_app(window, cx);
                            cx.stop_propagation();
                        }
                    }))
            })
            .child(mark)
            .child(said);
        let button = labelled(ringed(look, button, focus, window).track_focus(focus), Role::Button, said).into_any_element();
        let control = div().flex().flex_col().items_start().gap(look.length(token::SPACE_2)).child(button).children(self.sign_in_app.refused.as_ref().map(|said| refused_line(look, "sign-in-app-refused", said)));
        let row = Row { id: "sign-in-app".into(), title: SharedString::from(TITLE), tip: None, chips: Vec::new(), says: vec![says(look, WHAT).into_any_element()], changed: false, glyph: None, control: control.into_any_element() };
        Some(split_row(look, spread, row, true).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext, VisualTestContext};

    use super::super::mock::MockFleet;
    use super::super::wire::Wire;
    use super::*;

    const REFUSED: &str = "GitHub could not be reached from this Mac.";

    fn opened<'a>(cx: &'a mut TestAppContext, with: impl FnOnce(&mut MockFleet)) -> (Entity<SettingsOverlay>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let mut fleet = MockFleet::new();
        with(&mut fleet);
        let list = fleet.list.clone();
        let (overlay, cx) = cx.add_window_view(|window, cx| SettingsOverlay::new(Wire::Mock(Box::new(fleet)), window, cx));
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                this.open = true;
                this.live = true;
                this.picked = "fleet:account";
                this.set_list(list, window, cx);
            });
        });
        cx.run_until_parked();
        (overlay, cx)
    }

    fn on_page(overlay: &Entity<SettingsOverlay>, role: Role, name: &str, cx: &mut VisualTestContext) -> bool {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = overlay;
            window.draw(cx).clear(cx);
            cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some())
        })
    }

    fn press(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) {
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.register_sign_in_app(window, cx)));
        cx.run_until_parked();
    }

    fn calls(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> (usize, Vec<String>) {
        cx.update(|_, cx| match &overlay.read(cx).wire {
            Wire::Mock(mock) => (mock.sign_in_app.calls, mock.handed.clone()),
            Wire::Live(_) => (0, Vec::new()),
        })
    }

    #[gpui::test]
    fn the_press_posts_once_and_opens_the_page_it_is_answered_with(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |_| {});
        assert!(on_page(&overlay, Role::Button, REGISTER, cx));
        press(&overlay, cx);
        let (posted, opened) = calls(&overlay, cx);
        assert_eq!(posted, 1);
        assert_eq!(opened, ["http://127.0.0.1:7777/accounts/sign_in_app/form?state=mock"]);
    }

    #[gpui::test]
    fn a_second_press_while_the_first_is_out_posts_nothing(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |mock| mock.sign_in_app.wait = Some(Duration::from_secs(5)));
        press(&overlay, cx);
        press(&overlay, cx);
        assert!(overlay.read_with(cx, |this, _| this.sign_in_app.out));
        assert_eq!(calls(&overlay, cx).0, 1);
        assert!(on_page(&overlay, Role::Button, OPENING, cx));
    }

    #[gpui::test]
    fn the_row_is_not_drawn_once_a_client_id_is_set(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |mock| {
            if let Some(one) = mock.list.settings.iter_mut().find(|one| one.key == KEY) {
                one.value = SettingValue::Text("Iv1.abc123".into());
            }
        });
        assert!(!overlay.read_with(cx, |this, _| this.sign_in_app_owed()));
        assert!(!on_page(&overlay, Role::Button, REGISTER, cx));
    }

    #[gpui::test]
    fn a_refusal_shows_fleets_sentence_and_gives_the_press_back(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |mock| mock.sign_in_app.refusal = Some(REFUSED.into()));
        press(&overlay, cx);
        assert!(on_page(&overlay, Role::Alert, REFUSED, cx));
        assert!(on_page(&overlay, Role::Button, REGISTER, cx));
        assert_eq!(calls(&overlay, cx), (1, Vec::new()));
        // The press may be tried again.
        press(&overlay, cx);
        assert_eq!(calls(&overlay, cx).0, 2);
    }
}

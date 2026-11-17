//! Site settings behind the lock: what the page's site may use, and what every site may use unless it chose for itself
//! (`SiteLock`, `SiteSettingsPanel` in `job-browser/JobBrowserPane.tsx`; `SitePermissionRows.tsx`).
//!
//! The lock is a button before the address. It opens a popover with two views, This site and All sites, over nine rows of
//! Ask, Allow and Block. **Allow on Local network is confirmed first**, in a dialog of its own while the popover is put away:
//! Esc cancels and Enter confirms.
//!
//! **Fully keyboard driven**: Tab goes from one group of segments to the next and stops once in each, on the chosen segment;
//! the arrows move the choice inside a group, as they do in Classic's radio groups; Esc closes and gives the keys back to the lock.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    actions, anchored, canvas, deferred, div, point, prelude::*, svg, AnyElement, Bounds, Context, FocusHandle, FontWeight, KeyBinding, Pixels, Role,
    SharedString, Window,
};

use super::choices::{host_of, Choice, Permission, Settings};
use super::tab::BrowserTab;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

actions!(job_browser_site, [SegmentNext, SegmentPrevious, CloseSite, ConfirmAllow, CancelAllow]);

pub const SITE_CONTEXT: &str = "job-browser-site";
pub const SEGMENTS_CONTEXT: &str = "job-browser-segments";
pub const CONFIRM_CONTEXT: &str = "job-browser-confirm";

/// The panel's keys, which are its own where its focus is: the arrows within a group, Esc to close, Enter to confirm.
pub fn bind(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("right", SegmentNext, Some(SEGMENTS_CONTEXT)),
        KeyBinding::new("down", SegmentNext, Some(SEGMENTS_CONTEXT)),
        KeyBinding::new("left", SegmentPrevious, Some(SEGMENTS_CONTEXT)),
        KeyBinding::new("up", SegmentPrevious, Some(SEGMENTS_CONTEXT)),
        KeyBinding::new("escape", CloseSite, Some(SITE_CONTEXT)),
        KeyBinding::new("enter", ConfirmAllow, Some(CONFIRM_CONTEXT)),
        KeyBinding::new("escape", CancelAllow, Some(CONFIRM_CONTEXT)),
    ]);
}

/// Which choices a row sets: this site's, or the default for every site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Site,
    All,
}

impl Scope {
    pub const ALL: [Scope; 2] = [Scope::Site, Scope::All];

    pub fn label(self) -> &'static str {
        match self {
            Scope::Site => "This site",
            Scope::All => "All sites",
        }
    }
}

/// The glyph each permission is known by. Camera and Microphone borrow Image and Radio, and Fullscreen borrows Monitor, as Classic's
/// rows do: the registry has none of them.
pub fn glyph_of(permission: Permission) -> &'static str {
    match permission {
        Permission::LocalNetwork => "network",
        Permission::Camera => "image",
        Permission::Microphone => "radio",
        Permission::Notifications => "bell",
        Permission::Clipboard => "clipboard-list",
        Permission::Location => "map-pin",
        Permission::Downloads => "download",
        Permission::Popups => "external-link",
        Permission::Fullscreen => "monitor",
    }
}

/// What a Local network confirmation says, for one site or for every one (`said`).
pub fn confirm_words(scope: Scope, host: &str) -> (String, &'static str) {
    match scope {
        Scope::Site => (
            format!("Allow local network for {host}?"),
            "This site can reach apps on this Mac and devices on your network. That includes pages the agent opens on it.",
        ),
        Scope::All => (
            "Allow local network on all sites?".to_string(),
            "Every site can reach apps on this Mac and devices on your network. That includes pages the agent opens.",
        ),
    }
}

/// The choice one step along a group of segments, wrapping at the ends (`Segments`' arrows).
pub fn stepped(current: Choice, step: isize) -> Choice {
    let at = Choice::ALL.iter().position(|one| *one == current).unwrap_or(0) as isize;
    Choice::ALL[(at + step).rem_euclid(Choice::ALL.len() as isize) as usize]
}

/// What a press on a segment does: sets the choice, or asks first where it would allow Local network that is not allowed yet.
#[derive(Debug, PartialEq, Eq)]
pub enum Press {
    Set,
    Confirm,
}

pub fn press_of(permission: Permission, now: Choice, pressed: Choice) -> Press {
    if permission == Permission::LocalNetwork && pressed == Choice::Allow && now != Choice::Allow {
        Press::Confirm
    } else {
        Press::Set
    }
}

/// The lock's popover and the confirmation, held by the tab.
pub struct SiteState {
    pub open: bool,
    pub scope: Scope,
    /// The choices as last read from the file, and as changed here.
    pub settings: Settings,
    /// Allow on Local network is waiting on its confirmation, for this scope.
    pub confirm: Option<Scope>,
    pub lock: FocusHandle,
    pub frame: FocusHandle,
    pub scopes: FocusHandle,
    pub rows: Vec<FocusHandle>,
    pub confirm_frame: FocusHandle,
    pub confirm_allow: FocusHandle,
    pub confirm_cancel: FocusHandle,
    pub lock_at: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl SiteState {
    pub fn new<T: 'static>(cx: &mut Context<T>) -> SiteState {
        SiteState {
            open: false,
            scope: Scope::Site,
            settings: Settings::default(),
            confirm: None,
            lock: cx.focus_handle().tab_stop(true),
            frame: cx.focus_handle(),
            scopes: cx.focus_handle().tab_stop(true),
            rows: Permission::ALL.iter().map(|_| cx.focus_handle().tab_stop(true)).collect(),
            confirm_frame: cx.focus_handle(),
            confirm_allow: cx.focus_handle().tab_stop(true),
            confirm_cancel: cx.focus_handle().tab_stop(true),
            lock_at: Rc::default(),
        }
    }
}

/// One group of segments: a radio group of which one is chosen. Tab stops once, on the group; the arrows move the choice.
fn segments(look: &Look, id: SharedString, handle: &FocusHandle, name: SharedString, labels: &[&'static str], chosen: usize, on_step: impl Fn(isize, &mut Window, &mut gpui::App) + 'static, on_pick: impl Fn(usize, &mut Window, &mut gpui::App) + 'static) -> AnyElement {
    let ring = look.color(token::ACCENT);
    let on_pick = Rc::new(on_pick);
    let on_step = Rc::new(on_step);
    let mut row = div()
        .id(id.clone())
        .key_context(SEGMENTS_CONTEXT)
        .track_focus(handle)
        .tab_stop(true)
        .flex()
        .flex_none()
        .p(look.length(token::BORDER_WIDTH))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .rounded(look.length(token::RADIUS_SM))
        .bg(look.color(token::BG_SUNKEN))
        .focus_visible(move |style| style.border_color(ring))
        .on_action({
            let on_step = on_step.clone();
            move |_: &SegmentNext, window, cx| on_step(1, window, cx)
        })
        .on_action({
            let on_step = on_step.clone();
            move |_: &SegmentPrevious, window, cx| on_step(-1, window, cx)
        });
    for (at, label) in labels.iter().enumerate() {
        let picked = at == chosen;
        let on_pick = on_pick.clone();
        let hover = look.color(token::BG_HOVER);
        let segment = div()
            .id(SharedString::from(format!("{id}:{label}")))
            .flex()
            .items_center()
            .h(look.length(token::H_CONTROL_SM))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .cursor_pointer()
            .when(picked, |one| one.bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)))
            .when(!picked, |one| one.text_color(look.color(token::FG_MUTED)).hover(move |style| style.bg(hover)))
            .on_click(move |_, window, cx| on_pick(at, window, cx))
            .child(*label);
        row = row.child(labelled(segment, Role::RadioButton, *label));
    }
    labelled(row, Role::RadioGroup, name).into_any_element()
}

impl BrowserTab {
    /// The lock before the address: a button that opens Site settings.
    pub(super) fn site_lock(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        host_of(self.address_now())?;
        let at = self.site().lock_at.clone();
        let ring = look.color(token::ACCENT);
        let hover = look.color(token::BG_HOVER);
        let lock = div()
            .id("job-browser-lock")
            .track_focus(&self.site().lock)
            .tab_stop(true)
            .relative()
            .size(look.length(token::H_CONTROL_SM))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .focus_visible(move |style| style.border_color(ring))
            .on_click(cx.listener(|this, _, window, cx| this.toggle_site(window, cx)))
            .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Site settings").build(window, cx))
            .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
            .child(svg().path(icon_path("lock-keyhole")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)));
        Some(labelled(lock, Role::Button, "Site settings").into_any_element())
    }

    /// The popover, hung under the lock, and the confirmation where Allow on Local network waits on one.
    pub(super) fn site_layers(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        let mut layers = Vec::new();
        let state = self.site();
        if state.open {
            if let (Some(at), Some(host)) = (state.lock_at.get(), host_of(self.address_now())) {
                let under = point(at.origin.x, at.origin.y + at.size.height + look.length(token::SPACE_1));
                layers.push(
                    deferred(anchored().position(under).snap_to_window_with_margin(look.length(token::SPACE_1)).child(self.site_panel(&host, look, cx)))
                        .with_priority(1)
                        .into_any_element(),
                );
            }
        }
        if let (Some(scope), Some(host)) = (state.confirm, host_of(self.address_now())) {
            let size = window.viewport_size();
            layers.push(
                deferred(anchored().position(point(size.width / 2., size.height / 3.)).anchor(gpui::Anchor::TopLeft).child(self.confirm_card(scope, &host, look, cx)))
                    .with_priority(2)
                    .into_any_element(),
            );
        }
        layers
    }

    fn site_panel(&self, host: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let state = self.site();
        let scope = state.scope;
        let me = cx.entity().downgrade();
        let scopes = {
            let (step, pick) = (me.clone(), me.clone());
            segments(
                look,
                "job-browser-site-scope".into(),
                &state.scopes,
                "Settings for".into(),
                &Scope::ALL.map(Scope::label),
                Scope::ALL.iter().position(|one| *one == scope).unwrap_or(0),
                move |by, window, cx| {
                    let _ = step.update(cx, |this, cx| this.step_scope(by, window, cx));
                },
                move |at, window, cx| {
                    let _ = pick.update(cx, |this, cx| this.set_scope(Scope::ALL[at], window, cx));
                },
            )
        };
        let rows = div().flex().flex_col().gap(look.length(token::SPACE_1)).children(Permission::ALL.iter().enumerate().map(|(at, permission)| {
            let permission = *permission;
            let value = self.site_value(permission);
            let (step, pick) = (me.clone(), me.clone());
            let chosen = Choice::ALL.iter().position(|one| *one == value).unwrap_or(0);
            div()
                .id(SharedString::from(format!("job-browser-site-row:{}", permission.key())))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_color(look.color(token::FG_MUTED))
                .child(svg().path(icon_path(glyph_of(permission))).size(look.length(token::SPACE_4)).flex_none().text_color(look.color(token::FG_MUTED)))
                .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(permission.label()))
                .child(segments(
                    look,
                    SharedString::from(format!("job-browser-site-choice:{}", permission.key())),
                    &state.rows[at],
                    permission.label().into(),
                    &Choice::ALL.map(Choice::label),
                    chosen,
                    move |by, window, cx| {
                        let _ = step.update(cx, |this, cx| this.step_choice(permission, by, window, cx));
                    },
                    move |picked, window, cx| {
                        let _ = pick.update(cx, |this, cx| this.press_choice(permission, Choice::ALL[picked], window, cx));
                    },
                ))
        }));
        let body = div()
            .id("job-browser-site-panel")
            .key_context(SITE_CONTEXT)
            .track_focus(&state.frame)
            .on_action(cx.listener(|this, _: &CloseSite, window, cx| this.close_site(window, cx)))
            .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close_site(window, cx)))
            .occlude()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .min_w(look.length(token::SPACE_12) * 7.)
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .child(scopes)
            .children((scope == Scope::Site).then(|| {
                div()
                    .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(host.to_string()))
            }))
            .child(rows);
        labelled(body, Role::Dialog, "Site settings").into_any_element()
    }

    /// The confirmation Allow on Local network asks first: Esc cancels, Enter confirms.
    fn confirm_card(&self, scope: Scope, host: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let state = self.site();
        let (title, said) = confirm_words(scope, host);
        let ring = look.color(token::ACCENT);
        let press = |id: &'static str, handle: &FocusHandle, name: &'static str, primary: bool, allow: bool, cx: &Context<Self>| {
            let body = div()
                .id(id)
                .track_focus(handle)
                .tab_stop(true)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if primary { look.color(token::ACCENT) } else { look.color(token::BORDER_STRONG) })
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .cursor_pointer()
                .focus_visible(move |style| style.border_color(ring))
                .on_click(cx.listener(move |this, _, window, cx| if allow { this.confirm_local_network(window, cx) } else { this.cancel_local_network(window, cx) }))
                .child(name);
            let body = if primary { body.bg(look.color(token::ACCENT)).text_color(look.color(token::TEXT_ON_ACCENT)) } else { body.bg(look.color(token::BG_SUNKEN)).text_color(look.color(token::FG_DEFAULT)) };
            labelled(body, Role::Button, name)
        };
        let card = div()
            .id("job-browser-confirm")
            .key_context(CONFIRM_CONTEXT)
            .track_focus(&state.confirm_frame)
            .on_action(cx.listener(|this, _: &ConfirmAllow, window, cx| this.confirm_local_network(window, cx)))
            .on_action(cx.listener(|this, _: &CancelAllow, window, cx| this.cancel_local_network(window, cx)))
            .occlude()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .w(look.length(token::SPACE_12) * 6.)
            .p(look.length(token::SPACE_4))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .child(div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title.clone())))
            .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(said))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(look.length(token::SPACE_2))
                    .child(press("job-browser-confirm-cancel", &state.confirm_cancel, "Cancel", false, false, cx))
                    .child(press("job-browser-confirm-allow", &state.confirm_allow, "Allow", true, true, cx)),
            );
        labelled(card, Role::Dialog, SharedString::from(title)).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arrow_steps_ask_allow_block_and_wraps_at_the_ends() {
        assert_eq!(stepped(Choice::Ask, 1), Choice::Allow);
        assert_eq!(stepped(Choice::Allow, 1), Choice::Block);
        assert_eq!(stepped(Choice::Block, 1), Choice::Ask);
        assert_eq!(stepped(Choice::Ask, -1), Choice::Block);
    }

    #[test]
    fn only_allow_on_local_network_is_asked_about_first() {
        assert_eq!(press_of(Permission::LocalNetwork, Choice::Ask, Choice::Allow), Press::Confirm);
        assert_eq!(press_of(Permission::LocalNetwork, Choice::Block, Choice::Allow), Press::Confirm);
        assert_eq!(press_of(Permission::LocalNetwork, Choice::Allow, Choice::Allow), Press::Set, "already allowed");
        assert_eq!(press_of(Permission::LocalNetwork, Choice::Allow, Choice::Block), Press::Set);
        assert_eq!(press_of(Permission::Camera, Choice::Ask, Choice::Allow), Press::Set);
    }

    #[test]
    fn every_permission_has_a_glyph_of_its_own() {
        let glyphs: Vec<_> = Permission::ALL.iter().map(|one| glyph_of(*one)).collect();
        let mut distinct = glyphs.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), glyphs.len());
    }

    #[test]
    fn the_confirmation_names_the_site_or_every_site() {
        assert_eq!(confirm_words(Scope::Site, "example.com").0, "Allow local network for example.com?");
        assert_eq!(confirm_words(Scope::All, "example.com").0, "Allow local network on all sites?");
    }
}

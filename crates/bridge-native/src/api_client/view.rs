//! The tab drawn from what `mod.rs` holds: the request line on top, a quiet crumb under it, the list of saved requests and this
//! Job's sends on the left, and the request with its answer on the right. Every value is a token.

use gpui::{deferred, div, list, prelude::*, px, svg, AnyElement, Context, Div, Focusable as _, FontWeight, IntoElement, Render, Role, SharedString, Stateful, Window};
use gpui_component::input::{InlineTokenContext, Input, Textarea};

use super::model::{self, Filter, Line, Outcome};
use super::{ApiClient, Load, Part, Pick, Reply, Rows, Show};
use crate::assets::icon_path;
use crate::generated::actions::ActionScope;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::{bone, refusal, ring_colour, tip};
use crate::titlebar::parts::{mono, sans};

pub(super) fn glyph(look: &Look, name: &str, ink: Token) -> gpui::Svg {
    svg().flex_none().path(icon_path(name)).size(look.length(token::SPACE_3)).text_color(look.color(ink))
}

/// An accent-tinted token for a `{{name}}` of the address.
fn variable_chip(look: &Look, context: &InlineTokenContext) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(context.line_height())
        .max_w(context.available_width())
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .bg(if context.is_selected() { look.color(token::SELECT_TINT) } else { look.color(token::ACCENT_FAINT) })
        .text_color(look.color(token::ACCENT))
        .font_family(mono(look))
        .child(context.token().label().clone())
}

/// A bordered field a text box sits in. It takes the focus ring while the box has the keys.
pub(super) fn field(look: &Look, id: impl Into<gpui::ElementId>, focused: bool) -> Stateful<Div> {
    div()
        .id(id)
        .key_context(crate::keymap::TEXT_BOX)
        .flex_1()
        .min_w_0()
        .h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_2))
        .flex()
        .items_center()
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if focused { ring_colour(look) } else { look.color(token::BORDER_DEFAULT) })
        .bg(look.color(token::BG_BASE))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_XS))
}

/// One part of a segmented control: filled with a hairline while it is the one shown.
fn segment(look: &Look, id: impl Into<gpui::ElementId>, word: &str, on: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .map(|part| {
            if on {
                part.bg(look.color(token::BG_RAISED)).border_color(look.color(token::BORDER_DEFAULT)).text_color(look.color(token::FG_DEFAULT))
            } else {
                part.border_color(gpui::transparent_black())
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
            }
        })
        .aria_selected(on)
        .child(SharedString::from(word.to_string()))
}

fn segments(look: &Look, name: &'static str, parts: Vec<(Stateful<Div>, &'static str)>) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(format!("api-{name}")))
            .flex_none()
            .flex()
            .p(px(2.))
            .gap(px(2.))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_SUNKEN))
            .children(parts.into_iter().map(|(part, word)| labelled(part, Role::Tab, word))),
        Role::TabList,
        name,
    )
    .into_any_element()
}

/// A row of the list: a hairline round the one open, a fill under the one the keys are on.
pub(super) fn row_shell(look: &Look, id: String, held: bool, under_keys: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .w_full()
        .h(look.length(token::H_ROW))
        .px(look.length(token::SPACE_2))
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .text_size(look.length(token::TEXT_XS))
        .map(|row| {
            if held {
                row.bg(look.color(token::ACCENT_FAINT)).border_color(look.color(token::ACCENT)).text_color(look.color(token::FG_DEFAULT))
            } else if under_keys {
                row.bg(look.color(token::BG_HOVER)).border_color(look.color(token::BORDER_STRONG)).text_color(look.color(token::FG_DEFAULT))
            } else {
                row.border_color(gpui::transparent_black()).text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::BG_HOVER)))
            }
        })
        .cursor_pointer()
        .aria_selected(held)
}

/// Skeleton shapes where the list will be.
fn list_bones(look: &Look) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_2))
        .children((0..6).map(|at| {
            div()
                .h(look.length(token::H_ROW))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(bone(look, look.length(token::SPACE_8), look.length(token::LEADING_XS)))
                .child(bone(look, look.length(token::SPACE_12) * (1.5 + (at % 3) as f32 * 0.5), look.length(token::LEADING_XS)))
        }))
        .into_any_element()
}

impl Render for ApiClient {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let look = &look;
        let line = self.request_line(look, window, cx);
        let crumb = self.crumb(look, window, cx);
        let left = self.left(look, window, cx);
        let right = self.right(look, window, cx);
        div()
            .id("api-client")
            .key_context(ActionScope::ApiClient.key_context().unwrap_or_default())
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_act))
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| this.escape(event, window, cx)))
            .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, window, cx| {
                if crate::keymap::traverse(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(look))
            .text_color(look.color(token::FG_DEFAULT))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(line)
            .children(crumb)
            .child(div().flex_1().min_h_0().flex().child(left).child(right))
            .children(self.menu_element())
    }
}

impl ApiClient {
    // ---- the request line --------------------------------------------------------------------------

    fn request_line(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let height = look.length(token::H_CONTROL_LG);
        let focused = self.address.read(cx).focus_handle(cx).is_focused(window);
        let text = self.address_text(cx);
        let missing = model::filled(&text, self.environment_read()).missing;

        let method = labelled(
            div()
                .id("api-method")
                .track_focus(&self.method_focus)
                .tab_stop(true)
                .flex_none()
                .flex()
                .items_center()
                .h_full()
                .px(look.length(token::SPACE_3))
                .rounded_l(look.length(token::RADIUS_MD))
                .font_family(mono(look))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(model::method_token(&self.method)))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Method"))
                .on_click(cx.listener(|this, _, window, cx| this.pick(Pick::Method, false, window, cx)))
                .child(SharedString::from(self.method.clone())),
            Role::Button,
            "Method",
        );

        let chip_look = look.clone();
        let address = labelled(
            div()
                .id("api-address")
                .key_context(crate::keymap::TEXT_BOX)
                .flex_1()
                .min_w_0()
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_SM))
                .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| this.finding_key(event, window, cx)))
                .child(Input::new(&self.address).appearance(false).aria_label("Address").token(move |context, _, _| variable_chip(&chip_look, context))),
            Role::TextInput,
            "Address",
        );

        let environment = self.environment_read().map(|one| one.name.clone());
        let caution = (!missing.is_empty()).then(|| {
            let names = missing.iter().map(|name| format!("{{{{{name}}}}}")).collect::<Vec<_>>().join(", ");
            div().id("api-missing").flex_none().child(glyph(look, "triangle-alert", token::NOTICE_CAUTION)).tooltip(tip(format!("No value for {names}")))
        });
        let chip = labelled(
            div()
                .id("api-environment")
                .track_focus(&self.environment_focus)
                .tab_stop(true)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_2))
                .mr(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::BG_HOVER))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(if environment.is_some() { token::FG_DEFAULT } else { token::FG_MUTED }))
                .cursor_pointer()
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .tooltip(tip("Environment, \u{2318}E cycles"))
                .on_click(cx.listener(|this, _, window, cx| this.pick(Pick::Environment, false, window, cx)))
                .child(SharedString::from(environment.clone().unwrap_or_else(|| "No environment".to_string())))
                .children(caution),
            Role::Button,
            "Environment",
        );

        let found = self.finding(window, cx).then(|| self.found_menu(look, cx));
        let line_box = div()
            .id("api-line-box")
            .relative()
            .flex_1()
            .min_w_0()
            .h(height)
            .flex()
            .items_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if focused { ring_colour(look) } else { look.color(token::BORDER_DEFAULT) })
            .bg(look.color(token::BG_RAISED))
            .child(method)
            .child(address)
            .child(chip)
            .children(found);

        let can = self.can_send(cx);
        let sending = self.sending.is_some();
        let why = self.bad_header(cx).map(|_| "A header name has no spaces").filter(|_| !sending);
        let send = labelled(
            div()
                .id("api-send")
                .track_focus(&self.send_focus)
                .tab_stop(can)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(height)
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::ACCENT))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_size(look.length(token::TEXT_SM))
                .whitespace_nowrap()
                .map(|button| {
                    if can {
                        button
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                            .on_click(cx.listener(|this, _, window, cx| this.send(window, cx)))
                            .focus_visible(|style| style.border_2().border_color(ring_colour(look)))
                    } else {
                        button.opacity(0.5)
                    }
                })
                .when_some(why, |button, why| button.tooltip(tip(why)))
                .child(if sending { "Sending" } else { "Send" })
                .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).opacity(0.7).child("\u{2318}\u{21b5}")),
            Role::Button,
            "Send",
        );

        div()
            .id("api-line")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_4))
            .pb(look.length(token::SPACE_2))
            .child(line_box)
            .child(send)
            .into_any_element()
    }

    /// The saved requests the address found, under the line.
    fn found_menu(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let rows = self.found.iter().enumerate().filter_map(|(at, (collection, request))| {
            let (placed, saved) = self.saved_at(*collection, *request)?;
            let on = self.picked == Some(at);
            let (collection, request) = (*collection, *request);
            let name = saved.name.clone();
            let row = div()
                .id(SharedString::from(format!("api-found-{at}")))
                .w_full()
                .h(look.length(token::H_MENU_ITEM))
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_SM))
                .text_size(look.length(token::TEXT_XS))
                .map(|row| if on { row.bg(look.color(token::ACCENT_FAINT)) } else { row.hover(|style| style.bg(look.color(token::BG_HOVER))) })
                .cursor_pointer()
                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(cx.listener(move |this, _, window, cx| this.open_saved(collection, request, window, cx)))
                .child(div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_2)).font_family(mono(look)).text_color(look.color(model::method_token(&saved.method))).child(SharedString::from(saved.method.clone())))
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(saved.name.clone())))
                .child(div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(placed.item.name.clone())));
            Some(labelled(row, Role::ListBoxOption, name))
        });
        deferred(
            labelled(
                div()
                    .id("api-found")
                    .absolute()
                    .top(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1))
                    .left_0()
                    .right_0()
                    .p(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .children(rows),
                Role::ListBox,
                "Saved requests found",
            ),
        )
        .with_priority(2)
        .into_any_element()
    }

    // ---- the list ----------------------------------------------------------------------------------

    fn left(&mut self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let chips = Filter::ALL.iter().enumerate().map(|(at, filter)| {
            let (filter, on) = (*filter, self.filter == *filter);
            labelled(
                div()
                    .id(SharedString::from(format!("api-filter-{}", filter.word())))
                    .track_focus(&self.chip_focus[at])
                    .tab_stop(true)
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_BADGE))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .map(|chip| {
                        if on {
                            chip.bg(look.color(token::ACCENT_FAINT)).border_color(look.color(token::ACCENT)).text_color(look.color(token::ACCENT))
                        } else {
                            chip.border_color(look.color(token::BORDER_DEFAULT))
                                .text_color(look.color(token::FG_MUTED))
                                .cursor_pointer()
                                .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                        }
                    })
                    .focus_visible(|style| style.border_color(ring_colour(look)))
                    .aria_selected(on)
                    .tooltip(tip(filter.tip()))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_filter(filter, cx)))
                    .child(filter.word()),
                Role::RadioButton,
                filter.word(),
            )
        });
        let chips = labelled(
            div().id("api-filters").flex_none().flex().flex_wrap().gap(look.length(token::SPACE_1)).p(look.length(token::SPACE_3)).children(chips),
            Role::RadioGroup,
            "Show",
        );
        let unread = self.requests().map(|read| read.not_read.clone()).unwrap_or_default().into_iter().enumerate().map(|(at, one)| {
            let title = format!("{} would not read", model::unread_file(&one));
            labelled(
                div()
                    .id(SharedString::from(format!("api-unread-{at}")))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .mx(look.length(token::SPACE_3))
                    .mb(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::NOTICE_CAUTION))
                    .bg(look.color(token::NOTICE_CAUTION_BG))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::NOTICE_CAUTION)).child(SharedString::from(title.clone())))
                    .child(div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(model::unread_said(&one)))),
                Role::Alert,
                title,
            )
        });
        let body = match &self.read {
            Load::Reading => labelled(div().id("api-list-loading").flex_1().child(list_bones(look)), Role::Image, "Loading").into_any_element(),
            Load::Failed(why) => div()
                .id("api-list-failed")
                .flex_1()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_4))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Not read"))
                .child(refusal(look, why.clone()))
                .into_any_element(),
            Load::Read(_) if self.lines.is_empty() => div()
                .id("api-list-empty")
                .flex_1()
                .px(look.length(token::SPACE_4))
                .text_color(look.color(token::FG_SUBTLE))
                .child(if self.filter == Filter::Saved { "No saved requests." } else { "No requests in this Job." })
                .into_any_element(),
            Load::Read(_) => {
                let _ = window;
                labelled(
                    div()
                        .id("api-list")
                        .track_focus(&self.list_focus)
                        .tab_stop(true)
                        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| this.list_key(event, window, cx)))
                        .flex_1()
                        .min_h_0()
                        .pb(look.length(token::SPACE_3))
                        .child(list(self.list.clone(), cx.processor(|this, at: usize, window, cx| this.line(at, window, cx))).size_full()),
                    Role::ListBox,
                    "Requests",
                )
                .into_any_element()
            }
        };
        div()
            .id("api-left")
            .flex_none()
            .w(look.length(token::SPACE_12) * 6.25)
            .h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_SUNKEN))
            .child(chips)
            .children(unread)
            .child(body)
            .into_any_element()
    }

    /// One row of the list: a heading, a saved request, or one of this Job's sends.
    fn line(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look().clone();
        let look = &look;
        let Some(line) = self.lines.get(at).cloned() else { return div().into_any_element() };
        let held = self.is_open(&line);
        let under_keys = self.cursor == Some(at) && self.list_focus.is_focused(window);
        let method_column = look.length(token::SPACE_12) + look.length(token::SPACE_2);
        let shell = |id: String, held: bool| row_shell(look, id, held, under_keys);
        let outer = |row: AnyElement| div().w_full().px(look.length(token::SPACE_2)).child(row).into_any_element();
        match line {
            Line::Heading(said) => labelled(
                div()
                    .id(SharedString::from(format!("api-heading-{at}")))
                    .w_full()
                    .h(look.length(token::H_ROW))
                    .px(look.length(token::SPACE_4))
                    .flex()
                    .items_end()
                    .pb(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(SharedString::from(said.to_uppercase())),
                Role::Heading,
                said,
            )
            .into_any_element(),
            Line::Collection(of) => {
                let Some(placed) = self.requests().and_then(|read| read.collections.get(of)) else { return div().into_any_element() };
                let said = format!("Saved in {}", placed.item.name);
                let repository = placed.place == ipc::RequestPlace::Repository;
                let row = shell(format!("api-collection-{at}"), held)
                    .h(look.length(token::H_ROW))
                    .px(look.length(token::SPACE_2))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.cursor = Some(at);
                        this.open_collection(of, window, cx);
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(look.length(token::TEXT_2XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(if held { token::FG_DEFAULT } else { token::FG_SUBTLE }))
                            .child(SharedString::from(said.to_uppercase())),
                    )
                    .children(repository.then(|| {
                        div()
                            .id(SharedString::from(format!("api-collection-mark-{at}")))
                            .flex_none()
                            .child(glyph(look, "folder-git-2", token::FG_MUTED))
                            .tooltip(tip("The repository's"))
                    }));
                outer(labelled(row, Role::ListBoxOption, said).into_any_element())
            }
            Line::Run(run) => self.run_line(look, run, at, held, under_keys, cx),
            Line::RunsLoading => self.runs_loading_line(look),
            Line::RunsFailed(said) => self.runs_failed_line(look, said),
            Line::Saved(collection, request) => {
                let Some((_, saved)) = self.saved_at(collection, request) else { return div().into_any_element() };
                let (method, name) = (saved.method.clone(), saved.name.clone());
                let row = shell(format!("api-saved-{at}"), held)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.cursor = Some(at);
                        this.open_saved(collection, request, window, cx);
                    }))
                    .child(div().flex_none().w(method_column).font_family(mono(look)).text_color(look.color(model::method_token(&method))).child(SharedString::from(method.clone())))
                    .child(div().flex_1().min_w_0().truncate().child(SharedString::from(name.clone())));
                outer(labelled(row, Role::ListBoxOption, format!("{method} {name}")).into_any_element())
            }
            Line::Sent(sent) => {
                let Some(one) = self.sent.get(sent).cloned() else { return div().into_any_element() };
                let (icon, ink) = model::outcome_of(one.status).look();
                let path = model::path_of(&one.url);
                let said = model::status_said(one.status, one.error.as_deref());
                let who = one.who();
                let mark = div()
                    .id(SharedString::from(format!("api-sent-mark-{at}")))
                    .flex_none()
                    .child(glyph(look, icon, ink))
                    .tooltip(tip(format!("{said}, {} {path}, {who}", one.method)));
                let row = shell(format!("api-sent-{at}"), held)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.cursor = Some(at);
                        this.open_sent(sent, window, cx);
                    }))
                    .child(mark)
                    .child(div().flex_none().w(method_column).font_family(mono(look)).text_color(look.color(model::method_token(&one.method))).child(SharedString::from(one.method.clone())))
                    .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(path.clone())))
                    .child(div().flex_none().w(look.length(token::SPACE_8) + look.length(token::SPACE_1)).text_color(look.color(token::FG_SUBTLE)).child(who));
                let status = one.status.map_or("No answer".to_string(), |status| status.to_string());
                outer(labelled(row, Role::ListBoxOption, format!("{who}: {status} {} {path}", one.method)).into_any_element())
            }
        }
    }

    // ---- the request and its answer ----------------------------------------------------------------

    fn right(&mut self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let shell = |content: Vec<AnyElement>| {
            div().id("api-right").flex_1().min_w_0().h_full().overflow_y_scroll().flex().flex_col().gap(look.length(token::SPACE_4)).p(look.length(token::SPACE_4)).children(content).into_any_element()
        };
        if let Some(collection) = self.collection_view(look, window, cx) {
            return shell(vec![collection]);
        }
        let mut parts = Vec::new();
        for (at, part) in Part::ALL.into_iter().enumerate() {
            let word = part.word();
            parts.push((segment(look, SharedString::from(format!("api-part-{}", word.to_lowercase())), word, self.part == part).track_focus(&self.part_focus[at]).tab_stop(true).on_click(cx.listener(move |this, _, _, cx| this.set_part(part, cx))), word));
        }
        let parts = segments(look, "Request part", parts);
        let panel = match self.part {
            Part::Headers => self.headers_panel(look, window, cx),
            Part::Body => self.body_panel(look, window, cx),
            Part::Environment => self.environment_panel(look, window, cx),
        };
        // The answer is the request's: the Environment part has none beside it.
        let answer = if self.part == Part::Environment { None } else { self.answer(look, cx) };
        let mut content = vec![parts, panel];
        content.extend(answer);
        shell(content)
    }

    fn headers_panel(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let name_column = look.length(token::SPACE_12) * 3.;
        let last_column = look.length(token::SPACE_12) * 2.5;
        let inherited = self.collection().filter(|collection| !collection.item.headers.is_empty()).map(|collection| (collection.item.name.clone(), collection.item.headers.clone()));
        let dimmed = inherited.map(|(name, headers)| {
            headers
                .into_iter()
                .enumerate()
                .map(|(at, header)| {
                    let said = format!("From {name}");
                    labelled(
                        div()
                            .id(SharedString::from(format!("api-inherited-{at}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .h(look.length(token::H_CONTROL))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child(div().flex_none().w(name_column).truncate().font_family(mono(look)).child(SharedString::from(header.name.clone())))
                            .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(header.value.clone())))
                            .child(div().flex_none().w(last_column).truncate().child(SharedString::from(said.clone()))),
                        Role::Row,
                        format!("{}: {}, {said}", header.name, header.value),
                    )
                    .into_any_element()
                })
                .collect::<Vec<_>>()
        });
        let own = self.header_rows(look, Rows::Request, window, cx);
        let add = self.add_header_button(look, Rows::Request, cx);
        labelled(
            div()
                .id("api-headers")
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .children(dimmed.unwrap_or_default())
                .children(own)
                .child(div().flex().child(add)),
            Role::Group,
            "Headers",
        )
        .into_any_element()
    }

    /// The editable rows of headers, the request's own or a collection's shared ones, in the same columns.
    pub(super) fn header_rows(&self, look: &Look, which: Rows, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let name_column = look.length(token::SPACE_12) * 3.;
        let last_column = look.length(token::SPACE_12) * 2.5;
        let key = if which == Rows::Request { "header" } else { "cheader" };
        let rows = match which {
            Rows::Request => self.headers.as_slice(),
            Rows::Collection => self.collection_edit.as_ref().map(|edit| edit.rows.as_slice()).unwrap_or_default(),
        };
        (0..rows.len())
            .map(|at| {
                let (name, value) = (rows[at].name.clone(), rows[at].value.clone());
                let name_focused = name.read(cx).focus_handle(cx).is_focused(window);
                let value_focused = value.read(cx).focus_handle(cx).is_focused(window);
                let n = at + 1;
                let remove = labelled(
                    div()
                        .id(SharedString::from(format!("api-{key}-remove-{at}")))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(look.length(token::H_CONTROL_SM))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_color(look.color(token::FG_SUBTLE))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                        .tooltip(tip("Remove"))
                        .on_click(cx.listener(move |this, _, window, cx| this.remove_header(which, at, window, cx)))
                        .child(svg().flex_none().path(icon_path("x")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE))),
                    Role::Button,
                    format!("Remove header {n}"),
                );
                div()
                    .id(SharedString::from(format!("api-{key}-{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(div().flex_none().w(name_column).flex().child(labelled(field(look, SharedString::from(format!("api-{key}-name-{at}")), name_focused).child(Input::new(&name).appearance(false)), Role::TextInput, format!("Header {n} name"))))
                    .child(div().flex_1().min_w_0().flex().child(labelled(field(look, SharedString::from(format!("api-{key}-value-{at}")), value_focused).child(Input::new(&value).appearance(false)), Role::TextInput, format!("Header {n} value"))))
                    .child(div().flex_none().w(last_column).flex().justify_end().child(remove))
                    .into_any_element()
            })
            .collect()
    }

    pub(super) fn add_header_button(&self, look: &Look, which: Rows, cx: &mut Context<Self>) -> AnyElement {
        let focus = match which {
            Rows::Request => self.add_focus.clone(),
            Rows::Collection => self.collection_edit.as_ref().map(|edit| edit.add_focus.clone()).unwrap_or_else(|| self.add_focus.clone()),
        };
        labelled(
            div()
                .id(if which == Rows::Request { "api-header-add" } else { "api-cheader-add" })
                .track_focus(&focus)
                .tab_stop(true)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                .on_click(cx.listener(move |this, _, window, cx| this.add_header(which, window, cx)))
                .child(svg().flex_none().path(icon_path("plus")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_DEFAULT)))
                .child("Add header"),
            Role::Button,
            "Add header",
        )
        .into_any_element()
    }

    fn body_panel(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let own = self.own_headers(cx);
        let stated = own
            .iter()
            .chain(self.collection().map(|collection| collection.item.headers.iter()).into_iter().flatten())
            .find(|header| header.name.eq_ignore_ascii_case("content-type"))
            .map(|header| header.value.clone());
        let focused = self.body.read(cx).focus_handle(cx).is_focused(window);
        labelled(
            div()
                .id("api-body-panel")
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .children(stated.map(|stated| {
                    labelled(
                        div()
                            .id("api-content-type")
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .child(div().flex_none().text_color(look.color(token::FG_SUBTLE)).child("Content type"))
                            .child(div().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(stated.clone()))),
                        Role::Label,
                        format!("Content type {stated}"),
                    )
                }))
                .child(labelled(
                    div()
                        .id("api-body")
                        .key_context(crate::keymap::TEXT_BOX)
                        .p(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(if focused { ring_colour(look) } else { look.color(token::BORDER_DEFAULT) })
                        .bg(look.color(token::BG_SUNKEN))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_XS))
                        .child(Textarea::new(&self.body).appearance(false)),
                    Role::TextInput,
                    "Body",
                )),
            Role::Group,
            "Body",
        )
        .into_any_element()
    }

    // ---- the answer --------------------------------------------------------------------------------

    fn answer(&mut self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let card = |band: Div, content: Vec<AnyElement>| {
            div()
                .id("api-answer")
                .flex_none()
                .flex()
                .flex_col()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .overflow_hidden()
                .child(band)
                .children(content)
        };
        let band = |ground: Token| {
            div().flex_none().flex().items_center().gap(look.length(token::SPACE_3)).h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1)).px(look.length(token::SPACE_3)).bg(look.color(ground))
        };
        if self.sending.is_some() {
            let waiting = band(token::BG_RAISED)
                .child(bone(look, look.length(token::SPACE_8), look.length(token::LEADING_SM)))
                .child(bone(look, look.length(token::SPACE_12), look.length(token::LEADING_XS)));
            let lines = div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_3))
                .bg(look.color(token::BG_SUNKEN))
                .children((0..3).map(|at| bone(look, look.length(token::SPACE_12) * (3. - at as f32 * 0.5), look.length(token::LEADING_XS))))
                .into_any_element();
            return Some(labelled(card(waiting, vec![lines]), Role::Region, "Answer").into_any_element());
        }
        let reply = self.reply.as_ref()?;
        match reply {
            Reply::Refused(said) => {
                let head = band(token::STATUS_COMPLETED_FAILED_BG)
                    .child(glyph(look, "x", token::STATUS_COMPLETED_FAILED))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child("Not sent"));
                let words = div().p(look.length(token::SPACE_3)).bg(look.color(token::BG_SUNKEN)).child(refusal(look, said.clone())).into_any_element();
                Some(labelled(card(head, vec![words]), Role::Alert, "Not sent").into_any_element())
            }
            Reply::Answered(answered) => {
                let sent = &answered.sent;
                let outcome = model::outcome_of(sent.status);
                let (icon, ink) = outcome.look();
                let ground = if outcome == Outcome::Answered { token::STATUS_COMPLETED_SUCCESS_BG } else { token::STATUS_COMPLETED_FAILED_BG };
                let code = sent.status.map_or("No answer".to_string(), |status| status.to_string());
                // A send reopened from the thread keeps no answer headers, so it offers no Headers to switch to.
                let show = if sent.headers.is_empty() { Show::Body } else { self.show };
                let switch = (!sent.headers.is_empty()).then(|| segments(
                    look,
                    "Answer part",
                    vec![
                        (segment(look, "api-show-body", "Body", show == Show::Body).track_focus(&self.show_focus[0]).tab_stop(true).on_click(cx.listener(|this, _, _, cx| this.set_show(Show::Body, cx))), "Body"),
                        (segment(look, "api-show-headers", "Headers", show == Show::Headers).track_focus(&self.show_focus[1]).tab_stop(true).on_click(cx.listener(|this, _, _, cx| this.set_show(Show::Headers, cx))), "Headers"),
                    ],
                ));
                let head = band(ground)
                    .child(glyph(look, icon, ink))
                    .child(
                        div()
                            .id("api-answer-code")
                            .flex_none()
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_size(look.length(token::TEXT_BASE))
                            .text_color(look.color(ink))
                            .child(SharedString::from(code.clone())),
                    )
                    .child(div().flex_none().font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(model::time_of(sent.ms))))
                    .children(sent.truncated.then(|| div().flex_none().text_color(look.color(token::FG_MUTED)).child("Body cut at 256 KiB")))
                    .child(div().flex_1())
                    .children(switch);
                let mut content: Vec<AnyElement> = Vec::new();
                if let Some(error) = sent.error.as_ref() {
                    content.push(div().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_3)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(SharedString::from(error.clone())).into_any_element());
                }
                if let Some(note) = sent.note.as_ref() {
                    content.push(div().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(note.clone())).into_any_element());
                }
                match show {
                    Show::Body if !answered.lines.is_empty() => {
                        let rows = answered.lines.len().clamp(1, 16) as f32;
                        let said = if answered.json { "Body, JSON" } else { "Body" };
                        content.push(
                            labelled(
                                div()
                                    .id("api-answer-body")
                                    .h(look.length(token::LEADING_XS) * rows + look.length(token::SPACE_3) * 2.)
                                    .m(look.length(token::SPACE_3))
                                    .mt(look.length(token::SPACE_3))
                                    .p(look.length(token::SPACE_3))
                                    .rounded(look.length(token::RADIUS_MD))
                                    .bg(look.color(token::BG_SUNKEN))
                                    .font_family(mono(look))
                                    .child(list(self.reply_list.clone(), cx.processor(|this, at: usize, _, cx| this.reply_line(at, cx))).size_full()),
                                Role::Log,
                                said,
                            )
                            .into_any_element(),
                        );
                    }
                    Show::Headers if !sent.headers.is_empty() => {
                        let name_column = look.length(token::SPACE_12) * 3.;
                        content.push(
                            labelled(
                                div()
                                    .id("api-answer-headers")
                                    .flex()
                                    .flex_col()
                                    .gap(look.length(token::SPACE_1))
                                    .m(look.length(token::SPACE_3))
                                    .p(look.length(token::SPACE_3))
                                    .rounded(look.length(token::RADIUS_MD))
                                    .bg(look.color(token::BG_SUNKEN))
                                    .font_family(mono(look))
                                    .children(sent.headers.iter().enumerate().map(|(at, header)| {
                                        labelled(
                                            div()
                                                .id(SharedString::from(format!("api-answer-header-{at}")))
                                                .flex()
                                                .gap(look.length(token::SPACE_2))
                                                .child(div().flex_none().w(name_column).truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(header.name.clone())))
                                                .child(div().flex_1().min_w_0().child(SharedString::from(header.value.clone()))),
                                            Role::Row,
                                            format!("{}: {}", header.name, header.value),
                                        )
                                    })),
                                Role::Group,
                                "Response headers",
                            )
                            .into_any_element(),
                        );
                    }
                    _ => {}
                }
                Some(labelled(card(head, content), Role::Region, format!("Answer, {code}")).into_any_element())
            }
        }
    }

    /// One line of the answer's body.
    fn reply_line(&mut self, at: usize, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look().clone();
        let line = match &self.reply {
            Some(Reply::Answered(answered)) => answered.lines.get(at).cloned().unwrap_or_default(),
            _ => SharedString::default(),
        };
        // An empty line still takes its height.
        let line = if line.is_empty() { SharedString::from("\u{a0}") } else { line };
        div()
            .w_full()
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .child(line)
            .into_any_element()
    }
}

//! Start a sub project: the sheet the row menu's Start a sub project (⇧S) opens in the project picker's place and look, for the
//! Jobs selected or the row alone. A name, the features those Jobs write to now, and Start it; a refusal is said in the sheet.

use super::*;
use crate::titlebar::parts::{kbd, Btn, Weight};

/// The draft's place, so a name typed and left is there when the sheet opens again.
const KEEP: &str = "jobs:sub-project-name";

pub(crate) struct NameSheet {
    list: gpui::WeakEntity<JobsList>,
    /// The Jobs it groups, in his order.
    ids: Vec<SharedString>,
    at: Point<Pixels>,
    corner: Anchor,
    field: Entity<InputState>,
    frame: FocusHandle,
    refused: Option<SharedString>,
    sending: bool,
    _subs: Vec<Subscription>,
}

impl gpui::EventEmitter<DismissEvent> for NameSheet {}

impl NameSheet {
    pub(super) fn new(list: gpui::WeakEntity<JobsList>, ids: Vec<SharedString>, at: Point<Pixels>, corner: Anchor, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        if let Some(kept) = crate::drafts::kept(KEEP, cx) {
            field.update(cx, |input, cx| input.set_value(kept, window, cx));
        }
        let typed = cx.subscribe_in(&field, window, |this, field, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let words = field.read(cx).value().to_string();
                crate::drafts::keep(KEEP, &words, cx);
                if this.refused.take().is_some() {
                    cx.notify();
                }
            }
            InputEvent::PressEnter { .. } => this.start(window, cx),
            _ => {}
        });
        // Esc leaves before the text box can take it.
        let me = cx.entity().downgrade();
        let escape = cx.intercept_keystrokes(move |event, window, cx| {
            if event.keystroke.key != "escape" {
                return;
            }
            let Some(me) = me.upgrade() else { return };
            me.update(cx, |this, cx| {
                if this.frame.contains_focused(window, cx) {
                    cx.emit(DismissEvent);
                }
            });
        });
        let first = field.clone();
        window.defer(cx, move |window, cx| first.update(cx, |input, cx| input.focus(window, cx)));
        NameSheet { list, ids, at, corner, field, frame: cx.focus_handle(), refused: None, sending: false, _subs: vec![typed, escape] }
    }

    /// The Jobs it groups, for a test.
    #[cfg(test)]
    pub(super) fn ids(&self) -> &[SharedString] {
        &self.ids
    }

    /// The name in the box, for a test to type.
    #[cfg(test)]
    pub(super) fn type_name(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.field.update(cx, |input, cx| input.set_value(name.to_string(), window, cx));
    }

    /// Enter or Start it: Fleet keeps the sub project and the sheet goes, or says why not and stays.
    pub(super) fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        let Some(list) = self.list.upgrade() else { return };
        let name = self.field.read(cx).value().trim().to_string();
        self.sending = true;
        let ids = self.ids.clone();
        let sent = list.update(cx, |list, cx| list.start_sub_project(&ids, &name, cx));
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update(cx, |this, cx| {
                this.sending = false;
                match answer {
                    Ok(()) => {
                        crate::drafts::clear(KEEP, cx);
                        cx.emit(DismissEvent);
                    }
                    Err(why) => this.refused = Some(why.into()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// The features the Jobs write to now, each with the titles of those Jobs, from the libraries the list holds.
    fn comes_with(&self, cx: &App) -> Vec<(SharedString, SharedString)> {
        let Some(list) = self.list.upgrade() else { return Vec::new() };
        let list = list.read(cx);
        list.leads
            .shelves()
            .into_iter()
            .filter_map(|shelf| {
                let titles: Vec<String> = self.ids.iter().filter(|id| shelf.jobs.contains(&id.as_ref())).filter_map(|id| list.record(id)).map(|job| titled(job.title.as_deref())).collect();
                (!titles.is_empty()).then(|| (SharedString::from(shelf.title.to_string()), SharedString::from(titles.join(", "))))
            })
            .collect()
    }
}

impl Render for NameSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let glyph = look.length(token::SPACE_4);
        let mono = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default();
        let focused = self.field.focus_handle(cx).is_focused(window);
        let accent = look.color(token::ACCENT);
        let field = labelled(
            div()
                .id("sub-project-name")
                .flex_none()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if focused { accent } else { look.color(token::BORDER_DEFAULT) })
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .child(svg().path(icon_path("boxes")).flex_none().size(glyph).text_color(look.color(token::FG_SUBTLE)))
                .child(div().flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).child(Input::new(&self.field).appearance(false)))
                .child(kbd("Esc", look)),
            Role::TextInput,
            "Name",
        );
        let comes = self.comes_with(cx);
        let comes = (!comes.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .child(div().h(look.length(token::H_MENU_ITEM)).flex().items_center().px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Comes with"))
                .children(comes.into_iter().enumerate().map(|(at, (title, jobs))| {
                    let name: SharedString = format!("{title}, {jobs}").into();
                    labelled(
                        div()
                            .id(("comes-with", at))
                            .min_h(look.length(token::H_MENU_ITEM))
                            .py(look.length(token::SPACE_1))
                            .px(look.length(token::SPACE_2))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_3))
                            .text_size(look.length(token::TEXT_SM))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(svg().path(icon_path("layers")).flex_none().size(glyph).text_color(look.color(token::FG_MUTED)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(div().truncate().child(title))
                                    .child(div().truncate().font_family(mono.clone()).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(jobs)),
                            ),
                        Role::ListItem,
                        name,
                    )
                }))
        });
        let refusal = self.refused.clone().map(|why| labelled(div().id("sub-project-refused").text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why));
        let start = Btn::new("sub-project-start", "Start it", Weight::Primary)
            .icon("check")
            .pending(self.sending)
            .draw(look, cx)
            .on_click(cx.listener(|this, _, window, cx| this.start(window, cx)))
            .child(kbd("\u{21a9}", look));
        let foot = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .pt(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_1))
            .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).children(refusal))
            .child(labelled(start, Role::Button, "Start it"));
        deferred(
            anchored().position(self.at).anchor(self.corner).snap_to_window_with_margin(look.length(token::SPACE_1)).child(labelled(
                div()
                    .id("sub-project-sheet")
                    .track_focus(&self.frame)
                    .on_mouse_down_out(cx.listener(|_, event: &gpui::MouseDownEvent, window, cx| {
                        if crate::room::of(window, cx).contains(&event.position) {
                            cx.emit(DismissEvent);
                        }
                    }))
                    .occlude()
                    .w(look.length(token::W_MENU) + look.length(token::SPACE_12))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .p(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                    .child(field)
                    .children(comes)
                    .child(foot),
                Role::Dialog,
                "Start a sub project",
            )),
        )
        .with_priority(4)
    }
}

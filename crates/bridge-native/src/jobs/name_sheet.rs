//! Start a sub project: the sheet the row menu's Start a sub project (⇧S) opens in the project picker's place and look, for the
//! Jobs selected or the row alone. One box, Goal, in his words, which names the sub project too; the features those Jobs write
//! to now; and Start it. Turn into orchestrator (⇧O) opens the same sheet for one running Job. A refusal is said in the sheet.

use super::*;
use crate::titlebar::parts::{kbd, Btn, Weight};

/// The draft's place, so a goal typed and left is there when the sheet opens again.
const KEEP: &str = "jobs:sub-project-goal";

/// What the sheet starts.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum For {
    /// A sub project of these Jobs, in his order.
    Start(Vec<SharedString>),
    /// A sub project this running Job coordinates.
    Turn(SharedString),
}

impl For {
    fn ids(&self) -> &[SharedString] {
        match self {
            For::Start(ids) => ids,
            For::Turn(id) => std::slice::from_ref(id),
        }
    }

    /// What the sheet is called, for VoiceOver and a walk.
    fn named(&self) -> &'static str {
        match self {
            For::Start(_) => "Start a sub project",
            For::Turn(_) => "Turn into orchestrator",
        }
    }

    /// The draft's place: one for a new sub project, one for each Job turned.
    fn keep(&self) -> String {
        match self {
            For::Start(_) => KEEP.to_string(),
            For::Turn(id) => format!("jobs:orchestrator-goal:{id}"),
        }
    }
}

pub(crate) struct NameSheet {
    list: gpui::WeakEntity<JobsList>,
    purpose: For,
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
    pub(super) fn new(list: gpui::WeakEntity<JobsList>, purpose: For, at: Point<Pixels>, corner: Anchor, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = cx.new(|cx| InputState::new(window, cx).placeholder("Goal"));
        let keep = purpose.keep();
        if let Some(kept) = crate::drafts::kept(&keep, cx) {
            field.update(cx, |input, cx| input.set_value(kept, window, cx));
        }
        let typed = cx.subscribe_in(&field, window, move |this, field, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let words = field.read(cx).value().to_string();
                crate::drafts::keep(&keep, &words, cx);
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
        NameSheet { list, purpose, at, corner, field, frame: cx.focus_handle(), refused: None, sending: false, _subs: vec![typed, escape] }
    }

    /// The Jobs it groups, for a test.
    #[cfg(test)]
    pub(super) fn ids(&self) -> &[SharedString] {
        self.purpose.ids()
    }

    /// The goal in the box, for a test to type.
    #[cfg(test)]
    pub(super) fn type_name(&mut self, goal: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.field.update(cx, |input, cx| input.set_value(goal.to_string(), window, cx));
    }

    /// Enter or Start it: Fleet keeps the sub project and the sheet goes, or says why not and stays.
    pub(super) fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        let Some(list) = self.list.upgrade() else { return };
        let goal = self.field.read(cx).value().trim().to_string();
        self.sending = true;
        let purpose = self.purpose.clone();
        let sent = list.update(cx, |list, cx| match &purpose {
            For::Start(ids) => list.start_sub_project(ids, &goal, cx),
            For::Turn(id) => list.turn_into_orchestrator(id, &goal, cx),
        });
        let keep = purpose.keep();
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update(cx, |this, cx| {
                this.sending = false;
                match answer {
                    Ok(()) => {
                        crate::drafts::clear(&keep, cx);
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

    /// The features the Jobs write to now, each with the titles of those Jobs, from their rows.
    fn comes_with(&self, cx: &App) -> Vec<(SharedString, SharedString)> {
        let Some(list) = self.list.upgrade() else { return Vec::new() };
        let list = list.read(cx);
        let ids = self.purpose.ids();
        leads::shelves(&list.jobs)
            .into_iter()
            .filter_map(|shelf| {
                let titles: Vec<String> = ids.iter().filter(|id| shelf.jobs.contains(&id.as_ref())).filter_map(|id| list.record(id)).map(|job| titled(job.title.as_deref())).collect();
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
        let glyph_of_sheet = match self.purpose {
            For::Start(_) => "boxes",
            For::Turn(_) => super::ORCHESTRATOR_GLYPH,
        };
        let field = labelled(
            div()
                .id("sub-project-goal")
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
                .child(svg().path(icon_path(glyph_of_sheet)).flex_none().size(glyph).text_color(look.color(token::FG_SUBTLE)))
                .child(div().flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).child(Input::new(&self.field).appearance(false)))
                .child(kbd("Esc", look)),
            Role::TextInput,
            "Goal",
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
                self.purpose.named(),
            )),
        )
        .with_priority(4)
    }
}

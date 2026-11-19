//! The Members sheet the switcher's Members button opens: the repositories a project holds, each ticked, and the
//! served ones it does not hold to add. Save sends `edit_project` with `members` alone, which replaces the list.
//! A folder member is listed as it is and can be taken out or put back; none is made here (New project does).

use gpui::{div, prelude::*, Context, DismissEvent, EventEmitter, FocusHandle, Focusable, KeyDownEvent, Role, SharedString, Task, Window};
use ipc::{ProjectMember, Projects};

use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::new_project::wire::Wire;
use crate::titlebar::parts::{glyph, kbd, sans, Btn, Weight};

/// One line of the list: a repository, or a folder of one, held or not.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub folder: Option<String>,
    pub label: SharedString,
    pub on: bool,
}

/// Fleet saved the members: the title bar reads the new list.
#[derive(Clone, Debug)]
pub struct Saved(pub Projects);

impl EventEmitter<Saved> for MembersSheet {}
impl EventEmitter<DismissEvent> for MembersSheet {}

/// The project's own members first, in their order, then each served repository it does not hold whole.
/// `held` is `(repository name, folder)`; a member whose repository Fleet no longer serves is dropped.
pub fn rows_of(held: &[(SharedString, Option<String>)], served: &[(String, SharedString)]) -> Vec<Row> {
    let mut rows: Vec<Row> = held
        .iter()
        .filter_map(|(name, folder)| {
            let (id, _) = served.iter().find(|(_, one)| one == name)?;
            let label = folder.as_ref().map_or_else(|| name.clone(), |folder| format!("{name}/{folder}").into());
            Some(Row { id: id.clone(), folder: folder.clone(), label, on: true })
        })
        .collect();
    for (id, name) in served {
        if !rows.iter().any(|row| row.id == *id && row.folder.is_none()) {
            rows.push(Row { id: id.clone(), folder: None, label: name.clone(), on: false });
        }
    }
    rows
}

/// What Save sends: the ticked rows, in the list's order.
pub fn members_of(rows: &[Row]) -> Vec<ProjectMember> {
    rows.iter().filter(|row| row.on).map(|row| ProjectMember { manifest_id: ipc::ManifestId::carried(row.id.clone()), folder: row.folder.clone() }).collect()
}

pub struct MembersSheet {
    wire: Wire,
    open: bool,
    project: String,
    name: SharedString,
    rows: Vec<Row>,
    at: usize,
    sending: bool,
    refused: Option<SharedString>,
    frame: FocusHandle,
    keys_were: Option<FocusHandle>,
    act: Option<Task<()>>,
}

impl MembersSheet {
    pub fn new(wire: Wire, cx: &mut Context<Self>) -> Self {
        MembersSheet { wire, open: false, project: String::new(), name: SharedString::default(), rows: Vec::new(), at: 0, sending: false, refused: None, frame: cx.focus_handle(), keys_were: None, act: None }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(&mut self, project: String, name: SharedString, rows: Vec<Row>, window: &mut Window, cx: &mut Context<Self>) {
        self.keys_were = window.focused(cx);
        self.open = true;
        self.project = project;
        self.name = name;
        self.rows = rows;
        self.at = 0;
        self.sending = false;
        self.refused = None;
        let frame = self.frame.clone();
        window.defer(cx, move |window, cx| window.focus(&frame, cx));
        cx.notify();
    }

    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.act = None;
        if let Some(keys) = self.keys_were.take() {
            window.focus(&keys, cx);
        }
        cx.notify();
    }

    pub fn toggle(&mut self, at: usize, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.get_mut(at) {
            row.on = !row.on;
            self.at = at;
            self.refused = None;
            cx.notify();
        }
    }

    pub fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        self.sending = true;
        self.refused = None;
        let task = self.wire.set_members(&self.project, members_of(&self.rows), cx);
        self.act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| match answer {
                Ok(projects) => {
                    this.close(window, cx);
                    cx.emit(Saved(projects));
                }
                Err(why) => {
                    this.sending = false;
                    this.refused = Some(why.into());
                    cx.notify();
                }
            });
        }));
    }

    fn on_key(&mut self, key: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let held = &key.keystroke;
        match held.key.as_str() {
            "escape" => self.close(window, cx),
            "enter" if held.modifiers.platform => self.save(window, cx),
            "down" | "j" => self.at = (self.at + 1).min(self.rows.len().saturating_sub(1)),
            "up" | "k" => self.at = self.at.saturating_sub(1),
            "space" => self.toggle(self.at, cx),
            _ => return,
        }
        cx.notify();
    }
}

impl Focusable for MembersSheet {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.frame.clone()
    }
}

impl Render for MembersSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let look = cx.look();
        let view = crate::room::of(window, cx).size;
        let width = look.length(token::PALETTE_WIDTH).min(view.width - look.length(token::SPACE_6) * 2.);
        let top = (view.height * 0.12).max(look.length(token::H_TITLE_BAR));
        let focused = self.frame.is_focused(window);
        let rows = self.rows.iter().enumerate().map(|(i, row)| {
            let here = focused && self.at == i;
            let accent = look.color(token::ACCENT);
            let tick = div()
                .flex_none()
                .size(look.length(token::SPACE_4))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if row.on { accent } else { look.color(token::BORDER_STRONG) })
                .when(row.on, |tick| tick.bg(accent).child(glyph("check", look.length(token::SPACE_3), look.color(token::TEXT_ON_ACCENT))));
            let line = div()
                .id(("member-row", i))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .when(here, |line| line.bg(look.color(token::ACCENT_MUTED)))
                .when(!here, |line| line.hover(|style| style.bg(look.color(token::BG_HOVER))))
                .on_click(cx.listener(move |this, _, _, cx| this.toggle(i, cx)))
                .child(tick)
                .child(div().flex_1().min_w_0().child(row.label.clone()));
            labelled(line, Role::CheckBox, row.label.clone())
        });
        let ring = look.color(token::ACCENT);
        let refusal = self.refused.clone().map(|why| labelled(div().id("members-refused").text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why));
        let cancel = Btn::new("members-cancel", "Cancel", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.close(window, cx)));
        let save = Btn::new("members-save", "Save", Weight::Primary)
            .pending(self.sending)
            .draw(look, cx)
            .shadow(look.shadows(token::SHADOW_PRIMARY))
            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)))
            .child(kbd("⌘↩", look));
        let title: SharedString = format!("Members of {}", self.name).into();
        let frame = div()
            .id("members-frame")
            .key_context("project_members")
            .track_focus(&self.frame)
            .on_key_down(cx.listener(|this, key: &KeyDownEvent, window, cx| this.on_key(key, window, cx)))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(width)
            .max_h(view.height - top - look.length(token::SPACE_6))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if focused { ring } else { look.color(token::BORDER_DEFAULT) })
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .child(glyph("folder", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)))
                    .child(div().flex_1().text_size(look.length(token::TEXT_SM)).child(title.clone()))
                    .child(kbd("Esc", look)),
            )
            .child(div().id("members-body").flex_1().min_h_0().overflow_y_scroll().p(look.length(token::SPACE_3)).flex().flex_col().children(rows))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_2XS)).children(refusal))
                    .child(labelled(cancel, Role::Button, "Cancel"))
                    .child(labelled(save, Role::Button, "Save")),
            );
        let scrim = div()
            .id("members-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .bg(look.color(token::SCRIM))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)));
        let column = div().absolute().top(top).left((view.width - width) / 2.).child(frame);
        labelled(div().id("members").absolute().inset_0().child(scrim).child(column), Role::Dialog, title).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn served() -> Vec<(String, SharedString)> {
        vec![("web".into(), "web".into()), ("api".into(), "api".into()), ("docs".into(), "docs".into())]
    }

    #[test]
    fn held_members_lead_and_the_rest_follow_unticked() {
        let rows = rows_of(&[("api".into(), None), ("web".into(), Some("app/checkout".into()))], &served());
        let seen: Vec<_> = rows.iter().map(|row| (row.label.to_string(), row.on)).collect();
        assert_eq!(seen, [("api".into(), true), ("web/app/checkout".into(), true), ("web".into(), false), ("docs".into(), false)]);
    }

    #[test]
    fn save_sends_the_ticked_rows_only() {
        let mut rows = rows_of(&[("api".into(), None)], &served());
        rows[1].on = true;
        rows[0].on = false;
        let sent = members_of(&rows);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].manifest_id.as_str(), "web");
        assert_eq!(sent[0].folder, None);
    }

    #[gpui::test]
    fn adding_and_removing_then_save_edits_the_project_and_closes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let (sheet, cx) = cx.add_window_view(|_, cx| MembersSheet::new(Wire::Mock(Box::new(crate::new_project::wire::Mock::new())), cx));
        let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
        let heard = saved.clone();
        let _keep = cx.update(|_, cx| cx.subscribe(&sheet, move |_, Saved(projects): &Saved, _| *heard.borrow_mut() = Some(projects.clone())));
        sheet.update_in(cx, |sheet, window, cx| {
            let rows = rows_of(&[("api".into(), None)], &served());
            sheet.show("p".into(), "P".into(), rows, window, cx);
            sheet.toggle(0, cx);
            sheet.toggle(1, cx);
            assert_eq!(members_of(&sheet.rows).len(), 1, "api off, web on");
            sheet.save(window, cx);
        });
        cx.run_until_parked();
        sheet.read_with(cx, |sheet, _| assert!(!sheet.is_open(), "a saved sheet goes"));
        assert!(saved.borrow().is_some(), "the screen is told");
    }
}

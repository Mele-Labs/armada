//! Deleting a project from the picker: the confirm that names the project and its tasks, and `POST
//! /projects/project/:id/delete` once it is pressed. A project a copy of another Mac's has no Delete, so none is asked here.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Keystroke, MouseButton, Role, SharedString, Window};

use super::overlay::SettingsOverlay;
use super::view::ringed;
use super::wire::Refused;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The project the confirm is asking about.
pub struct Deleting {
    pub id: String,
    pub name: String,
    /// The tasks that go with it.
    pub tasks: usize,
    /// Fleet's refusal, in its words.
    pub refused: Option<String>,
    pub sending: bool,
}

impl SettingsOverlay {
    /// The picker's Delete on the project being edited: ask first, the keys on Cancel.
    pub fn ask_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, _)) = self.scope.project.as_ref() else { return };
        let Some(one) = self.scope.projects.as_ref().and_then(|all| all.projects.iter().find(|one| one.id == *id)) else { return };
        if one.mac.is_some() {
            return;
        }
        self.scope.deleting = Some(Deleting { id: one.id.clone(), name: one.name.clone(), tasks: one.rows.len(), refused: None, sending: false });
        let cancel = self.scope.cancel_stop.clone();
        window.defer(cx, move |window, cx| window.focus(&cancel, cx));
        cx.notify();
    }

    /// Cancel, Esc, and a press outside the card: nothing is changed, and the keys go back to the picker.
    pub fn cancel_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scope.deleting.as_ref().is_none_or(|asking| asking.sending) {
            return;
        }
        self.scope.deleting = None;
        window.focus(&self.scope.picker, cx);
        cx.notify();
    }

    /// Delete: Fleet takes the project and answers every one that is left, and the picker falls back to Every project.
    /// A refusal is said under the confirm, which stays.
    pub fn confirm_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(asking) = self.scope.deleting.as_mut() else { return };
        if asking.sending {
            return;
        }
        asking.sending = true;
        asking.refused = None;
        let id = asking.id.clone();
        let sent = self.wire.delete_project(&id, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| this.deleted(&id, answer, window, cx));
        })
        .detach();
        cx.notify();
    }

    fn deleted(&mut self, id: &str, answer: Result<ipc::Projects, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        match answer {
            Ok(projects) => {
                let editing = self.scope.project.as_ref().is_some_and(|(project, _)| project == id);
                self.scope.projects = Some(projects);
                self.scope.deleting = None;
                if editing {
                    self.edit(None, window, cx);
                }
                window.focus(&self.scope.picker, cx);
            }
            Err(why) => {
                if let Some(asking) = self.scope.deleting.as_mut() {
                    asking.sending = false;
                    asking.refused = Some(why.said);
                }
            }
        }
        cx.notify();
    }

    /// The confirm's keys, while it is asked: Tab and the arrows between the two, Enter and Space press the
    /// one the keys are on, Esc cancels. Nothing else reaches Settings behind it; ⌘ keys go to the app.
    pub(super) fn delete_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.scope.deleting.is_none() {
            return false;
        }
        let (delete, cancel) = (self.scope.delete_stop.clone(), self.scope.cancel_stop.clone());
        let held = &key.modifiers;
        if held.platform || held.control || held.alt {
            return false;
        }
        match key.key.as_str() {
            "escape" => self.cancel_delete(window, cx),
            "tab" | "left" | "right" | "up" | "down" => {
                // Two stops: any move goes to the other.
                let to = if cancel.is_focused(window) { &delete } else { &cancel };
                window.focus(to, cx);
                cx.notify();
            }
            "enter" | "space" if delete.is_focused(window) => self.confirm_delete(window, cx),
            "enter" | "space" => self.cancel_delete(window, cx),
            _ => {}
        }
        true
    }

    /// The confirm over the page: the sentence, Fleet's refusal under it, then Delete and Cancel.
    pub(super) fn render_delete(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let asking = self.scope.deleting.as_ref()?;
        let said = SharedString::from(super::project::delete_sentence(&asking.name, asking.tasks));
        let refused = asking.refused.clone().map(|why| labelled(super::view::refusal(look, why.clone()).id("project-delete-refused"), Role::Alert, why));
        let failed = look.color(token::STATUS_COMPLETED_FAILED);
        let button = |id: &'static str, label: &'static str, danger: bool| {
            let base = div()
                .id(id)
                .flex_none()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .when(self.scope.deleting.as_ref().is_some_and(|asking| asking.sending), |button| button.opacity(0.5))
                .cursor_pointer();
            if danger {
                base.border_color(failed).bg(failed).text_color(look.color(token::FG_INVERSE)).hover(|style| style.opacity(0.9))
            } else {
                base.border_color(look.color(token::BORDER_DEFAULT)).bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)).hover(|style| style.bg(look.color(token::BG_HOVER)))
            }
            .child(label)
        };
        let delete = ringed(look, button("project-delete-confirm", "Delete", true), &self.scope.delete_stop, window)
            .track_focus(&self.scope.delete_stop)
            .on_click(cx.listener(|this, _, window, cx| this.confirm_delete(window, cx)));
        let cancel = ringed(look, button("project-delete-cancel", "Cancel", false), &self.scope.cancel_stop, window)
            .track_focus(&self.scope.cancel_stop)
            .on_click(cx.listener(|this, _, window, cx| this.cancel_delete(window, cx)));
        let card = div()
            .id("project-delete")
            .occlude()
            .w_full()
            .max_w(look.length(token::W_DIALOG))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::PAD_CARD))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            // A press inside the card is not a press on the scrim.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).child(said.clone()))
            .children(refused)
            .child(div().flex().gap(look.length(token::SPACE_2)).child(labelled(delete, Role::Button, "Delete")).child(labelled(cancel, Role::Button, "Cancel")));
        Some(
            div()
                .id("project-delete-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(look.length(token::SPACE_6))
                .bg(look.color(token::SCRIM))
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                    this.cancel_delete(window, cx);
                    cx.stop_propagation();
                }))
                .child(labelled(card, Role::Dialog, said))
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext, VisualTestContext};

    use super::super::mock::MockFleet;
    use super::super::wire::Wire;
    use super::*;
    use crate::dropdown::Entry;

    fn opened(cx: &mut TestAppContext) -> (Entity<SettingsOverlay>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (overlay, cx) = cx.add_window_view(|window, cx| {
            let mut mock = MockFleet::new();
            mock.deletable_projects();
            SettingsOverlay::new(Wire::Mock(Box::new(mock)), window, cx)
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.show(None, false, window, cx)));
        cx.run_until_parked();
        (overlay, cx)
    }

    fn editing(overlay: &Entity<SettingsOverlay>, id: &str, name: &str, cx: &mut VisualTestContext) {
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.edit(Some((id.into(), name.into())), window, cx)));
        cx.run_until_parked();
    }

    fn deletes(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Vec<String> {
        let rows = overlay.read_with(cx, |this, _| super::super::project::picker_rows(&this.scope.project, this.scope.projects.as_ref()));
        rows.iter()
            .filter_map(|entry| match entry {
                Entry::Act { id: "delete", label, .. } => Some(label.to_string()),
                _ => None,
            })
            .collect()
    }

    #[gpui::test]
    fn deleting_the_project_being_edited_falls_back_to_every_project(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        editing(&overlay, "storefront", "Storefront", cx);
        assert_eq!(deletes(&overlay, cx), ["Delete Storefront"], "one Delete, for the project being edited");
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.ask_delete(window, cx)));
        let asked = overlay.read_with(cx, |this, _| this.scope.deleting.as_ref().map(|asking| (asking.name.clone(), asking.tasks)));
        assert_eq!(asked, Some(("Storefront".into(), 4)));
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.confirm_delete(window, cx)));
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            assert_eq!(this.scope.project, None, "the picker falls back to Every project");
            assert!(this.scope.deleting.is_none(), "the confirm goes");
            assert!(this.scope.projects.as_ref().is_some_and(|all| all.projects.iter().all(|one| one.id != "storefront")), "Fleet's answer is what the picker lists");
        });
    }

    #[gpui::test]
    fn cancel_changes_nothing(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        editing(&overlay, "storefront", "Storefront", cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.ask_delete(window, cx)));
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.cancel_delete(window, cx)));
        overlay.read_with(cx, |this, _| {
            assert!(this.scope.deleting.is_none());
            assert_eq!(this.scope.project, Some(("storefront".into(), "Storefront".into())));
            assert!(this.scope.projects.as_ref().is_some_and(|all| all.projects.iter().any(|one| one.id == "storefront")));
        });
    }

    #[gpui::test]
    fn every_project_and_a_copy_from_another_mac_have_no_delete(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        assert!(deletes(&overlay, cx).is_empty(), "Every project has no Delete");
        editing(&overlay, "pricing", "Pricing", cx);
        assert!(deletes(&overlay, cx).is_empty(), "Pricing is Armada's copy of another Mac's");
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.ask_delete(window, cx)));
        overlay.read_with(cx, |this, _| assert!(this.scope.deleting.is_none(), "a copy is never asked about"));
    }

    #[gpui::test]
    fn fleet_refusing_a_copy_is_said_in_its_words_and_the_confirm_stays(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        overlay.update(cx, |this, _| this.scope.deleting = Some(Deleting { id: "pricing".into(), name: "Pricing".into(), tasks: 3, refused: None, sending: false }));
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.confirm_delete(window, cx)));
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            let asking = this.scope.deleting.as_ref().expect("the confirm stays");
            assert_eq!(asking.refused.as_deref(), Some("This project is held on another Mac, which keeps it"));
            assert!(!asking.sending);
        });
    }

    #[test]
    fn the_sentence_names_the_project_and_its_tasks() {
        use super::super::project::delete_sentence;
        assert_eq!(delete_sentence("Pricing", 4), "Delete Pricing? Its 4 tasks go with it.");
        assert_eq!(delete_sentence("Pricing", 1), "Delete Pricing? Its 1 task goes with it.");
        assert_eq!(delete_sentence("Pricing", 0), "Delete Pricing?");
    }
}

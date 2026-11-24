//! The project chips over the Features list: one chip for each project Fleet knows, the one on screen ringed in the accent, and a
//! press (or its number, `1` to `9`) shows that project's library. The choice is kept on this Mac (`chosen.rs`).

use gpui::{div, prelude::*, AnyElement, Context, Div, FontWeight, Role, SharedString, Stateful, Toggled};

use super::{chosen, mock, FeaturesScreen, Load, Source};
use crate::labels::labelled;
use crate::look::{token, Look};

impl FeaturesScreen {
    /// Every project to draw a chip for, as `(id, name)` in Fleet's order. None where Fleet has not said.
    pub(super) fn chips(&self, cx: &gpui::App) -> Vec<(String, String)> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().projects.as_ref().map(|read| read.projects.iter().map(|one| (one.id.clone(), one.name.clone())).collect()).unwrap_or_default(),
            Source::Mock => mock::projects(),
        }
    }

    /// The project the chips ring: the library on screen, and until one arrives the project asked for.
    pub(super) fn shown_project(&self, cx: &gpui::App) -> Option<String> {
        self.library_project.clone().or_else(|| self.wanted(cx))
    }

    /// The project the Cockpit's floor draws for: the library on screen, else the one asked for, else the first Fleet lists.
    pub(super) fn floor_project(&self, cx: &gpui::App) -> Option<String> {
        self.library_project.clone().filter(|project| !project.is_empty()).or_else(|| self.wanted(cx)).or_else(|| self.chips(cx).into_iter().next().map(|(id, _)| id))
    }

    /// A press on a chip, or its number: show that project.
    pub(super) fn choose(&mut self, project: String, cx: &mut Context<Self>) {
        if self.explicit.as_deref() == Some(project.as_str()) && self.library_project.as_deref() == Some(project.as_str()) {
            return;
        }
        self.explicit = Some(project.clone());
        chosen::set(&project, cx);
        self.press = super::librarian::Press::Idle;
        self.ran = None;
        match &self.source {
            Source::Live(_) => {
                if self.entered {
                    self.read(cx);
                }
            }
            Source::Mock => {
                self.library = Load::Read(mock::library_of(&project));
                self.library_project = Some(project);
                self.cursor = None;
                self.on_project = None;
                self.close_page(cx);
                self.relist(cx);
            }
        }
        cx.notify();
    }

    /// The chip the number `1` to `9` is on.
    pub(super) fn choose_number(&mut self, digit: u8, cx: &mut Context<Self>) -> bool {
        // Inside the Project place no chips are drawn: the title bar picks the project.
        if self.section.is_some() {
            return false;
        }
        let Some((id, _)) = (digit as usize).checked_sub(1).and_then(|at| self.chips(cx).into_iter().nth(at)) else { return false };
        self.choose(id, cx);
        true
    }

    /// The row of chips (`.chips`), wrapping where the projects do not fit. Absent where Fleet names no project.
    pub(super) fn render_chips(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let chips = self.chips(cx);
        if chips.is_empty() {
            return None;
        }
        let shown = self.shown_project(cx);
        let mut row = div().id("features-projects").flex_none().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).mx(look.length(token::SPACE_3)).mt(look.length(token::SPACE_3));
        for (id, name) in chips {
            let on = shown.as_deref() == Some(id.as_str());
            let press = id.clone();
            let pill = project_pill(look, format!("features-project-{id}"), &name, on).on_click(cx.listener(move |this, _, _, cx| this.choose(press.clone(), cx)));
            row = row.child(labelled(pill, Role::Button, name));
        }
        Some(row.into_any_element())
    }
}

/// A project's chip, ringed in the accent where it is the one on screen. The caller gives it its press.
pub(super) fn project_pill(look: &Look, id: String, name: &str, on: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::H_CONTROL_SM) / 2.)
        .border(look.length(token::FOCUS_RING_WIDTH))
        .border_color(if on { look.color(token::ACCENT) } else { gpui::transparent_black() })
        .bg(if on { look.color(token::ACCENT_MUTED) } else { look.color(token::BG_HOVER) })
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
        .whitespace_nowrap()
        .cursor_pointer()
        .when(!on, |pill| pill.hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.14))))
        .aria_toggled(if on { Toggled::True } else { Toggled::False })
        .child(SharedString::from(name.to_string()))
}

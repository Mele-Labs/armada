//! The project switcher, take B (picked 10 Nov): the trigger reads the project with its repositories
//! muted beside it, and its popover is two panes. On the left Find, All, the projects, what is in no
//! project and New project; on the right the row the keys are on, its members and what it holds, with
//! Switch, Settings and Members. ↑↓ move, Enter switches, ⌘, opens Settings, Esc shuts.
//!
//! Fleet does not serve projects yet: a scene draws them, and live the projects list is empty.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, KeyDownEvent, Role, SharedString, Window};

use super::face::Face;
use super::parts::{glyph, Btn, Weight};
use super::{Asked, TitleBar};
use crate::labels::labelled;
use crate::look::{token, Look};

/// A row of the left pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pick {
    All,
    Project(usize),
    /// A repository in no project, by place.
    Lone(usize),
    New,
}

/// What the switcher holds while it is open: what is typed in Find, and the row the keys are on.
#[derive(Clone, Debug, Default)]
pub(super) struct Switcher {
    pub query: String,
    pub at: usize,
}

/// The rows Find leaves, in the order they stand: All, the projects, what is in no project, New project.
pub(super) fn picks(face: &Face, query: &str) -> Vec<Pick> {
    let wanted = query.trim().to_lowercase();
    let has = |name: &str| wanted.is_empty() || name.to_lowercase().contains(&wanted);
    let mut picks = Vec::new();
    if has("All") {
        picks.push(Pick::All);
    }
    for (at, project) in face.projects.iter().enumerate() {
        let member = project.members.iter().any(|member| {
            face.repositories.get(member.repository).is_some_and(|one| has(&one.name)) || member.folder.as_deref().is_some_and(|folder| has(folder))
        });
        if has(&project.name) || member {
            picks.push(Pick::Project(at));
        }
    }
    picks.extend(face.lone().into_iter().filter(|&at| has(&face.repositories[at].name)).map(Pick::Lone));
    picks.push(Pick::New);
    picks
}

/// The row the window is on now, which the keys start at.
fn current(face: &Face) -> Pick {
    match (face.scoped, face.chosen) {
        (Some(at), _) => Pick::Project(at),
        (None, Some(at)) => Pick::Lone(at),
        (None, None) => Pick::All,
    }
}

impl TitleBar {
    /// Open the switcher on the row the window is on.
    pub(super) fn open_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let here = current(&self.face);
        let at = picks(&self.face, "").iter().position(|pick| *pick == here).unwrap_or(0);
        self.switcher = Switcher { query: String::new(), at };
        self.open_popup(super::Popup::Projects, window, cx);
    }

    fn picked_row(&self) -> Option<Pick> {
        picks(&self.face, &self.switcher.query).get(self.switcher.at).copied()
    }

    /// Switch the window to a row: All, a project or a lone repository. New project asks for one.
    fn switch_to(&mut self, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        self.popup = None;
        self.give_keys_back(window, cx);
        match pick {
            Pick::All | Pick::Lone(_) => {
                let at = if let Pick::Lone(at) = pick { Some(at) } else { None };
                self.answer(|face| {
                    face.chosen = at;
                    face.scoped = None;
                }, cx);
                cx.emit(Asked::Repository(at));
            }
            Pick::Project(at) => {
                self.answer(|face| {
                    face.scoped = Some(at);
                    face.chosen = None;
                }, cx);
                cx.emit(Asked::Project(at));
            }
            Pick::New => cx.emit(Asked::NewProject),
        }
        cx.notify();
    }

    /// What Settings opens for a row: the project's or the repository's, else the whole of Settings.
    pub(super) fn settings_of(pick: Option<Pick>) -> Asked {
        match pick {
            Some(Pick::Project(at)) => Asked::ProjectSettings(at),
            Some(Pick::Lone(at)) => Asked::RepositorySettings(at),
            _ => Asked::Settings,
        }
    }

    /// The Settings button's act: the one the window is on.
    pub(super) fn settings_here(&self) -> Asked {
        Self::settings_of(Some(current(&self.face)))
    }

    fn switcher_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        let rows = picks(&self.face, &self.switcher.query).len();
        match keys.key.as_str() {
            "escape" => self.shut(window, cx),
            "up" => self.switcher.at = self.switcher.at.saturating_sub(1),
            "down" => self.switcher.at = (self.switcher.at + 1).min(rows.saturating_sub(1)),
            "enter" => {
                if let Some(pick) = self.picked_row() {
                    self.switch_to(pick, window, cx);
                }
            }
            "," if keys.modifiers.platform => {
                let asked = Self::settings_of(self.picked_row());
                self.leave_with(asked, window, cx);
            }
            "backspace" => {
                self.switcher.query.pop();
                self.switcher.at = 0;
            }
            _ => {
                let Some(typed) = keys.key_char.as_ref().filter(|_| !keys.modifiers.platform && !keys.modifiers.control) else { return };
                if typed.chars().all(|c| !c.is_control()) {
                    self.switcher.query.push_str(typed);
                    self.switcher.at = 0;
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The popover: the list on the left, the row the keys are on on the right.
    pub(super) fn switcher_panel(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let face = &self.face;
        let rows = picks(face, &self.switcher.query);
        let on = rows.get(self.switcher.at).copied();
        let mut left: Vec<AnyElement> = vec![self.find_box(look)];
        let mut said_projects = false;
        let mut said_lone = false;
        for (at, pick) in rows.iter().enumerate() {
            match pick {
                Pick::Project(_) if !said_projects => {
                    said_projects = true;
                    left.push(caption("Projects", look));
                }
                Pick::Lone(_) if !said_lone => {
                    said_lone = true;
                    left.push(caption("Not in a project", look));
                }
                _ => {}
            }
            left.push(self.pick_row(at, *pick, Some(*pick) == on, look, cx));
        }
        labelled(
            div()
                .id("project-switcher")
                .track_focus(&self.popup_focus)
                .on_key_down(cx.listener(Self::switcher_key))
                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                    if this.menu.is_none() && this.dialog.is_none() {
                        this.shut(window, cx);
                    }
                }))
                .occlude()
                .w(look.length(token::SPACE_12) * 13.5)
                .flex()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_STRONG))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .overflow_hidden()
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .font_family(super::parts::sans(look))
                .child(
                    labelled(
                        div()
                            .id("project-list")
                            .flex_none()
                            .w(look.length(token::SPACE_12) * 5.)
                            .flex()
                            .flex_col()
                            .py(look.length(token::SPACE_2))
                            .border_r(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_SUBTLE))
                            .children(left),
                        Role::ListBox,
                        "Projects",
                    ),
                )
                .child(div().flex_1().min_w_0().p(look.length(token::SPACE_4)).children(on.map(|pick| self.pick_detail(pick, look, cx)))),
            Role::Dialog,
            "Switch project",
        )
        .into_any_element()
    }

    fn find_box(&self, look: &Look) -> AnyElement {
        let query = &self.switcher.query;
        let (said, ink) = if query.is_empty() { ("Find".to_string(), token::FG_SUBTLE) } else { (query.clone(), token::FG_DEFAULT) };
        labelled(
            div()
                .id("project-find")
                .mx(look.length(token::SPACE_2))
                .mb(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_2))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_STRONG))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(ink))
                .child(glyph("search", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_SUBTLE)))
                .child(div().min_w_0().truncate().child(said)),
            Role::SearchInput,
            "Find a project",
        )
        .into_any_element()
    }

    fn pick_row(&self, at: usize, pick: Pick, on: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let face = &self.face;
        let (icon, ink, name, detail): (&str, crate::look::Token, SharedString, Option<&str>) = match pick {
            Pick::All => ("globe", token::FG_SUBTLE, "All".into(), None),
            Pick::Project(i) => ("layers", token::ACCENT, face.projects[i].name.clone(), None),
            Pick::Lone(i) => ("git-branch", token::FG_SUBTLE, face.repositories[i].name.clone(), face.repositories[i].not_set_up.then_some("Not set up")),
            Pick::New => ("plus", token::FG_SUBTLE, "New project".into(), None),
        };
        let quiet = pick == Pick::New;
        labelled(
            div()
                .id(SharedString::from(format!("pick:{at}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .cursor_pointer()
                .when(on, |row| row.bg(look.color(token::ACCENT_MUTED)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))))
                .when(!on, |row| row.hover(|style| style.bg(look.color(token::BG_HOVER))))
                .when(quiet, |row| row.text_color(look.color(token::FG_SUBTLE)))
                .child(glyph(icon, look.length(token::SPACE_4), look.color(ink)))
                .child(div().flex_1().min_w_0().truncate().child(name.clone()))
                .children(detail.map(|said| div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(said)))
                .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    // One press puts the keys on the row; a second, or a double press, switches.
                    if this.switcher.at == at || event.click_count() > 1 || pick == Pick::New {
                        this.switch_to(pick, window, cx);
                    } else {
                        this.switcher.at = at;
                        cx.notify();
                    }
                })),
            Role::ListBoxOption,
            name,
        )
        .into_any_element()
    }

    /// The right pane: the row's name, its members, what it holds, and its acts.
    fn pick_detail(&self, pick: Pick, look: &Look, cx: &Context<Self>) -> AnyElement {
        let face = &self.face;
        let member = |repository: SharedString, folder: Option<SharedString>| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .child(glyph("folder", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                .child(div().flex_none().child(repository))
                .children(folder.map(|folder| div().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(format!("/ {folder}"))))
        };
        let (name, members, lines): (SharedString, Vec<gpui::Div>, Vec<(&str, SharedString)>) = match pick {
            Pick::All => (
                "All".into(),
                face.repositories.iter().filter(|one| !one.not_set_up).map(|one| member(one.name.clone(), None)).collect(),
                Vec::new(),
            ),
            Pick::Project(at) => {
                let project = &face.projects[at];
                let members = project
                    .members
                    .iter()
                    .filter_map(|one| face.repositories.get(one.repository).map(|repository| member(repository.name.clone(), one.folder.clone())))
                    .collect();
                let mut lines = Vec::new();
                if let Some(tracker) = &project.tracker {
                    lines.push(("Tracker", tracker.clone()));
                }
                if let Some(kept) = &project.kept {
                    lines.push(("Kept", capitalised(&kept.phrase()).into()));
                }
                if let Some(worn) = &project.look {
                    lines.push(("Look", worn.clone()));
                }
                (project.name.clone(), members, lines)
            }
            Pick::Lone(at) => {
                let one = &face.repositories[at];
                let mut lines = Vec::new();
                if let Some(kept) = &one.kept {
                    lines.push(("Kept", capitalised(&kept.phrase()).into()));
                }
                if one.not_set_up {
                    lines.push(("Manifest", "Not set up".into()));
                }
                (one.name.clone(), vec![member(one.name.clone(), None)], lines)
            }
            Pick::New => return div().into_any_element(),
        };
        let settings = Self::settings_of(Some(pick));
        let acts = div()
            .flex()
            .gap(look.length(token::SPACE_2))
            .child(labelled(
                Btn::new("switch", "Switch", Weight::Primary).icon("chevron-right").draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.switch_to(pick, window, cx))),
                Role::Button,
                "Switch",
            ))
            .when(pick != Pick::All, |acts| {
                acts.child(labelled(
                    Btn::new("pick-settings", "Settings", Weight::Secondary)
                        .icon("sliders-horizontal")
                        .draw(look, cx)
                        .on_click(cx.listener(move |this, _, window, cx| this.leave_with(settings.clone(), window, cx))),
                    Role::Button,
                    "Settings",
                ))
            })
            .when_some(if let Pick::Project(at) = pick { Some(at) } else { None }, |acts, at| {
                acts.child(labelled(
                    Btn::new("members", "Members", Weight::Secondary)
                        .icon("pencil")
                        .draw(look, cx)
                        .on_click(cx.listener(move |this, _, window, cx| this.leave_with(Asked::ProjectMembers(at), window, cx))),
                    Role::Button,
                    "Members",
                ))
            });
        labelled(
            div()
                .id("project-detail")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(name.clone()))
                .children(members)
                .when(!lines.is_empty(), |pane| {
                    pane.child(
                        div().flex().flex_col().gap(look.length(token::SPACE_1)).my(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).children(lines.into_iter().map(|(term, value)| {
                            div()
                                .flex()
                                .gap(look.length(token::SPACE_2))
                                .child(div().flex_none().w(look.length(token::SPACE_12) * 2.).text_color(look.color(token::FG_SUBTLE)).child(term))
                                .child(div().min_w_0().child(value))
                        })),
                    )
                })
                .child(acts),
            Role::Region,
            name,
        )
        .into_any_element()
    }
}

/// A heading of the left pane, as `PROJECTS`.
fn caption(said: &'static str, look: &Look) -> AnyElement {
    div()
        .px(look.length(token::SPACE_3))
        .pt(look.length(token::SPACE_2))
        .pb(look.length(token::SPACE_1))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_SUBTLE))
        .child(said.to_uppercase())
        .into_any_element()
}

fn capitalised(said: &str) -> String {
    let mut chars = said.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titlebar::scenes;

    #[test]
    fn find_keeps_a_project_whose_member_matches_and_new_project_stays() {
        let face = scenes::rest();
        assert_eq!(picks(&face, ""), vec![Pick::All, Pick::Project(0), Pick::Project(1), Pick::Lone(3), Pick::New]);
        assert_eq!(picks(&face, "pricing"), vec![Pick::Project(1), Pick::New]);
        assert_eq!(picks(&face, "docs"), vec![Pick::Lone(3), Pick::New]);
    }

    #[test]
    fn the_trigger_reads_the_project_and_its_repositories_muted() {
        let mut face = scenes::rest();
        face.scoped = Some(1);
        assert_eq!(face.chosen_said(), ("Storefront".into(), Some("sample-web, sample-api".into())));
        assert_eq!(face.settings_said(), SharedString::from("Settings for Storefront"));
        face.scoped = None;
        assert_eq!(face.chosen_said(), ("All".into(), None));
        assert_eq!(face.settings_said(), SharedString::from("Settings"));
    }
}

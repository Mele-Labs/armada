//! The project switcher, take B (picked 10 Nov): the trigger reads the project with its repositories
//! muted beside it, and its popover is two panes. On the left Find, All, the projects, what is in no
//! project and New project; on the right the row the keys are on, its members and what it holds, with
//! Switch and Edit (Settings for a lone repository). ↑↓ move, Enter switches, ⌘, opens Settings, Esc shuts.
//!
//! The projects are Fleet's (`GET /projects`, kept current by `projects.changed`), read through
//! [`super::face::projects_of`]; a walk's scene reads the same from the mock. Fleet has no route that
//! sets a window's project, so a pick is the window's own and sends nothing.

use gpui::{div, prelude::*, AnyElement, Context, Entity, FontWeight, KeyDownEvent, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};

use super::face::{from_said, Face, RunsPlaceKind, Unread};
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
#[derive(Default)]
pub(super) struct Switcher {
    pub query: String,
    pub at: usize,
    /// Find, a text box as the palette's is, so typing reaches it through the platform's text input.
    find: Option<(Entity<InputState>, Subscription)>,
}

/// The rows Find leaves, in the order they stand: All, his projects and then the copies of another Mac's, what is in
/// no project, New project.
pub(super) fn picks(face: &Face, query: &str) -> Vec<Pick> {
    let wanted = query.trim().to_lowercase();
    let has = |name: &str| wanted.is_empty() || name.to_lowercase().contains(&wanted);
    let mut picks = Vec::new();
    if has("All") {
        picks.push(Pick::All);
    }
    for (at, project, _) in face.listed() {
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

/// The projects Fleet could not read that Find leaves, drawn dimmed after the projects. **Never a pick.**
pub(super) fn unread_shown<'a>(face: &'a Face, query: &str) -> Vec<&'a Unread> {
    let wanted = query.trim().to_lowercase();
    face.unread.iter().filter(|one| wanted.is_empty() || one.name.to_lowercase().contains(&wanted)).collect()
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
        let find = cx.new(|cx| InputState::new(window, cx).placeholder("Find"));
        let typed = cx.subscribe_in(&find, window, |this, find, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.switcher.query = find.read(cx).value().to_string();
                this.switcher.at = 0;
                cx.notify();
            }
        });
        self.switcher = Switcher { query: String::new(), at, find: Some((find.clone(), typed)) };
        self.open_popup(super::Popup::Projects, window, cx);
        // After the popover takes the keys, Find has them.
        window.defer(cx, move |window, cx| find.update(cx, |find, cx| find.focus(window, cx)));
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

    /// A project made outside the switcher, by the New project sheet: Fleet's projects as it answered, and the
    /// window goes to the new one as a pick of it would.
    pub fn made_project(&mut self, served: &[ipc::RepositorySummary], read: &ipc::Projects, id: &str, cx: &mut Context<Self>) {
        self.set_repositories(served, cx);
        self.set_projects(served, read, cx);
        let Some(at) = self.face.projects.iter().position(|one| one.id.as_ref() == id) else { return };
        self.answer(|face| {
            face.scoped = Some(at);
            face.chosen = None;
        }, cx);
        cx.emit(Asked::Project(at));
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
        let was = self.switcher.at;
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
            // Letters, Backspace and the rest are Find's own.
            _ => return,
        }
        if self.switcher.at != was && matches!(keys.key.as_str(), "up" | "down") {
            crate::ui_sounds::moved();
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The popover: the list on the left, the row the keys are on on the right.
    pub(super) fn switcher_panel(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let face = &self.face;
        let rows = picks(face, &self.switcher.query);
        let on = rows.get(self.switcher.at).copied();
        let mut left: Vec<AnyElement> = self.find_box(look).into_iter().collect();
        let mut said_projects = false;
        let mut said_lone = false;
        let mut dimmed = Some(unread_shown(face, &self.switcher.query)).filter(|unread| !unread.is_empty());
        for (at, pick) in rows.iter().enumerate() {
            // New project ends every list, so the dimmed rows always land after the projects.
            if matches!(pick, Pick::Lone(_) | Pick::New) {
                if let Some(unread) = dimmed.take() {
                    if !said_projects {
                        said_projects = true;
                        left.push(caption("Projects", look));
                    }
                    left.extend(unread.into_iter().map(|one| unread_row(one, look)));
                }
            }
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

    fn find_box(&self, look: &Look) -> Option<AnyElement> {
        let (find, _) = self.switcher.find.as_ref()?;
        Some(labelled(
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
                .text_color(look.color(token::FG_DEFAULT))
                .child(glyph("search", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_SUBTLE)))
                .child(div().flex_1().min_w_0().child(Input::new(find).appearance(false))),
            Role::SearchInput,
            "Find a project",
        )
        .into_any_element())
    }

    fn pick_row(&self, at: usize, pick: Pick, on: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let face = &self.face;
        let (icon, ink, name, detail): (&str, crate::look::Token, SharedString, Option<&str>) = match pick {
            Pick::All => ("globe", token::FG_SUBTLE, "All".into(), None),
            Pick::Project(i) => ("layers", token::ACCENT, face.project_at(i).map(|one| one.name.clone()).unwrap_or_default(), None),
            Pick::Lone(i) => ("git-branch", token::FG_SUBTLE, face.repositories[i].name.clone(), face.repositories[i].not_set_up.then_some("Not set up")),
            Pick::New => ("plus", token::FG_SUBTLE, "New project".into(), None),
        };
        let quiet = pick == Pick::New;
        // A copy names the Mac that holds it on a line under its own, as the move picker does.
        let from = match pick {
            Pick::Project(i) => face.held_by(i).map(|mac| from_said(crate::pairing::macs(cx), mac)),
            _ => None,
        };
        let label = match &from {
            Some(from) => SharedString::from(format!("{name}, {from}")),
            None => name.clone(),
        };
        labelled(
            div()
                .id(SharedString::from(format!("pick:{at}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .cursor_pointer()
                .when(on, |row| row.bg(look.color(token::ACCENT_MUTED)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))))
                .when(!on, |row| row.hover(|style| style.bg(look.color(token::BG_HOVER))))
                .when(quiet, |row| row.text_color(look.color(token::FG_SUBTLE)))
                .child(glyph(icon, look.length(token::SPACE_4), look.color(ink)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().truncate().child(name.clone()))
                        .children(from.map(|from| div().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(from)))),
                )
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
            label,
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
                let Some(project) = face.project_at(at) else { return div().into_any_element() };
                let held_by = face.held_by(at).map(|mac| crate::job_view::name_of(crate::pairing::macs(cx), mac, false));
                let members = project
                    .members
                    .iter()
                    .filter_map(|one| face.repositories.get(one.repository).map(|repository| member(repository.name.clone(), one.folder.clone())))
                    .collect();
                let mut lines = Vec::new();
                if let Some(tracker) = &project.tracker {
                    lines.push(("Tracker", tracker.clone()));
                }
                // A copy's place is on the Mac that keeps it, which `on this Mac` would misname.
                if let Some(kept) = project.kept.as_ref().filter(|kept| held_by.is_none() || kept.kind != RunsPlaceKind::Mac) {
                    lines.push(("Kept", capitalised(&kept.phrase()).into()));
                }
                if let Some(mac) = held_by {
                    lines.push(("Kept by", mac.into()));
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
            .when(matches!(pick, Pick::Lone(_)), |acts| {
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
                    Btn::new("edit", "Edit", Weight::Secondary)
                        .icon("pencil")
                        .draw(look, cx)
                        .on_click(cx.listener(move |this, _, window, cx| this.leave_with(Asked::ProjectMembers(at), window, cx))),
                    Role::Button,
                    "Edit",
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

/// A project whose file did not read: its name and Fleet's reason under it, dimmed, with nothing to press.
fn unread_row(one: &Unread, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(format!("unread:{}", one.id)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_3))
            .text_color(look.color(token::FG_MUTED))
            .child(glyph("layers", look.length(token::SPACE_4), look.color(token::FG_SUBTLE)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().truncate().child(one.name.clone()))
                    .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(one.said.clone())),
            ),
        Role::ListBoxOption,
        format!("{}: {}", one.name, one.said),
    )
    .into_any_element()
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

    fn served() -> Vec<ipc::RepositorySummary> {
        crate::mock::repositories()
    }

    #[test]
    fn the_projects_are_what_fleet_serves_and_one_whose_file_did_not_read_is_left_out() {
        let face = scenes::rest();
        let names: Vec<&str> = face.projects.iter().map(|one| one.name.as_ref()).collect();
        assert_eq!(names, vec!["Armada", "Storefront"]);
        let storefront = &face.projects[1];
        assert_eq!(storefront.id.as_ref(), "storefront");
        assert_eq!(
            storefront.members,
            vec![
                super::super::face::Member { repository: 1, folder: Some("web/checkout".into()) },
                super::super::face::Member { repository: 2, folder: Some("services/pricing".into()) },
            ]
        );
        assert_eq!(storefront.kept.as_ref().map(|place| place.phrase()), Some("on this Mac".to_string()));
    }

    #[test]
    fn a_project_fleet_could_not_read_is_listed_dimmed_with_its_reason_and_is_not_a_pick() {
        let face = scenes::rest();
        let unread: Vec<(&str, &str)> = face.unread.iter().map(|one| (one.name.as_ref(), one.said.as_ref())).collect();
        assert_eq!(unread, vec![("Ledger", "members: a repository is listed twice")], "Fleet's own reason, as it said it");
        assert_eq!(picks(&face, "ledger"), vec![Pick::New], "it cannot be picked");
        let shown: Vec<&str> = unread_shown(&face, "ledger").iter().map(|one| one.name.as_ref()).collect();
        assert_eq!(shown, vec!["Ledger"], "but Find still lists it");
        assert!(unread_shown(&face, "store").is_empty());
    }

    #[test]
    fn a_member_fleet_serves_no_repository_for_is_left_out_of_its_project() {
        let served = served();
        let read = crate::mock::projects();
        let fewer = &served[..2];
        let listed = super::super::face::projects_of(fewer, &read);
        assert_eq!(listed[1].members.len(), 1, "sample-api is not served here");
        assert_eq!(listed[1].members[0].repository, 1);
    }

    #[test]
    fn a_copy_of_another_macs_project_is_not_one_of_his_own_but_is_listed_after_them() {
        let served = served();
        let json = r#"{"projects":[
            {"id":"own","name":"Armada","rows":[],"members":[],"place":{"place":"this_mac"}},
            {"id":"theirs","name":"Armada","rows":[],"members":[],"place":{"place":"this_mac"},"mac":"866126a3"}],"loose":[],"left_out":[]}"#;
        let read: ipc::Projects = ipc::decode("Projects", json.as_bytes()).expect("projects");
        let ids: Vec<String> = super::super::face::projects_of(&served, &read).iter().map(|one| one.id.to_string()).collect();
        assert_eq!(ids, vec!["own"]);
        let copies: Vec<(String, String)> = super::super::face::copies_of(&served, &read).iter().map(|one| (one.project.id.to_string(), one.mac.to_string())).collect();
        assert_eq!(copies, vec![("theirs".to_string(), "866126a3".to_string())]);
    }

    /// His projects as the mock has them, with a copy of another Mac's after them.
    fn read_with_a_copy() -> ipc::Projects {
        let mut read = crate::mock::projects();
        let mut copy = read.projects[0].clone();
        (copy.id, copy.name, copy.mac) = ("pricing-pro".to_string(), "Pricing".to_string(), Some("mac-pro".to_string()));
        read.projects.push(copy);
        read
    }

    #[test]
    fn the_switcher_lists_a_copy_after_his_own_projects_names_its_mac_and_scopes_the_window_to_it() {
        let mut face = scenes::rest();
        face.copies = super::super::face::copies_of(&served(), &read_with_a_copy());
        let own = face.projects.len();
        let listed: Vec<(usize, &str, Option<&str>)> = face.listed().map(|(at, one, mac)| (at, one.name.as_ref(), mac.map(|mac| mac.as_ref()))).collect();
        assert_eq!(listed, vec![(0, "Armada", None), (1, "Storefront", None), (2, "Pricing", Some("mac-pro"))]);
        let all = picks(&face, "");
        assert_eq!(all[..4], [Pick::All, Pick::Project(0), Pick::Project(1), Pick::Project(own)], "the copy follows his own");
        assert!(picks(&face, "pricing").contains(&Pick::Project(own)), "Find lists it by its name");
        assert_eq!(super::super::face::from_said(&[ipc::Mac { id: "mac-pro".into(), name: "Studio".into(), ..Default::default() }], "mac-pro"), "from Studio");
        assert_eq!(super::super::face::from_said(&[], "mac-gone"), "from another Mac");
        face.scoped = Some(own);
        assert_eq!(face.scoped_project(), Some("pricing-pro"), "the Jobs list and every project-scoped read take the copy's id");
        assert_eq!(face.chosen_said().0, SharedString::from("Pricing"));
        assert_eq!(face.settings_said(), SharedString::from("Settings for Pricing"));
        assert_eq!(current(&face), Pick::Project(own));
        assert_eq!(TitleBar::settings_of(Some(Pick::Project(own))), Asked::ProjectSettings(own));
    }

    #[gpui::test]
    fn a_copy_stays_scoped_across_refreshes_and_all_takes_the_window_back_once_it_is_not_listed(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        let (bar, cx) = cx.add_window_view(|window, cx| TitleBar::new(window, cx));
        let served = served();
        let read = read_with_a_copy();
        bar.update(cx, |bar, cx| {
            bar.set_repositories(&served, cx);
            bar.set_projects(&served, &read, cx);
        });
        let scoped = |bar: &gpui::Entity<TitleBar>, cx: &mut gpui::VisualTestContext| bar.read_with(cx, |bar, _| (bar.face.scoped, bar.face.scoped_project().map(str::to_string)));
        assert_eq!(scoped(&bar, cx), (None, None));
        bar.update_in(cx, |bar, window, cx| bar.switch_to(Pick::Project(2), window, cx));
        assert_eq!(scoped(&bar, cx), (Some(2), Some("pricing-pro".to_string())));
        // Fleet answers again with one more of his own ahead of the copy: the window stays on the copy.
        let mut more = read.clone();
        let mut extra = more.projects[0].clone();
        (extra.id, extra.name) = ("extra".to_string(), "Extra".to_string());
        more.projects.insert(0, extra);
        bar.update(cx, |bar, cx| bar.set_projects(&served, &more, cx));
        assert_eq!(scoped(&bar, cx), (Some(3), Some("pricing-pro".to_string())), "found by its id among the copies");
        // The same answer again changes nothing.
        bar.update(cx, |bar, cx| bar.set_projects(&served, &more, cx));
        assert_eq!(scoped(&bar, cx), (Some(3), Some("pricing-pro".to_string())));
        // Fleet no longer lists it: All.
        let mut without = read.clone();
        without.projects.retain(|one| one.mac.is_none());
        bar.update(cx, |bar, cx| bar.set_projects(&served, &without, cx));
        assert_eq!(scoped(&bar, cx), (None, None));
    }

    #[test]
    fn a_project_kept_in_a_synced_folder_or_a_bucket_says_where() {
        let served = served();
        let json = r#"{"projects":[
            {"id":"a","name":"A","rows":[],"members":[],"place":{"place":"synced","synced_folder":"iCloud Drive"}},
            {"id":"b","name":"B","rows":[],"members":[],"place":{"place":"cloud","cloud":{"bucket":"s3://b/p"}}},
            {"id":"c","name":"C","rows":[],"members":[],"place":{"place":"cloud","cloud":{"store":{}}}}],"loose":[],"left_out":[]}"#;
        let read: ipc::Projects = ipc::decode("Projects", json.as_bytes()).expect("projects");
        let said: Vec<String> = super::super::face::projects_of(&served, &read).iter().filter_map(|one| one.kept.as_ref().map(|place| place.phrase())).collect();
        assert_eq!(said, vec!["in iCloud Drive", "in s3://b/p", "in cloud storage"]);
    }
}

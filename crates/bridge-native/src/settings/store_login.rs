//! Settings › Claims store: the login a project joins its store as, from the logins signed in on this Mac
//! (`set_project_store` with `account_id`), and a member repository's own login over the project's
//! (`set_repository_profile`), drawn only where a repository has one or he opens one. `docs/concepts/account.md`,
//! *Which login a project joins as*.
//!
//! **Fleet says a repository's login only where it names one.** A repository set to None is kept by Fleet with no
//! profile to list it under, so its row stays while the page is open and is not drawn the next time.

use std::collections::{BTreeMap, BTreeSet};

use gpui::{div, prelude::*, svg, AnyElement, App, Context, DismissEvent, FocusHandle, Focusable, FontWeight, Keystroke, Role, SharedString, Task, Window};
use ipc::{Accounts, JoinStore, ProjectStoreState};

use super::overlay::{SettingsOverlay, AT};
use super::rows::{says, split_row, Row, Spread};
use super::store::refused_line;
use super::view::{bounds_of, ringed, tip};
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, Look};

/// The page's own part: the repositories he opened a row for, the act out and what it said, and each press's key.
pub struct Logins {
    pub(super) opened: BTreeSet<String>,
    pub(super) keys: BTreeMap<String, FocusHandle>,
    pub(super) out: bool,
    pub(super) refused: Option<String>,
    _act: Option<Task<()>>,
}

impl Logins {
    pub fn new() -> Logins {
        Logins { opened: BTreeSet::new(), keys: BTreeMap::new(), out: false, refused: None, _act: None }
    }
}

/// A repository of the project: its root, its folder's name, and the login it claims as over the project's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub root: String,
    pub name: String,
    pub login: Option<String>,
}

/// The project's repositories, in its order, each with the login a person chose for it where Fleet names one.
pub fn members(project: &ipc::Project, roots: &[(String, String, String)], accounts: Option<&Accounts>) -> Vec<Member> {
    let mut seen = BTreeSet::new();
    project
        .members
        .iter()
        .filter_map(|member| roots.iter().find(|(id, _, _)| id == member.manifest_id.as_str()))
        .filter(|(_, root, _)| seen.insert(root.clone()))
        .map(|(_, root, name)| {
            let login = accounts.and_then(|all| all.profiles.iter().find(|one| one.repositories.iter().any(|bound| bound.root == *root))).map(|one| one.login.clone());
            Member { root: root.clone(), name: name.clone(), login }
        })
        .collect()
}

/// The repositories drawn: those with a login of their own, and those he opened.
pub fn shown<'a>(members: &'a [Member], opened: &BTreeSet<String>) -> Vec<&'a Member> {
    members.iter().filter(|one| one.login.is_some() || opened.contains(&one.root)).collect()
}

/// The login a store names, where this Mac is signed in to it.
fn login_of(state: &ProjectStoreState, accounts: Option<&Accounts>) -> Option<String> {
    let id = state.store.as_ref()?.account_id?;
    accounts?.profiles.iter().find(|one| one.account_id == id).map(|one| one.login.clone())
}

impl SettingsOverlay {
    fn accounts_held(&self) -> Option<&Accounts> {
        self.account.accounts.as_ref().and_then(|read| read.as_ref().ok())
    }

    /// The project picked's repositories, with the login each claims as.
    fn store_members(&self, cx: &App) -> Vec<Member> {
        let Some((id, _)) = &self.scope.project else { return Vec::new() };
        let Some(project) = self.scope.projects.as_ref().and_then(|all| all.projects.iter().find(|one| one.id == *id)) else { return Vec::new() };
        members(project, &self.wire.repository_roots(cx), self.accounts_held())
    }

    pub(super) fn login_keys(&mut self, cx: &mut Context<Self>) {
        let mut names = vec!["joins-as".to_string(), "choose-repository".to_string()];
        names.extend(self.store_members(cx).into_iter().map(|one| format!("repo:{}", one.root)));
        for name in names {
            self.store.logins.keys.entry(name).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
    }

    /// The stops of the two rows, in the order they are drawn.
    pub(super) fn login_stops(&self, cx: &App) -> Vec<FocusHandle> {
        let keys = &self.store.logins.keys;
        let mut stops: Vec<FocusHandle> = keys.get("joins-as").cloned().into_iter().collect();
        let members = self.store_members(cx);
        stops.extend(shown(&members, &self.store.logins.opened).iter().filter_map(|one| keys.get(&format!("repo:{}", one.root)).cloned()));
        if members.iter().any(|one| one.login.is_none() && !self.store.logins.opened.contains(&one.root)) {
            stops.extend(keys.get("choose-repository").cloned());
        }
        stops
    }

    /// Enter, Space and ↓ open the menu of the press the keys are on.
    pub(super) fn login_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt || !matches!(key.key.as_str(), "enter" | "space" | "down") {
            return false;
        }
        let Some(name) = self.store.logins.keys.iter().find(|(_, focus)| focus.is_focused(window)).map(|(name, _)| name.clone()) else { return false };
        self.press_login(&name, true, window, cx);
        true
    }

    fn press_login(&mut self, name: &str, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.store.logins.out || self.kept_by(cx).is_some() {
            return;
        }
        let logins: Vec<(u64, String)> = self.accounts_held().map(|all| all.profiles.iter().map(|one| (one.account_id, one.login.clone())).collect()).unwrap_or_default();
        let members = self.store_members(cx);
        let (heard, entries): (String, Vec<Entry>) = if name == "joins-as" {
            let now = self.store.state.as_ref().and_then(|read| read.as_ref().ok()).and_then(|state| state.store.as_ref()).and_then(|store| store.account_id);
            let entries = logins.iter().enumerate().take(AT.len()).map(|(n, (id, login))| Entry::Choice { id: AT[n], label: login.clone().into(), on: now == Some(*id) }).collect();
            (self.joins_heard(cx), entries)
        } else if name == "choose-repository" {
            let open: Vec<&Member> = members.iter().filter(|one| one.login.is_none() && !self.store.logins.opened.contains(&one.root)).collect();
            let entries = open.iter().enumerate().take(AT.len()).map(|(n, one)| Entry::Act { id: AT[n], label: one.name.clone().into(), icon: Some("folder-git-2"), danger: false }).collect();
            ("Choose a login for a repository".to_string(), entries)
        } else if let Some(root) = name.strip_prefix("repo:") {
            let Some(member) = members.iter().find(|one| one.root == root) else { return };
            let mut entries: Vec<Entry> = logins.iter().enumerate().take(AT.len() - 1).map(|(n, (_, login))| Entry::Choice { id: AT[n], label: login.clone().into(), on: member.login.as_deref() == Some(login.as_str()) }).collect();
            entries.push(Entry::Rule);
            entries.push(Entry::Act { id: "none", label: "None".into(), icon: None, danger: false });
            (repo_heard(member), entries)
        } else {
            return;
        };
        if entries.is_empty() {
            return;
        }
        let at = bounds_of(window, cx, Role::ComboBox, &heard).map(|bounds| bounds.bottom_left()).unwrap_or_default();
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new(heard.clone(), entries, at, gpui::Anchor::TopLeft, from_keys, cx));
        let name = name.to_string();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            let at = AT.iter().position(|one| one == id);
            if name == "joins-as" {
                if let Some((account_id, _)) = at.and_then(|n| logins.get(n)) {
                    this.pick_store_login(*account_id, window, cx);
                }
            } else if name == "choose-repository" {
                let open: Vec<&Member> = members.iter().filter(|one| one.login.is_none() && !this.store.logins.opened.contains(&one.root)).collect();
                if let Some(one) = at.and_then(|n| open.get(n)) {
                    let root = one.root.clone();
                    this.store.logins.opened.insert(root.clone());
                    if let Some(to) = this.store.logins.keys.get(&format!("repo:{root}")).cloned() {
                        window.defer(cx, move |window, cx| window.focus(&to, cx));
                    }
                    cx.notify();
                }
            } else if let Some(root) = name.strip_prefix("repo:") {
                let account_id = match *id {
                    "none" => None,
                    _ => match at.and_then(|n| logins.get(n)) {
                        Some((account_id, _)) => Some(*account_id),
                        None => return,
                    },
                };
                this.pick_repository_login(root.to_string(), account_id, window, cx);
            }
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.popup = None;
            if let Some(back) = &back {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some((menu, vec![picked, dismissed]));
        cx.notify();
    }

    /// `POST /projects/project/:project/store` with the store it names and the login picked.
    fn pick_store_login(&mut self, account_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.store.project.clone() else { return };
        let Some(store) = self.store.state.as_ref().and_then(|read| read.as_ref().ok()).and_then(|state| state.store.clone()) else { return };
        self.store.logins.out = true;
        self.store.logins.refused = None;
        let press = self.wire.set_project_store(&project, Some(JoinStore { address: store.address, team: store.team, account_id: Some(account_id) }), cx);
        self.store.logins._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update(cx, |this, cx| {
                this.store.logins.out = false;
                match answer {
                    Ok(state) if this.store.project.as_deref() == Some(state.project.as_str()) => this.store.state = Some(Ok(state)),
                    Ok(_) => {}
                    Err(refused) => this.store.logins.refused = Some(refused.said),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// `POST /accounts/set_repository_profile`: a repository's own login, or None.
    fn pick_repository_login(&mut self, root: String, account_id: Option<u64>, window: &mut Window, cx: &mut Context<Self>) {
        self.store.logins.out = true;
        self.store.logins.refused = None;
        self.store.logins.opened.insert(root.clone());
        let press = self.wire.set_repository_profile(&root, account_id, cx);
        self.store.logins._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update(cx, |this, cx| {
                this.store.logins.out = false;
                match answer {
                    Ok(accounts) => this.account.accounts = Some(Ok(accounts)),
                    Err(refused) => this.store.logins.refused = Some(refused.said),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn joins_heard(&self, cx: &App) -> String {
        let state = self.store.state.as_ref().and_then(|read| read.as_ref().ok());
        match state.and_then(|state| login_of(state, self.accounts_held())) {
            Some(login) => format!("Joins as {login}"),
            None => "Joins as, none picked".to_string(),
        }
        .to_string()
        + if self.kept_by(cx).is_some() { ", kept on another Mac" } else { "" }
    }

    /// The rows under the store: Joins as, and the repositories with a login of their own. None without a store.
    pub(super) fn login_rows(&self, state: &ProjectStoreState, look: &Look, spread: Spread, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        if state.store.is_none() {
            return Vec::new();
        }
        let hue = look.color(token::KIND_DELIVERY);
        let accounts = self.accounts_held();
        let signed_in = accounts.is_some_and(|all| !all.profiles.is_empty());
        let login = login_of(state, accounts);
        let joins = self.select(look, "joins-as", login.as_deref().unwrap_or("Pick a login"), login.is_none(), !signed_in, self.joins_heard(cx), window, cx);
        let control = div()
            .flex()
            .flex_col()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(joins)
            .children((!signed_in).then(|| says(look, "Add a GitHub login on Account first.")))
            .children(self.store.logins.refused.as_ref().map(|said| refused_line(look, "store-login-refused", said)));
        let mut rows = vec![self.login_row(look, spread, "store-row-joins-as", "Joins as", "The login this Mac joins the store as. Its repositories claim as it.", "user-check", hue, control.into_any_element())];

        let members = self.store_members(cx);
        if members.is_empty() {
            return rows;
        }
        let drawn = shown(&members, &self.store.logins.opened);
        let mut list = div().w_full().flex().flex_col().gap(look.length(token::SPACE_2));
        for member in &drawn {
            let said = member.login.clone().unwrap_or_else(|| "Project's login".to_string());
            let item = div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .child(svg().flex_none().path(icon_path("folder-git-2")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(member.name.clone())))
                .child(div().flex_none().w(look.length(token::SPACE_12) * 3.).flex().justify_end().child(self.select(look, &format!("repo:{}", member.root), &said, member.login.is_none(), !signed_in, repo_heard(member), window, cx)));
            list = list.child(labelled(item.id(SharedString::from(format!("store-repo-{}", member.name))), Role::ListItem, member.name.clone()));
        }
        let more = members.iter().any(|one| one.login.is_none() && !self.store.logins.opened.contains(&one.root));
        let control = div()
            .w_full()
            .flex()
            .flex_col()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .children((!drawn.is_empty()).then(|| labelled(list.id("store-repos"), Role::List, "Repositories with their own login")))
            .children(more.then(|| self.choose_press(look, signed_in, window, cx)));
        rows.push(self.login_row(look, spread, "store-row-repositories", "Repositories", "A repository may join as another login than the project's.", "folder-git-2", hue, control.into_any_element()));
        rows
    }

    #[allow(clippy::too_many_arguments)]
    fn login_row(&self, look: &Look, spread: Spread, id: &'static str, title: &'static str, said: &'static str, glyph: &'static str, hue: gpui::Hsla, control: AnyElement) -> AnyElement {
        let row = Row { id: id.into(), title: title.into(), tip: None, chips: Vec::new(), says: vec![says(look, said).into_any_element()], changed: false, glyph: Some((glyph, hue)), control };
        labelled(split_row(look, spread, row, false), Role::Row, title).into_any_element()
    }

    /// A press that opens a list of choices, reading what is chosen; amber where nothing is.
    #[allow(clippy::too_many_arguments)]
    fn select(&self, look: &Look, key: &str, said: &str, blank: bool, off: bool, heard: String, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.store.logins.keys.get(key).cloned() else { return div().into_any_element() };
        let held = self.kept_by(cx);
        let off = off || held.is_some() || self.store.logins.out;
        let ink = if blank { look.color(token::STATUS_AWAITING_REVIEW) } else { look.color(token::FG_DEFAULT) };
        let name = key.to_string();
        let button = div()
            .id(SharedString::from(format!("store-{key}")))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_STRONG))
            .bg(look.color(token::BG_OVERLAY))
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(ink)
            .when(off, |button| button.opacity(0.5))
            .when_some(held, |button, mac| button.tooltip(tip(format!("Kept on {mac}. Only {mac} can change its store."))))
            .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| this.press_login(&name, false, window, cx))))
            .child(SharedString::from(said.to_string()))
            .child(svg().flex_none().path(icon_path("chevron-down")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)));
        labelled(ringed(look, button, &focus, window).track_focus(&focus), Role::ComboBox, heard).into_any_element()
    }

    fn choose_press(&self, look: &Look, signed_in: bool, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.store.logins.keys.get("choose-repository").cloned() else { return div().into_any_element() };
        let off = !signed_in || self.store.logins.out || self.kept_by(cx).is_some();
        let heard = "Choose a login for a repository";
        let button = div()
            .id("store-choose-repository")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(look.color(token::FG_MUTED))
            .when(off, |button| button.opacity(0.5))
            .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(|this, _, window, cx| this.press_login("choose-repository", false, window, cx))))
            .child(svg().flex_none().path(icon_path("plus")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
            .child("Choose for a repository");
        labelled(ringed(look, button, &focus, window).track_focus(&focus), Role::ComboBox, heard).into_any_element()
    }
}

fn repo_heard(member: &Member) -> String {
    match &member.login {
        Some(login) => format!("{} joins as {login}", member.name),
        None => format!("{} joins as the project's login", member.name),
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext, VisualTestContext};
    use ipc::{ManifestId, Project, ProjectMember};

    use super::super::mock::MockFleet;
    use super::super::mock_account::{AccountCall, MockAccounts, PERSONAL, WORK};
    use super::super::mock_store::StoreCall;
    use super::super::wire::Wire;
    use super::*;

    fn project(repositories: &[&str]) -> Project {
        Project { id: "armada".into(), name: "Armada".into(), members: repositories.iter().map(|one| ProjectMember { manifest_id: ManifestId::carried(*one), folder: None }).collect(), ..Project::default() }
    }

    fn roots() -> Vec<(String, String, String)> {
        [("armada", "/Users/user/Development/armada"), ("armada-docs", "/Users/user/Development/armada-docs"), ("sample-api", "/r/sample-api")].map(|(id, root)| (id.to_string(), root.to_string(), root.rsplit('/').next().unwrap().to_string())).to_vec()
    }

    #[test]
    fn a_member_names_the_login_its_repository_claims_as_where_fleet_says_one() {
        let accounts = MockAccounts::new().accounts;
        let found = members(&project(&["armada", "armada-docs", "armada", "gone"]), &roots(), Some(&accounts));
        assert_eq!(found.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["armada", "armada-docs"], "a repository is listed once, and one this Mac has not is left out");
        assert_eq!(found[0].login.as_deref(), Some("nmele-work"));
        assert_eq!(found[1].login, None);
        assert!(members(&project(&["armada"]), &roots(), None)[0].login.is_none(), "no logins read, none named");
    }

    #[test]
    fn a_repository_row_is_drawn_where_it_has_a_login_or_he_opened_it() {
        let accounts = MockAccounts::new().accounts;
        let found = members(&project(&["armada", "armada-docs", "sample-api"]), &roots(), Some(&accounts));
        let names = |opened: &BTreeSet<String>| shown(&found, opened).iter().map(|one| one.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&BTreeSet::new()), ["armada"]);
        assert_eq!(names(&BTreeSet::from(["/r/sample-api".to_string()])), ["armada", "sample-api"]);
    }

    fn opened<'a>(cx: &'a mut TestAppContext) -> (Entity<SettingsOverlay>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (overlay, cx) = cx.add_window_view(|window, cx| SettingsOverlay::new(Wire::Mock(Box::new(MockFleet::new())), window, cx));
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                this.open = true;
                this.live = true;
                this.picked = "project:claims-store";
                this.scope.project = Some(("ledger".into(), "Ledger".into()));
                this.store_keys(cx);
                this.read_store(window, cx);
            })
        });
        cx.run_until_parked();
        (overlay, cx)
    }

    #[gpui::test]
    fn picking_a_login_joins_the_store_at_its_address_as_that_login(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        assert_eq!(overlay.read_with(cx, |this, cx| this.joins_heard(cx)), "Joins as, none picked", "Ledger's store picks no login");
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.pick_store_login(WORK, window, cx)));
        cx.run_until_parked();
        overlay.read_with(cx, |this, cx| {
            let Wire::Mock(mock) = &this.wire else { unreachable!() };
            let Some(StoreCall::Set { project, store: Some(store) }) = mock.stores.calls.last() else { panic!("{:?}", mock.stores.calls) };
            assert_eq!((project.as_str(), store.address.as_str(), store.team.as_deref(), store.account_id), ("ledger", "https://armada-store-acme.acme.workers.dev", Some("acme"), Some(WORK)));
            assert_eq!(this.joins_heard(cx), "Joins as nmele-work");
        });
    }

    #[gpui::test]
    fn a_repositorys_login_is_set_by_its_root_and_none_keeps_its_row(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        let root = "/Users/user/Development/armada".to_string();
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.pick_repository_login(root.clone(), Some(PERSONAL), window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.pick_repository_login(root.clone(), None, window, cx)));
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            let Wire::Mock(mock) = &this.wire else { unreachable!() };
            assert_eq!(mock.accounts.calls, [AccountCall::SetRepository { root: root.clone(), account_id: Some(PERSONAL) }, AccountCall::SetRepository { root: root.clone(), account_id: None }]);
            assert!(this.store.logins.opened.contains(&root), "a row set to None stays while the page is open");
            assert!(this.store.logins.refused.is_none());
        });
    }

    #[gpui::test]
    fn a_login_this_mac_has_not_is_refused_in_fleets_words(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.pick_repository_login("/r/sample-api".into(), Some(1), window, cx)));
        cx.run_until_parked();
        assert_eq!(overlay.read_with(cx, |this, _| this.store.logins.refused.clone()).as_deref(), Some("This Mac has not joined that account."));
    }
}

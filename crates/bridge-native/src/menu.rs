//! The application menu. Mirrors `apps/desktop/src/main/app-menu.ts`: the app menu spelled out,
//! with "Revert to the build before…" after About, then Edit and Window.
//!
//! **The template is a pure function of its inputs** ([`template`]), so a test reads the items and
//! their enabled state with no app running; [`menus`] turns it into gpui's. Only the Revert item
//! depends on anything: it is on while [`revert::can_revert`] says so, and [`MenuController`] sets
//! the menus again when that moves and not otherwise.
//!
//! **The keys are the platform's**, the ones every Mac app's menu has (⌘Q, ⌘H, ⌥⌘H, ⌘M), bound
//! here and not in `actions.toml`, which holds Bridge's own acts and says of ⌘M that it is "the
//! platform's Minimize". Cut, Copy, Paste, Select All, Undo and Redo are gpui-component's text
//! editing actions, which it binds itself; they are enabled while a field that handles them has focus.
//!
//! **No File or View menu**, and no Close: Electron's roles bring those, and gpui offers no
//! counterpart, so they are not drawn until a screen asks for one.

use gpui::{
    actions, App, Context, Entity, KeyBinding, Menu, MenuItem, OsAction, Subscription, SystemMenuType,
};

use crate::fleet::FleetConnection;
use crate::log::Note;
use crate::revert;

actions!(app_menu, [About, RevertToBuildBefore, Hide, HideOthers, ShowAll, Quit, Minimize, Zoom]);

/// What an item does. Several are gpui-component's, which a text field handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    About,
    RevertToBuildBefore,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Minimize,
    Zoom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Item { label: String, command: Command, enabled: bool },
    Separator,
    /// The system's own Services submenu.
    Services,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Top {
    pub label: String,
    pub entries: Vec<Entry>,
}

fn item(label: &str, command: Command) -> Entry {
    Entry::Item { label: label.into(), command, enabled: true }
}

/// The menus. `name` is the app's; `revert` is whether Revert is on.
pub fn template(name: &str, revert: bool) -> Vec<Top> {
    vec![
        Top {
            label: name.into(),
            entries: vec![
                item(&format!("About {name}"), Command::About),
                Entry::Item { label: revert::REVERT_ITEM.into(), command: Command::RevertToBuildBefore, enabled: revert },
                Entry::Separator,
                Entry::Services,
                Entry::Separator,
                item(&format!("Hide {name}"), Command::Hide),
                item("Hide Others", Command::HideOthers),
                item("Show All", Command::ShowAll),
                Entry::Separator,
                item(&format!("Quit {name}"), Command::Quit),
            ],
        },
        Top {
            label: "Edit".into(),
            entries: vec![
                item("Undo", Command::Undo),
                item("Redo", Command::Redo),
                Entry::Separator,
                item("Cut", Command::Cut),
                item("Copy", Command::Copy),
                item("Paste", Command::Paste),
                item("Select All", Command::SelectAll),
            ],
        },
        Top { label: "Window".into(), entries: vec![item("Minimize", Command::Minimize), item("Zoom", Command::Zoom)] },
    ]
}

fn menu_item(entry: &Entry) -> MenuItem {
    use gpui_component::input as text;
    let (label, command, enabled) = match entry {
        Entry::Separator => return MenuItem::separator(),
        Entry::Services => return MenuItem::os_submenu("Services", SystemMenuType::Services),
        Entry::Item { label, command, enabled } => (label.as_str(), *command, *enabled),
    };
    let built = match command {
        Command::About => MenuItem::action(label, About),
        Command::RevertToBuildBefore => MenuItem::action(label, RevertToBuildBefore),
        Command::Hide => MenuItem::action(label, Hide),
        Command::HideOthers => MenuItem::action(label, HideOthers),
        Command::ShowAll => MenuItem::action(label, ShowAll),
        Command::Quit => MenuItem::action(label, Quit),
        Command::Undo => MenuItem::os_action(label, text::Undo, OsAction::Undo),
        Command::Redo => MenuItem::os_action(label, text::Redo, OsAction::Redo),
        Command::Cut => MenuItem::os_action(label, text::Cut, OsAction::Cut),
        Command::Copy => MenuItem::os_action(label, text::Copy, OsAction::Copy),
        Command::Paste => MenuItem::os_action(label, text::Paste, OsAction::Paste),
        Command::SelectAll => MenuItem::os_action(label, text::SelectAll, OsAction::SelectAll),
        Command::Minimize => MenuItem::action(label, Minimize),
        Command::Zoom => MenuItem::action(label, Zoom),
    };
    built.disabled(!enabled)
}

/// gpui's menus for `template`.
pub fn menus(template: &[Top]) -> Vec<Menu> {
    template.iter().map(|top| Menu::new(top.label.clone()).items(top.entries.iter().map(menu_item))).collect()
}

/// The app's name, as the first menu is titled.
const NAME: &str = "Armada";

/// The platform's keys, bound before the menus are set so the menu can show them.
fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// Every mode's menu: the keys, what each item does, and the menus with Revert off. Live mode's
/// [`MenuController`] sets them again with Revert following Fleet.
pub fn install(cx: &mut App) {
    cx.bind_keys(bindings());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Minimize, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.minimize_window());
        }
    });
    cx.on_action(|_: &Zoom, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.zoom_window());
        }
    });
    cx.on_action(|_: &About, cx| {
        cx.spawn(async move |cx| {
            let protocol = ipc::ProtocolId::current().to_string();
            revert::tell(cx, NAME, &format!("Protocol {protocol}")).await;
        })
        .detach();
    });
    cx.set_menus(menus(&template(NAME, false)));
}

/// Keeps the menu's Revert item in step with Fleet's build report, and runs the revert it asks for.
pub struct MenuController {
    fleet: Entity<FleetConnection>,
    /// Whether Revert was on in the menus last set; they are set again only when this moves.
    revert: bool,
    /// A confirm is open, or the revert it led to is being asked: a second press waits for it.
    asking: bool,
    _watch: Subscription,
}

impl MenuController {
    pub fn new(fleet: &Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        let watch = cx.observe(fleet, |this, fleet, cx| {
            let revert = revert::can_revert(fleet.read(cx).held().build.as_ref());
            if revert != this.revert {
                this.revert = revert;
                cx.set_menus(menus(&template(NAME, revert)));
            }
        });
        MenuController { fleet: fleet.clone(), revert: false, asking: false, _watch: watch }
    }

    /// Make Revert do something. The controller's entity is held by the window's root, and the handler
    /// holds the controller: both live as long as the app.
    pub fn handle_revert(this: &Entity<Self>, cx: &mut App) {
        let this = this.clone();
        cx.on_action(move |_: &RevertToBuildBefore, cx| this.update(cx, |this, cx| this.ask(cx)));
    }

    fn ask(&mut self, cx: &mut Context<Self>) {
        if self.asking {
            return;
        }
        self.asking = true;
        let fleet = self.fleet.clone();
        cx.spawn(async move |this, cx| {
            revert::ask(fleet, cx).await;
            let _ = this.update(cx, |this, _| this.asking = false);
        })
        .detach();
        Note::info("revert to the build before was asked from the menu").emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(top: &Top) -> Vec<String> {
        top.entries
            .iter()
            .map(|entry| match entry {
                Entry::Item { label, .. } => label.clone(),
                Entry::Separator => "-".into(),
                Entry::Services => "Services".into(),
            })
            .collect()
    }

    fn revert_item(menus: &[Top]) -> (&str, bool) {
        menus[0]
            .entries
            .iter()
            .find_map(|entry| match entry {
                Entry::Item { label, command: Command::RevertToBuildBefore, enabled } => Some((label.as_str(), *enabled)),
                _ => None,
            })
            .expect("the app menu has a revert item")
    }

    #[test]
    fn the_app_menu_is_named_for_the_app_and_follows_electrons_order() {
        let menus = template("Armada", true);
        assert_eq!(menus[0].label, "Armada");
        assert_eq!(
            labels(&menus[0]),
            [
                "About Armada",
                "Revert to the build before…",
                "-",
                "Services",
                "-",
                "Hide Armada",
                "Hide Others",
                "Show All",
                "-",
                "Quit Armada"
            ]
        );
    }

    #[test]
    fn edit_and_window_follow_the_app_menu() {
        let menus = template("Armada", true);
        assert_eq!(menus.iter().map(|top| top.label.as_str()).collect::<Vec<_>>(), ["Armada", "Edit", "Window"]);
        assert_eq!(labels(&menus[1]), ["Undo", "Redo", "-", "Cut", "Copy", "Paste", "Select All"]);
        assert_eq!(labels(&menus[2]), ["Minimize", "Zoom"]);
    }

    #[test]
    fn only_the_revert_item_follows_the_report() {
        assert_eq!(revert_item(&template("Armada", true)), ("Revert to the build before…", true));
        assert_eq!(revert_item(&template("Armada", false)), ("Revert to the build before…", false));
        let disabled = |menus: Vec<Top>| {
            menus
                .iter()
                .flat_map(|top| top.entries.iter())
                .filter(|entry| matches!(entry, Entry::Item { enabled: false, .. }))
                .count()
        };
        assert_eq!(disabled(template("Armada", true)), 0);
        assert_eq!(disabled(template("Armada", false)), 1);
    }

    #[test]
    fn quit_hide_and_minimize_have_the_platforms_keys() {
        // A keystroke gpui cannot read panics where it is built, so building them is the check.
        let keys = bindings();
        assert_eq!(keys.len(), 4);
        assert!(keys[0].action().as_any().is::<Quit>());
        assert_eq!(keys[0].keystrokes().len(), 1);
    }

    #[test]
    fn every_template_item_becomes_a_gpui_menu_item() {
        let built = menus(&template("Armada", true));
        assert_eq!(built.len(), 3);
        assert_eq!(built.iter().map(|menu| menu.items.len()).collect::<Vec<_>>(), [10, 7, 2]);
    }
}

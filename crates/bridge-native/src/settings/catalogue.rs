//! One list of what Settings holds, for anything that offers a setting outside the overlay: ⌘K's settings rows
//! read it, and each act goes back through the overlay's own save, so a change from the palette is saved, refused
//! and drawn as one made on the page. An entry the overlay does not list is not here either, so what is held from
//! the overlay is held from the palette.

use gpui::{App, Context, Hsla, Window};
use ipc::{ModKind, SettingKind, SettingValue};

use super::entries::{self, Body, Entry};
use super::overlay::{held_text, typed_value, SettingsOverlay};

/// One of a Pick's choices: the word saved, the title said, the group it is listed under, and its colours where
/// it has them (a theme's swatch).
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub word: String,
    pub title: String,
    pub group: Option<&'static str>,
    pub swatch: Option<Vec<Hsla>>,
}

/// What an item does when it is taken.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// One of several, the one in force named by its word.
    Pick { choices: Vec<Choice>, now: String },
    /// On or off, as it is now.
    Toggle(bool),
    /// A value typed, with what it holds now.
    Input(String),
    /// Settings opened on the entry.
    Open,
}

/// One thing Settings holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// `entry:<entry>`, `setting:<key>`, `theme` or `mod:<name>`.
    pub id: String,
    pub label: String,
    /// The entry it is drawn on, by id and name, and that entry's group.
    pub entry: &'static str,
    pub category: &'static str,
    pub group: &'static str,
    /// Words that find it besides its label.
    pub keywords: Vec<String>,
    pub kind: Kind,
}

/// What the person chose for an item.
#[derive(Clone, Debug, PartialEq)]
pub enum Applied {
    Pick(String),
    Toggle(bool),
    Input(String),
    Open,
}

fn item(entry: &'static Entry, id: String, label: String, keywords: Vec<String>, kind: Kind) -> Item {
    Item { id, label, entry: entry.id, category: entry.label, group: entry.group.label(), keywords, kind }
}

/// A schema setting as an item: a switch toggles, a choice picks, anything else is typed.
fn setting_item(entry: &'static Entry, one: &ipc::Setting) -> Item {
    let kind = match &one.kind {
        SettingKind::Boolean => Kind::Toggle(matches!(one.value, SettingValue::Bool(true))),
        SettingKind::Choice { options } => Kind::Pick {
            choices: options.iter().map(|word| Choice { word: word.clone(), title: word.clone(), group: None, swatch: None }).collect(),
            now: held_text(&one.value),
        },
        _ => Kind::Input(held_text(&one.value)),
    };
    let keywords = vec![one.key.clone(), one.description.clone(), one.section.clone()];
    item(entry, format!("setting:{}", one.key), one.title.clone(), keywords, kind)
}

impl SettingsOverlay {
    /// Everything Settings holds now, entry by entry in the overlay's order: each entry opens, then its settings.
    /// Before Fleet has answered, the entries alone.
    pub fn catalogue(&self, cx: &App) -> Vec<Item> {
        let mut items = Vec::new();
        for entry in entries::visible("", self.list.as_ref(), false).into_iter().filter(|one| self.offered(one, cx)) {
            items.push(item(entry, format!("entry:{}", entry.id), entry.label.to_string(), vec![entries::words_of(entry, self.list.as_ref())], Kind::Open));
            match entry.body {
                Body::Schema(ids) => {
                    for id in ids {
                        if let Some(list) = &self.list {
                            items.extend(entries::settings_in(list, id).into_iter().map(|one| setting_item(entry, one)));
                        }
                    }
                }
                Body::Features => {
                    if let Some(list) = &self.list {
                        items.extend(entries::settings_in(list, "features").into_iter().map(|one| setting_item(entry, one)));
                    }
                }
                Body::Theme => {
                    let choices = self.theme_choices(cx).into_iter().map(|(group, one)| Choice { word: one.id, title: one.title, group: Some(group), swatch: one.strip }).collect();
                    items.push(item(entry, "theme".into(), "Theme".into(), vec!["colours".into(), "dark".into(), "light".into(), "appearance".into()], Kind::Pick { choices, now: self.theme_word(cx) }));
                    items.extend(self.mod_items(entry, super::mods::Page::Theme, cx));
                }
                Body::Mods(Some(_)) => items.extend(self.mod_items(entry, super::mods::page_of(entry.body).unwrap_or(super::mods::Page::All), cx)),
                Body::Mods(None) | Body::Repository | Body::ProjectValues | Body::Keyboard => {}
            }
        }
        items
    }

    /// Whether the overlay lists an entry now, as `visible` decides it with nothing searched.
    fn offered(&self, entry: &Entry, cx: &App) -> bool {
        match entry.body {
            Body::Repository => self.setting(super::view::CONNECTORS_OFF).is_some(),
            Body::Mods(Some(kind)) => self.wire.mods(cx).iter().any(|row| row.kind == Some(kind)),
            Body::Schema(ids) => self.list.as_ref().is_none_or(|list| ids.iter().any(|id| !entries::settings_in(list, id).is_empty())),
            _ => true,
        }
    }

    /// A page's mods, each its switch.
    fn mod_items(&self, entry: &'static Entry, page: super::mods::Page, cx: &App) -> Vec<Item> {
        self.mod_rows(page, cx)
            .into_iter()
            .map(|one| item(entry, format!("mod:{}", one.name), one.title.clone(), vec!["mod".into(), super::mods::kind_said(one.kind).to_string()], Kind::Toggle(one.enabled)))
            .collect()
    }

    /// Take an item: through the save the overlay's own control uses, so a refusal is held and said as it is there.
    pub fn apply(&mut self, id: &str, applied: Applied, window: &mut Window, cx: &mut Context<Self>) {
        match (id.split_once(':'), applied) {
            (Some(("entry", entry)), Applied::Open) => {
                let Some(entry) = entries::entry(entry) else { return };
                self.show(None, window, cx);
                self.pick(entry.id, cx);
            }
            (Some(("setting", key)), applied) => {
                let Some(one) = self.setting(key).cloned() else { return };
                let value = match applied {
                    Applied::Toggle(on) => Some(SettingValue::Bool(on)),
                    Applied::Pick(word) => Some(SettingValue::Text(word)),
                    Applied::Input(text) => typed_value(&one.kind, &text),
                    Applied::Open => None,
                };
                if let Some(value) = value {
                    self.save(one.key, Some(value), window, cx);
                }
            }
            (Some(("mod", name)), Applied::Toggle(on)) => {
                let Some(kind) = self.wire.mods(cx).iter().find(|row| row.name == name).map(|row| row.kind.unwrap_or(ModKind::Theme)) else { return };
                self.switch_mod(name.to_string(), kind, on, window, cx);
            }
            (None, Applied::Pick(word)) if id == "theme" => self.choose_theme(word, window, cx),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> ipc::SettingsList {
        super::super::mock::settings()
    }

    #[test]
    fn a_switch_toggles_a_choice_picks_and_the_rest_is_typed() {
        let list = list();
        let entry = entries::entry("fleet:features").expect("Features");
        let items: Vec<Item> = list.settings.iter().filter(|one| entries::drawn(one)).map(|one| setting_item(entry, one)).collect();
        let kind_of = |pred: &dyn Fn(&ipc::Setting) -> bool| list.settings.iter().filter(|one| entries::drawn(one)).position(pred).map(|at| items[at].kind.clone());
        assert!(matches!(kind_of(&|one| matches!(one.kind, SettingKind::Boolean)), Some(Kind::Toggle(_))));
        assert!(matches!(kind_of(&|one| matches!(one.kind, SettingKind::Choice { .. })), Some(Kind::Pick { .. })));
        assert!(matches!(kind_of(&|one| matches!(one.kind, SettingKind::Integer { .. })), Some(Kind::Input(_))));
    }
}

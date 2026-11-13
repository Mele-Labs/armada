//! One list of what Settings holds, for anything that offers a setting outside the overlay: ⌘K's settings rows
//! read it, and each act goes back through the overlay's own save, so a change from the palette is saved, refused
//! and drawn as one made on the page. An entry the overlay does not list is not here either, so what is held from
//! the overlay is held from the palette. Keyboard shortcuts and the folders a Job reaches only open their page: a
//! key is bound by pressing it, and a folder list is edited row by row, neither of which one typed line can be.

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
    /// `entry:<entry>`, `setting:<key>`, `theme`, `mod:<name>`, `connectors:<manifest>`, `model:<name>`, `workflow:<row>`, or a run's
    /// model or effort by its place on Models, `run:<n>` and `effort:<n>`.
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
        for entry in entries::visible("", self.list.as_ref()).into_iter().filter(|one| self.offered(one, cx)) {
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
                Body::Repository => {
                    let set = super::view::connectors_set(self.setting(super::view::CONNECTORS_OFF).map(|one| &one.value));
                    for (id, name, yml_off) in self.wire.repositories(cx).unwrap_or_default() {
                        let on = !set.get(&id).copied().unwrap_or(yml_off);
                        items.push(item(entry, format!("connectors:{id}"), format!("Connectors in {name}"), vec!["connectors".into(), "notion".into(), "drive".into(), "slack".into(), name], Kind::Toggle(on)));
                    }
                }
                Body::Models => items.extend(self.model_items(entry)),
                Body::Workflows => items.extend(self.workflow_items(entry, cx)),
                Body::Mods(None) | Body::Values(_) | Body::Keyboard | Body::Macs | Body::Layout | Body::Time | Body::Ways => {}
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

    /// Models once Fleet has listed them: whether Jobs may use each, then each run's model and effort.
    fn model_items(&self, entry: &'static Entry) -> Vec<Item> {
        use super::models::{effort_said, Read, RUNS};
        let Read::Read(models) = self.kit_models() else { return Vec::new() };
        let on = models.iter().filter(|one| one.on).count();
        let mut items: Vec<Item> = models
            .iter()
            .filter(|one| !(one.on && on == 1))
            .map(|one| item(entry, format!("model:{}", one.name), format!("{} may be used", one.display), vec!["model".into(), "roster".into(), one.name.clone()], Kind::Toggle(one.on)))
            .collect();
        let choices: Vec<Choice> = models.iter().filter(|one| one.on).map(|one| Choice { word: one.name.clone(), title: one.display.clone(), group: None, swatch: None }).collect();
        let steps: Vec<Choice> = self.effort_steps(&models).into_iter().map(|word| Choice { title: effort_said(&word), word, group: None, swatch: None }).collect();
        for (at, run) in RUNS.iter().enumerate() {
            items.push(item(entry, format!("run:{at}"), run.label.to_string(), vec!["model".into(), "default models".into()], Kind::Pick { choices: choices.clone(), now: self.slot_word(run.model) }));
            if let Some(slot) = run.effort {
                items.push(item(entry, format!("effort:{at}"), format!("{} effort", run.label), vec!["effort".into(), "reasoning".into()], Kind::Pick { choices: steps.clone(), now: self.slot_word(slot) }));
            }
        }
        items
    }

    /// Workflows: each row's switch, where it switches.
    fn workflow_items(&self, entry: &'static Entry, cx: &App) -> Vec<Item> {
        self.workflow_switches(cx)
            .into_iter()
            .map(|(key, said, on)| item(entry, format!("workflow:{key}"), said, vec!["workflow".into(), "switch".into()], Kind::Toggle(on)))
            .collect()
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
                self.show(None, false, window, cx);
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
            (Some(("connectors", manifest)), Applied::Toggle(on)) => {
                let Some(yml_off) = self.wire.repositories(cx).unwrap_or_default().into_iter().find(|(id, _, _)| id == manifest).map(|(_, _, off)| off) else { return };
                self.switch_connectors(manifest, yml_off, on, window, cx);
            }
            (Some(("workflow", key)), Applied::Toggle(on)) => self.apply_workflow(key, on, window, cx),
            (Some(("model", name)), Applied::Toggle(on)) => self.switch_model(name, on, window, cx),
            (Some(("run", at)), Applied::Pick(word)) => {
                if let Some(run) = at.parse::<usize>().ok().and_then(|at| super::models::RUNS.get(at)) {
                    self.choose_slot(run.model, word, window, cx);
                }
            }
            (Some(("effort", at)), Applied::Pick(word)) => {
                if let Some(slot) = at.parse::<usize>().ok().and_then(|at| super::models::RUNS.get(at)).and_then(|run| run.effort) {
                    self.choose_slot(slot, word, window, cx);
                }
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

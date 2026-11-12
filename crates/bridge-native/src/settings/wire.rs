//! Settings' routes, each a request off the main thread on the port the Fleet connection read, or answered at
//! once by the walk's and the tests' mock. `crates/ipc/operations/get_settings.toml`, `save_settings.toml`,
//! `get_values.toml` and `edit_project.toml`.

use std::collections::BTreeMap;

use gpui::{App, AppContext, Entity, Task, Window};
use ipc::{EditProject, Projects, ResolvedValues, SaveSettings, SettingValue, SettingsList, ValuesSet};
use serde::de::DeserializeOwned;

use super::mock::MockFleet;
use crate::fleet::FleetConnection;

/// Why a save did not take: Fleet's own sentence, and the key it named where it named one
/// (`fleet.unacceptable_settings` carries it in `fields.key`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub key: Option<String>,
    pub said: String,
}

impl Refused {
    pub fn said(said: impl Into<String>) -> Refused {
        Refused { key: None, said: said.into() }
    }
}

pub enum Wire {
    Live(Entity<FleetConnection>),
    Mock(Box<MockFleet>),
}

enum Ask {
    Get(String),
    Post(String, String),
}

fn request<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, cx: &App) -> Task<Result<T, Refused>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| Refused::said(why.to_string()))?;
        let (status, bytes) = match ask {
            Ask::Get(path) => runtime.block_on(crate::transport::get(port, &path)),
            Ask::Post(path, body) => runtime.block_on(crate::transport::post(port, &path, body)),
        }
        .map_err(Refused::said)?;
        if !(200..300).contains(&status) {
            return Err(match ipc::decode::<ipc::WireError>("WireError", &bytes) {
                Ok(error) => {
                    let key = match error.fields.get("key") {
                        Some(ipc::WireValue::Str(key)) => Some(key.clone()),
                        _ => None,
                    };
                    Refused { key, said: error.message }
                }
                Err(_) => Refused::said(format!("Fleet answered {status}")),
            });
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| Refused::said(why.to_string()))
    })
}

fn body(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn unreachable<T: Send + 'static>() -> Task<Result<T, Refused>> {
    Task::ready(Err(Refused::said("Fleet is not connected")))
}

impl Wire {
    fn port(&self, cx: &App) -> Option<u16> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().port(),
            Wire::Mock(_) => None,
        }
    }

    /// Whether a change can be sent: a live Fleet that answers, or the mock, which always does.
    pub fn live(&self, cx: &App) -> bool {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().port().is_some(),
            Wire::Mock(mock) => mock.connected,
        }
    }

    /// `GET /settings`.
    pub fn settings(&mut self, cx: &App) -> Task<Result<SettingsList, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.settings());
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/settings".into()), "SettingsList", cx))
    }

    /// `POST /settings/save`: each change set, or removed where it is `None`. Fleet answers with the whole list.
    pub fn save(&mut self, changes: BTreeMap<String, Option<SettingValue>>, cx: &App) -> Task<Result<SettingsList, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.save(&changes));
        }
        let said = body(&SaveSettings { changes });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/settings/save".into(), said), "SettingsList", cx))
    }

    /// The repositories Fleet serves, by Manifest id, name and whether its armada.yml turns connectors off.
    pub fn repositories(&self, cx: &App) -> Option<Vec<(String, String, bool)>> {
        let held = match self {
            Wire::Live(fleet) => fleet.read(cx).held().repositories.clone()?,
            Wire::Mock(mock) => mock.repositories.clone(),
        };
        Some(held.into_iter().filter_map(|one| one.manifest.map(|manifest| (manifest.id.as_str().to_string(), manifest.repository, manifest.connectors_off))).collect())
    }

    /// `GET /projects/values?project=<id>`.
    pub fn values(&mut self, project: &str, cx: &App) -> Task<Result<ResolvedValues, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.values(project));
        }
        let path = format!("/projects/values?project={}", crate::link::percent_encoded(project));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "ResolvedValues", cx))
    }

    /// `GET /projects`, read fresh before a project's values are written whole.
    pub fn projects_now(&mut self, cx: &App) -> Task<Result<Projects, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.projects.clone()));
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/projects".into()), "Projects", cx))
    }

    /// `POST /projects/project/:id/edit` with the project's whole block of values.
    pub fn set_values(&mut self, project: &str, values: ValuesSet, cx: &App) -> Task<Result<Projects, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.set_values(project, values));
        }
        let path = format!("/projects/project/{}/edit", crate::link::percent_encoded(project));
        let said = body(&EditProject { name: None, members: None, place: None, values: Some(values) });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "Projects", cx))
    }

    /// The person's key bindings as Fleet last said them, or the mock's.
    pub fn key_bindings(&self, cx: &App) -> String {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().key_bindings.clone().unwrap_or_default(),
            Wire::Mock(mock) => mock.preferences.key_bindings.clone(),
        }
    }

    /// `POST /preferences/save` with a preference that is text. Fleet answers with every preference in force.
    pub fn save_preference(&mut self, name: &str, text: String, cx: &App) -> Task<Result<ipc::Preferences, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.save_preference(name, text));
        }
        let said = body(&ipc::SavePreference { name: name.to_string(), value: false, text: Some(text) });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/preferences/save".into(), said), "Preferences", cx))
    }

    /// Every preference as Fleet answered a save: folded into what the connection holds, so the look and the keys
    /// follow it as they do a read. The mock's walk has no controller to follow it, so it is put in force here.
    pub fn preferences_saved(&mut self, answer: ipc::Preferences, window: &mut Window, cx: &mut App) {
        match self {
            Wire::Live(fleet) => fleet.update(cx, |fleet, cx| fleet.preferences_saved(answer, cx)),
            Wire::Mock(mock) => {
                crate::keymap::install(&crate::keymap::Rebinding::read(&answer.key_bindings), cx);
                crate::look::install(super::theme::look_of(&answer.theme, &mock.sheets, window), cx);
            }
        }
    }

    /// A theme picked, drawn before Fleet answers. Only the mock draws it here: over a live Fleet the look
    /// follows the answer, which is the one place the theme in force is decided.
    pub fn show_theme(&self, word: &str, window: &mut Window, cx: &mut App) {
        if let Wire::Mock(mock) = self {
            crate::look::install(super::theme::look_of(word, &mock.sheets, window), cx);
        }
    }

    /// The theme word in force, as Fleet last said it.
    pub fn theme(&self, cx: &App) -> String {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().theme.clone().unwrap_or_else(|| ipc::DEFAULT_THEME.to_string()),
            Wire::Mock(mock) => mock.preferences.theme.clone(),
        }
    }

    /// Fleet's mod list as it stands.
    pub fn mods(&self, cx: &App) -> Vec<ipc::ModSummary> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().mods.clone().unwrap_or_default(),
            Wire::Mock(mock) => mock.mods.clone(),
        }
    }

    /// The checked stylesheets of the theme mods that are on and valid, by name.
    pub fn theme_sheets(&self, cx: &App) -> BTreeMap<String, String> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().theme_sheets(),
            Wire::Mock(mock) => mock.sheets.clone(),
        }
    }

    /// `POST /mods/enable`. Fleet answers with the mod's row, and publishes the list again.
    pub fn set_mod_enabled(&mut self, name: &str, enabled: bool, cx: &App) -> Task<Result<ipc::ModSummary, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.set_mod_enabled(name, enabled));
        }
        let said = body(&ipc::SetModEnabled { name: name.to_string(), enabled });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/enable".into(), said), "ModSummary", cx))
    }

    /// `POST /mods/promote`, into the first repository Fleet serves, as Classic's Promote sends it.
    pub fn promote_mod(&mut self, name: &str, cx: &App) -> Task<Result<ipc::ModPromoted, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.promote_mod(name));
        }
        let said = body(&ipc::PromoteMod { name: name.to_string(), manifest_id: None });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/promote".into(), said), "ModPromoted", cx))
    }

    /// The connection a mod's panel reads its row from, or none for the mock, whose panel reads the walk's mods.
    pub fn fleet(&self) -> Option<Entity<FleetConnection>> {
        match self {
            Wire::Live(fleet) => Some(fleet.clone()),
            Wire::Mock(_) => None,
        }
    }

    /// `GET /mods/validate?name=`: the file Fleet checked for a mod, which its row opens to.
    pub fn validate_mod(&mut self, name: &str, cx: &App) -> Task<Result<ipc::ModChecked, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.validate_mod(name));
        }
        let path = format!("/mods/validate?name={}", crate::link::percent_encoded(name));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "ModChecked", cx))
    }

    /// `POST /mods/try`: a tracker mod tried against the repository Fleet picks for it.
    pub fn try_tracker_mod(&mut self, name: &str, cx: &App) -> Task<Result<ipc::TrackerTry, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.try_tracker_mod(name));
        }
        let said = body(&ipc::TryTrackerMod { name: name.to_string(), manifest_id: None });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/try".into(), said), "TrackerTry", cx))
    }

    /// `POST /mods/session` with nothing set: Create mod's empty Mod Job (`startEmptyModJob`).
    pub fn start_mod_session(&mut self, cx: &App) -> Task<Result<ipc::ModSessionStarted, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.start_mod_session());
        }
        let said = body(&ipc::StartModSession { title: None, model: None, effort: None, aim: None, note: None, said: None, from_job: None, from_tab: None });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/session".into(), said), "ModSessionStarted", cx))
    }

    /// Open settings.json in the editor settings.json names, off the main thread. The mock opens nothing.
    pub fn open_file(&mut self, list: &SettingsList, cx: &App) -> Task<Result<(), String>> {
        if let Wire::Mock(mock) = self {
            mock.opened += 1;
            return Task::ready(Ok(()));
        }
        let (editor, path) = (crate::editor::configured(list), list.path.clone());
        cx.background_spawn(async move { crate::editor::open(&editor, &path, 1) })
    }
}

/// A project's own values with a new `reviews`, every other key it sets kept as it was: the edit replaces the
/// whole block, so a save that sent only `reviews` would take the rest back.
pub fn with_reviews(own: Option<&ValuesSet>, reviews: ipc::ReviewsSet) -> ValuesSet {
    ValuesSet { reviews: Some(reviews), ..own.cloned().unwrap_or_default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{LookSet, PrMode, ReviewsSet};

    #[test]
    fn a_reviews_change_keeps_the_projects_other_five_values() {
        let own = ValuesSet {
            pr_mode: PrMode::from_wire("draft"),
            auto_merge: Some("checks-pass".into()),
            review_model: Some("opus".into()),
            look: Some(LookSet { layout: None, theme: Some("dusk".into()), density: Some("roomy".into()) }),
            tracker: Some([("provider".to_string(), "linear".to_string())].into_iter().collect()),
            reviews: Some(ReviewsSet { plans: Some("review".into()), pull_requests: Some("off".into()), walks: Some("off".into()) }),
        };
        let wanted = ReviewsSet { plans: Some("review_and_approve".into()), pull_requests: Some("review".into()), walks: Some("review".into()) };
        let sent = with_reviews(Some(&own), wanted.clone());
        assert_eq!(sent, ValuesSet { reviews: Some(wanted), ..own });
    }

    #[test]
    fn a_project_that_sets_nothing_sends_its_reviews_alone() {
        let wanted = ReviewsSet { plans: Some("review".into()), pull_requests: None, walks: None };
        assert_eq!(with_reviews(None, wanted.clone()), ValuesSet { reviews: Some(wanted), ..ValuesSet::default() });
    }
}

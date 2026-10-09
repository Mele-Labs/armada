//! A person's Bridge preferences — `crate::limits`'s shape one table over,
//! without the admission overlay: nothing here is applied to a running
//! Drone, so there is no "shipped, overlaid by saved" to compute.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Preferences, SavePreference};
use store::LoadJobError;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The store's shape, as the wire spells it. A plain field-for-field copy
/// rather than a `From` impl: one call site, and `ipc` already depends on
/// nothing here that would make an `impl` worth naming.
fn as_wire(preferences: store::Preferences) -> Preferences {
    Preferences {
        where_things_are_open: preferences.where_things_are_open,
        draft_pull_requests: preferences.draft_pull_requests,
        theme: preferences.theme,
        layout_choices: preferences.layout_choices,
    }
}

/// A `save_preferences` for `theme` carrying nothing a theme can be called. A 422.
const UNACCEPTABLE_THEME: &str = "fleet.unacceptable_theme";
/// A `save_preferences` for `layout_choices` carrying text `layout.json` would refuse. A 422.
const UNACCEPTABLE_LAYOUT: &str = "fleet.unacceptable_layout";

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Every preference, read straight from the store. **No cache**, unlike
    /// `limits_in_force`: this is read once per connection rather than on
    /// every admission turn, so there is nothing here worth keeping in memory
    /// between saves.
    pub(crate) async fn preferences_in_force(&self) -> Result<Preferences, Adrift> {
        let store = self.store().lock().await;
        store
            .preferences()
            .map(as_wire)
            .map_err(|fault| Adrift::Reading(LoadJobError::Database(fault)))
    }

    /// Save one preference, then answer with every preference now in force.
    ///
    /// **Answers `Refusal` rather than `Adrift`**, unlike `preferences_in_force`
    /// — commanding.rs's `impl Commands` is at the gate's own line limit, and
    /// mapping here rather than at the one call site is what keeps that arm to
    /// a single line.
    pub(crate) async fn save_preference_now(
        &self,
        save: SavePreference,
    ) -> Result<Preferences, Refusal> {
        let saving_theme = save.name == "theme";
        if saving_theme {
            let text = save.text.as_deref().unwrap_or_default();
            if let Some(why) = crate::mods::theme_id_problem(text) {
                return Err(Refusal::Unacceptable(
                    ipc::WireError::raised(UNACCEPTABLE_THEME, why, self.run_id())
                        .with_field("theme", ipc::WireValue::Str(text.to_string())),
                ));
            }
        }
        let saving_layout = save.name == "layout_choices";
        if saving_layout {
            let text = save.text.as_deref().unwrap_or_default();
            // Empty takes the choices back; anything else is a layout.json or it is refused.
            if let Some(first) = ipc::layout::problems(text).first().filter(|_| !text.is_empty()) {
                return Err(Refusal::Unacceptable(ipc::WireError::raised(
                    UNACCEPTABLE_LAYOUT,
                    format!("the layout choices are not a layout.json: {first}"),
                    self.run_id(),
                )));
            }
        }
        let mut store = self.store().lock().await;
        let saved = match (saving_theme, saving_layout) {
            (true, _) => store.save_theme(save.text.as_deref().unwrap_or_default()),
            (_, true) => store.save_layout_choices(save.text.as_deref().unwrap_or_default()),
            _ => store.save_preference(&save.name, save.value),
        };
        saved
            .map(as_wire)
            .map_err(Adrift::Writing)
            .map_err(|why| self.refusal(why))
    }
}

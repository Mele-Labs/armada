//! The owner's folder list, read for the guard and written by an "always" answer. Kept in
//! settings.json under `features.reachFolders`, where his other choices have been since 10 Oct 2026,
//! so Settings shows it and a hand edit reaches the next call.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Key};
use ipc::reach_folders::ReachFolders;
use ipc::SaveSettings;
use store::settings_file;

use crate::daemon::Fleet;

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
    /// Both lists, off the last good read of the file. Absent is empty.
    pub(crate) fn reach_folders(&self) -> ReachFolders {
        let value: Option<settings_file::SettingValue> = self.machine_settings().now().get(keys::REACH_FOLDERS);
        value.map(|value| ReachFolders::read(&value.to_json_text())).unwrap_or_default()
    }

    /// `folder` put on the allow list or the block list, and off the other.
    pub(crate) async fn reach_folder_kept(&self, folder: &str, blocked: bool) -> Result<(), Refusal> {
        let kept = self.reach_folders().with(folder, blocked);
        let text = ipc::encode(&kept).unwrap_or_default();
        let value = settings_file::SettingValue::from_json_text(&text)
            .ok()
            .map(|value| crate::settings::wire_value(&value));
        let change = SaveSettings {
            changes: [(keys::REACH_FOLDERS.name().to_string(), value)].into_iter().collect(),
        };
        self.save_settings_now(change).await.map(|_| ())
    }
}

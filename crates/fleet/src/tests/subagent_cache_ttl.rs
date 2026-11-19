//! A hosted Job is started with `tokens.subagentCacheTtl` as it stands at that moment: five minutes
//! where nothing is saved, and what a save changed it to for the Jobs started after the save.
//! `docs/concepts/session/where-the-tokens-went.md`, *Settings that change it*.

use super::session_host::{eventually, rig_fitted, Rig};
use super::tmp::TempDir;

/// A rig whose settings.json holds `contents`.
pub(super) fn rig_with(contents: &str) -> (Rig, TempDir) {
    let home = TempDir::new();
    let file = super::settings_json::settings_file_in(&home);
    std::fs::write(&file, contents).unwrap();
    let rig = rig_fitted(move |fittings| {
        fittings.settings =
            crate::settings::MachineSettings::at(file, super::settings_json::supplied(), config::settings::Env::none());
    });
    (rig, home)
}

/// A rig whose settings.json holds `contents`, and the first process a Job started on it got.
async fn started_with(contents: &str) -> (Rig, crate::session_host::Start, TempDir) {
    let (rig, home) = rig_with(contents);
    let id = rig.start().await;
    rig.send(&id, "go").await;
    eventually(|| async { !rig.stand_in.starts().is_empty() }).await;
    let start = rig.stand_in.starts().into_iter().next().expect("a process started");
    (rig, start, home)
}

#[tokio::test]
async fn a_started_job_is_handed_five_minutes_until_the_setting_says_an_hour() {
    let (_rig, start, _home) = started_with("{}").await;
    assert_eq!(start.subagent_cache_ttl, "5m", "nothing saved ships five minutes");
    let (_rig, start, _home) = started_with(r#"{"tokens.subagentCacheTtl": "1h"}"#).await;
    assert_eq!(start.subagent_cache_ttl, "1h");
}

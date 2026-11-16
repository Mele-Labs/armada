//! Kit's routes, each a request off the main thread on the port the Fleet connection read.
//! `crates/ipc/operations/get_kit_inventory.toml` and the Kit operations beside it.

use std::collections::BTreeMap;

use gpui::{App, Task};
use ipc::{KitInventory, KitPieceInstalled, KitPieceRemoved, KitServers, KitSourceFound, SettingValue, SettingsList};

use crate::settings::wire::{body, request, scoped, Ask, Refused};

/// `GET /kit/inventory`.
pub fn inventory(port: u16, cx: &App) -> Task<Result<KitInventory, Refused>> {
    request(port, Ask::Get("/kit/inventory".into()), "KitInventory", cx)
}

/// `GET /kit/servers`, in the Manifest the switch writes to.
pub fn servers(port: u16, manifest: &str, cx: &App) -> Task<Result<KitServers, Refused>> {
    request(port, Ask::Get(scoped("/kit/servers", manifest)), "KitServers", cx)
}

/// `GET /settings`, for the skills each repository's Jobs start without.
pub fn settings(port: u16, cx: &App) -> Task<Result<SettingsList, Refused>> {
    request(port, Ask::Get("/settings".into()), "SettingsList", cx)
}

/// `POST /settings/save`.
pub fn save(port: u16, changes: BTreeMap<String, Option<SettingValue>>, cx: &App) -> Task<Result<SettingsList, Refused>> {
    request(port, Ask::Post("/settings/save".into(), body(&ipc::SaveSettings { changes })), "SettingsList", cx)
}

/// `POST /kit/pieces/remove`.
pub fn remove(port: u16, piece: ipc::RemoveKitPiece, cx: &App) -> Task<Result<KitPieceRemoved, Refused>> {
    request(port, Ask::Post("/kit/pieces/remove".into(), body(&piece)), "KitPieceRemoved", cx)
}

/// `POST /kit/pieces/restore`.
pub fn restore(port: u16, trashed: String, cx: &App) -> Task<Result<KitInventory, Refused>> {
    request(port, Ask::Post("/kit/pieces/restore".into(), body(&ipc::RestoreKitPiece { trashed })), "KitInventory", cx)
}

/// `POST /kit/servers/manifest_reach`: this Manifest's word over one server. Fleet answers with the servers again.
pub fn reach(port: u16, manifest: &str, name: &str, on: bool, cx: &App) -> Task<Result<KitServers, Refused>> {
    let reach = ipc::ManifestReach::from_wire(if on { "extended" } else { "restricted" });
    let said = body(&ipc::SetManifestServerReach { name: name.to_string(), reach });
    request(port, Ask::Post(scoped("/kit/servers/manifest_reach", manifest), said), "KitServers", cx)
}

/// `POST /kit/servers/forget`. Fleet answers with the servers again.
pub fn forget(port: u16, manifest: &str, name: &str, cx: &App) -> Task<Result<KitServers, Refused>> {
    let said = body(&ipc::ForgetKitServer { name: name.to_string() });
    request(port, Ask::Post(scoped("/kit/servers/forget", manifest), said), "KitServers", cx)
}

/// `POST /kit/allowed_commands/remove`.
pub fn remove_allowed(port: u16, run: &str, cx: &App) -> Task<Result<ipc::KitAllowedCommands, Refused>> {
    request(port, Ask::Post("/kit/allowed_commands/remove".into(), body(&ipc::RemoveKitAllowedCommand { run: run.to_string() })), "KitAllowedCommands", cx)
}

/// `POST /kit/pieces/look`: what a pasted source is, running nothing.
pub fn look(port: u16, source: &str, cx: &App) -> Task<Result<KitSourceFound, Refused>> {
    request(port, Ask::Post("/kit/pieces/look".into(), body(&ipc::LookAtKitSource { source: source.to_string() })), "KitSourceFound", cx)
}

/// `POST /kit/pieces/install`.
pub fn install(port: u16, install: ipc::InstallKitPiece, cx: &App) -> Task<Result<KitPieceInstalled, Refused>> {
    request(port, Ask::Post("/kit/pieces/install".into(), body(&install)), "KitPieceInstalled", cx)
}

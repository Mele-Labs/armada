//! Kit's routes, each a request off the main thread on the port the Fleet connection read.
//! `crates/ipc/operations/get_kit_inventory.toml` and the Kit operations beside it.

use std::collections::BTreeMap;

use gpui::{App, AppContext as _, Task};
use ipc::{KitInventory, KitPieceInstalled, KitPieceRemoved, KitServers, KitSourceFound, SettingValue, SettingsList};

use crate::refusal::Refusal;
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

/// `GET /kit/link`: each kind's switch, and each row against every linked Mac.
pub fn link(port: u16, cx: &App) -> Task<Result<ipc::KitLink, Refused>> {
    request(port, Ask::Get("/kit/link".into()), "KitLink", cx)
}

/// `POST /kit/link/kinds/set`: one switch sends this Mac's items of the kind and takes the linked Macs'.
pub fn cross_kind(port: u16, ask: ipc::SetKitLinkKind, cx: &App) -> Task<Result<ipc::KitLink, Refused>> {
    request(port, Ask::Post("/kit/link/kinds/set".into(), body(&ask)), "KitLink", cx)
}

/// `POST /kit/link/items/set`: one item crosses by itself while its kind is off.
pub fn cross_item(port: u16, ask: ipc::SetKitLinkItem, cx: &App) -> Task<Result<ipc::KitLink, Refused>> {
    request(port, Ask::Post("/kit/link/items/set".into(), body(&ask)), "KitLink", cx)
}

/// `POST /kit/link/settle`: one side of a clash kept, on both Macs.
pub fn settle(port: u16, ask: ipc::SettleKitClash, cx: &App) -> Task<Result<ipc::KitLink, Refused>> {
    request(port, Ask::Post("/kit/link/settle".into(), body(&ask)), "KitLink", cx)
}

/// Why a save did not take: Fleet's own words, its code, and what is on disk now where it says (`on_disk`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub said: String,
    pub code: Option<String>,
    pub on_disk: Option<String>,
}

impl Saved {
    pub fn said(said: impl Into<String>) -> Saved {
        Saved { said: said.into(), code: None, on_disk: None }
    }
}

fn query(file: ipc::KitFileKind, name: &str, plugin: Option<&str>) -> String {
    let kind = serde_json::to_value(file).ok().and_then(|word| word.as_str().map(str::to_string)).unwrap_or_default();
    let plugin = plugin.map(|plugin| format!("&plugin={}", crate::link::percent_encoded(plugin))).unwrap_or_default();
    format!("/kit/file?kind={kind}&name={}{plugin}", crate::link::percent_encoded(name))
}

/// `GET /kit/file`: one piece's file, whole.
pub fn file(port: u16, kind: ipc::KitFileKind, name: &str, plugin: Option<&str>, cx: &App) -> Task<Result<ipc::KitFile, Refused>> {
    request(port, Ask::Get(query(kind, name, plugin)), "KitFile", cx)
}

/// `POST /kit/file/save`. A save over a file that moved is a 409 carrying `on_disk`, so this reads the error itself.
pub fn save_file(port: u16, ask: ipc::SaveKitFile, cx: &App) -> Task<Result<ipc::KitFile, Saved>> {
    let said = body(&ask);
    let answer = cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| (Saved::said(why.to_string()), None))?;
        let sent = said.len();
        let (status, bytes) = runtime.block_on(crate::transport::post(port, "/kit/file/save", said)).map_err(|why| (Saved::said(why), None))?;
        if !(200..300).contains(&status) {
            let refusal = Refusal::read("POST", "/kit/file/save", status, &bytes).sent(sent);
            // `on_disk` rides in the error's fields, which a Refusal does not keep.
            let on_disk = ipc::decode::<ipc::WireError>("WireError", &bytes).ok().and_then(|error| match error.fields.get("on_disk") {
                Some(ipc::WireValue::Str(text)) => Some(text.clone()),
                _ => None,
            });
            return Err((Saved { said: refusal.to_string(), code: refusal.code.clone(), on_disk }, Some(refusal)));
        }
        ipc::decode::<ipc::KitFile>("KitFile", &bytes).map_err(|why| (Saved::said(why.to_string()), None))
    });
    cx.spawn(async move |cx| match answer.await {
        Ok(file) => Ok(file),
        Err((saved, refusal)) => {
            if let Some(refusal) = refusal {
                cx.update(|cx| crate::toasts::tell_refused("The file was not saved", refusal, cx));
            }
            Err(saved)
        }
    })
}

/// `POST /kit/pieces/open`: the piece in the person's editor, or in Finder.
pub fn open(port: u16, ask: ipc::OpenKitPiece, cx: &App) -> Task<Result<ipc::KitPieceOpened, Refused>> {
    request(port, Ask::Post("/kit/pieces/open".into(), body(&ask)), "KitPieceOpened", cx)
}

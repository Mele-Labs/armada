//! A project's system map is read from its own files: a Cargo workspace shaped like this one gives its entry points, the
//! service behind them, a store and what it reaches outside, and a web app with an API gives its frontend, extension, server
//! and state files. Each level names what the code names and nothing it only could.

use std::path::Path;

use ipc::{PartOrigin, SystemMap, SystemPartKind};
use store::KeptFeature;

use super::tmp::TempDir;
use crate::system_map::read_for_test;

fn write(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
    std::fs::write(file, text).expect("written");
}

fn cargo(name: &str, deps: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n\n[dependencies]\n{deps}\n")
}

/// A workspace shaped like this repository: an app and an HTTP API as entry points, a daemon that serves the API, a store,
/// a library everything uses and a crate that reaches something outside.
fn a_workspace(root: &Path) {
    write(root, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\", \"xtask\"]\n");
    write(root, "xtask/Cargo.toml", &cargo("xtask", ""));
    write(root, "xtask/src/main.rs", "fn main() {}\n");
    write(root, "crates/bridge-native/Cargo.toml", &cargo("bridge-native", "gpui = \"1\"\nipc = { path = \"../ipc\" }"));
    write(root, "crates/bridge-native/src/main.rs", "fn main() {}\n");
    write(root, "crates/bridge-native/src/project_board.rs", "//! Draws the board and asks Fleet for its layout.\nfn board(port: u16) { let _ = format!(\"http://127.0.0.1:{port}/projects/project/{}/board\", 1); }\n");
    write(root, "crates/api/Cargo.toml", &cargo("api", "axum = \"0.7\"\nipc = { path = \"../ipc\" }"));
    write(root, "crates/api/src/lib.rs", "pub mod routes;\n");
    write(root, "crates/api/src/routes.rs", "//! The routes Fleet serves.\nfn router() { r.route(\"/projects/project/:project/board\", get(board)); }\n");
    write(root, "crates/fleet/Cargo.toml", &cargo("fleet", "api = { path = \"../api\" }\nstore = { path = \"../store\" }\nadapters = { path = \"../adapters\" }\nipc = { path = \"../ipc\" }"));
    write(root, "crates/fleet/src/lib.rs", "mod project_board;\n");
    write(root, "crates/fleet/src/project_board.rs", "//! Reads and saves a project's board.\nuse store::Store;\nuse adapters::forge_client;\nfn read(s: &Store) {}\n");
    write(root, "crates/fleet/src/daemon.rs", "//! Jobs and the event stream.\nuse crate::project_board;\n");
    write(root, "crates/store/Cargo.toml", &cargo("store", "rusqlite = \"0.32\""));
    write(root, "crates/store/src/lib.rs", "const FILE: &str = \"armada.sqlite\";\nuse rusqlite::Connection;\n");
    write(root, "crates/ipc/Cargo.toml", &cargo("ipc", ""));
    write(root, "crates/ipc/src/lib.rs", "//! The wire's types.\npub struct Board;\n");
    write(root, "crates/adapters/Cargo.toml", &cargo("adapters", "octocrab = \"0.40\""));
    write(root, "crates/adapters/src/lib.rs", "pub mod forge_client;\n");
    write(root, "crates/adapters/src/forge_client.rs", "//! Reads pull requests.\nuse octocrab::Octocrab;\n");
    write(root, "crates/armada/Cargo.toml", &cargo("armada", "fleet = { path = \"../fleet\" }"));
    write(root, "crates/armada/src/main.rs", "fn main() {}\n");
}

fn part<'a>(map: &'a SystemMap, id: &str) -> &'a ipc::SystemPart {
    map.parts.iter().find(|one| one.id == id).unwrap_or_else(|| panic!("no part {id} among {:?}", map.parts.iter().map(|one| &one.id).collect::<Vec<_>>()))
}

fn linked(map: &SystemMap, from: &str, to: &str) -> Option<Option<String>> {
    map.links.iter().find(|link| link.from == from && link.to == to).map(|link| link.label.clone())
}

#[test]
fn a_workspace_shaped_like_this_one_maps_its_entry_points_service_store_and_outside() {
    let home = TempDir::new();
    a_workspace(home.path());
    let map = read_for_test(home.path(), "armada", None, None);

    let kinds: Vec<(&str, SystemPartKind)> = map.parts.iter().map(|one| (one.id.as_str(), one.kind)).collect();
    assert_eq!(part(&map, "people").kind, SystemPartKind::Person, "people are always there: {kinds:?}");
    assert_eq!(part(&map, "bridge-native").kind, SystemPartKind::Entry, "an app crate is an entry point");
    assert_eq!(part(&map, "api").kind, SystemPartKind::Entry, "an HTTP server crate nobody runs is the API people call");
    assert_eq!(part(&map, "api").name, "Fleet API", "it is named for the daemon that serves it");
    assert_eq!(part(&map, "armada").kind, SystemPartKind::Entry, "the binary named for the project is its command");
    assert_eq!(part(&map, "fleet").kind, SystemPartKind::Service, "what implements the API is the service behind it");
    assert_eq!(part(&map, "store-armada-sqlite").kind, SystemPartKind::Store, "a crate on rusqlite is a store, named for the file it opens");
    assert!(map.parts.iter().all(|one| !matches!(one.id.as_str(), "ipc" | "adapters" | "xtask" | "store")), "libraries and tooling are not parts: {kinds:?}");
    let outside = map.parts.iter().find(|one| one.kind == SystemPartKind::Outside).expect("a client dependency is something outside");
    assert!(matches!(&outside.origin, PartOrigin::Detected { .. }));

    for entry in ["bridge-native", "api", "armada"] {
        assert!(linked(&map, "people", entry).is_some(), "people use {entry}");
    }
    assert_eq!(linked(&map, "api", "fleet"), Some(Some("served by".to_string())));
    assert_eq!(
        linked(&map, "bridge-native", "fleet"),
        Some(Some("/projects/project/:project/board".to_string())),
        "a client that names a route the API declares reaches the service behind it, labelled with the route"
    );
    assert!(linked(&map, "fleet", "store-armada-sqlite").is_some(), "the service reaches the store through the crate it uses");
    assert!(linked(&map, "fleet", &outside.id).is_some(), "and what is outside through the library it uses");
    assert!(linked(&map, "bridge-native", &outside.id).is_none(), "an entry that never reaches a library does not reach what it reaches");
}

#[test]
fn zooming_into_a_part_names_its_pieces_and_what_they_call() {
    let home = TempDir::new();
    a_workspace(home.path());
    let map = read_for_test(home.path(), "armada", Some("fleet"), None);

    assert_eq!(map.part.as_deref(), Some("fleet"));
    let pieces: Vec<&str> = map.parts.iter().filter(|one| one.within.as_deref() == Some("fleet")).map(|one| one.name.as_str()).collect();
    assert!(pieces.contains(&"project_board") && pieces.contains(&"daemon"), "the folder's own files are its pieces: {pieces:?}");
    assert!(pieces.contains(&"adapters") && pieces.contains(&"ipc"), "the libraries it reaches are pieces of it: {pieces:?}");
    assert!(map.parts.iter().all(|one| one.kind == SystemPartKind::Piece || one.kind == SystemPartKind::Store || one.kind == SystemPartKind::Outside));
    assert!(map.links.iter().any(|link| link.from == "fleet/project-board" && link.to.starts_with("store-")), "a piece that uses the store reaches it: {:?}", map.links);
    assert!(map.links.iter().any(|link| link.from == "fleet/project-board" && link.to == "fleet/adapters"), "a piece that uses a library reaches it");
    assert!(map.links.iter().any(|link| link.from == "fleet/daemon" && link.to == "fleet/project-board"), "an import of a sibling is a link");

    let bridge = read_for_test(home.path(), "armada", Some("bridge-native"), None);
    let route = bridge.links.iter().find(|link| link.from == "bridge-native/project-board").expect("the board's piece reaches the API");
    assert_eq!(route.label.as_deref(), Some("/projects/project/:project/board"));
    assert_eq!(bridge.parts.iter().find(|one| one.id == route.to).and_then(|one| one.within.as_deref()), Some("api"), "a piece of another part is drawn within it");
}

#[test]
fn a_feature_is_its_files_by_part_in_level_ones_order() {
    let home = TempDir::new();
    a_workspace(home.path());
    let feature = KeptFeature {
        id: "f1".to_string(),
        project: "p".to_string(),
        area: "board".to_string(),
        slug: "dashboard".to_string(),
        title: "Dashboard".to_string(),
        summary: String::new(),
        state: "confirmed".to_string(),
        manifest_id: String::new(),
        entry_points: Vec::new(),
        paths: vec!["crates/store".to_string(), "crates/fleet/src/project_board.rs".to_string(), "crates/api/src/routes.rs".to_string(), "crates/bridge-native/src/project_board.rs".to_string()],
        position: 0,
        added_at: String::new(),
    };
    let map = read_for_test(home.path(), "armada", None, Some(&feature));
    assert_eq!(map.feature.as_deref(), Some("dashboard"));
    let order: Vec<(u32, &str)> = map.stops.iter().map(|stop| (stop.step, stop.part.as_str())).collect();
    assert_eq!(order, vec![(1, "api"), (2, "bridge-native"), (3, "fleet"), (4, "store-armada-sqlite")], "entries, then services, then stores, each by id");
    assert_eq!(map.stops[2].files, vec!["crates/fleet/src/project_board.rs".to_string()]);
    let steps: Vec<(u32, &str, &str)> = map.links.iter().map(|link| (link.step.expect("stepped"), link.from.as_str(), link.to.as_str())).collect();
    assert_eq!(steps, vec![(1, "api", "bridge-native"), (2, "bridge-native", "fleet"), (3, "fleet", "store-armada-sqlite")]);
}

/// A web app with an API shaped like a small importer: a React frontend, a browser extension, a FastAPI backend, a command
/// and session files.
fn a_web_app(root: &Path) {
    write(root, "requirements.txt", "fastapi\nuvicorn[standard]\nrequests\n");
    write(root, "frontend/package.json", "{ \"name\": \"importer-frontend\", \"dependencies\": { \"react\": \"^18\" }, \"devDependencies\": { \"vite\": \"^6\" } }\n");
    write(root, "frontend/index.html", "<div id=root></div>\n");
    write(root, "frontend/src/api.js", "const get = (p) => fetch(p);\nexport const session = () => get(\"/api/session\");\nexport const review = () => get(\"/api/review\");\n");
    write(root, "frontend/src/screens/Review.jsx", "import { review } from \"../api\";\n");
    write(root, "frontend/src/screens/Submit.jsx", "import { submit } from \"../api\";\nfetch(`/api/submit`, { method: \"POST\" });\n");
    write(root, "frontend/src/screens/Upload.jsx", "export default function Upload() {}\n");
    write(root, "frontend/src/screens/Token.jsx", "export default function Token() {}\n");
    write(root, "extension/manifest.json", "{ \"manifest_version\": 3, \"name\": \"Importer - Order Grabber\", \"host_permissions\": [\"http://localhost:8731/*\"], \"content_scripts\": [{ \"matches\": [\"*://*.shop-one.com/*\", \"*://*.shop-two.com/*\"], \"js\": [\"content.js\"] }] }\n");
    write(root, "extension/background.js", "const INGEST_URL = \"http://localhost:8731/api/orders/ingest\";\nfetch(INGEST_URL);\n");
    write(root, "extension/content.js", "// reads the page\n");
    write(root, "backend/app/main.py", "from fastapi import FastAPI\nfrom . import ynab_client, persistence\napp = FastAPI()\n\n@app.get(\"/api/session\")\ndef session(): pass\n\n@app.get(\"/api/review\")\ndef review(): pass\n\n@app.post(\"/api/submit\")\ndef submit(): pass\n\n@app.post(\"/api/orders/ingest\")\ndef ingest(): pass\n");
    write(root, "backend/app/ynab_client.py", "\"\"\"The only part that calls the budget service.\"\"\"\nAPI = \"https://api.ynab.com/v1\"\n");
    write(root, "backend/app/persistence.py", "\"\"\"Session state, saved as JSON.\"\"\"\nimport json, os\nDIR = os.path.expanduser(\"~/.importer/sessions\")\ndef save(s):\n    json.dump(s, open(DIR + \"/x.json\", \"w\"))\n");
    write(root, "import_cli.py", "\"\"\"Batch import without the app.\"\"\"\nimport argparse\n\nif __name__ == \"__main__\":\n    argparse.ArgumentParser().parse_args()\n");
}

#[test]
fn a_web_app_with_an_api_maps_its_frontend_extension_server_command_state_and_outside() {
    let home = TempDir::new();
    a_web_app(home.path());
    let map = read_for_test(home.path(), "importer", None, None);
    let ids: Vec<&str> = map.parts.iter().map(|one| one.id.as_str()).collect();

    assert_eq!(part(&map, "frontend").kind, SystemPartKind::Entry, "a React app is an entry point: {ids:?}");
    assert!(part(&map, "frontend").zooms);
    assert_eq!(part(&map, "extension").name, "Order Grabber", "a browser extension is named by its manifest: {ids:?}");
    assert_eq!(part(&map, "extension").kind, SystemPartKind::Entry);
    assert_eq!(part(&map, "fastapi-backend").kind, SystemPartKind::Service, "a file that starts a FastAPI app is the server: {ids:?}");
    assert_eq!(part(&map, "fastapi-backend").name, "FastAPI backend");
    assert_eq!(part(&map, "import-cli-py").kind, SystemPartKind::Entry, "a script with arguments is a command: {ids:?}");
    assert_eq!(part(&map, "store-session-files").name, "Session files", "a folder of state files a file names is a store: {ids:?}");
    assert_eq!(part(&map, "outside-api-ynab-com").name, "YNAB API", "an `api.` address the code names is outside: {ids:?}");
    assert!(map.parts.iter().any(|one| one.kind == SystemPartKind::Outside && one.summary.contains("shop-one.com")), "the pages an extension reads are outside: {ids:?}");

    assert_eq!(linked(&map, "extension", "fastapi-backend"), Some(Some("/api/orders/ingest".to_string())), "the one route it calls");
    assert_eq!(linked(&map, "frontend", "fastapi-backend"), Some(None), "several routes are no one label");
    assert!(linked(&map, "fastapi-backend", "store-session-files").is_some());
    assert!(linked(&map, "fastapi-backend", "outside-api-ynab-com").is_some());
    assert!(linked(&map, "people", "frontend").is_some() && linked(&map, "people", "import-cli-py").is_some());
}

#[test]
fn a_servers_pieces_are_its_modules_and_the_frontends_are_its_screens() {
    let home = TempDir::new();
    a_web_app(home.path());
    let back = read_for_test(home.path(), "importer", Some("fastapi-backend"), None);
    let names: Vec<&str> = back.parts.iter().filter(|one| one.within.as_deref() == Some("fastapi-backend")).map(|one| one.name.as_str()).collect();
    assert!(names.contains(&"main") && names.contains(&"persistence") && names.contains(&"ynab_client"), "{names:?}");
    assert!(back.links.iter().any(|link| link.from == "fastapi-backend/main" && link.to == "fastapi-backend/ynab-client"), "an import is a link");
    assert!(back.links.iter().any(|link| link.from == "fastapi-backend/ynab-client" && link.to == "outside-api-ynab-com"), "the module that names the address reaches it");

    let front = read_for_test(home.path(), "importer", Some("frontend"), None);
    let screens: Vec<&str> = front.parts.iter().filter(|one| one.within.as_deref() == Some("frontend")).map(|one| one.name.as_str()).collect();
    assert!(screens.contains(&"Review") && screens.contains(&"Submit"), "a folder of screens stands in its place: {screens:?}");
    let route = front.links.iter().find(|link| link.from == "frontend/submit" && link.to == "fastapi-backend/main").expect("the screen that names a route reaches the server");
    assert_eq!(route.label.as_deref(), Some("/api/submit"));
    assert!(front.links.iter().any(|link| link.from == "frontend/review" && link.to == "frontend/api"), "a screen that imports the client reaches it");
}

fn package(name: &str, description: &str, deps: &str, dev: &str) -> String {
    format!("{{ \"name\": \"{name}\", \"description\": \"{description}\", \"private\": true, \"dependencies\": {{ {deps} }}, \"devDependencies\": {{ {dev} }} }}\n")
}

/// A monorepo shaped like this repository: a Rust workspace beside an Electron app, a phone web app, a Worker and packages
/// that the apps share, with scratch tools beside them.
fn a_monorepo(root: &Path) {
    write(root, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/bridge-native\"]\n");
    write(root, "pnpm-workspace.yaml", "packages:\n  - apps/*\n  - packages/*\n  - packages/surfaces/*\n");
    write(root, "crates/armada/Cargo.toml", &cargo("armada", "fleet = { path = \"../fleet\" }\napi = { path = \"../api\" }\npocket = { path = \"../pocket\" }\naxum = \"0.7\""));
    write(root, "crates/armada/src/main.rs", "fn main() { axum::serve(); }\n");
    write(root, "crates/api/Cargo.toml", &cargo("api", "axum = \"0.7\""));
    write(root, "crates/api/src/lib.rs", "pub mod routes;\n");
    write(root, "crates/api/src/routes.rs", "//! The routes Fleet serves.\nfn router() { r.route(\"/events\", get(events)); r.route(\"/jobs/:id\", get(job)); }\n");
    write(root, "crates/pocket/Cargo.toml", &cargo("pocket", "axum = \"0.7\""));
    write(root, "crates/pocket/src/lib.rs", "//! The gateway the phone is let in by.\nfn router() { r.route(\"/gateway/pair\", post(pair)); }\n");
    write(root, "crates/fleet/Cargo.toml", &cargo("fleet", "api = { path = \"../api\" }\nstore = { path = \"../store\" }\nipc = { path = \"../ipc\" }"));
    write(root, "crates/fleet/src/lib.rs", "//! Jobs and the event stream.\nuse store::Store;\n");
    write(root, "crates/store/Cargo.toml", &cargo("store", "rusqlite = \"0.32\""));
    write(root, "crates/store/src/lib.rs", "use rusqlite::Connection;\nfn open(dir: &Path) { Connection::open(dir.join(\"armada.db\")); }\n");
    write(root, "crates/ipc/Cargo.toml", &cargo("ipc", ""));
    write(root, "crates/ipc/src/lib.rs", "//! The wire's types.\n");
    write(root, "crates/bridge-native/Cargo.toml", &cargo("bridge-native", "gpui = \"1\"\nipc = { path = \"../ipc\" }").replace("version = \"0.0.0\"\n", "version = \"0.0.0\"\ndescription = \"Bridge: the native desktop application.\"\n"));
    write(root, "crates/bridge-native/src/main.rs", "fn main() {}\n");
    write(root, "crates/bridge-native/src/jobs.rs", "fn get() { let _ = \"http://127.0.0.1:40000/events\"; }\n");
    write(root, "apps/desktop/package.json", &package("@armada/desktop", "Bridge Classic: the Electron desktop application.", "", "\"@armada/components\": \"workspace:*\", \"@armada/cleanup\": \"workspace:*\", \"electron\": \"^33\", \"react\": \"^18\", \"vite\": \"^6\""));
    write(root, "apps/desktop/src/main/fleet.ts", "export const events = () => fetch(\"/events\");\n");
    write(root, "apps/pocket/package.json", &package("@armada/pocket", "Pocket: the phone app.", "\"react\": \"^18\"", "\"vite\": \"^6\""));
    write(root, "apps/pocket/index.html", "<div id=root></div>\n");
    write(root, "apps/pocket/src/app.tsx", "export default function App() {}\n");
    write(root, "apps/store/package.json", &package("@armada/store", "The central store: claims.", "", "\"wrangler\": \"^4\""));
    write(root, "apps/store/wrangler.jsonc", "{}\n");
    write(root, "apps/store/src/index.ts", "export default {};\n");
    write(root, "packages/components/package.json", &package("@armada/components", "Bridge's components.", "\"@armada/brand\": \"workspace:*\", \"react\": \"^18\"", "\"vite\": \"^6\""));
    write(root, "packages/components/src/index.ts", "export {};\n");
    write(root, "packages/brand/package.json", &package("@armada/brand", "The mark.", "\"react\": \"^18\"", ""));
    write(root, "packages/brand/src/index.ts", "export {};\n");
    write(root, "packages/surfaces/cleanup/package.json", &package("@armada/cleanup", "The Cleanup surface.", "\"@armada/components\": \"workspace:*\", \"react\": \"^18\"", "\"@armada/desktop\": \"workspace:*\""));
    write(root, "packages/surfaces/cleanup/src/index.ts", "export {};\n");
    write(root, "tools/lm-train/main.py", "from fastapi import FastAPI\napp = FastAPI()\n");
    write(root, "tools/lm-train/requirements.txt", "fastapi\n");
}

#[test]
fn a_monorepo_shaped_like_this_one_has_its_apps_for_entries_and_its_libraries_folded_in() {
    let home = TempDir::new();
    a_monorepo(home.path());
    let map = read_for_test(home.path(), "armada", None, None);
    let found: Vec<(&str, SystemPartKind)> = map.parts.iter().map(|one| (one.id.as_str(), one.kind)).collect();
    let of = |kind: SystemPartKind| -> Vec<&str> { map.parts.iter().filter(|one| one.kind == kind).map(|one| one.id.as_str()).collect() };

    assert_eq!(of(SystemPartKind::Entry), ["desktop", "pocket", "api", "armada", "bridge-native"], "an app, the HTTP API and the project's command: {found:?}");
    assert_eq!(of(SystemPartKind::Service), ["store", "fleet"], "a Worker and the daemon behind the API: {found:?}");
    assert_eq!(of(SystemPartKind::Store), ["store-armada-db"], "named for the file the code opens: {found:?}");
    assert_eq!(part(&map, "pocket").name, "Pocket", "a package is named by the words its description leads with");
    assert_eq!(part(&map, "store").name, "Central store");
    assert_eq!(part(&map, "desktop").name, "Bridge Classic", "a description that leads with a name names the package");
    assert_eq!(part(&map, "bridge-native").name, "Bridge", "a Cargo package is named by its description too");
    assert!(linked(&map, "fleet", "api").is_none(), "a service is not a client of the API it serves");
    assert_eq!(part(&map, "api").name, "Fleet API");
    assert!(map.parts.iter().all(|one| !matches!(one.id.as_str(), "components" | "brand" | "cleanup" | "lm-train" | "pocket-2" | "ipc")), "libraries, scratch tools and a server no client calls are not parts: {found:?}");
    for entry in ["desktop", "pocket", "armada", "bridge-native"] {
        assert!(linked(&map, "people", entry).is_some(), "people use {entry}");
    }
    assert!(map.links.iter().filter(|link| link.from == "people").all(|link| of(SystemPartKind::Entry).contains(&link.to.as_str())), "people reach entries and nothing else");

    let desktop = read_for_test(home.path(), "armada", Some("desktop"), None);
    let names: Vec<&str> = desktop.parts.iter().filter(|one| one.within.as_deref() == Some("desktop")).map(|one| one.name.as_str()).collect();
    assert!(names.contains(&"@armada/components") && names.contains(&"@armada/cleanup"), "the packages an app builds on are its pieces: {names:?}");
}

#[test]
fn a_web_app_names_what_it_is_for_and_leaves_the_rest_out() {
    let home = TempDir::new();
    a_web_app(home.path());
    let map = read_for_test(home.path(), "importer", None, None);
    let named: Vec<(&str, &str)> = map.parts.iter().filter(|one| one.kind != SystemPartKind::Person).map(|one| (one.name.as_str(), kind_word(one.kind))).collect();
    assert_eq!(
        named,
        [
            ("Frontend", "entry"),
            ("Order Grabber", "entry"),
            ("FastAPI backend", "service"),
            ("import_cli.py", "entry"),
            ("Session files", "store"),
            ("YNAB API", "outside"),
            ("Pages it reads", "outside"),
        ],
        "the screens, the extension, the command, the server, its state and what it reaches"
    );
}

fn kind_word(kind: SystemPartKind) -> &'static str {
    match kind {
        SystemPartKind::Entry => "entry",
        SystemPartKind::Service => "service",
        SystemPartKind::Store => "store",
        SystemPartKind::Outside => "outside",
        _ => "other",
    }
}

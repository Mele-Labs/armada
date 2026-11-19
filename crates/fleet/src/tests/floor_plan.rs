//! The floor plan, written before the code: **a feature's page carries the project's modules as rooms, lit where the
//! feature owns a file, and the dependencies out of a lit room as doors, drawn as contracts where they lead to a wire or
//! API module.** Modules come from build manifests, from top folders where a tree has none, and cost no model. **Each room
//! holds its tracked files with git's sizes**, the feature's and a live Job's alone and the rest by folder past a cap. Each
//! test lays a repository out in a temp directory and reads it as Fleet does. `docs/concepts/feature-catalog.md`, *Floor plan*.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use api::{Features, Projects};
use ipc::{AreaSeed, FeatureSeed, FloorFile, FloorPlan, NewProject, ProjectMember, UpsertFeatures};

use super::agent_starts::rig;
use super::tmp::TempDir;
use crate::feature_library::mapping::FeatureMap;
use crate::clashing::Who;
use crate::feature_library::{floor_plan_of_layouts, is_contract, read_floor, read_modules};

fn put(root: &Path, path: &str, text: &str) {
    let at = root.join(path);
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("made");
    std::fs::write(at, text).expect("written");
}

fn feature(id: &str, paths: &[&str]) -> store::KeptFeature {
    store::KeptFeature {
        id: id.to_string(),
        project: "p1".to_string(),
        area: "a".to_string(),
        slug: id.to_string(),
        title: id.to_string(),
        summary: String::new(),
        state: "seeded".to_string(),
        manifest_id: String::new(),
        entry_points: Vec::new(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        position: 0,
        added_at: "2026-11-16T09:00:00.000Z".to_string(),
    }
}

/// The plan of `own` among `others`, over the repository at `root`.
fn planned(root: &Path, own: &store::KeptFeature, others: Vec<store::KeptFeature>, filed: &[(&str, &str)]) -> FloorPlan {
    let mut all = vec![own.clone()];
    all.extend(others);
    let map = FeatureMap::new(all, filed.iter().map(|(feature, path)| (feature.to_string(), path.to_string())));
    floor_plan_of_layouts(&[("m1".to_string(), read_modules(root))], &map, own, &BTreeMap::new(), 1500)
}

fn lit(plan: &FloorPlan) -> Vec<&str> {
    plan.rooms.iter().filter(|room| room.lit).map(|room| room.module.as_str()).collect()
}

fn doors(plan: &FloorPlan) -> Vec<(&str, &str, bool)> {
    plan.doors.iter().map(|door| (door.from.as_str(), door.to.as_str(), door.contract)).collect()
}

fn cargo_workspace(root: &Path) {
    put(root, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    put(root, "crates/core/Cargo.toml", "[package]\nname = \"core-model\"\n");
    put(root, "crates/ipc/Cargo.toml", "[package]\nname = \"ipc\"\n[dependencies]\ncore-model = { path = \"../core\" }\n");
    put(
        root,
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\n[dependencies]\nipc = { path = \"../ipc\" }\ncore-model.workspace = true\nserde = \"1\"\n[dev-dependencies]\nhelper = \"1\"\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n",
    );
    put(root, "crates/app/src/lib.rs", "");
    put(root, "target/debug/Cargo.toml", "[package]\nname = \"built\"\n");
}

#[test]
fn a_cargo_workspace_is_its_packages_and_the_virtual_root_is_no_room() {
    let dir = TempDir::new();
    cargo_workspace(dir.path());
    let one = feature("start", &["crates/app/src"]);
    let plan = planned(dir.path(), &one, vec![], &[]);
    let rooms: Vec<(&str, &str, &str)> = plan.rooms.iter().map(|room| (room.module.as_str(), room.path.as_str(), room.kind.as_str())).collect();
    assert_eq!(rooms, [("app", "crates/app", "cargo"), ("core-model", "crates/core", "cargo"), ("ipc", "crates/ipc", "cargo")], "no `.` for the virtual root, none from target/");
    assert_eq!(lit(&plan), ["app"], "a prefix inside a module lights that module alone");
    assert_eq!(doors(&plan), [("app", "core-model", false), ("app", "ipc", true)], "out of the lit room only, `ipc` a contract, and neither dev nor foreign dependencies");
}

#[test]
fn a_prefix_over_modules_lights_each_unless_another_feature_owns_one_more_closely() {
    let dir = TempDir::new();
    cargo_workspace(dir.path());
    let wide = feature("wide", &["crates"]);
    let narrow = feature("narrow", &["crates/ipc"]);
    let plan = planned(dir.path(), &wide, vec![narrow], &[]);
    assert_eq!(lit(&plan), ["app", "core-model"], "the longest prefix wins, so `ipc` is the narrow feature's");
}

#[test]
fn a_file_a_job_filed_lights_its_room_with_no_prefix() {
    let dir = TempDir::new();
    cargo_workspace(dir.path());
    let one = feature("start", &[]);
    let plan = planned(dir.path(), &one, vec![], &[("start", "crates/core/src/a.rs")]);
    assert_eq!(lit(&plan), ["core-model"]);
    assert!(plan.doors.is_empty(), "core-model depends on nothing here");
}

#[test]
fn a_pnpm_workspace_is_its_packages_and_a_package_named_for_an_api_is_a_contract() {
    let dir = TempDir::new();
    let root = dir.path();
    put(root, "pnpm-workspace.yaml", "packages:\n  - 'packages/*'\n");
    put(root, "package.json", "{ \"name\": \"shop\", \"private\": true }");
    put(
        root,
        "packages/ui/package.json",
        "{\n  \"name\": \"@shop/ui\",\n  \"dependencies\": { \"@shop/api-client\": \"workspace:*\", \"react\": \"^19\" },\n  \"devDependencies\": { \"@shop/tools\": \"workspace:*\" }\n}",
    );
    put(root, "packages/api-client/package.json", "{\"name\": \"@shop/api-client\", \"dependencies\": {\"@shop/core\": \"1\"}}");
    put(root, "packages/core/package.json", "{\"name\": \"@shop/core\"}");
    put(root, "packages/tools/package.json", "{\"name\": \"@shop/tools\"}");
    put(root, "node_modules/left-pad/package.json", "{\"name\": \"left-pad\"}");
    let one = feature("checkout", &["packages/ui"]);
    let plan = planned(root, &one, vec![], &[]);
    let names: Vec<&str> = plan.rooms.iter().map(|room| room.module.as_str()).collect();
    assert_eq!(names, ["@shop/api-client", "@shop/core", "@shop/tools", "@shop/ui"], "the workspace root and node_modules are no rooms");
    assert!(plan.rooms.iter().all(|room| room.kind == "npm"));
    assert_eq!(lit(&plan), ["@shop/ui"]);
    assert_eq!(doors(&plan), [("@shop/ui", "@shop/api-client", true)], "react is not a module here, and a dev dependency is no door");
}

#[test]
fn an_npm_workspaces_field_makes_its_package_a_root_not_a_room() {
    let dir = TempDir::new();
    let root = dir.path();
    put(root, "package.json", "{\"name\": \"mono\", \"workspaces\": [\"libs/*\"]}");
    put(root, "libs/a/package.json", "{\"name\": \"a\"}");
    let plan = planned(root, &feature("f", &[]), vec![], &[]);
    assert_eq!(plan.rooms.iter().map(|room| room.module.as_str()).collect::<Vec<_>>(), ["a"]);
}

#[test]
fn go_modules_are_read_with_their_requires_and_local_replaces() {
    let dir = TempDir::new();
    let root = dir.path();
    put(
        root,
        "svc/go.mod",
        "module github.com/acme/svc\n\ngo 1.22\n\nrequire (\n\tgithub.com/acme/proto v1.0.0\n\tgithub.com/other/thing v2.0.0 // indirect\n)\n\nreplace github.com/acme/lib => ../lib\nrequire github.com/acme/lib v0.0.0\n",
    );
    put(root, "lib/go.mod", "module github.com/acme/lib\n");
    put(root, "proto/go.mod", "module github.com/acme/proto\n");
    let plan = planned(root, &feature("pay", &["svc"]), vec![], &[]);
    assert_eq!(
        plan.rooms.iter().map(|room| (room.module.as_str(), room.kind.as_str())).collect::<Vec<_>>(),
        [("github.com/acme/lib", "go"), ("github.com/acme/proto", "go"), ("github.com/acme/svc", "go")]
    );
    assert_eq!(doors(&plan), [("github.com/acme/svc", "github.com/acme/lib", false), ("github.com/acme/svc", "github.com/acme/proto", true)]);
}

#[test]
fn a_tree_with_no_manifest_is_its_top_folders_and_a_proto_or_openapi_file_makes_one_a_contract() {
    let dir = TempDir::new();
    let root = dir.path();
    put(root, "web/index.html", "");
    put(root, "server/main.py", "");
    put(root, "wire/messages.proto", "syntax = \"proto3\";");
    put(root, "docs/openapi.yaml", "openapi: 3.0.0");
    put(root, "node_modules/x/a.js", "");
    put(root, ".git/config", "");
    put(root, "README.md", "");
    let plan = planned(root, &feature("f", &["server"]), vec![], &[]);
    assert_eq!(
        plan.rooms.iter().map(|room| (room.module.as_str(), room.kind.as_str())).collect::<Vec<_>>(),
        [("docs", "folder"), ("server", "folder"), ("web", "folder"), ("wire", "folder")]
    );
    assert_eq!(lit(&plan), ["server"]);
    let layout = read_modules(root);
    let contracts: Vec<&str> = layout.modules.iter().filter(|module| is_contract(module)).map(|module| module.name.as_str()).collect();
    assert_eq!(contracts, ["docs", "wire"], "a folder holding an OpenAPI or `.proto` file is a contract, by its files and not its name");
}

#[test]
fn the_other_manifests_name_their_modules_and_dependencies() {
    let dir = TempDir::new();
    let root = dir.path();
    put(root, "py/pyproject.toml", "[project]\nname = \"billing_core\"\ndependencies = [\"Shared_Lib>=1.0\", \"requests\"]\n");
    put(root, "shared/pyproject.toml", "[project]\nname = \"shared-lib\"\n");
    put(root, "tools/pyproject.toml", "[tool.ruff]\nline-length = 100\n");
    put(root, "settings.gradle.kts", "rootProject.name = \"x\"\ninclude(\":app\", \":lib:core\")\n");
    put(root, "app/build.gradle.kts", "dependencies { implementation(project(\":lib:core\")) }\n");
    put(root, "lib/core/build.gradle.kts", "");
    put(
        root,
        "java/pom.xml",
        "<project><parent><artifactId>root</artifactId></parent><artifactId>web</artifactId><dependencies><dependency><artifactId>domain</artifactId></dependency><dependency><artifactId>junit</artifactId></dependency></dependencies></project>",
    );
    put(root, "java/domain/pom.xml", "<project><artifactId>domain</artifactId></project>");
    put(root, "dn/Web/Web.csproj", "<Project><ItemGroup><ProjectReference Include=\"..\\Lib\\Lib.csproj\" /></ItemGroup></Project>");
    put(root, "dn/Lib/Lib.csproj", "<Project />");
    put(root, "sw/Package.swift", "let package = Package(\n    name: \"Kit\",\n    dependencies: [.package(path: \"../sw2\"), .package(url: \"https://x/y/Other.git\", from: \"1.0.0\")]\n)\n");
    put(root, "sw2/Package.swift", "let package = Package(name: \"Kit2\")\n");
    let layout = read_modules(root);
    let names: Vec<&String> = layout.modules.iter().map(|module| &module.name).collect();
    let named = |name: &str| layout.modules.iter().position(|module| module.name == name).unwrap_or_else(|| panic!("{name} in {names:?}"));
    let edges: Vec<(&str, &str)> = layout.uses.iter().map(|(from, to)| (layout.modules[*from].name.as_str(), layout.modules[*to].name.as_str())).collect();
    for (from, to) in [("billing_core", "shared-lib"), ("app", "lib:core"), ("web", "domain"), ("Web", "Lib"), ("Kit", "Kit2")] {
        assert!(edges.contains(&(from, to)), "{from} -> {to} in {edges:?}");
    }
    assert_eq!(layout.uses.len(), 5, "and no more: {edges:?}");
    assert_eq!(layout.modules[named("lib:core")].root, "lib/core");
    assert_eq!(layout.modules[named("lib:core")].kind.label(), "gradle");
    assert!(layout.modules.iter().all(|module| module.root != "tools"), "a pyproject that names nothing is configuration");
    assert!(layout.modules.iter().all(|module| !module.root.is_empty()), "the Gradle settings folder only lists");
}

#[test]
fn the_walk_stops_at_its_depth_and_its_module_count() {
    let dir = TempDir::new();
    let root = dir.path();
    let deep = (0..12).map(|n| format!("d{n}")).collect::<Vec<_>>().join("/");
    put(root, &format!("{deep}/package.json"), "{\"name\": \"too-deep\"}");
    put(root, "near/package.json", "{\"name\": \"near\"}");
    for n in 0..450 {
        put(root, &format!("many/m{n:03}/package.json"), &format!("{{\"name\": \"m{n:03}\"}}"));
    }
    let layout = read_modules(root);
    assert!(layout.modules.iter().all(|module| module.name != "too-deep"));
    assert!(layout.modules.len() <= 400, "{}", layout.modules.len());
    assert!(layout.modules.iter().any(|module| module.name == "near"), "shallowest first, so the near one is kept");
}

#[test]
fn two_modules_of_one_name_are_told_apart_by_their_folders() {
    let dir = TempDir::new();
    put(dir.path(), "a/package.json", "{\"name\": \"same\"}");
    put(dir.path(), "b/package.json", "{\"name\": \"same\"}");
    let plan = planned(dir.path(), &feature("f", &[]), vec![], &[]);
    assert_eq!(plan.rooms.iter().map(|room| room.module.as_str()).collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn this_repository_reads_as_its_crates_with_fleet_reaching_ipc_through_a_contract_door() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let started = std::time::Instant::now();
    let layout = read_modules(&root);
    let took = started.elapsed();
    let at = |name: &str| layout.modules.iter().position(|module| module.name == name).unwrap_or_else(|| panic!("{name} among the modules"));
    assert_eq!(layout.modules[at("fleet")].root, "crates/fleet");
    assert!(layout.uses.contains(&(at("fleet"), at("ipc"))));
    assert!(is_contract(&layout.modules[at("ipc")]));
    assert!(!is_contract(&layout.modules[at("fleet")]));
    assert!(layout.modules.iter().all(|module| !module.root.starts_with("target") && !module.root.contains("node_modules")));
    eprintln!("read {} modules in {took:?}", layout.modules.len());
}

#[tokio::test]
async fn a_feature_page_carries_the_floor_plan_and_a_section_read_does_not() {
    let rig = rig(None);
    let root = rig.fleet.repositories().first().expect("served").root().to_string();
    cargo_workspace(Path::new(&root));
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let project = rig.fleet.add_project(NewProject { name: "Shop".to_string(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone();
    let upsert = UpsertFeatures {
        project: project.clone(),
        areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }],
        features: vec![FeatureSeed {
            slug: "start".into(),
            area: "takes".into(),
            title: "Start".into(),
            summary: "One line.".into(),
            state: None,
            paths: vec!["crates/app/src".into()],
            entry_points: Vec::new(),
            manifest_id: None,
        }],
    };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");

    let page = rig.fleet.get_feature_page(None, Some(project.clone()), "start".into(), None).await.expect("page");
    let plan = page.floor_plan.expect("a floor plan");
    assert_eq!(plan.rooms.iter().filter(|room| room.lit).map(|room| room.module.as_str()).collect::<Vec<_>>(), ["app"]);
    assert!(plan.doors.iter().any(|door| door.from == "app" && door.to == "ipc" && door.contract));

    let one = rig.fleet.get_feature_page(None, Some(project), "start".into(), Some("test".into())).await.expect("section");
    assert_eq!(one.floor_plan, None, "a section read is that section alone");
}

fn git(root: &Path, args: &[&str]) {
    let done = Command::new("git").args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"]).args(args).current_dir(root).output().expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
}

/// `paths` under `root` committed as its `HEAD`.
fn committed(root: &Path, paths: &[&str]) {
    git(root, &["init", "-q"]);
    git(root, &[&["add", "--"][..], paths].concat());
    git(root, &["commit", "-q", "-m", "files"]);
}

/// The plan of `own` over the repository at `root` with its files, `changing` the files a live Job wrote, at most `most` entries.
fn planned_with_files(root: &Path, own: &store::KeptFeature, changing: &[&str], most: usize) -> FloorPlan {
    let map = FeatureMap::new(vec![own.clone()], Vec::new());
    let changing: BTreeMap<String, BTreeSet<String>> = [("m1".to_string(), changing.iter().map(|path| path.to_string()).collect())].into();
    floor_plan_of_layouts(&[("m1".to_string(), read_floor(root))], &map, own, &changing, most)
}

fn files_of<'a>(plan: &'a FloorPlan, module: &str) -> &'a [FloorFile] {
    &plan.rooms.iter().find(|room| room.module == module).unwrap_or_else(|| panic!("{module} among the rooms")).files
}

fn one(path: &str, bytes: u64) -> FloorFile {
    FloorFile { path: path.to_string(), bytes, lit: false, changing: false, grouped: 0 }
}

#[test]
fn a_rooms_files_are_its_tracked_files_with_gits_sizes_and_a_lock_file_or_asset_is_absent() {
    let dir = TempDir::new();
    let root = dir.path();
    cargo_workspace(root);
    put(root, "crates/app/src/lib.rs", "pub fn a() {}\n");
    put(root, "crates/app/Cargo.lock", "# a lock file\n");
    put(root, "crates/app/icon.png", "not really a png");
    put(root, "crates/app/src/__snapshots__/view.txt", "a snapshot");
    committed(root, &["crates"]);
    put(root, "crates/app/src/lib.rs", "pub fn a() {}\npub fn b() {}\n");
    put(root, "crates/app/src/new.rs", "not committed");

    let plan = planned_with_files(root, &feature("start", &[]), &[], 1500);

    let manifest = std::fs::metadata(root.join("crates/app/Cargo.toml")).expect("written").len();
    assert_eq!(files_of(&plan, "app"), [one("crates/app/Cargo.toml", manifest), one("crates/app/src/lib.rs", 14)], "HEAD's size, not the edited file's; nothing untracked; no lock file, image or snapshot");
    assert_eq!(files_of(&plan, "ipc").iter().map(|file| file.path.as_str()).collect::<Vec<_>>(), ["crates/ipc/Cargo.toml"]);
}

#[test]
fn the_features_files_and_a_live_jobs_stand_alone_and_past_the_cap_the_rest_are_taken_together_by_folder() {
    let dir = TempDir::new();
    let root = dir.path();
    for n in 0..10 {
        put(root, &format!("app/a/f{n}.rs"), "0123456789");
        put(root, &format!("app/b/g{n}.rs"), "0123456789");
    }
    put(root, "app/b/edit.rs", "12345");
    put(root, "app/owned.rs", "12345");
    put(root, "app/big.rs", &"x".repeat(1000));
    put(root, "lib/x.rs", "0123456789");
    committed(root, &["app", "lib"]);

    let plan = planned_with_files(root, &feature("start", &["app/owned.rs"]), &["app/b/edit.rs"], 10);

    let app = files_of(&plan, "app");
    let entries = plan.rooms.iter().map(|room| room.files.len()).sum::<usize>();
    assert!(entries <= 10, "{entries} entries past a cap of 10: {app:?}");
    assert!(app.contains(&FloorFile { lit: true, ..one("app/owned.rs", 5) }), "the feature's own file stands alone, small as it is: {app:?}");
    assert!(app.contains(&FloorFile { changing: true, ..one("app/b/edit.rs", 5) }), "so does the file a live Job wrote: {app:?}");
    assert_eq!(app[0], one("app/big.rs", 1000), "the largest stands alone");
    let groups: Vec<(&str, u64, u32)> = app.iter().filter(|file| file.grouped > 0).map(|file| (file.path.as_str(), file.bytes, file.grouped)).collect();
    assert_eq!(groups, [("app/b/", 100, 10), ("app/a/", 70, 7)], "the rest by folder, their bytes summed: {app:?}");
    assert_eq!(app.iter().map(|file| file.bytes).sum::<u64>(), 1000 + 200 + 10, "no byte lost");
    assert_eq!(files_of(&plan, "lib"), [one("lib/x.rs", 10)], "a folder of one file is that file");
}

#[test]
fn under_the_cap_every_file_stands_alone() {
    let dir = TempDir::new();
    let root = dir.path();
    for n in 0..5 {
        put(root, &format!("app/a/f{n}.rs"), "0123456789");
    }
    committed(root, &["app"]);
    let plan = planned_with_files(root, &feature("f", &[]), &[], 5);
    assert_eq!(files_of(&plan, "app").iter().filter(|file| file.grouped == 0).count(), 5);
}

#[tokio::test]
async fn a_feature_pages_rooms_carry_their_files_and_mark_what_a_live_job_wrote() {
    let rig = rig(None);
    let root = rig.fleet.repositories().first().expect("served").root().to_string();
    cargo_workspace(Path::new(&root));
    put(Path::new(&root), "crates/app/src/lib.rs", "pub fn a() {}\n");
    committed(Path::new(&root), &["crates"]);
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let project = rig.fleet.add_project(NewProject { name: "Shop".to_string(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone();
    let seed = FeatureSeed {
        slug: "start".into(),
        area: "takes".into(),
        title: "Start".into(),
        summary: "One line.".into(),
        state: None,
        paths: vec!["crates/app/src".into()],
        entry_points: Vec::new(),
        manifest_id: None,
    };
    let upsert = UpsertFeatures { project: project.clone(), areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }], features: vec![seed] };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");
    rig.fleet.writes().wrote(&Who::Session("sess-a".into()), rig.manifest.as_str(), ["crates/ipc/Cargo.toml".to_string()], &rig.fleet.now());

    let page = rig.fleet.get_feature_page(None, Some(project), "start".into(), None).await.expect("page");
    let plan = page.floor_plan.expect("a floor plan");
    assert!(files_of(&plan, "app").contains(&FloorFile { lit: true, ..one("crates/app/src/lib.rs", 14) }), "{plan:?}");
    let ipc = files_of(&plan, "ipc");
    assert!(ipc.iter().any(|file| file.path == "crates/ipc/Cargo.toml" && file.changing && !file.lit), "{ipc:?}");
}

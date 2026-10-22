//! The import-closure collector: what it makes of the script's lines, and what the
//! script and TypeScript make of a small package written to a temp directory.

use std::path::{Path, PathBuf};

use adapter_traits::{CollectorSource, Lap, LapCost, LapGrain, TestCollector, TestsToRecord};

use crate::test_map_static::{laps_from, TypeScriptImports};

#[test]
fn the_collector_says_what_it_costs_and_how_sure_it_is() {
    let collector = TypeScriptImports::new("/");
    assert_eq!(collector.source(), CollectorSource::Static);
    assert_eq!(collector.lap_cost(), LapCost::Nothing);
    assert!(!collector.needs_rebuild());
}

#[test]
fn a_line_becomes_a_file_grain_lap_with_paths_relative_to_the_root() {
    let printed = r#"{"test":"/repo/apps/web/src/a.test.ts","files":["/repo/apps/web/src/z.ts","/repo/apps/web/src/a.test.ts","/repo/apps/web/src/lib/b.ts"]}"#;
    let laps = laps_from(printed, Path::new("/repo"), "apps/web").expect("read");
    assert_eq!(
        laps,
        vec![Lap {
            member: "apps/web".into(),
            test: "apps/web/src/a.test.ts".into(),
            grain: LapGrain::File,
            files: vec![
                "apps/web/src/a.test.ts".into(),
                "apps/web/src/lib/b.ts".into(),
                "apps/web/src/z.ts".into(),
            ],
        }]
    );
}

#[test]
fn node_modules_and_files_outside_the_repository_are_dropped() {
    let printed = r#"{"test":"/repo/web/a.test.ts","files":["/repo/web/a.test.ts","/repo/web/node_modules/dep/index.d.ts","/repo/node_modules/.pnpm/ts/node_modules/typescript/lib/typescript.d.ts","/elsewhere/web/x.ts","/repository/web/y.ts","/repo/web/kept.ts"]}"#;
    let laps = laps_from(printed, Path::new("/repo"), "web").expect("read");
    assert_eq!(laps[0].files, vec!["web/a.test.ts", "web/kept.ts"]);
}

#[test]
fn a_test_outside_the_repository_or_in_node_modules_is_no_lap() {
    let printed = [
        r#"{"test":"/elsewhere/a.test.ts","files":["/elsewhere/a.test.ts"]}"#,
        r#"{"test":"/repo/web/node_modules/dep/b.test.ts","files":[]}"#,
        r#"{"test":"/repo/web/c.test.ts","files":[]}"#,
    ]
    .join("\n");
    let laps = laps_from(&printed, Path::new("/repo"), "web").expect("read");
    let tests: Vec<_> = laps.iter().map(|lap| lap.test.as_str()).collect();
    assert_eq!(tests, ["web/c.test.ts"]);
    // A test that reaches nothing else is still a lap, with itself.
    assert_eq!(laps[0].files, ["web/c.test.ts"]);
}

#[test]
fn a_check_at_the_root_has_no_member_and_a_garbled_line_is_not_recorded() {
    let printed = r#"{"test":"/repo/a.test.ts","files":[]}"#;
    assert_eq!(laps_from(printed, Path::new("/repo"), ".").expect("read")[0].member, "");
    assert_eq!(laps_from(printed, Path::new("/repo"), "").expect("read")[0].member, "");
    assert!(laps_from("not json", Path::new("/repo"), "web").is_err());
}

/// This repository's own `typescript`, where pnpm keeps it, or nothing where the
/// dependencies are not installed.
fn typescript_package() -> Option<PathBuf> {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let modules = ancestor.join("node_modules");
        if modules.join("typescript/package.json").is_file() {
            return Some(modules.join("typescript"));
        }
        let Ok(store) = std::fs::read_dir(modules.join(".pnpm")) else {
            continue;
        };
        for entry in store.flatten() {
            let package = entry.path().join("node_modules/typescript");
            if entry.file_name().to_string_lossy().starts_with("typescript@") && package.join("package.json").is_file() {
                return Some(package);
            }
        }
    }
    None
}

/// A test's own note on its stderr, for `--nocapture`: this is not Fleet logging.
fn skipped(why: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "skipped: {why}");
}

fn node_runs() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|said| said.status.success())
        .unwrap_or(false)
}

fn write(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("directory");
    std::fs::write(path, text).expect("written");
}

/// A package `web/` with one `paths` alias, three sources and two tests, under a
/// temp directory that is the repository. `None` where node or typescript is not
/// here to run it, after saying so.
fn fixture(name: &str) -> Option<PathBuf> {
    let Some(typescript) = typescript_package().filter(|_| node_runs()) else {
        skipped("needs node and a typescript package under this repository's node_modules");
        return None;
    };
    let root = std::env::temp_dir().join(format!("armada-static-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let modules = root.join("web/node_modules");
    std::fs::create_dir_all(&modules).expect("modules");
    std::os::unix::fs::symlink(typescript, modules.join("typescript")).expect("linked");
    write(
        &root,
        "web/tsconfig.json",
        r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@lib/*": ["src/lib/*"] } } }"#,
    );
    write(&root, "web/src/a.ts", "export const a = 1;\n");
    write(&root, "web/src/b.ts", "import { a } from './a';\nexport const b = a;\n");
    write(&root, "web/src/lib/c.ts", "export const c = 3;\n");
    write(&root, "web/src/one.test.ts", "import { b } from './b';\ntest('one', () => b);\n");
    // An alias, a built-in and a package: the last two are not the repository's.
    write(
        &root,
        "web/src/two.test.ts",
        "import { c } from '@lib/c';\nimport 'node:fs';\nimport 'typescript';\ntest('two', () => c);\n",
    );
    write(&root, "web/node_modules/dep/dep.test.js", "test('not ours', () => {});\n");
    Some(root)
}

#[test]
fn each_test_file_reaches_what_it_imports_through_its_alias_and_nothing_more() {
    let Some(root) = fixture("every") else { return };
    let laps = TypeScriptImports::new(&root)
        .record("web", "web", &TestsToRecord::All)
        .expect("recorded");
    let _ = std::fs::remove_dir_all(&root);
    let read: Vec<(&str, Vec<&str>)> = laps
        .iter()
        .map(|lap| (lap.test.as_str(), lap.files.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(
        read,
        vec![
            ("web/src/one.test.ts", vec!["web/src/a.ts", "web/src/b.ts", "web/src/one.test.ts"]),
            ("web/src/two.test.ts", vec!["web/src/lib/c.ts", "web/src/two.test.ts"]),
        ]
    );
    assert!(laps.iter().all(|lap| lap.member == "web" && lap.grain == LapGrain::File));
}

#[test]
fn a_named_test_records_that_file_and_a_name_that_is_no_file_records_nothing() {
    let Some(root) = fixture("named") else { return };
    let collector = TypeScriptImports::new(&root);
    let named = |names: &[&str]| TestsToRecord::Named(names.iter().map(|name| name.to_string()).collect());
    let two = collector.record("web", "web", &named(&["web/src/two.test.ts"])).expect("recorded");
    let none = collector.record("web", "web", &named(&["web/src/gone.test.ts"])).expect("recorded");
    let empty = collector.record("web", "web", &named(&[])).expect("recorded");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(two.iter().map(|lap| lap.test.as_str()).collect::<Vec<_>>(), ["web/src/two.test.ts"]);
    assert!(none.is_empty() && empty.is_empty());
}

#[test]
fn a_directory_without_typescript_is_not_recorded_and_says_why() {
    if !node_runs() {
        skipped("needs node");
        return;
    }
    let root = std::env::temp_dir().join(format!("armada-static-bare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(&root, "web/a.test.ts", "");
    let said = TypeScriptImports::new(&root).record("web", "web", &TestsToRecord::All);
    let _ = std::fs::remove_dir_all(&root);
    assert!(said.expect_err("no typescript").0.contains("no typescript package"));
}

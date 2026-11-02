//! The claim, written before the code: **Fleet finds a repository's stylesheets from the code a Job
//! works on, in any repository, keeps a usable copy by content hash, and a `mock` node of a sketch
//! carries the hash it was drawn with.** `.armada/documents/hi-fi-sketches.html`, *How Fleet finds
//! the styles*.

use std::collections::BTreeSet;
use std::path::Path;

use api::{Refusal, Sessions};
use ipc::{KeptStyles, MockNode, SessionId, ShowSketch, StyleKind, StyleSources};

use super::session_host::{eventually, rig_placing};
use super::tmp::TempDir;
use crate::session_host::styling::choosing::style_mocks;
use crate::session_host::styling::keeping;
use crate::session_host::styling::sheet;
use crate::session_host::styling::walk::sources_near;

/// A temporary repository holding `files`, each as a path from its root and its text.
fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new();
    fill(dir.path(), files);
    dir
}

fn fill(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
}

fn root_of(dir: &TempDir) -> std::path::PathBuf {
    std::fs::canonicalize(dir.path()).unwrap()
}

fn near(dir: &TempDir, start: Option<&str>) -> StyleSources {
    sources_near(&root_of(dir), start, &[])
}

fn paths(found: &StyleSources) -> Vec<&str> {
    found.sources.iter().map(|one| one.path.as_str()).collect()
}

fn kind_of<'a>(found: &'a StyleSources, path: &str) -> &'a ipc::StyleSource {
    found.sources.iter().find(|one| one.path == path).unwrap_or_else(|| panic!("{path} not found in {:?}", paths(found)))
}

#[test]
fn a_file_gets_the_sheet_it_imports_then_those_beside_it_then_its_packages_entry() {
    let repo = repo(&[
        ("package.json", r#"{"name":"app"}"#),
        ("src/index.css", "body { margin: 0 }"),
        ("src/ui/Card.tsx", "import \"./Card.css\";\nexport const Card = 1;"),
        ("src/ui/Card.css", ".card { color: red }"),
        ("src/ui/Button.css", ".btn { color: blue }"),
    ]);
    let found = near(&repo, Some("src/ui/Card.tsx"));
    assert_eq!(paths(&found), ["src/ui/Card.css", "src/ui/Button.css", "src/index.css"]);
    assert_eq!(found.start.as_deref(), Some("src/ui/Card.tsx"));
    assert!(found.sources.iter().all(|one| one.usable && one.kind == StyleKind::Css), "{found:?}");
    assert!(found.packages.is_empty());
}

#[test]
fn a_css_module_is_a_kind_of_its_own_and_usable() {
    let repo = repo(&[
        ("package.json", r#"{"name":"app"}"#),
        ("src/Card.tsx", "import styles from './Card.module.css';"),
        ("src/Card.module.css", ".root { color: red }"),
    ]);
    let found = near(&repo, Some("src/Card.tsx"));
    let module = kind_of(&found, "src/Card.module.css");
    assert_eq!((module.kind, module.usable), (StyleKind::CssModule, true));
}

#[test]
fn a_monorepo_gives_the_package_the_file_is_in_and_the_packages_it_depends_on_and_no_others() {
    let repo = repo(&[
        ("package.json", r#"{"name":"root","private":true}"#),
        ("packages/app/package.json", r#"{"name":"@x/app","dependencies":{"@x/tokens":"workspace:*","react":"1"}}"#),
        ("packages/app/src/App.tsx", "export const App = 1;"),
        ("packages/app/src/index.css", ".app { color: var(--fg) }"),
        ("packages/tokens/package.json", r#"{"name":"@x/tokens","exports":{"./base.css":"./base.css","./tokens.css":"./tokens.css"}}"#),
        ("packages/tokens/tokens.css", ":root { --fg: #fff; --gap: 4px }"),
        ("packages/tokens/base.css", "html { color: red }"),
        ("packages/other/package.json", r#"{"name":"@x/other"}"#),
        ("packages/other/src/index.css", ".other { color: green }"),
    ]);
    let found = near(&repo, Some("packages/app/src/App.tsx"));
    // The entry named for the package wins over the other stylesheet its exports list.
    assert_eq!(paths(&found), ["packages/app/src/index.css", "packages/tokens/tokens.css"]);
    let tokens = kind_of(&found, "packages/tokens/tokens.css");
    assert_eq!(tokens.tokens.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["--fg", "--gap"]);
}

#[test]
fn a_tailwind_theme_file_is_listed_after_the_plain_css_and_is_usable() {
    let repo = repo(&[
        ("package.json", r#"{"name":"app","dependencies":{"@x/tokens":"1"}}"#),
        ("src/App.tsx", "export const App = 1;"),
        ("src/index.css", "body { margin: 0 }"),
        ("tokens/package.json", r#"{"name":"@x/tokens","exports":{"./tokens.css":"./tokens.css","./tokens.theme.css":"./tokens.theme.css"}}"#),
        ("tokens/tokens.css", ":root { --c: red }"),
        ("tokens/tokens.theme.css", "@theme inline { --color-c: var(--c); }"),
    ]);
    let found = near(&repo, Some("src/App.tsx"));
    assert_eq!(paths(&found), ["src/index.css", "tokens/tokens.css", "tokens/tokens.theme.css"]);
    let theme = kind_of(&found, "tokens/tokens.theme.css");
    assert_eq!((theme.kind, theme.usable), (StyleKind::ThemeCss, true));
}

#[test]
fn compiled_tailwind_output_on_disk_is_listed_and_no_reset_is_offered() {
    let repo = repo(&[
        ("package.json", r#"{"name":"app","devDependencies":{"tailwindcss":"4"}}"#),
        ("src/App.tsx", "export const App = 1;"),
        ("dist/assets/app.css", "/*! tailwindcss v4.0.0 | MIT */\n.flex{display:flex}"),
        ("node_modules/tailwindcss/preflight.css", "*,::after{box-sizing:border-box}"),
    ]);
    let found = near(&repo, Some("src/App.tsx"));
    assert_eq!(kind_of(&found, "dist/assets/app.css").kind, StyleKind::TailwindOutput);
    assert!(found.sources.iter().all(|one| one.kind != StyleKind::TailwindReset), "{found:?}");
}

#[test]
fn tailwind_with_no_output_gets_the_installed_reset_found_by_walking_node_modules_upward() {
    let repo = repo(&[
        ("package.json", r#"{"name":"root","private":true}"#),
        ("web/package.json", r#"{"name":"web","devDependencies":{"tailwindcss":"4"}}"#),
        ("web/src/App.tsx", "export const App = 1;"),
        ("web/src/app.css", "@import \"tailwindcss\";\n.x { color: red }"),
        ("node_modules/tailwindcss/preflight.css", "*,::after{box-sizing:border-box;border:0 solid;color:--theme(--default-color, red)}"),
    ]);
    let found = near(&repo, Some("web/src/App.tsx"));
    let reset = kind_of(&found, "node_modules/tailwindcss/preflight.css");
    assert_eq!((reset.kind, reset.usable), (StyleKind::TailwindReset, true));
    assert_eq!(found.sources.last().unwrap().path, reset.path, "the reset is the farthest");
}

#[test]
fn scss_variables_are_listed_with_their_tokens_and_are_not_usable() {
    let repo = repo(&[
        ("package.json", r#"{"name":"app"}"#),
        ("src/App.tsx", "export const App = 1;"),
        ("src/styles/_variables.scss", "$brand: #ff0000;\n$gap: 8px; // spacing\n"),
    ]);
    let found = near(&repo, Some("src/App.tsx"));
    let scss = kind_of(&found, "src/styles/_variables.scss");
    assert_eq!((scss.kind, scss.usable), (StyleKind::Scss, false));
    let named: Vec<(&str, &str)> = scss.tokens.iter().map(|one| (one.name.as_str(), one.value.as_str())).collect();
    assert_eq!(named, [("$brand", "#ff0000"), ("$gap", "8px")]);
}

#[test]
fn a_tailwind_config_and_a_theme_module_are_listed_with_tokens_and_are_not_usable() {
    let repo = repo(&[
        ("package.json", r#"{"name":"app"}"#),
        ("src/App.tsx", "export const App = 1;"),
        ("tailwind.config.js", "module.exports = { theme: { colors: {\n  brand: '#123456',\n  text: 'inherit',\n} } }"),
        ("src/theme.ts", "export const theme = {\n  accent: \"#abcdef\",\n  radius: '6px',\n};"),
    ]);
    let found = near(&repo, Some("src/App.tsx"));
    let config = kind_of(&found, "tailwind.config.js");
    assert_eq!((config.kind, config.usable), (StyleKind::TailwindConfig, false));
    assert_eq!(config.tokens.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["brand"]);
    let module = kind_of(&found, "src/theme.ts");
    assert_eq!((module.kind, module.usable), (StyleKind::ThemeModule, false));
    assert_eq!(module.tokens.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["accent", "radius"]);
}

#[test]
fn a_rails_shaped_checkout_with_no_manifest_is_searched_and_ranked_by_the_variables_a_file_sets() {
    let repo = repo(&[
        ("app/views/users/show.html.erb", "<div class=\"card\"></div>"),
        ("app/assets/stylesheets/application.css", ":root { --bg: #fff; --fg: #111; --pad: 12px; --gap: 4px }"),
        ("app/assets/stylesheets/print.css", ":root { --bg: #fff }"),
        ("vendor/bundle/gem/style.css", ":root { --a: #000; --b: #111; --c: #222; --d: #333; --e: #444 }"),
        ("README.md", "x"),
    ]);
    let found = near(&repo, None);
    assert_eq!(paths(&found), ["app/assets/stylesheets/application.css", "app/assets/stylesheets/print.css"]);
    // Named, the view's own folder has none beside it and the search is the walk's last resort.
    assert_eq!(near(&repo, Some("app/views/users/show.html.erb")).sources.len(), 2);
}

#[test]
fn several_packages_and_no_starting_point_list_the_packages_and_guess_nothing() {
    let repo = repo(&[
        ("package.json", r#"{"name":"root"}"#),
        ("apps/a/package.json", r#"{"name":"a"}"#),
        ("apps/a/src/index.css", ".a { color: red }"),
        ("apps/b/package.json", r#"{"name":"b"}"#),
        ("apps/b/src/index.css", ".b { color: red }"),
    ]);
    let found = near(&repo, None);
    assert!(found.sources.is_empty(), "{found:?}");
    assert_eq!(found.packages, [".", "apps/a", "apps/b"]);
}

#[test]
fn the_first_tier_with_a_file_is_the_start_and_a_file_that_is_gone_is_passed_over() {
    let repo = repo(&[
        ("package.json", r#"{"name":"root"}"#),
        ("apps/a/package.json", r#"{"name":"a"}"#),
        ("apps/a/src/A.tsx", "x"),
        ("apps/a/src/index.css", ".a { color: red }"),
        ("apps/b/package.json", r#"{"name":"b"}"#),
        ("apps/b/src/B.tsx", "x"),
        ("apps/b/src/index.css", ".b { color: red }"),
    ]);
    let tiers = [vec![String::from("crates/x/lib.rs"), String::from("apps/gone/G.tsx")], vec![String::from("apps/b/src/B.tsx")], vec![String::from("apps/a/src/A.tsx")]];
    let found = sources_near(&root_of(&repo), None, &tiers);
    assert_eq!(paths(&found).first().copied(), Some("apps/b/src/index.css"), "{found:?}");
    assert!(found.start.is_none(), "the call named none, so Fleet chose");
}

#[test]
fn this_repositorys_job_card_gets_its_sheet_then_the_components_entry_then_the_tokens_with_the_theme_listed() {
    let root = std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    let start = "packages/components/src/compositions/JobCard/JobCard.tsx";
    let found = sources_near(&root, Some(start), &[]);
    let listed = paths(&found);
    assert_eq!(
        listed[..3],
        ["packages/components/src/compositions/JobCard/JobCard.css", "packages/components/src/index.css", "packages/tokens/tokens.css"],
        "{listed:?}"
    );
    assert!(listed.contains(&"packages/tokens/tokens.theme.css"), "{listed:?}");
    assert!(kind_of(&found, "packages/components/src/compositions/JobCard/JobCard.css").usable);
    assert!(kind_of(&found, "packages/tokens/tokens.css").usable);
    // Its 213 imports weigh over a megabyte once they are in, past what a kept sheet is capped at.
    assert!(!kind_of(&found, "packages/components/src/index.css").usable);
    assert!(kind_of(&found, "packages/tokens/tokens.theme.css").usable);
    // The answer leaves out file text and stays small.
    let weight: usize = found.sources.iter().flat_map(|one| &one.tokens).map(|one| one.name.len() + one.value.len()).sum();
    assert!(weight < 60 * 1024, "{weight}");
}

#[test]
fn the_real_tokens_css_keeps_its_five_plex_faces_with_their_weights_as_written() {
    let root = std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    let set = keeping::assemble(&root, &[String::from("packages/tokens/tokens.css")], &BTreeSet::new(), false).unwrap();
    let mut faces: Vec<(String, String)> = set.fonts.iter().map(|one| (one.family.clone(), one.weight.clone())).collect();
    faces.sort();
    let plex = |family: &str, weight: &str| (family.to_string(), weight.to_string());
    assert_eq!(
        faces,
        [
            plex("IBM Plex Mono", "400"),
            plex("IBM Plex Mono", "500"),
            plex("IBM Plex Sans", "400"),
            plex("IBM Plex Sans", "500"),
            plex("IBM Plex Sans", "600"),
        ]
    );
    assert!(!set.sheets[0].css.contains("@font-face") && !set.sheets[0].css.contains("url("), "faces and addresses are out of the css");
}

fn made(files: &[(&str, &str)], start: &str) -> Result<sheet::Made, String> {
    let repo = repo(files);
    let root = root_of(&repo);
    sheet::make(&root, &root.join(start))
}

#[test]
fn imports_are_inlined_in_place_a_cycle_ends_and_a_remote_one_is_dropped() {
    let made = made(
        &[
            ("a.css", "@import \"./b.css\" layer(base);\n@import url(\"https://example.test/x.css\");\n.a { color: red }"),
            ("b.css", "@import \"a.css\";\n.b { color: blue }"),
        ],
        "a.css",
    )
    .unwrap();
    assert!(!made.css.contains("@import") && !made.css.contains("example.test"), "{}", made.css);
    assert!(made.css.contains("@layer base {") && made.css.find(".b {") < made.css.find(".a {"), "{}", made.css);
    assert_eq!(made.read.len(), 2);
}

#[test]
fn a_bare_import_resolves_through_node_modules_and_one_that_leaves_the_checkout_is_dropped() {
    let made = made(
        &[
            ("src/a.css", "@import \"@x/tokens/t.css\";\n@import \"../../outside.css\";\n.a { color: var(--c) }"),
            ("node_modules/@x/tokens/t.css", ":root { --c: red }"),
        ],
        "src/a.css",
    )
    .unwrap();
    assert!(made.css.contains("--c: red") && !made.css.contains("@import"), "{}", made.css);
}

#[test]
fn a_font_face_becomes_font_bytes_and_leaves_the_css_and_the_weight_is_kept_as_written() {
    let repo = repo(&[
        ("css/a.css", "@font-face { font-family: \"Plex\"; font-weight: 100 900; font-style: italic; src: url(\"../fonts/p.woff\") format(\"woff\"), url(\"../fonts/p.woff2\") format(\"woff2\"); }\n@font-face { font-family: Remote; src: url(https://example.test/r.woff2); }\n.a { font-family: Plex }"),
        ("fonts/p.woff2", "two"),
        ("fonts/p.woff", "one"),
    ]);
    let root = root_of(&repo);
    let set = keeping::assemble(&root, &[String::from("css/a.css")], &BTreeSet::new(), false).unwrap();
    assert_eq!(set.fonts.len(), 1, "a face with no local file is dropped: {:?}", set.fonts);
    let font = &set.fonts[0];
    assert_eq!((font.family.as_str(), font.weight.as_str(), font.style.as_deref()), ("Plex", "100 900", Some("italic")));
    use base64::Engine;
    assert_eq!(base64::engine::general_purpose::STANDARD.decode(&font.bytes).unwrap(), b"two", "woff2 is preferred");
    assert!(set.sheets[0].css.contains(".a { font-family: Plex }") && !set.sheets[0].css.contains("@font-face"));
}

#[test]
fn an_address_removes_its_declaration_and_leaves_the_rest_of_the_rule() {
    let made = made(
        &[("a.css", ".a { color: red; background: url(\"img/x.png\") no-repeat; margin: 0 }\n.b { background-image: image-set(\"a.png\" 1x) }\n.c { color: blue }")],
        "a.css",
    )
    .unwrap();
    assert!(!made.css.contains("url(") && !made.css.contains("image-set"), "{}", made.css);
    assert!(made.css.contains("color: red") && made.css.contains("margin: 0") && made.css.contains(".c { color: blue }"), "{}", made.css);
}

#[test]
fn tailwind_theme_becomes_root_its_other_rules_go_and_the_reset_function_becomes_var() {
    let made = made(
        &[(
            "a.css",
            "@source \"../src\";\n@plugin \"x\";\n@custom-variant dark (&:is(.dark *));\n@utility tab { tab-size: 4 }\n@theme inline { --color-c: var(--c); }\n.x { @apply flex p-2; color: --theme(--default-color, red); }\n@tailwind utilities;",
        )],
        "a.css",
    )
    .unwrap();
    let css = &made.css;
    assert!(css.contains(":root {") && css.contains("--color-c: var(--c)"), "{css}");
    for gone in ["@source", "@plugin", "@custom-variant", "@utility", "@apply", "@theme", "@tailwind", "--theme("] {
        assert!(!css.contains(gone), "{gone} in {css}");
    }
    assert!(css.contains("var(--default-color, red)"), "{css}");
}

#[test]
fn a_sheet_past_its_cap_is_not_usable_and_the_walk_says_so() {
    let big = format!(".a {{ color: red }}\n{}", ".b { color: blue }\n".repeat(20_000));
    let repo = repo(&[("package.json", r#"{"name":"app"}"#), ("src/A.tsx", "x"), ("src/index.css", &big), ("src/A.css", ".a { color: red }")]);
    let found = near(&repo, Some("src/A.tsx"));
    assert!(kind_of(&found, "src/A.css").usable);
    let index = kind_of(&found, "src/index.css");
    assert!(!index.usable && index.bytes > 256 * 1024, "{index:?}");
}

#[test]
fn fonts_past_their_cap_are_left_out_and_the_sheet_stays() {
    let heavy = "x".repeat(600 * 1024);
    let repo = repo(&[
        ("a.css", "@font-face { font-family: A; src: url(a.woff2) }\n@font-face { font-family: B; src: url(b.woff2) }\n.a { color: red }"),
        ("a.woff2", &heavy),
        ("b.woff2", &heavy),
    ]);
    let set = keeping::assemble(&root_of(&repo), &[String::from("a.css")], &BTreeSet::new(), false).unwrap();
    assert!(set.fonts.is_empty() && set.sheets[0].css.contains(".a { color: red }"));
}

#[test]
fn the_same_content_has_the_same_hash_and_a_set_is_written_once_and_never_rewritten() {
    let repo = repo(&[("a.css", ".a { color: red }")]);
    let home = TempDir::new();
    let folder = keeping::folder(home.path().to_str().unwrap());
    let root = root_of(&repo);
    let one = keeping::assemble(&root, &[String::from("a.css")], &BTreeSet::new(), false).unwrap();
    let two = keeping::assemble(&root, &[String::from("a.css")], &BTreeSet::new(), false).unwrap();
    let hash = keeping::keep(&folder, &one).unwrap();
    assert_eq!(hash.len(), 32);
    assert!(keeping::is_hash(&hash));
    assert_eq!(keeping::keep(&folder, &two).unwrap(), hash);
    let file = folder.join(format!("{hash}.json"));
    let stamp = std::fs::metadata(&file).unwrap().modified().unwrap();
    std::fs::write(root.join("a.css"), ".a { color: blue }").unwrap();
    let moved = keeping::assemble(&root, &[String::from("a.css")], &BTreeSet::new(), false).unwrap();
    assert_ne!(keeping::keep(&folder, &moved).unwrap(), hash, "other content, other file");
    assert_eq!(std::fs::metadata(&file).unwrap().modified().unwrap(), stamp);
    assert_eq!(keeping::read(&folder, &hash).unwrap(), one);
}

#[test]
fn only_32_lower_case_hex_characters_name_a_kept_set() {
    let home = TempDir::new();
    let folder = keeping::folder(home.path().to_str().unwrap());
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(home.path().join("secret.json"), "{}").unwrap();
    for hash in ["../secret", "..", "", "ABCDEF0123456789ABCDEF0123456789", "0123456789abcdef0123456789abcde", "0123456789abcdef0123456789abcdef0", "0123456789abcdef0123456789abcdeg"] {
        assert!(!keeping::is_hash(hash), "{hash}");
        assert!(keeping::read(&folder, hash).is_none(), "{hash}");
    }
}

fn git(root: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.test", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn a_sheet_the_checkout_holds_uncommitted_changes_to_is_marked_edited() {
    let repo = repo(&[("a.css", ".a { color: red }"), ("b.css", "@import \"./c.css\";"), ("c.css", ".c { color: red }")]);
    let root = root_of(&repo);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "first"]);
    std::fs::write(root.join("c.css"), ".c { color: blue }").unwrap();
    let mock = MockNode { id: String::from("m"), styles: vec![String::from("a.css"), String::from("b.css")], has_css: false };
    let home = TempDir::new();
    let styled = style_mocks(root.to_str().unwrap(), &[], home.path().to_str().unwrap(), &[mock]);
    let set = keeping::read(&keeping::folder(home.path().to_str().unwrap()), &styled.kept["m"]).unwrap();
    let edited: Vec<(&str, bool)> = set.sheets.iter().map(|one| (one.path.as_str(), one.edited)).collect();
    assert_eq!(edited, [("a.css", false), ("b.css", true)], "an import the checkout changed counts");
}

fn mock(id: &str, styles: &[&str], css: bool) -> MockNode {
    MockNode { id: id.to_string(), styles: styles.iter().map(|one| one.to_string()).collect(), has_css: css }
}

fn choose(repo: &TempDir, tiers: &[Vec<String>], mocks: &[MockNode]) -> (super::tmp::TempDir, crate::session_host::styling::choosing::Styled) {
    let home = TempDir::new();
    let styled = style_mocks(repo.path().to_str().unwrap(), tiers, home.path().to_str().unwrap(), mocks);
    (home, styled)
}

#[test]
fn styles_the_agent_names_must_be_inside_the_checkout_exist_and_be_usable_or_the_mock_is_refused_naming_the_path() {
    let outside = repo(&[("secret.css", ".s { color: red }")]);
    let repo = repo(&[
        ("package.json", r#"{"name":"app"}"#),
        ("src/A.tsx", "x"),
        ("src/a.css", ".a { color: red }"),
        ("src/vars.scss", "$c: red;"),
    ]);
    std::os::unix::fs::symlink(outside.path().join("secret.css"), repo.path().join("src/link.css")).unwrap();
    let tiers = [vec![String::from("src/A.tsx")]];
    let (_home, styled) = choose(
        &repo,
        &tiers,
        &[
            mock("up", &["../x.css"], false),
            mock("link", &["src/link.css"], false),
            mock("gone", &["src/none.css"], false),
            mock("scss", &["src/vars.scss"], false),
            mock("ok", &["src/a.css"], false),
        ],
    );
    assert_eq!(styled.kept.keys().collect::<Vec<_>>(), ["ok"]);
    assert!(styled.refused["up"].contains("`../x.css` is outside this job's checkout"), "{}", styled.refused["up"]);
    assert!(styled.refused["link"].contains("`src/link.css` is not a file inside this job's checkout"), "{}", styled.refused["link"]);
    assert!(styled.refused["gone"].contains("`src/none.css` is not a file inside this job's checkout"), "{}", styled.refused["gone"]);
    let scss = &styled.refused["scss"];
    assert!(scss.contains("`src/vars.scss` is a scss file, which a browser cannot use as it stands"), "{scss}");
    assert!(scss.contains("the walk from this job's work found src/a.css (css)"), "what the walk found is named: {scss}");
}

#[test]
fn a_mock_with_no_styles_is_drawn_with_the_usable_sheets_of_the_walk_and_mocks_asking_for_the_same_files_share_one_set() {
    let repo = repo(&[
        ("package.json", r#"{"name":"app"}"#),
        ("src/A.tsx", "x"),
        ("src/A.css", ".a { color: var(--c) }"),
        ("src/index.css", ":root { --c: red }"),
    ]);
    let tiers = [vec![String::from("src/A.tsx")]];
    let (home, styled) = choose(&repo, &tiers, &[mock("one", &[], false), mock("two", &[], true), mock("three", &["src/A.css", "src/index.css"], false)]);
    assert!(styled.refused.is_empty(), "{:?}", styled.refused);
    assert_eq!(styled.kept["one"], styled.kept["two"]);
    let set = keeping::read(&keeping::folder(home.path().to_str().unwrap()), &styled.kept["one"]).unwrap();
    let applied: Vec<&str> = set.sheets.iter().map(|one| one.path.as_str()).collect();
    assert_eq!(applied, ["src/index.css", "src/A.css"], "the nearest applies last");
    assert_eq!(std::fs::read_dir(keeping::folder(home.path().to_str().unwrap())).unwrap().count(), 2, "two different sets, not three");
}

#[test]
fn a_mock_with_nothing_usable_and_no_css_is_sent_to_a_screen_and_one_with_css_is_kept_without_a_hash() {
    let repo = repo(&[("package.json", r#"{"name":"app"}"#), ("src/A.tsx", "x")]);
    let tiers = [vec![String::from("src/A.tsx")]];
    let (_home, styled) = choose(&repo, &tiers, &[mock("bare", &[], false), mock("own", &[], true)]);
    assert!(styled.kept.is_empty());
    let said = &styled.refused["bare"];
    assert!(said.contains("Draw a `screen` node instead"), "{said}");
    assert!(!styled.refused.contains_key("own"));
}

#[test]
fn several_packages_and_no_starting_point_refuse_the_mock_and_list_the_packages() {
    let repo = repo(&[
        ("package.json", r#"{"name":"root"}"#),
        ("apps/a/package.json", r#"{"name":"a"}"#),
        ("apps/b/package.json", r#"{"name":"b"}"#),
    ]);
    let (_home, styled) = choose(&repo, &[], &[mock("m", &[], true)]);
    let said = &styled.refused["m"];
    assert!(said.contains("3 packages") && said.contains("apps/a") && said.contains("apps/b"), "{said}");
    assert!(said.contains("Name the mock's `styles`, or call get_style_sources with a path"), "{said}");
}

#[test]
fn a_job_with_no_checkout_behaves_as_nothing_usable() {
    let home = TempDir::new();
    let styled = style_mocks("/nonexistent/checkout", &[], home.path().to_str().unwrap(), &[mock("bare", &[], false), mock("own", &[], true), mock("named", &["a.css"], true)]);
    assert!(styled.refused["bare"].contains("Draw a `screen` node instead"));
    assert!(styled.kept.is_empty() && !styled.refused.contains_key("own"));
    assert!(styled.refused["named"].contains("no checkout"));
}

fn mock_scene(styles: &str, css: &str) -> String {
    format!(
        r#"{{"nodes":[{{"id":"m","kind":"mock","x":0,"y":0,"w":300,"html":"<b class=\"a\">x</b>"{css}{styles}}}],"edges":[]}}"#
    )
}

fn drawn(id: &SessionId, title: &str, scene: &str, against: Option<&str>) -> ShowSketch {
    let against = against.map(|one| format!(r#","against":"{one}""#)).unwrap_or_default();
    ipc::decode(
        "a sketch to show",
        format!(r#"{{"title":"{title}","session_id":"{}","scene":{scene}{against}}}"#, id.as_str()).as_bytes(),
    )
    .unwrap()
}

fn scene_of(record: &ipc::SessionRecord, title: &str) -> serde_like::Mock {
    let one = record.attachments.iter().find(|one| one.target == format!("sketch:{title}")).expect("held");
    serde_like::mock_of(&one.detail["scene"])
}

/// The `kept` and `styles` of the one mock a scene holds, read through `ipc`'s codec.
mod serde_like {
    #[derive(Debug)]
    pub(super) struct Mock {
        pub kept: Option<String>,
    }

    pub(super) fn mock_of(scene: &str) -> Mock {
        #[derive(serde::Deserialize)]
        struct Node {
            kept: Option<String>,
        }
        #[derive(serde::Deserialize)]
        struct Scene {
            nodes: Vec<Node>,
        }
        let scene: Scene = ipc::decode("a scene", scene.as_bytes()).unwrap();
        Mock { kept: scene.nodes.into_iter().next().unwrap().kept }
    }
}

#[tokio::test]
async fn a_shown_mock_carries_the_hash_of_its_styles_and_keeps_that_look_across_a_redraw_and_against() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let root = Path::new(&rig.root);
    fill(root, &[("src/a.css", ".a { color: red }")]);

    let styles = r#","styles":["src/a.css"]"#;
    let record = rig.fleet.show_sketch(None, drawn(&id, "Card", &mock_scene(styles, ""), None)).await.unwrap();
    let first = scene_of(&record, "Card").kept.expect("kept is written on the node");
    let set: KeptStyles = rig.fleet.get_kept_styles(first.clone()).await.unwrap();
    assert_eq!(set.sheets[0].path, "src/a.css");
    assert!(set.sheets[0].css.contains("color: red"));

    // The app's CSS moves on. The drawing keeps what it was drawn with.
    std::fs::write(root.join("src/a.css"), ".a { color: blue }").unwrap();
    assert!(rig.fleet.get_kept_styles(first.clone()).await.unwrap().sheets[0].css.contains("color: red"));

    // A redraw is a revision like any other; it takes the look of now, and its `against` still shows the earlier one.
    let record = rig.fleet.show_sketch(None, drawn(&id, "Card", &mock_scene(styles, ""), None)).await.unwrap();
    let second = scene_of(&record, "Card").kept.expect("kept");
    assert_ne!(first, second);
    let card = record.attachments.iter().find(|one| one.target == "sketch:Card").unwrap();
    let revisions: Vec<String> = ipc::decode("revisions", card.detail["revisions"].as_bytes()).unwrap();
    assert!(std::fs::read_to_string(&revisions[0]).unwrap().contains(&first), "the first revision keeps its hash");

    let record = rig.fleet.show_sketch(None, drawn(&id, "Card two", &mock_scene(styles, ""), Some("Card"))).await.unwrap();
    let against = record.attachments.iter().find(|one| one.target == "sketch:Card two").unwrap().detail["against"].clone();
    assert!(against.contains(&second), "the sketch it is read against keeps its look: {against}");
    assert!(scene_of(&record, "Card two").kept.is_some());
}

#[tokio::test]
async fn a_shown_sketch_with_a_mock_that_cannot_be_drawn_is_refused_telling_the_agent_what_to_do() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let bare = rig.fleet.show_sketch(None, drawn(&id, "Card", &mock_scene("", ""), None)).await;
    let Err(Refusal::Unacceptable(refusal)) = bare else { panic!("{bare:?}") };
    assert!(refusal.message.contains("Draw a `screen` node instead") && refusal.message.contains("call show_sketch again"), "{}", refusal.message);
    let own = rig.fleet.show_sketch(None, drawn(&id, "Card", &mock_scene("", r#","css":".a{color:red}""#), None)).await.unwrap();
    assert!(scene_of(&own, "Card").kept.is_none(), "css of its own is kept with no hash");
}

#[tokio::test]
async fn a_plans_sketch_keeps_the_styles_of_its_mocks_too() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    fill(Path::new(&rig.root), &[("src/a.css", ".a { color: red }")]);
    let mut recorded = super::session_plan::first(&id);
    recorded.sketch = Some(
        ipc::decode("a plan sketch", format!(r#"{{"title":"Plan card","scene":{}}}"#, mock_scene(r#","styles":["src/a.css"]"#, "")).as_bytes()).unwrap(),
    );
    let record = rig.fleet.record_plan(None, recorded, false).await.expect("recorded");
    assert!(scene_of(&record, "Plan card").kept.is_some());
}

#[tokio::test]
async fn a_document_page_with_a_mock_carries_its_style_sets_beside_it_and_a_mock_that_cannot_be_drawn_is_told() {
    use adapter_traits::{CallDetail, DroneEvent};
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    rig.stand_in.init(0);
    fill(Path::new(&rig.root), &[("src/a.css", ".a { color: red }")]);
    let good = mock_scene(r#","styles":["src/a.css"]"#, "");
    let bad = mock_scene("", "");
    let page = format!(
        r#"<html><head><title>T</title></head><body><armada-sketch title="Good"><script type="application/json">{good}</script></armada-sketch><armada-sketch title="Bad"><script type="application/json">{bad}</script></armada-sketch></body></html>"#
    );
    let file = Path::new(&rig.root).join(".armada/documents/page.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, page).unwrap();
    let path = file.to_string_lossy().into_owned();
    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: "toolu_1".into(), detail: CallDetail::of(&format!("{path} +10")) },
            DroneEvent::Answered { call: "toolu_1".into(), failed: false },
        ],
    );
    eventually(|| async {
        let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
        listed.iter().find(|one| one.id == id).is_some_and(|one| one.attachments.iter().any(|a| a.target == path && a.detail.contains_key("kept_styles")))
    })
    .await;
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    let record = listed.iter().find(|one| one.id == id).unwrap();
    let document = record.attachments.iter().find(|one| one.target == path).unwrap();
    let kept: std::collections::BTreeMap<String, String> = ipc::decode("kept styles", document.detail["kept_styles"].as_bytes()).unwrap();
    assert_eq!(kept.keys().collect::<Vec<_>>(), ["Good#m"]);
    assert!(rig.fleet.get_kept_styles(kept["Good#m"].clone()).await.is_ok());
    eventually(|| async {
        rig.rows(&id).await.iter().any(|row| matches!(row, ipc::SessionRow::Message { notice: Some(word), text, .. } if word == "document_sketch" && text.contains("sketch 2 of 2 (\"Bad\")") && text.contains("Draw a `screen` node")))
    })
    .await;
}

#[tokio::test]
async fn get_kept_styles_refuses_a_hash_that_could_walk_the_disk_and_one_it_holds_nothing_under() {
    let rig = rig_placing(None);
    for hash in ["../../etc/passwd", "ABCDEF0123456789ABCDEF0123456789", "0123456789abcdef0123456789abcdef"] {
        let read = rig.fleet.get_kept_styles(hash.to_string()).await;
        assert!(matches!(read, Err(Refusal::NotFound(_))), "{hash}: {read:?}");
    }
}

#[tokio::test]
async fn get_style_sources_starts_from_the_path_named_else_from_a_file_the_job_read() {
    use adapter_traits::{CallDetail, DroneEvent};
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "look at the card").await;
    rig.stand_in.init(0);
    let root = Path::new(&rig.root);
    fill(root, &[("package.json", r#"{"name":"app"}"#), ("src/Card.tsx", "x"), ("src/Card.css", ".c { color: red }")]);

    let named = rig.fleet.get_style_sources(None, Some(id.clone()), Some(String::from("src/Card.tsx"))).await.unwrap();
    assert_eq!(named.start.as_deref(), Some("src/Card.tsx"));
    assert_eq!(named.sources[0].path, "src/Card.css");
    let outside = rig.fleet.get_style_sources(None, Some(id.clone()), Some(String::from("../elsewhere"))).await;
    assert!(matches!(outside, Err(Refusal::Unacceptable(_))), "{outside:?}");

    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Read".into(), call: "toolu_r".into(), detail: CallDetail::of(&format!("{}/src/Card.tsx", rig.root)) },
            DroneEvent::Answered { call: "toolu_r".into(), failed: false },
        ],
    );
    eventually(|| async {
        let found = rig.fleet.get_style_sources(None, Some(id.clone()), None).await.unwrap();
        found.sources.first().is_some_and(|one| one.path == "src/Card.css") && found.start.is_none()
    })
    .await;
}

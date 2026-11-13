//! The claim, written before the code: **a Job reads what a mock is drawn with in one call — tokens
//! by role with each theme's value, components with their real classes and markup, the glyphs a
//! mock may name and the repository's house patterns — cut to what it asked for, and Fleet reads a
//! checkout again only when one of those files changed.**

use std::path::Path;
use std::sync::Arc;

use api::{Refusal, Sessions};
use ipc::{CatalogueArea, CatalogueAsked, DesignCatalogue, TokenRole, MOCK_GLYPHS};

use super::session_host::rig_placing;
use super::tmp::TempDir;
use crate::session_host::styling::catalogue::{answer, catalogue_of, components, inputs, Whole, ANSWER_AT_MOST};

fn fill(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
}

fn repo(files: &[(&str, &str)]) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new();
    fill(dir.path(), files);
    let root = std::fs::canonicalize(dir.path()).unwrap();
    (dir, root)
}

const TOKENS: &str = r#"
/* The document's tokens. */
:root, [data-theme] {
  --bg-raised: #1a1a1a;
  --fg-muted: #9a9a9a;
  --status-running: oklch(0.7 0.1 250);
  --surface-card: var(--bg-raised);
  --space-2: 8px;
  --text-sm: 14px;
  --radius-md: 6px;
  --shadow-card: 0 1px 2px rgb(0 0 0 / 0.3);
  --ease-out: cubic-bezier(0.2, 0, 0, 1);
}
[data-theme="light"] {
  --bg-raised: #ffffff;
}
@media (prefers-color-scheme: dark) {
  :root { --fg-muted: #888888; }
}
@theme inline {
  --color-bg-raised: var(--bg-raised);
  --text-sm: 14px;
  --breakpoint-wide: 1400px;
}
.armada-card { --card-pad: 12px; }
"#;

fn token<'a>(catalogue: &'a DesignCatalogue, name: &str) -> (&'a ipc::TokenGroup, &'a ipc::CatalogueToken) {
    for group in &catalogue.tokens {
        if let Some(token) = group.tokens.iter().find(|token| token.name == name) {
            return (group, token);
        }
    }
    panic!("{name} is not in {:#?}", catalogue.tokens)
}

#[test]
fn tokens_come_by_role_and_family_with_each_themes_value_and_aliases_resolved() {
    let (_dir, root) = repo(&[("packages/tokens/tokens.css", TOKENS)]);
    let tokens = answer(&catalogue_of(&root), CatalogueArea::Tokens, None);

    assert_eq!(tokens.themes, ["default", "light", "dark"]);
    assert_eq!(tokens.sheets, ["packages/tokens/tokens.css"]);
    let (group, raised) = token(&tokens, "--bg-raised");
    assert_eq!((group.role, group.family.as_str()), (TokenRole::Colour, "--bg"));
    assert_eq!(raised.value, "#1a1a1a");
    assert_eq!(raised.themes.get("light").map(String::as_str), Some("#ffffff"));
    assert_eq!(token(&tokens, "--fg-muted").1.themes.get("dark").map(String::as_str), Some("#888888"));

    let (group, card) = token(&tokens, "--surface-card");
    assert_eq!(group.role, TokenRole::Colour, "an alias takes the role of what it resolves to");
    assert_eq!(card.value, "#1a1a1a");
    assert_eq!(card.alias.as_deref(), Some("var(--bg-raised)"));
    assert_eq!(card.themes.get("light").map(String::as_str), Some("#ffffff"), "an alias follows the theme");

    assert_eq!(token(&tokens, "--space-2").0.role, TokenRole::Space);
    assert_eq!(token(&tokens, "--text-sm").0.role, TokenRole::Type);
    assert_eq!(token(&tokens, "--radius-md").0.role, TokenRole::Radius);
    assert_eq!(token(&tokens, "--shadow-card").0.role, TokenRole::Shadow);
    assert_eq!(token(&tokens, "--ease-out").0.role, TokenRole::Motion);
    assert_eq!(token(&tokens, "--status-running").0.family, "--status");

    let names: Vec<&str> = tokens.tokens.iter().flat_map(|group| group.tokens.iter().map(|token| token.name.as_str())).collect();
    assert!(!names.contains(&"--card-pad"), "a component's own property is not a token: {names:?}");
    assert!(!names.contains(&"--color-bg-raised"), "a @theme re-export of a token is left out: {names:?}");
    assert!(names.contains(&"--breakpoint-wide"), "a @theme value of its own is kept: {names:?}");
    assert_eq!(names.iter().filter(|name| **name == "--text-sm").count(), 1);
}

const KBD_TSX: &str = r#"
import type { HTMLAttributes, ReactNode } from "react";
export function Kbd({ className, ...rest }: HTMLAttributes<HTMLElement>) {
  return <kbd className={className ? `armada-kbd ${className}` : "armada-kbd"} {...rest} />;
}
export function KbdChord({ keys }: { keys: ReactNode[] }) {
  const sorted = keys.length < 2 ? keys : keys.slice();
  return (
    <Kbd>
      {sorted.map((key, i) => (
        <span className="armada-kbd__key" key={i}>
          {key}
        </span>
      ))}
    </Kbd>
  );
}
"#;

const KBD_CSS: &str = r#"
/* .not-a-class in a comment */
.armada-kbd { display: inline-flex; padding: 0 0.25rem; background: url(key.png); }
.armada-kbd__key + .armada-kbd__key { margin-left: 2px; }
.armada-kbd:hover { color: var(--fg-muted); }
"#;

const ROW_TSX: &str = r#"
export function SettingRow({ label, children }) {
  return (
    <div className="setting-row flex items-center gap-2">
      <span className="setting-row__label text-sm">{label}</span>
      {children}
    </div>
  );
}
"#;

#[test]
fn a_component_lists_its_sheets_classes_and_a_sample_read_from_its_own_tags() {
    let (_dir, root) = repo(&[
        ("src/primitives/Kbd/Kbd.tsx", KBD_TSX),
        ("src/primitives/Kbd/Kbd.css", KBD_CSS),
        ("src/primitives/Kbd/Kbd.stories.tsx", "export const A = () => <Kbd className=\"story\" />;"),
        ("src/settings/SettingRow.tsx", ROW_TSX),
        ("src/use-thing.tsx", "export const x = <div className=\"hook\" />;"),
    ]);
    let found = answer(&catalogue_of(&root), CatalogueArea::Components, None);
    let names: Vec<&str> = found.components.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["Kbd", "SettingRow"], "stories and hooks are not components");

    let kbd = &found.components[0];
    assert_eq!(kbd.file, "src/primitives/Kbd/Kbd.tsx");
    assert_eq!(kbd.sheet.as_deref(), Some("src/primitives/Kbd/Kbd.css"));
    assert_eq!(kbd.classes, ["armada-kbd", "armada-kbd__key"]);
    let sample = kbd.sample.as_deref().unwrap();
    assert!(sample.contains(r#"<kbd class="armada-kbd"></kbd>"#), "{sample}");
    assert!(sample.contains(r#"<span class="armada-kbd__key"></span>"#), "{sample}");

    let row = &found.components[1];
    assert_eq!(row.sheet, None);
    assert_eq!(row.classes, ["setting-row", "flex", "items-center", "gap-2", "setting-row__label", "text-sm"], "no sheet: the literals it writes");
    assert_eq!(
        row.sample.as_deref(),
        Some("<div class=\"setting-row flex items-center gap-2\">\n  <span class=\"setting-row__label text-sm\"></span>\n</div>")
    );
}

#[test]
fn a_type_parameter_or_a_comparison_is_not_read_as_a_tag() {
    let tree = components::elements_in("const a = useState<string>(x); if (n < limit) {}\nreturn <div className=\"a\"><b className=\"b\" /></div>;");
    assert_eq!(tree.len(), 1, "{tree:?}");
    assert_eq!((tree[0].tag.as_str(), tree[0].children[0].tag.as_str()), ("div", "b"));
}

const PATTERNS: &str = r#"<!doctype html>
<!-- The house patterns a mock copies. -->
<template data-pattern="chip" data-note="A fact beside a title">
  <span class="armada-badge">
    <i data-icon="clock"></i> 2m
  </span>
</template>
<template data-pattern="kbd"><kbd class="armada-kbd">⌘K</kbd></template>
<template data-pattern="">ignored</template>
"#;

#[test]
fn the_patterns_a_repository_keeps_are_read_from_its_patterns_file() {
    let (_dir, root) = repo(&[(inputs::PATTERNS_FILE, PATTERNS)]);
    let found = answer(&catalogue_of(&root), CatalogueArea::Patterns, None);
    let names: Vec<&str> = found.patterns.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["chip", "kbd"]);
    assert_eq!(found.patterns[0].note.as_deref(), Some("A fact beside a title"));
    assert_eq!(found.patterns[0].html, "<span class=\"armada-badge\">\n  <i data-icon=\"clock\"></i> 2m\n</span>");

    let one = answer(&catalogue_of(&root), CatalogueArea::Patterns, Some("KBD"));
    assert_eq!(one.patterns.len(), 1, "match is case-blind");
}

#[test]
fn icons_are_the_glyphs_a_mock_may_name() {
    let (_dir, root) = repo(&[]);
    let all = answer(&catalogue_of(&root), CatalogueArea::Icons, None);
    assert_eq!(all.icons.len(), MOCK_GLYPHS.len());
    let circles = answer(&catalogue_of(&root), CatalogueArea::Icons, Some("circle"));
    assert!(circles.icons.len() > 3 && circles.icons.iter().all(|name| name.contains("circle")), "{:?}", circles.icons);
}

#[test]
fn contents_name_each_area_and_hold_no_values() {
    let (_dir, root) = repo(&[
        ("src/tokens.css", TOKENS),
        ("src/primitives/Kbd/Kbd.tsx", KBD_TSX),
        ("src/primitives/Kbd/Kbd.css", KBD_CSS),
        (inputs::PATTERNS_FILE, PATTERNS),
    ]);
    let contents = answer(&catalogue_of(&root), CatalogueArea::Contents, None);
    let listed = contents.contents.as_ref().unwrap();
    assert_eq!(listed.components, ["Kbd"]);
    assert_eq!(listed.patterns, ["chip", "kbd"]);
    assert_eq!(listed.icons, MOCK_GLYPHS.len());
    assert!(listed.tokens.iter().any(|family| family.family == "--space" && family.role == TokenRole::Space && family.tokens == 1));
    assert!(contents.tokens.is_empty() && contents.components.is_empty() && contents.patterns.is_empty() && contents.icons.is_empty());
}

#[test]
fn match_narrows_tokens_by_family_role_or_name() {
    let (_dir, root) = repo(&[("src/tokens.css", TOKENS)]);
    let whole = catalogue_of(&root);
    let space = answer(&whole, CatalogueArea::Tokens, Some("--space"));
    assert_eq!(space.tokens.len(), 1);
    assert_eq!(space.tokens[0].family, "--space");
    let colour = answer(&whole, CatalogueArea::Tokens, Some("colour"));
    assert!(!colour.tokens.is_empty() && colour.tokens.iter().all(|group| group.role == TokenRole::Colour));
    let one = answer(&whole, CatalogueArea::Tokens, Some("running"));
    assert_eq!(one.tokens.iter().map(|group| group.tokens.len()).sum::<usize>(), 1);
}

#[test]
fn an_answer_too_big_is_cut_under_the_cap_and_says_how_to_ask_for_the_rest() {
    let mut whole = Whole::default();
    for at in 0..400 {
        whole.components.push(ipc::CatalogueComponent {
            name: format!("Component{at}"),
            file: format!("src/c/Component{at}.tsx"),
            sheet: None,
            classes: vec![format!("component-{at}")],
            sample: Some(format!("<div class=\"component-{at}\">{}</div>", "x".repeat(100))),
        });
    }
    let cut = answer(&whole, CatalogueArea::Components, None);
    let weight = ipc::encode(&cut).unwrap().len();
    assert!(weight <= ANSWER_AT_MOST, "{weight}");
    assert!(cut.components.len() < 400 && !cut.components.is_empty());
    let said = cut.cut.as_deref().unwrap();
    assert!(said.contains(&format!("{} components left out", 400 - cut.components.len())) && said.contains("match"), "{said}");

    let narrowed = answer(&whole, CatalogueArea::Components, Some("Component399"));
    assert_eq!(narrowed.components.len(), 1);
    assert!(narrowed.cut.is_none());
}

#[test]
fn a_checkout_is_read_again_only_when_one_of_its_files_changed() {
    let (_dir, root) = repo(&[("src/tokens.css", ":root { --space-2: 8px; }"), ("README.md", "words")]);
    let first = catalogue_of(&root);
    let again = catalogue_of(&root);
    assert!(Arc::ptr_eq(&first, &again), "nothing changed, so nothing was read");

    std::fs::write(root.join("README.md"), "other words, longer").unwrap();
    assert!(Arc::ptr_eq(&first, &catalogue_of(&root)), "a file the catalogue does not read changes nothing");

    std::fs::write(root.join("src/tokens.css"), ":root { --space-2: 10px; }").unwrap();
    let changed = catalogue_of(&root);
    assert!(!Arc::ptr_eq(&first, &changed));
    assert_eq!(changed.tokens[0].tokens[0].value, "10px");

    fill(&root, &[("src/more.css", ":root { --radius-sm: 3px; }")]);
    assert_eq!(catalogue_of(&root).tokens.len(), 2, "a new sheet is read");
}

#[test]
fn git_decides_what_is_read_so_an_ignored_build_stays_out() {
    let (_dir, root) = repo(&[
        (".gitignore", "generated/\n"),
        ("src/tokens.css", ":root { --space-2: 8px; }"),
        ("generated/out.css", ":root { --space-9: 99px; }"),
        ("node_modules/lib/Button.tsx", "export const B = () => <b className=\"x\" />;"),
    ]);
    let init = std::process::Command::new("git").arg("-C").arg(&root).args(["init", "-q"]).status().unwrap();
    assert!(init.success());
    let listed = inputs::listed(&root);
    assert_eq!(listed.sheets, ["src/tokens.css"]);
    assert!(listed.components.is_empty(), "{:?}", listed.components);
}

#[tokio::test]
async fn a_job_reads_its_own_checkouts_catalogue_and_a_wrong_area_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    fill(Path::new(&rig.root), &[("src/tokens.css", ":root { --space-2: 8px; }")]);
    let asked = |area: &str| CatalogueAsked { area: Some(area.to_string()), ..CatalogueAsked::default() };

    let read = rig.fleet.get_design_catalogue(None, Some(id.clone()), asked("tokens")).await.unwrap();
    assert_eq!(read.area, CatalogueArea::Tokens);
    assert_eq!(read.tokens[0].tokens[0].name, "--space-2");

    let wrong = rig.fleet.get_design_catalogue(None, Some(id), asked("colours")).await;
    let Err(Refusal::Unacceptable(refusal)) = wrong else { panic!("{wrong:?}") };
    assert_eq!(refusal.code, "fleet.catalogue_area_unknown");
    assert!(refusal.message.contains("contents, tokens, components, icons or patterns"), "{}", refusal.message);

    let unplaced = rig.fleet.get_design_catalogue(None, None, CatalogueAsked::default()).await;
    assert!(matches!(unplaced, Err(Refusal::Unacceptable(_))), "{unplaced:?}");
}

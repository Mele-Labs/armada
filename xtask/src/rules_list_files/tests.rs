//! The pure halves: which paths `.gitattributes` declares, and whether a
//! file's text holds entries and nothing else.

use super::*;

#[test]
fn a_union_line_declares_its_path() {
    let text = "# why\npackages/a/index.ts merge=union\n\n*.png binary\n";
    assert_eq!(declared(text), vec!["packages/a/index.ts".to_string()]);
}

#[test]
fn a_line_that_is_not_union_declares_nothing() {
    assert!(declared("docs/x.md merge=ours\n*.png binary\n").is_empty());
}

#[test]
fn exports_and_comments_are_entries() {
    let text = "// header\n\nexport * from \"./a/A\";\nexport * from \"./b/B\";\n";
    assert_eq!(not_entries("index.ts", text), Vec::<usize>::new());
}

#[test]
fn code_among_exports_is_named_by_line() {
    let text = "export * from \"./a/A\";\nconst x = 1;\nexport * from \"./b/B\";\n";
    assert_eq!(not_entries("index.ts", text), vec![2]);
}

#[test]
fn imports_and_block_comments_are_entries_in_a_stylesheet() {
    let text = "/* one\n   two */\n@import \"./a.css\";\n@import \"./b.css\";\n";
    assert_eq!(not_entries("index.css", text), Vec::<usize>::new());
}

#[test]
fn a_rule_in_an_import_list_is_named() {
    let text = "@import \"./a.css\";\n.x { color: red; }\n";
    assert_eq!(not_entries("index.css", text), vec![2]);
}

#[test]
fn tables_are_entries_and_a_root_key_ahead_of_them_is_not() {
    assert!(not_entries("o.toml", "# h\n\n[a.b]\nk = 1\n\n[a.c]\nk = 2\n").is_empty());
    assert_eq!(
        not_entries("o.toml", "version = 1\n[a.b]\nk = 1\n"),
        vec![1]
    );
}

#[test]
fn a_mod_list_holds_mods_and_comments_only() {
    let text = "//! head\nmod a;\n/// doc\npub(crate) mod b;\n";
    assert!(not_entries("mod.rs", text).is_empty());
    assert_eq!(
        not_entries("mod.rs", "mod a;\n#[cfg(test)]\nmod b;\n"),
        vec![2]
    );
    assert_eq!(not_entries("mod.rs", "mod a;\npub use a::A;\n"), vec![2]);
}

#[test]
fn a_kind_with_no_entry_rule_is_refused_whole() {
    assert_eq!(not_entries("x.py", "a = 1\n"), vec![0]);
}

#[test]
fn a_row_list_holds_one_export_a_row_and_nothing_that_builds_one() {
    let text = "// header\nexport * from \"./scenarios/a\";\nexport * from \"./scenarios/b-c\";\n";
    assert!(not_entries("scenario-rows.ts", text).is_empty());
    // An import and a row built in place are what the file was split to be rid of.
    let mixed =
        "import { a } from \"./a\";\nexport * from \"./scenarios/a\";\nholding(\"x\", [a]),\n";
    assert_eq!(not_entries("scenario-rows.ts", mixed), vec![1, 3]);
}

#[test]
fn a_migration_list_holds_entries_after_its_header() {
    let list = "//! head\nuse x;\nconst L: &[M] = &[\n    // note\n    Migration::additive(\"a.b\", c::D),\n];\n";
    assert!(not_entries("migration_list.rs", list).is_empty());
    let stray = "const L: &[M] = &[\n    Migration::additive(\"a.b\", c::D),\n    helper(),\n];\n";
    assert_eq!(not_entries("migration_list.rs", stray), vec![3]);
}

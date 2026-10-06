//! Rule one's own negative tests, each against a tree written here.

use super::*;
use crate::Finding;

const TABLE: &str = "\
| Milestone | The claim | Where |
|---|---|---|
| Board | Every Job is a row | `crates/acceptance/tests/board.rs` |
";

/// Rule one's findings on a tree holding `files`, under a temporary root.
fn findings(name: &str, files: &[(&str, &str)]) -> Vec<String> {
    let root = std::env::temp_dir().join(format!("armada-acceptance-{name}"));
    let _ = fs::remove_dir_all(&root);
    for (rel, text) in files {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        fs::write(&path, text).expect("a file");
    }
    let report = acceptance_test_exists(&root);
    let _ = fs::remove_dir_all(&root);
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

#[test]
fn a_named_test_that_holds_a_test_passes() {
    let found = findings(
        "present",
        &[
            ("crates/acceptance/Cargo.toml", ""),
            (
                "crates/acceptance/tests/board.rs",
                "#[test]\nfn a_row() {}\n",
            ),
            (ACCEPTANCE_DOC, TABLE),
        ],
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_missing_acceptance_test_fails_and_names_its_milestone() {
    let found = findings(
        "missing",
        &[
            ("crates/acceptance/Cargo.toml", ""),
            ("crates/acceptance/tests/other.rs", "#[test]\nfn x() {}\n"),
            (ACCEPTANCE_DOC, TABLE),
        ],
    );
    assert_eq!(
        found,
        ["crates/acceptance/tests/board.rs — Board's acceptance test"]
    );
}

#[test]
fn a_named_file_with_no_test_in_it_fails() {
    let found = findings(
        "empty",
        &[
            ("crates/acceptance/Cargo.toml", ""),
            ("crates/acceptance/tests/board.rs", "// nothing yet\n"),
            (ACCEPTANCE_DOC, TABLE),
        ],
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("holds no `#[test]`"), "{}", found[0]);
}

#[test]
fn a_doc_naming_no_test_fails_rather_than_passing_over_nothing() {
    let found = findings(
        "untabled",
        &[
            ("crates/acceptance/Cargo.toml", ""),
            (
                "crates/acceptance/tests/board.rs",
                "#[test]\nfn a_row() {}\n",
            ),
            (ACCEPTANCE_DOC, "# Acceptance tests\n"),
        ],
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with(ACCEPTANCE_DOC), "{}", found[0]);
}

#[test]
fn the_real_doc_names_every_test_file_in_the_package() {
    let root = crate::repo_root();
    let doc = fs::read_to_string(root.join(ACCEPTANCE_DOC)).expect("the doc");
    let named: Vec<String> = milestone_tests(&doc)
        .into_iter()
        .map(|(_, rel)| rel)
        .collect();
    for rel in files_with_ext(&root, &root.join("crates/acceptance/tests"), &["rs"]) {
        if rel.matches('/').count() == 3 {
            assert!(
                named.contains(&rel),
                "{rel} is not in {ACCEPTANCE_DOC}'s table"
            );
        }
    }
}

#[test]
fn a_lucide_glyph_name_is_not_a_vendor_literal_but_the_same_text_elsewhere_is() {
    let source = "import { FolderGit2, Ban } from \"lucide-react\";\nconst glyph = FolderGit2;\nconst lib = \"git2\";\n";
    let blanked = super::without_lucide_names(&source.to_lowercase(), source);
    assert_eq!(blanked.matches("git2").count(), 1, "{blanked}");
    assert!(blanked.lines().nth(2).unwrap().contains("git2"));
}

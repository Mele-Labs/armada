//! How a citation carrying its tag is read — the half of rule eighteen that
//! needs no repository to check.

use super::*;

/// The shape a retired file is cited in, which the bare rule would have to
/// read as a v1 path.
#[test]
fn a_tag_and_a_path_are_split() {
    assert_eq!(
        a_path_carrying_its_tag("classic-final:apps/desktop/src/main/index.ts"),
        Some(("classic-final", "apps/desktop/src/main/index.ts"))
    );
    assert_eq!(
        a_path_carrying_its_tag("v1-final:docs/traps.md"),
        Some(("v1-final", "docs/traps.md"))
    );
}

/// A line reference names a place in the file, and the file is what has to
/// exist.
#[test]
fn a_line_reference_after_the_path_is_dropped() {
    assert_eq!(
        a_path_carrying_its_tag("classic-final:apps/desktop/src/renderer/src/sessions.tsx:1703"),
        Some(("classic-final", "apps/desktop/src/renderer/src/sessions.tsx"))
    );
    assert_eq!(
        a_path_carrying_its_tag("classic-final:packages/screens/src/session-state.ts:~24-33"),
        Some(("classic-final", "packages/screens/src/session-state.ts"))
    );
}

/// A bare path, a placeholder, a glob and a tag nobody keeps are not
/// citations this rule can hold to a tree.
#[test]
fn a_span_that_is_not_a_tagged_path_is_left_to_the_bare_rule() {
    assert_eq!(a_path_carrying_its_tag("apps/desktop/src/main.ts"), None);
    assert_eq!(a_path_carrying_its_tag("classic-final:<path>"), None);
    assert_eq!(a_path_carrying_its_tag("classic-final:apps/*/src"), None);
    assert_eq!(a_path_carrying_its_tag("classic-final:"), None);
    assert_eq!(a_path_carrying_its_tag("main:apps/desktop/x.ts"), None);
}

/// A directory resolves when a file sits under it, and a longer name that
/// merely begins the same way does not.
#[test]
fn a_directory_resolves_by_the_files_under_it() {
    let tree: std::collections::BTreeSet<String> = ["apps/desktop/src/main/index.ts"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert!(is_in_tree(&tree, "apps/desktop/src/main/index.ts"));
    assert!(is_in_tree(&tree, "apps/desktop/src/main/"));
    assert!(is_in_tree(&tree, "apps/desktop"));
    assert!(!is_in_tree(&tree, "apps/desk"));
    assert!(!is_in_tree(&tree, "apps/desktop/src/main/index"));
}

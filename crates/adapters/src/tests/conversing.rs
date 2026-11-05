//! How a write is recognised in a Job's stream. **Nothing here starts anything**, for `harness`'s
//! reason.

use crate::conversing::wrote_the_checkout;

/// A write is recognised so it can be recorded, and `Bash` is deliberately not
/// one: nothing in the stream says whether a shell line wrote a file.
#[test]
fn the_tools_that_change_a_checkout_are_named_and_bash_is_not_among_them() {
    for tool in ["Edit", "MultiEdit", "Write", "NotebookEdit"] {
        assert!(wrote_the_checkout(tool), "{tool} changes the checkout");
    }
    for tool in ["Bash", "Read", "Grep", "Glob", "WebFetch"] {
        assert!(!wrote_the_checkout(tool), "{tool} is not a write");
    }
}

/// The detail a write call leaves on a row is the path and how much moved, and
/// the event carries the path alone.
#[test]
fn the_file_a_write_named_is_read_back_out_of_its_row() {
    for (detail, path) in [
        (
            "~/armada/crates/api/src/lib.rs +3 -1",
            "~/armada/crates/api/src/lib.rs",
        ),
        ("~/armada/README.md +12", "~/armada/README.md"),
        ("~/armada/README.md", "~/armada/README.md"),
        ("a name with spaces.md +1 -0", "a name with spaces.md"),
        // Not a size: left alone rather than guessed at.
        ("notes -draft.md", "notes -draft.md"),
    ] {
        assert_eq!(crate::conversing::path_written(detail), path);
    }
}

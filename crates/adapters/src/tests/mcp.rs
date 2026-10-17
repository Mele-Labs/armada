//! The one entry Fleet adds to a repository's own agent configuration without
//! disturbing the rest of the file.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{publish_the_agents_door, Published};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "armada-adapters-mcp-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// The other document, and the property that makes it safe to write into a
/// file somebody else owns: the entry appears and nothing else moves.
#[test]
fn the_agents_door_is_added_and_everything_else_is_kept() {
    let at = scratch_path();
    std::fs::write(
        &at,
        r#"{"mcpServers":{"theirs":{"type":"stdio","command":"their-server"}},"other":[1,2]}"#,
    )
    .expect("a configuration somebody already wrote");

    let published = publish_the_agents_door(&at, "armada", &["mcp"]).expect("the entry written");
    let written = std::fs::read_to_string(&at).expect("the file read back");

    assert_eq!(published, Published::Written);
    assert!(written.contains(r#""armada-fleet""#), "{written}");
    assert!(written.contains(r#""command": "armada""#), "{written}");
    assert!(written.contains(r#""type": "stdio""#), "{written}");
    assert!(written.contains(r#""their-server""#), "{written}");
    assert!(
        written.contains(r#""other""#),
        "a key this crate never heard of survives"
    );

    let _ = std::fs::remove_file(&at);
}

/// **A Fleet restart does not dirty a working tree.** The entry carries no
/// port, so the second boot has nothing to change and changes nothing.
#[test]
fn publishing_twice_writes_once() {
    let at = scratch_path();

    assert_eq!(
        publish_the_agents_door(&at, "armada", &["mcp"]).expect("the file created"),
        Published::Written
    );
    let first = std::fs::read_to_string(&at).expect("the file read back");
    assert_eq!(
        publish_the_agents_door(&at, "armada", &["mcp"]).expect("the file left alone"),
        Published::AlreadyThere
    );

    assert_eq!(
        first,
        std::fs::read_to_string(&at).expect("the file read back again")
    );
    let _ = std::fs::remove_file(&at);
}

/// A file that will not parse is a file somebody is editing, or one written by
/// something else. Either way Armada does not get to replace it.
#[test]
fn a_file_that_will_not_parse_is_left_alone() {
    let at = scratch_path();
    std::fs::write(&at, "{ not json at all").expect("a file");

    let refused = publish_the_agents_door(&at, "armada", &["mcp"]).expect_err("refused");

    assert!(refused.to_string().contains("was left alone"), "{refused}");
    assert_eq!(
        std::fs::read_to_string(&at).expect("the file read back"),
        "{ not json at all"
    );
    let _ = std::fs::remove_file(&at);
}

use crate::{consulted, ConsultedKind::{Doc, Ticket}};

fn kind(tool: &str, target: &str) -> Option<crate::ConsultedKind> {
    consulted(tool, target).map(|read| read.kind)
}

#[test]
fn a_web_page_is_a_doc_and_an_issue_page_is_a_ticket() {
    assert_eq!(kind("WebFetch", "https://example.com/spec"), Some(Doc));
    assert_eq!(kind("WebFetch", "https://tracker.example.com/org/repo/issues/12"), Some(Ticket));
    assert_eq!(kind("WebFetch", "not an address"), None);
}

#[test]
fn a_server_that_reads_gives_a_doc_or_a_ticket_and_one_that_writes_gives_nothing() {
    assert_eq!(kind("mcp__plugin_productivity_notion__notion-fetch", "abc"), Some(Doc));
    assert_eq!(kind("mcp__plugin_productivity_linear__get_issue", "ENG-1"), Some(Ticket));
    assert_eq!(kind("mcp__atlassian__getJiraIssue", "KEY-9"), Some(Ticket));
    assert_eq!(kind("mcp__github__get_issue", "o/r#4"), Some(Ticket));
    assert_eq!(kind("mcp__github__get_file_contents", "o/r"), None);
    assert_eq!(kind("mcp__plugin_productivity_notion__notion-update-page", "abc"), None);
    assert_eq!(kind("mcp__armada-fleet__get_feature_page", "x"), None);
    assert_eq!(kind("mcp__gitnexus__context", "x"), None);
}

#[test]
fn a_file_is_a_doc_under_docs_or_as_markdown_outside_source_folders() {
    assert_eq!(kind("Read", "docs/concepts/feature-catalog.md"), Some(Doc));
    assert_eq!(kind("Read", "docs/mocks/start.html"), Some(Doc));
    assert_eq!(kind("Read", "README.md"), Some(Doc));
    assert_eq!(kind("Read", "crates/fleet/README.md"), None);
    assert_eq!(kind("Read", "crates/fleet/src/lib.rs"), None);
    assert_eq!(kind("Read", "CLAUDE.md"), None);
    assert_eq!(kind("Read", "/etc/notes.md"), None);
}

#[test]
fn a_shell_ticket_is_a_ticket() {
    assert_eq!(kind("Bash", "armada-a3f2"), Some(Ticket));
    assert_eq!(kind("Edit", "x"), None);
}

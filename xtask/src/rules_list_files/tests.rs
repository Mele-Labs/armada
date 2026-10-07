//! The pure half: which tables a file's text holds.

use super::*;

#[test]
fn a_table_and_its_usage_rows_are_one_key() {
    let text = "[icons.link]\n  means = \"x\"\n\n[[icons.link.usage]]\n  means = \"y\"\n";
    assert_eq!(tables("icons", text), vec!["link".to_string()]);
}

#[test]
fn a_dotted_key_keeps_its_quotes_off() {
    let text = "# why\n[operations.\"job.created\"]\nkind = \"event\"\n";
    assert_eq!(tables("operations", text), vec!["job.created".to_string()]);
}

#[test]
fn two_tables_are_two_keys() {
    let text = "[operations.a]\n\n[operations.b]\n";
    assert_eq!(tables("operations", text).len(), 2);
}

#[test]
fn a_comment_is_no_table() {
    assert!(tables("icons", "# [icons.link]\n").is_empty());
}

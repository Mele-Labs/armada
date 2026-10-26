//! Which issues a text links, read by the tracker itself: Jira's browse links on its own site, any
//! project's key, and the forge's links in a checkout whose origin is the forge's.

use adapter_traits::Tracker;

use super::jira::{block, settings, Scripted};
use crate::jira::JiraSettings;
use crate::jira_tracker::JiraTracker;

fn linked(text: &str) -> Vec<String> {
    let run = Scripted::saying(&[]);
    JiraTracker::with_runner(settings(), &run).issues_linked_in("/repo", text)
}

fn named(pairs: &[(&str, &str)], title: &str, branch: &str) -> Vec<String> {
    let run = Scripted::saying(&[]);
    let settings = JiraSettings::read(&block(pairs)).expect("a block");
    JiraTracker::with_runner(settings, &run).issues_named_by_pull_request(title, branch)
}

#[test]
fn a_pull_request_names_its_own_projects_keys_in_its_title_and_branch() {
    let acme = [("site", "acme"), ("project", "ABC")];
    assert_eq!(named(&acme, "[ABC-12] Wire it", "abc-31-wire-it"), ["ABC-12", "ABC-31"]);
    assert_eq!(named(&acme, "ABC-12, ABC-7 and ABC-12", "ABC-7_abc-9"), ["ABC-12", "ABC-7", "ABC-9"]);
    assert!(named(&acme, "Read UTF-8 and XYZ-4 and abc-5", "xyz-4-utf-8").is_empty());
    assert!(named(&acme, "XABC-1 ABC-2a ABC-", "é-abc-x").is_empty());
    assert_eq!(named(&acme, "Café ABC-3", "über/abc-4"), ["ABC-3", "ABC-4"]);
    assert!(named(&[("site", "acme"), ("jql", "labels = web")], "[ABC-12] Wire it", "abc-31").is_empty());
}

#[test]
fn a_browse_link_on_the_sites_host_names_its_key_in_any_project() {
    assert_eq!(
        linked("Fix https://acme.atlassian.net/browse/ABC-12 and acme.atlassian.net/browse/OPS-7/comments"),
        ["ABC-12", "OPS-7"]
    );
}

#[test]
fn a_key_is_named_once_and_a_sentence_end_or_markdown_link_takes_no_punctuation_with_it() {
    assert_eq!(
        linked("See [the ticket](https://acme.atlassian.net/browse/ABC-12) and again https://acme.atlassian.net/browse/ABC-12."),
        ["ABC-12"]
    );
}

#[test]
fn another_host_a_bare_key_and_a_forge_link_name_nothing_on_jira() {
    assert!(linked("https://other.atlassian.net/browse/ABC-12 ABC-13 https://forge.invalid/o/r/issues/5").is_empty());
}

#[test]
fn the_forge_names_the_numbers_of_its_own_repositorys_issues_a_text_links() {
    let dir = std::env::temp_dir().join(format!("armada-links-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory");
    for args in [&["init", "-q"][..], &["remote", "add", "origin", "https://forge.invalid/o/r.git"]] {
        let ran = std::process::Command::new("git").args(args).current_dir(&dir).output().expect("git");
        assert!(ran.status.success(), "{args:?}");
    }
    let root = dir.to_string_lossy().to_string();
    let words = "https://forge.invalid/o/r/issues/9 https://forge.invalid/other/r/issues/6 #7 https://forge.invalid/o/r/issues/2";
    assert_eq!(crate::IssueTracker.issues_linked_in(&root, words), ["2", "9"]);
    assert!(crate::IssueTracker.issues_linked_in("/nowhere/at/all", words).is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

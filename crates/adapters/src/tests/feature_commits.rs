//! The features a commit names are read from its trailer lines, against real git.

use super::repo::TempRepo;
use crate::feature_commits::{commits_naming, head, trailer_slugs};

#[test]
fn trailer_lines_in_the_last_paragraph_name_one_or_more_features() {
    let message = "Add a vote kind\n\nWhy it exists.\n\nFeature: voting-on-takes\nFeature: page-actions, voting-on-takes\nCo-Authored-By: X <x@example.com>\n";
    assert_eq!(trailer_slugs(message), ["voting-on-takes", "page-actions"]);
}

#[test]
fn a_feature_line_in_the_body_is_prose() {
    let message = "Explain trailers\n\nFeature: voting-on-takes is how a commit names one.\n\nCo-Authored-By: X <x@example.com>\n";
    assert!(trailer_slugs(message).is_empty());
    assert!(trailer_slugs("Feature: Not A Slug").is_empty(), "a slug is lowercase words joined by dashes");
    assert_eq!(trailer_slugs("subject only\n\nfeature: a-b"), ["a-b"], "the key is read in any case");
}

#[test]
fn commits_since_the_base_that_name_a_feature_come_back_with_their_files() {
    let repo = TempRepo::with_a_commit();
    repo.git(&["branch", "-f", "base"]);
    repo.commit_one("a/one.rs", "1\n", "Touch one\n\nFeature: alpha");
    repo.commit_one("b/two.rs", "2\n", "Touch two with no trailer");
    repo.commit_one("c/three.rs", "3\n", "Touch three\n\nFeature: beta\nFeature: alpha");
    let found = commits_naming(repo.root(), "base").expect("git answers");
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].features, ["alpha"]);
    assert_eq!(found[0].files, ["a/one.rs"]);
    assert_eq!(found[1].features, ["beta", "alpha"]);
    assert_eq!(found[1].files, ["c/three.rs"]);
    assert_eq!(head(repo.root()).as_deref(), Some(found[1].sha.as_str()));
}

#[test]
fn a_base_git_does_not_know_is_not_checked() {
    let repo = TempRepo::with_a_commit();
    assert!(commits_naming(repo.root(), "nowhere").is_err());
}

#[test]
fn a_feature_line_in_a_shell_command_or_a_brief_names_features_at_a_line_start_or_after_a_quote() {
    use crate::feature_commits::{backticked_slugs_in, feature_lines_in};
    assert_eq!(feature_lines_in(r#"git commit -m "Add votes" -m "Feature: voting-on-takes, page-actions""#), ["voting-on-takes", "page-actions"]);
    assert_eq!(feature_lines_in("git commit -m \"$(cat <<'EOF'\nAdd votes\n\nFeature: voting-on-takes\nCo-Authored-By: X\nEOF\n)\""), ["voting-on-takes"]);
    assert_eq!(feature_lines_in("Build it.\n  feature: a-b\nFeature: a-b, c"), ["a-b", "c"], "any case, any indent, each once");
    assert!(feature_lines_in("Add the Feature: line to a commit").is_empty(), "a sentence is prose");
    assert!(feature_lines_in("git commit -m \"Feature: Not A Slug\"").is_empty());
    assert_eq!(backticked_slugs_in("Use `voting-on-takes` and `cargo test`, then `page-actions`."), ["voting-on-takes", "page-actions"]);
}

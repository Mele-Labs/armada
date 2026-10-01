//! The rule against a library built here. The fixture is the pair that
//! collided: `ReviewDecision.css` and `MemberDecision.css`, both declaring
//! `.armada-decision`, before the first was renamed.

use super::*;
use crate::Finding;

/// A library of compositions under a temporary root.
struct Library {
    root: std::path::PathBuf,
}

impl Library {
    fn new(name: &str) -> Library {
        let root = std::env::temp_dir().join(format!("armada-claims-{name}"));
        let _ = fs::remove_dir_all(&root);
        Library { root }
    }

    /// One stylesheet, at a path relative to `compositions/`.
    fn sheet(self, rel: &str, text: &str) -> Library {
        let path = self.root.join(ROOT).join(COMPOSITIONS).join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        fs::write(&path, text).expect("a stylesheet");
        self
    }

    fn run(self) -> Vec<String> {
        let report = no_two_compositions_claim_one_class(&self.root);
        let _ = fs::remove_dir_all(&self.root);
        report
            .findings
            .iter()
            .map(|f| match f {
                Finding::Fail(what) | Finding::Warn(what) => what.clone(),
            })
            .collect()
    }
}

const REVIEW: &str = "ReviewDecision/ReviewDecision.css";
const MEMBER: &str = "JobMembers/MemberDecision.css";

#[test]
fn two_compositions_declaring_one_class_are_refused_naming_both_files() {
    let lines = Library::new("decision")
        .sheet(
            REVIEW,
            "/* flush with the record */\n\n.armada-decision {\n  padding: 0;\n}\n",
        )
        .sheet(
            MEMBER,
            "\n\n\n.armada-decision {\n  padding: var(--pad-card);\n}\n",
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("`.armada-decision` is declared by 2"),
        "{lines:?}"
    );
    assert!(
        lines[0].contains(&format!("{ROOT}/{COMPOSITIONS}/{MEMBER}:4")),
        "{lines:?}"
    );
    assert!(
        lines[0].contains(&format!("{ROOT}/{COMPOSITIONS}/{REVIEW}:3")),
        "{lines:?}"
    );
}

/// The rename that fixed it: each composition its own name.
#[test]
fn each_composition_with_its_own_name_reports_nothing() {
    let lines = Library::new("renamed")
        .sheet(REVIEW, ".armada-review-decision { padding: 0; }\n")
        .sheet(MEMBER, ".armada-decision { padding: var(--pad-card); }\n")
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

/// A pseudo-class or an attribute on the class is the same claim.
#[test]
fn a_state_of_the_class_is_the_same_claim() {
    let lines = Library::new("state")
        .sheet(
            REVIEW,
            ".armada-decision[data-open]:hover { padding: 0; }\n",
        )
        .sheet(MEMBER, ".armada-decision { padding: var(--pad-card); }\n")
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
}

/// What a composition may do: repeat its own class across its own files, and
/// style another's class inside or alongside its own name.
#[test]
fn a_repeat_within_one_composition_and_a_scoped_rule_claim_nothing() {
    let lines = Library::new("scoped")
        .sheet(MEMBER, ".armada-decision { padding: 0; }\n")
        .sheet(
            "JobMembers/JobMembers.css",
            ".armada-decision:hover { color: red; }\n",
        )
        .sheet(
            REVIEW,
            concat!(
                ".armada-review .armada-decision,\n",
                ".armada-review > .armada-decision { padding: 0; }\n",
                ".armada-decision.armada-review-decision { margin: 0; }\n",
                ".armada-review:not(.armada-decision) { margin: 0; }\n",
                "@keyframes pulse { 12.5% { opacity: 0; } }\n",
                "/* .armada-decision { } */\n",
            ),
        )
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

/// One selector list can claim in one place and be scoped in another, and a
/// claim inside `@media` is still a claim.
#[test]
fn a_claim_in_a_list_or_a_media_block_counts() {
    let lines = Library::new("media")
        .sheet(MEMBER, ".armada-decision { padding: 0; }\n")
        .sheet(
            REVIEW,
            "@media (width < 600px) {\n  .armada-review .x,\n  .armada-decision { padding: 0; }\n}\n",
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(&format!("{REVIEW}:2")), "{lines:?}");
}

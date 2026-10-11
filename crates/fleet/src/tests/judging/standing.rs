//! What the repository requires of every change, read from the file its
//! Manifest names and into the brief the Judge is handed.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model};
use config::Manifest;
use testkit::{FakeJudge, FakeWorkProduct};
use verification::{Lifted, Request, STANDING_RULES};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::rule_on;
use crate::judging::{standing, JudgeBudget, Judging, Marking};
use crate::policy::Policies;
use crate::tests::gate::{budget, diff_evidence, worktree};
use crate::tests::judging::judged_workflow;
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;

const HEADING: &str = "What this repository requires of every change";
const A_RULE: &str = "Prose the change makes wrong is fixed in the same change.";

fn manifest(standing_rules: Option<&str>) -> Manifest {
    let key = standing_rules
        .map(|path| format!("standing_rules: {path}\n"))
        .unwrap_or_default();
    Manifest::parse(
        Path::new("armada.yml"),
        &format!("version: 1\nid: a\n{key}"),
    )
    .expect("a manifest")
}

/// The one question the Judge was handed, for a repository whose checkout is
/// `root` and whose Manifest is `manifest`.
async fn handed(manifest: &Manifest, root: &Path) -> String {
    let workflow = judged_workflow();
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = Judging {
        client: Arc::clone(&judge) as Arc<_>,
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        second_opinion_model: Model::named("the-second-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: Marking::detached(),
        asked: Asked::nowhere(),
        standing: standing(
            manifest,
            &root.to_string_lossy(),
            verification::STANDING_RULES,
        ),
        reading: None,
        wording: verification::Wording::shipped(),
    };
    rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        &[],
        crate::gate::Began::At(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &crate::places::Room::ignoring_the_machine(crate::places::ChecksAtOnce::of(4)),
        &judging,
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &std::collections::BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        None,
    )
    .await;
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion is one call");
    asked[0].clone()
}

fn checkout_holding(path: &str, text: &str) -> TempDir {
    let root = TempDir::new();
    let file = root.path().join(path);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
    std::fs::write(file, text).expect("the rules file");
    root
}

#[tokio::test]
async fn a_manifest_naming_a_rules_file_puts_it_in_its_own_section_before_the_question() {
    let root = checkout_holding("docs/every-change.md", &format!("{A_RULE}\n"));
    let question = handed(&manifest(Some("docs/every-change.md")), root.path()).await;

    let section = question.find(HEADING).expect("the section is in the brief");
    let rule = question.find(A_RULE).expect("the file's text is in it");
    let request = question.find("What was asked for").expect("the request");
    let asked = question
        .find("The question, which is yes or no")
        .expect("the question");
    assert!(
        request < section && section < rule && rule < asked,
        "{question}"
    );
}

/// Today's brief, byte for byte: the only difference a named file makes is the
/// section it adds.
#[tokio::test]
async fn a_manifest_naming_none_gives_the_brief_it_always_did() {
    let root = checkout_holding("docs/every-change.md", &format!("{A_RULE}\n"));
    let without = handed(&manifest(None), root.path()).await;
    let with = handed(&manifest(Some("docs/every-change.md")), root.path()).await;

    assert!(!without.contains(HEADING), "{without}");
    let from = with.find(HEADING).expect("the section");
    let to = from + with[from..].find("\n\n  ").expect("its body") + 2;
    let to = to + with[to..].find("\n\n").expect("its end") + 2;
    assert_eq!(
        format!("{}{}", &with[..from], &with[to..]),
        without,
        "the section is the whole of the difference"
    );
}

#[tokio::test]
async fn a_rules_file_over_the_bound_is_cut_and_the_brief_says_so() {
    let line = "Every change carries the tests that prove it.\n";
    let long = line.repeat(STANDING_RULES / line.len() + 20);
    let root = checkout_holding("RULES.md", &format!("{long}THE LAST LINE\n"));
    let question = handed(&manifest(Some("RULES.md")), root.path()).await;

    assert!(question.contains(HEADING), "{question}");
    assert!(
        !question.contains("THE LAST LINE"),
        "what is past the bound is not shown"
    );
    assert!(
        question.contains(&format!(
            "RULES.md is {} bytes and a brief carries the first {STANDING_RULES}",
            long.len() + "THE LAST LINE\n".len()
        )),
        "{question}"
    );
    let carried = question.matches(line.trim_end()).count() * line.len();
    assert!(
        carried <= STANDING_RULES,
        "{carried} bytes of the file were carried"
    );
}

#[tokio::test]
async fn a_named_file_that_is_not_there_is_said_rather_than_dropped() {
    let root = TempDir::new();
    let question = handed(&manifest(Some("docs/gone.md")), root.path()).await;
    assert!(
        question.contains("This repository names docs/gone.md as what it requires of every change, and it could not be read"),
        "{question}"
    );
}

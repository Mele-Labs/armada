//! What a repository requires of every change: in every Judge brief where it
//! names one, before the question, and nowhere where it does not.

use adapter_traits::Patch;
use core_model::{GamingPattern, RepoPath, WriteTargets};

use crate::{ConvergenceBrief, GamingBrief, Request, Standing, WideningBrief, STANDING_RULES};

const A_RULE: &str = "Prose the change makes wrong is fixed in the same change.";

fn stated() -> Standing {
    Standing::read("docs/every-change.md", &format!("{A_RULE}\n"))
}

/// The section `stated` adds, exactly.
fn section() -> String {
    stated().told()
}

/// Every brief that is not `Brief::about`'s, which Fleet's own suite reads
/// through the gate. The second reading is built from the gaming brief, so it
/// carries whatever that one does.
fn every_other_brief(standing: &Standing) -> [String; 4] {
    let workflow = crate::tests::workflow();
    let step = crate::tests::gated(&workflow);
    let patch = Patch::of(String::from("+    let n = n - 1;\n"));
    let gaming = GamingBrief::about(
        step,
        GamingPattern::AssertionWeakened,
        &patch,
        None,
        standing,
    )
    .expect("a judged pattern has a question");
    let second = crate::SecondOpinion::about(
        &gaming,
        core_model::GamingFlag {
            pattern: GamingPattern::AssertionWeakened,
            cited: String::from("let n = n - 1"),
            at: None,
            asked: None,
            brief_path: None,
            cleared: None,
        },
    );
    let converging = ConvergenceBrief::about(step, &patch, None, &[], None, &[], standing);
    let widening = WideningBrief::about(
        step,
        Request::of(testkit::asked_for()),
        standing,
        &WriteTargets::of(vec![RepoPath::new("src")]),
        &[RepoPath::new("docs/guide.md")],
        "the guide names the function this renames",
    );
    [
        gaming.question().to_string(),
        second.question().to_string(),
        converging.question().to_string(),
        widening.question().to_string(),
    ]
}

#[test]
fn every_brief_carries_the_section_and_nothing_else_changes() {
    let without = every_other_brief(&Standing::unstated());
    let with = every_other_brief(&stated());
    for (without, with) in without.iter().zip(with.iter()) {
        assert!(!without.contains(A_RULE), "{without}");
        assert!(with.contains(A_RULE), "{with}");
        assert!(
            with.find(A_RULE) < with.rfind("question"),
            "before the question: {with}"
        );
        assert_eq!(&with.replacen(&section(), "", 1), without);
    }
}

#[test]
fn a_file_with_nothing_in_it_says_nothing() {
    assert_eq!(Standing::read("RULES.md", "\n  \n"), Standing::unstated());
    assert_eq!(Standing::unstated().told(), "");
}

/// A cut that fell inside a character would panic the assembler; this one
/// falls on a line instead, and no line is half-shown.
#[test]
fn a_cut_lands_on_a_whole_line_and_never_inside_a_character() {
    let line = "é".repeat(30) + "\n";
    let text = line.repeat(STANDING_RULES / line.len() + 5);
    let told = Standing::read("RULES.md", &text).told();
    assert!(told.contains("(Cut here. RULES.md is"), "{told}");
    for shown in told.lines().filter(|shown| shown.contains('é')) {
        assert_eq!(shown.trim(), line.trim_end(), "a whole line");
    }
}

//! The rule, held against the repository it is checked into, and the two claims about the emitted
//! vocabulary that no registry comparison can see.

use super::*;

/// The rule runs against the repository it is checked into, which is the only
/// place the real outputs exist. A green here says the checked-in files
/// are what the registries say today.
#[test]
fn the_repository_agrees_with_its_own_registries() {
    let report = the_generated_vocabulary_says_what_the_registries_say(&crate::repo_root());
    let complaints: Vec<&String> = report
        .findings
        .iter()
        .map(|f| match f {
            crate::Finding::Fail(what) | crate::Finding::Warn(what) => what,
        })
        .collect();
    assert!(complaints.is_empty(), "{complaints:#?}");
}

/// The one blank the vocabulary sanctions, spelled as the generated module
/// spells its rows.
///
/// `escalated` renders whichever escalation reason is set and never its own
/// name, because nobody says a Job escalated at step 3. It is the only variant
/// on any rendered vocabulary that is meant to reach a surface with no word.
const RENDERS_ITS_REASON: &str = "(\"escalated\"";

/// **No variant a surface draws reaches it as a wire spelling.**
///
/// The design contract says this map is one artifact with a test asserting
/// every variant has copy, so a new reason cannot ship without any. Until now
/// the generator only *counted* a missing verb: eight vocabularies had rows
/// that said nothing and two had no rows at all, and each was found by a person
/// reading `check_config_edited` or `nothing_writing` off a screen in mono.
/// Issue #110.
///
/// It reads the emitted module rather than the registry because the emitted
/// module is the set a surface can actually reach — a vocabulary nothing wants
/// has no rows here and is not this test's business, and the rule above already
/// holds the two in agreement.
#[test]
fn every_variant_a_surface_renders_has_a_word() {
    let module = crate::repo_root().join("crates/bridge-native/src/generated/vocabulary.rs");
    let text = std::fs::read_to_string(&module).expect("the generated vocabulary");

    let wordless: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("verb: None"))
        .filter(|line| !line.trim_start().starts_with(RENDERS_ITS_REASON))
        .collect();

    assert!(
        wordless.is_empty(),
        "these render as their wire spelling at a person, and each needs a `verb` in \
         crates/core-model/domain/enum-verbs.toml:\n{wordless:#?}"
    );
}

/// A vocabulary the generator wants and the registry has no rows for emits an
/// empty map, which the test above cannot see — there is no variant to key a
/// `verb: None` on. This is the other half of the same claim.
#[test]
fn no_vocabulary_a_surface_renders_is_empty() {
    let module = crate::repo_root().join("crates/bridge-native/src/generated/vocabulary.rs");
    let text = std::fs::read_to_string(&module).expect("the generated vocabulary");

    let lines: Vec<&str> = text.lines().collect();
    let empty: Vec<&str> = lines
        .windows(2)
        .filter(|pair| pair[0].ends_with("Rendering)] = &[") && pair[1] == "];")
        .map(|pair| pair[0])
        .collect();

    assert!(
        empty.is_empty(),
        "these have no rows in crates/core-model/domain/enum-verbs.toml at all, so every \
         variant of them renders as its wire spelling:\n{empty:#?}"
    );
}

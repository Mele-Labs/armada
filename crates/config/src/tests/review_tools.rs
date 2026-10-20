//! A review tool's file rules, and which place answers for an id.

use std::path::PathBuf;

use crate::review_tools::{carried, file_of, fit, id_of, resolve, Def, ToolKind, ToolSource, Written};

fn file(kind: &str, name: &str, prompt: &str) -> String {
    format!("---\nkind: {kind}\nname: {name}\n---\n{prompt}\n")
}

fn kit(name: &str, text: String) -> Written {
    Written::in_kit(PathBuf::from(format!("/kit/{name}.md")), text)
}

fn repository(name: &str, text: String) -> Written {
    Written::in_repository(PathBuf::from(format!("/repo/.armada/review-tools/{name}.md")), text)
}

fn found<'a>(defs: &'a [Def], id: &str) -> &'a Def {
    defs.iter().find(|one| one.id == id).unwrap_or_else(|| panic!("no definition of {id} in {defs:?}"))
}

#[test]
fn armada_carries_four_tools_and_four_explanations_that_all_fit() {
    let resolved = resolve(carried());
    assert!(resolved.left_out.is_empty(), "{:?}", resolved.left_out);
    let ids: Vec<&str> = resolved.defs.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(
        ids,
        ["risks", "possible-bugs", "api-breaking-changes", "how-it-works", "user-impact", "how-to-test", "why", "walk"]
    );
    assert_eq!(found(&resolved.defs, "walk").kind, ToolKind::Tool, "a tile in Bridge's tools");
    assert_eq!(found(&resolved.defs, "possible-bugs").name, "Possible bugs");
    assert_eq!(found(&resolved.defs, "why").kind, ToolKind::Explanation);
    assert!(resolved.defs.iter().all(|one| one.source == ToolSource::Armada && one.replaces.is_none()));
}

#[test]
fn kit_answers_over_the_repository_and_the_repository_over_armada() {
    let mut all = carried();
    all.push(kit("risks", file("tool", "Risks", "Kit's risks.")));
    all.push(kit("mine", file("tool", "Mine", "Only Kit has it.")));
    all.push(repository("risks", file("tool", "Risks", "The repository's risks.")));
    all.push(repository("possible-bugs", file("tool", "Possible bugs", "The repository's bugs.")));
    let resolved = resolve(all);
    let risks = found(&resolved.defs, "risks");
    assert_eq!((risks.source, risks.replaces), (ToolSource::Kit, Some(ToolSource::Repository)));
    assert_eq!(risks.prompt, "Kit's risks.");
    assert_eq!(risks.hides, Some(ToolSource::Repository));
    let bugs = found(&resolved.defs, "possible-bugs");
    assert_eq!((bugs.source, bugs.replaces), (ToolSource::Repository, Some(ToolSource::Armada)));
    assert_eq!(bugs.hides, None);
    let mine = found(&resolved.defs, "mine");
    assert_eq!((mine.source, mine.replaces, mine.hides), (ToolSource::Kit, None, None));
    assert_eq!(resolved.defs[0].id, "risks", "a replacement keeps the position of the one it replaced");
    assert_eq!(resolved.defs.last().map(|one| one.id.as_str()), Some("mine"), "a name nobody had is added at the end");
    assert!(resolved.left_out.is_empty());
}

#[test]
fn kits_copy_of_a_repository_tool_hides_nothing_and_kit_over_armada_hides_nothing() {
    let mut all = carried();
    all.push(kit("risks", file("tool", "Risks", "The same words.")));
    all.push(repository("risks", file("tool", "Risks", "The same words.")));
    all.push(kit("why", file("explanation", "Why", "Kit's why.")));
    let resolved = resolve(all);
    let risks = found(&resolved.defs, "risks");
    assert_eq!((risks.source, risks.replaces, risks.hides), (ToolSource::Kit, Some(ToolSource::Repository), None));
    let why = found(&resolved.defs, "why");
    assert_eq!((why.source, why.replaces, why.hides), (ToolSource::Kit, Some(ToolSource::Armada), None));
}

#[test]
fn a_name_in_other_case_is_the_same_definition() {
    let mut all = carried();
    all.push(kit("api", file("tool", "api  BREAKING changes", "Kit's.")));
    let resolved = resolve(all);
    assert_eq!(found(&resolved.defs, "api-breaking-changes").source, ToolSource::Kit);
    assert_eq!(id_of("  How to Test! "), "how-to-test");
}

#[test]
fn a_file_that_does_not_fit_is_left_out_and_named_and_the_next_place_answers() {
    let mut all = carried();
    all.push(kit("risks", "no front matter at all".to_string()));
    all.push(repository("bugs", file("tool", "Possible bugs", "")));
    all.push(repository("odd", file("rule", "Odd", "x")));
    all.push(repository("loose", "---\nkind: tool\nname: Loose\nflavour: x\n---\nbody\n".to_string()));
    all.push(repository("unclosed", "---\nkind: tool\nname: Unclosed\nbody\n".to_string()));
    let resolved = resolve(all);
    assert_eq!(found(&resolved.defs, "risks").source, ToolSource::Armada);
    assert_eq!(found(&resolved.defs, "possible-bugs").source, ToolSource::Armada);
    let said: Vec<String> = resolved.left_out.iter().map(|one| one.said.clone()).collect();
    assert_eq!(said.len(), 5, "{said:?}");
    assert!(said.iter().any(|one| one.contains("/kit/risks.md") && one.contains("`---`")), "{said:?}");
    assert!(said.iter().any(|one| one.contains("/bugs.md") && one.contains("no prompt")), "{said:?}");
    assert!(said.iter().any(|one| one.contains("/odd.md") && one.contains("kind")), "{said:?}");
    assert!(said.iter().any(|one| one.contains("/loose.md") && one.contains("flavour")), "{said:?}");
    assert!(said.iter().any(|one| one.contains("/unclosed.md") && one.contains("closing")), "{said:?}");
    assert!(resolved.left_out.iter().all(|one| one.source != ToolSource::Armada));
}

#[test]
fn two_files_of_one_id_in_one_place_are_both_left_out() {
    let mut all = carried();
    all.push(kit("a", file("tool", "Twice", "one")));
    all.push(kit("b", file("tool", "twice", "two")));
    let resolved = resolve(all);
    assert!(resolved.get("twice").is_none());
    assert_eq!(resolved.left_out.len(), 2);
    assert!(resolved.left_out.iter().all(|one| one.said.contains("/kit/a.md") && one.said.contains("/kit/b.md")));
}

#[test]
fn a_save_is_checked_before_it_is_written() {
    assert_eq!(fit(ToolKind::Tool, "Money: leaks", "Look at refunds."), Ok("money-leaks".to_string()));
    assert!(fit(ToolKind::Tool, "!!!", "x").is_err());
    assert!(fit(ToolKind::Tool, "Name", "   ").is_err());
}

#[test]
fn a_name_with_a_colon_or_a_quote_survives_the_round_trip() {
    let text = file_of(ToolKind::Explanation, "Why: \"now\"", "Because.");
    let resolved = resolve([kit("w", text)]);
    let one = &resolved.defs[0];
    assert_eq!((one.name.as_str(), one.kind, one.prompt.as_str()), ("Why: \"now\"", ToolKind::Explanation, "Because."));
}

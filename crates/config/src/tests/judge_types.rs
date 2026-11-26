//! A judge type's file rules, and which place answers for an id.

use std::path::PathBuf;

use crate::judge_types::{carried, file_of, fit, resolve, Def, Place, Written, ADVERSARIAL};

fn file(name: &str, prompt: &str) -> String {
    format!("---\nname: {name}\n---\n{prompt}\n")
}

fn project(name: &str, text: String) -> Written {
    Written::in_kit_project(PathBuf::from(format!("/kit/judges/projects/storefront/{name}.md")), text)
}

fn kit(name: &str, text: String) -> Written {
    Written::in_kit(PathBuf::from(format!("/kit/judges/{name}.md")), text)
}

fn repository(name: &str, text: String) -> Written {
    Written::in_repository(PathBuf::from(format!("/repo/.armada/judges/{name}.md")), text)
}

fn found<'a>(defs: &'a [Def], id: &str) -> &'a Def {
    defs.iter().find(|one| one.id == id).unwrap_or_else(|| panic!("no type {id} in {defs:?}"))
}

#[test]
fn armada_carries_five_types_that_all_fit_and_adversarial_asks_what_the_empty_prompt_did() {
    let resolved = resolve(carried());
    assert!(resolved.left_out.is_empty(), "{:?}", resolved.left_out);
    let ids: Vec<&str> = resolved.defs.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["adversarial", "code", "design", "system-design", "api-review"]);
    assert_eq!(found(&resolved.defs, "system-design").name, "System design");
    assert_eq!(found(&resolved.defs, "api-review").name, "API review");
    assert!(resolved.defs.iter().all(|one| one.place == Place::Armada && one.replaces.is_none()));
    let adversarial = resolved.get(ADVERSARIAL).expect("the default is carried");
    assert!(adversarial.prompt.starts_with("Find what the person or the agent missed in this work"), "{}", adversarial.prompt);
}

#[test]
fn the_project_answers_over_all_of_kit_over_the_repository_over_armada() {
    let mut people = vec![
        repository("code", file("Code", "The repository's code.")),
        kit("code", file("Code", "Kit's code.")),
        project("code", file("Code", "The project's code.")),
        repository("design", file("Design", "The repository's design.")),
        kit("design", file("Design", "Kit's design.")),
        repository("api-review", file("API review", "The repository's API review.")),
        kit("mine", file("Mine", "Only Kit has it.")),
    ];
    // Arrival order across places decides nothing.
    people.reverse();
    let mut all = carried();
    all.extend(people);
    let resolved = resolve(all);
    let code = found(&resolved.defs, "code");
    assert_eq!((code.place, code.replaces, code.prompt.as_str()), (Place::KitProject, Some(Place::Kit), "The project's code."));
    assert_eq!(code.hides, Some(Place::Repository), "a person's copy hides the repository's that says something else");
    let design = found(&resolved.defs, "design");
    assert_eq!((design.place, design.replaces, design.prompt.as_str()), (Place::Kit, Some(Place::Repository), "Kit's design."));
    let api = found(&resolved.defs, "api-review");
    assert_eq!((api.place, api.replaces, api.hides), (Place::Repository, Some(Place::Armada), None));
    let mine = found(&resolved.defs, "mine");
    assert_eq!((mine.place, mine.replaces, mine.hides), (Place::Kit, None, None));
    assert_eq!(resolved.defs[1].id, "code", "a replacement keeps the position of the one it replaced");
    assert_eq!(resolved.defs.last().map(|one| one.id.as_str()), Some("mine"), "a name nobody had is added at the end");
    assert!(resolved.left_out.is_empty());
}

#[test]
fn a_copy_that_says_the_same_as_the_repositorys_hides_nothing() {
    let mut all = carried();
    all.push(repository("code", file("Code", "The same words.")));
    all.push(kit("code", file("Code", "The same words.")));
    let resolved = resolve(all);
    let code = found(&resolved.defs, "code");
    assert_eq!((code.place, code.replaces, code.hides), (Place::Kit, Some(Place::Repository), None));
}

#[test]
fn a_file_that_does_not_fit_is_left_out_and_named_and_the_place_below_answers() {
    let mut all = carried();
    all.push(kit("code", "no front matter at all".to_string()));
    all.push(project("design", file("Design", "   ")));
    all.push(project("wrong-key", "---\nname: Wrong key\nkind: tool\n---\nA prompt.\n".to_string()));
    all.push(kit("no-name", "---\nother: 1\n---\nA prompt.\n".to_string()));
    let resolved = resolve(all);
    assert_eq!(found(&resolved.defs, "code").place, Place::Armada, "Armada's answers for the id its copy would have");
    assert_eq!(found(&resolved.defs, "design").place, Place::Armada);
    assert!(resolved.get("wrong-key").is_none() && resolved.get("no-name").is_none());
    let said: Vec<(String, Place)> = resolved.left_out.iter().map(|one| (one.path.display().to_string(), one.place)).collect();
    assert_eq!(
        said,
        [
            ("/kit/judges/code.md".to_string(), Place::Kit),
            ("/kit/judges/projects/storefront/design.md".to_string(), Place::KitProject),
            ("/kit/judges/projects/storefront/wrong-key.md".to_string(), Place::KitProject),
            ("/kit/judges/no-name.md".to_string(), Place::Kit),
        ]
    );
    assert!(resolved.left_out[0].said.contains("/kit/judges/code.md was left out, because it does not open with"), "{}", resolved.left_out[0].said);
    assert!(resolved.left_out[1].said.contains("no prompt"), "{}", resolved.left_out[1].said);
}

#[test]
fn two_files_in_one_place_that_share_an_id_are_both_left_out() {
    let mut all = carried();
    all.push(kit("one", file("Same", "First.")));
    all.push(kit("two", file("Same", "Second.")));
    let resolved = resolve(all);
    assert!(resolved.get("same").is_none());
    assert_eq!(resolved.left_out.len(), 2);
    assert!(resolved.left_out[0].said.contains("2 define `same` in Kit's place"), "{}", resolved.left_out[0].said);
}

#[test]
fn fit_and_file_of_agree_on_what_a_save_writes() {
    assert_eq!(fit("System design", "Look at the seams."), Ok("system-design".to_string()));
    assert!(fit("  ", "A prompt.").is_err());
    assert!(fit("!!!", "A prompt.").unwrap_err().contains("makes no id"));
    assert!(fit("Code", "  ").unwrap_err().contains("no prompt"));
    let text = file_of("Name: with a colon", "  A prompt.  ");
    let resolved = resolve([kit("x", text)]);
    let held = &resolved.defs[0];
    assert_eq!((held.name.as_str(), held.prompt.as_str(), held.id.as_str()), ("Name: with a colon", "A prompt.", "name-with-a-colon"));
}

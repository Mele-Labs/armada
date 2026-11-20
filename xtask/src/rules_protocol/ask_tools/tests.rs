//! The rule's own negative tests: each of its four refusals, and a placed inventory passing.

use super::*;
use crate::Finding;

fn failures(inventory: &str, registry: &str) -> Vec<String> {
    let hues = [("ask-tools.css", String::from("--ask-sketch: #AD9BF5;")), ("light.css", String::from("--ask-sketch: #6A4FC8;"))];
    let mut report = Report::new("test");
    check(inventory, registry, &hues, &|key| key == "layout-template", &mut report);
    report
        .findings
        .iter()
        .filter_map(|f| match f {
            Finding::Fail(what) => Some(what.clone()),
            Finding::Warn(_) => None,
        })
        .collect()
}

const REGISTRY_ONE: &str = r#"
[tools.sketch]
name = "Sketch"
glyph = "layout-template"
hue = "--ask-sketch"
line = "draw it with `show_sketch`"
"#;

#[test]
fn a_placed_inventory_passes() {
    let inventory = r#"
[operations.show_sketch]
ask_tool = ["sketch"]
kind = "command"
agent_access = "Yes"

[operations.get_health]
ask_tool = "none"
kind = "query"
agent_access = "On your words"

[operations.answer_sketch]
kind = "command"
agent_access = "No"
"#;
    assert_eq!(failures(inventory, REGISTRY_ONE), Vec::<String>::new());
}

#[test]
fn an_operation_an_agent_reaches_with_no_place_is_refused_naming_its_file() {
    let inventory = "\n[operations.show_sketch]\nkind = \"command\"\nagent_access = \"Operator\"\n";
    let failed = failures(inventory, REGISTRY_ONE);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].starts_with("crates/ipc/operations/show_sketch.toml — "), "{failed:?}");
    assert!(failed[0].contains("or `ask_tool = \"none\"`"), "{failed:?}");
}

#[test]
fn a_tool_the_list_does_not_hold_is_refused() {
    let inventory = "\n[operations.show_sketch]\nask_tool = [\"sketch\", \"kettle\"]\nkind = \"command\"\nagent_access = \"Yes\"\n";
    let failed = failures(inventory, REGISTRY_ONE);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].contains("`kettle`"), "{failed:?}");
}

#[test]
fn a_place_on_an_operation_no_agent_reaches_is_refused() {
    let inventory = "\n[operations.answer_sketch]\nask_tool = \"none\"\nkind = \"command\"\nagent_access = \"Bridge only\"\n";
    let failed = failures(inventory, REGISTRY_ONE);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].contains("no agent can reach"), "{failed:?}");
}

#[test]
fn a_tool_with_no_hue_glyph_or_line_is_refused() {
    let registry = "\n[tools.kettle]\nname = \"Kettle\"\nglyph = \"kettle\"\nhue = \"--ask-kettle\"\nline = \"\"\n";
    let inventory = "\n[operations.show_sketch]\nask_tool = \"kettle\"\nkind = \"command\"\nagent_access = \"Yes\"\n";
    let failed = failures(inventory, registry);
    assert_eq!(failed.iter().filter(|said| said.contains("does not declare")).count(), 2, "{failed:?}");
    assert!(failed.iter().any(|said| said.contains("has no file in")), "{failed:?}");
    assert!(failed.iter().any(|said| said.contains("has no `line`")), "{failed:?}");
}

#[test]
fn none_beside_a_tool_is_refused() {
    let inventory = "\n[operations.show_sketch]\nask_tool = [\"sketch\", \"none\"]\nkind = \"command\"\nagent_access = \"Yes\"\n";
    assert_eq!(failures(inventory, REGISTRY_ONE).len(), 1);
}

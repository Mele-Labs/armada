//! The `armada-mods` skill keeps a table of the regions a layout mod may name, and
//! `packages/shell/layout-registry.json` is the list Fleet and Bridge both check. The table is written
//! by hand, so a panel added to the registry and not to the table tells a session that a real id is a
//! typo, which is ignored without a word. This reads the table back against the registry.
//!
//! xtask has no JSON reader, so the registry is read by the shape it is written in: a region on one
//! line, then one entry per line.

use std::fs;

struct Region {
    name: String,
    ordered: bool,
    firstable: bool,
    ids: Vec<String>,
    fixed: Vec<String>,
}

fn quoted(line: &str) -> Option<&str> {
    line.split('"').nth(1)
}

fn registry(text: &str) -> Vec<Region> {
    let mut regions: Vec<Region> = Vec::new();
    for line in text.lines() {
        if line.contains("\"ordered\":") {
            regions.push(Region {
                name: quoted(line).expect("a region line names its region").to_string(),
                ordered: line.contains("\"ordered\": true"),
                firstable: line.contains("\"firstable\": true"),
                ids: Vec::new(),
                fixed: Vec::new(),
            });
        } else if line.contains("\"id\":") {
            let region = regions.last_mut().expect("an entry sits under its region");
            let id = line.split("\"id\": \"").nth(1).and_then(quoted_prefix).expect("an entry names its id");
            region.ids.push(id.clone());
            if line.contains("\"hideable\": false") {
                region.fixed.push(id);
            }
        }
    }
    regions
}

fn quoted_prefix(rest: &str) -> Option<String> {
    rest.split('"').next().map(str::to_string)
}

/// The words in backticks in one table cell, in order.
fn names(cell: &str) -> Vec<String> {
    cell.split('`').skip(1).step_by(2).map(str::to_string).collect()
}

#[test]
fn the_skill_names_the_registrys_regions_ids_and_flags() {
    let root = crate::repo_root();
    let skill = fs::read_to_string(root.join(".claude/skills/armada-mods/SKILL.md")).expect("the skill is there");
    let shipped = registry(&fs::read_to_string(root.join("packages/shell/layout-registry.json")).expect("the registry is there"));
    assert!(!shipped.is_empty(), "the registry file was read as no regions");

    let rows: Vec<Vec<&str>> = skill
        .lines()
        .map(|line| line.split('|').map(str::trim).collect::<Vec<_>>())
        .filter(|cells| cells.len() == 7 && cells[1].starts_with('`'))
        .collect();
    let yes_no = |on: bool| if on { "yes" } else { "no" };
    assert_eq!(
        rows.iter().map(|cells| names(cells[1])).collect::<Vec<_>>(),
        shipped.iter().map(|region| vec![region.name.clone()]).collect::<Vec<_>>(),
        "the skill's regions table names the registry's regions, in its order"
    );
    for (cells, region) in rows.iter().zip(&shipped) {
        assert_eq!(names(cells[2]), region.ids, "{}: ids", region.name);
        assert_eq!(names(cells[3]), region.fixed, "{}: cannot be hidden", region.name);
        assert_eq!((cells[4], cells[5]), (yes_no(region.ordered), yes_no(region.firstable)), "{}: order, first", region.name);
    }
}

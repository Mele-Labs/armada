//! What a WorkflowDef declares, read off `workflowdef-fields.toml` as a list.
//!
//! **The file is the authority and this reads it rather than restating it.** A
//! second copy of the field list is a list that is wrong the week a field is
//! added, and the agent authoring a definition is the reader who would act on
//! the stale one. Only the shape the file is written in is read — a `[fields.x]`
//! header, then one-line `type`, `parent` and `purpose` keys, and the keys of a
//! `[fields.x.values]` table — and the notes, the open questions and the reasoning
//! stay in the file for a person to read.
//!
//! No TOML parser is in this workspace and one is not worth adding for a file
//! this regular: a line that does not fit the shape is skipped, and the test
//! below fails if a row stops yielding a type and a purpose.

const FIELDS: &str = include_str!("../../core-model/domain/workflowdef-fields.toml");

/// One field of a WorkflowDef, or of a step, or of an object nested in one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    /// The row this one nests under. `None` at the top level, where the field
    /// belongs to the WorkflowDef itself.
    pub parent: Option<String>,
    pub kind: String,
    pub purpose: String,
    /// The legal values, where the file lists them as a table.
    pub values: Vec<String>,
}

/// Every field, in the order the file declares them.
pub fn workflow_fields() -> Vec<Field> {
    let mut fields: Vec<Field> = Vec::new();
    let mut in_values = false;
    for line in FIELDS.lines() {
        if let Some(header) = line.strip_prefix("[fields.") {
            let header = header.trim_end_matches(']');
            if let Some(name) = header.strip_suffix(".values") {
                in_values = fields
                    .last()
                    .is_some_and(|last| last.name == unquoted(name));
                continue;
            }
            in_values = false;
            fields.push(Field {
                name: unquoted(header),
                parent: None,
                kind: String::new(),
                purpose: String::new(),
                values: Vec::new(),
            });
            continue;
        }
        if line.starts_with('[') {
            in_values = false;
            continue;
        }
        let Some(last) = fields.last_mut() else {
            continue;
        };
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        if in_values {
            last.values.push(unquoted(key));
            continue;
        }
        match key {
            "type" => last.kind = unquoted(value),
            "parent" => last.parent = Some(unquoted(value)),
            "purpose" => last.purpose = unquoted(value).replace("\\\"", "\""),
            _ => {}
        }
    }
    fields
}

fn unquoted(text: &str) -> String {
    text.trim().trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_says_what_it_is_and_what_it_is_for() {
        let fields = workflow_fields();
        assert!(!fields.is_empty());
        for field in &fields {
            assert!(!field.kind.is_empty(), "`{}` has no type", field.name);
            assert!(!field.purpose.is_empty(), "`{}` has no purpose", field.name);
        }
    }

    #[test]
    fn a_step_field_names_its_parent_and_an_enum_lists_its_values() {
        let fields = workflow_fields();
        let id = fields.iter().find(|f| f.name == "id").expect("`id`");
        assert_eq!(id.parent.as_deref(), Some("steps[]"));
        let gate = fields
            .iter()
            .find(|f| f.name == "advance_gate")
            .expect("`advance_gate`");
        assert!(gate.values.iter().any(|v| v == "human_always"));
        let steps = fields.iter().find(|f| f.name == "steps[]").expect("steps");
        assert_eq!(steps.parent, None);
    }
}

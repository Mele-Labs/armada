//! A definition's id and name, read with no roster and no Manifest: what a package says a
//! workflow is, before any repository is asked whether it fits.

use serde_yaml_ng::Value;

/// The two keys that say which workflow a file is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowHead {
    pub id: String,
    pub name: String,
}

/// `workflow_id` and `name` of a definition's text, JSON or YAML. Says what is missing, and
/// nothing about whether the rest of the file would load.
pub fn head_of(text: &str) -> Result<WorkflowHead, String> {
    let root: Value = serde_yaml_ng::from_str(text).map_err(|why| format!("is not a definition: {why}"))?;
    let key = |name: &str| match root.get(name) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        Some(_) => Err(format!("has a `{name}` that is not text")),
        None => Err(format!("has no `{name}`")),
    };
    Ok(WorkflowHead { id: key("workflow_id")?, name: key("name")? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_json_and_yaml_alike() {
        let expected = WorkflowHead { id: "bug".into(), name: "Fix a bug".into() };
        assert_eq!(head_of(r#"{"version":1,"workflow_id":"bug","name":"Fix a bug","steps":[]}"#), Ok(expected.clone()));
        assert_eq!(head_of("version: 1\nworkflow_id: bug\nname: Fix a bug\nsteps: []\n"), Ok(expected));
    }

    #[test]
    fn names_what_is_missing() {
        assert_eq!(head_of("name: x\n"), Err("has no `workflow_id`".into()));
        assert_eq!(head_of("workflow_id: 3\nname: x\n"), Err("has a `workflow_id` that is not text".into()));
        assert!(head_of("- a\n- b\n").is_err());
    }
}

//! A need a Drone declares: a path and what is needed there, in its own words.
//! Read by `declare_scope`, where a Drone adds one mid-work as it corrects its
//! scope, and by each task of `record_plan` and `add_task`, where a plan
//! carries them. `#1059`, `decisions/2026-10-02-a-plan-leases-its-numbers.md`.
//!
//! **No kinds.** A need is not a `migration` or a `minor`: the file is the
//! resource, so a repository that numbers its migrations differently declares
//! the same call on a different path.

use serde_json::{json, Map, Value};

use super::tools::{closed, text, NotAnArgument};

/// The fields of one need.
pub const NEED_FIELDS: &[&str] = &["path", "what", "took"];

/// What a Drone says it needs on one path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeedClaim {
    /// Repository-relative.
    pub path: String,
    /// What is needed there, in the declarer's words.
    pub what: String,
    /// The value taken once it has chosen: what a later declarer is told.
    pub took: Option<String>,
}

/// `needs`, where the call carries it. **Absent is none**, unlike
/// `context_paths`: most calls need nothing from anyone.
pub(super) fn read(
    arguments: &Map<String, Value>,
    tool: &'static str,
) -> Result<Vec<NeedClaim>, NotAnArgument> {
    let Some(listed) = arguments.get("needs") else {
        return Ok(Vec::new());
    };
    let listed = listed
        .as_array()
        .ok_or(NotAnArgument::NotOptions { field: "needs" })?;
    let mut claims = Vec::with_capacity(listed.len());
    for entry in listed {
        let entry = entry
            .as_object()
            .ok_or(NotAnArgument::NotOptions { field: "needs" })?;
        closed(entry, tool, NEED_FIELDS)?;
        let path = text(entry, "path")?.trim().to_string();
        let what = text(entry, "what")?.trim().to_string();
        if path.is_empty() {
            return Err(NotAnArgument::Blank { field: "path" });
        }
        if what.is_empty() {
            return Err(NotAnArgument::Blank { field: "what" });
        }
        let took = match entry.get("took") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                value
                    .as_str()
                    .ok_or(NotAnArgument::NotText { field: "took" })?
                    .trim()
                    .to_string(),
            )
            .filter(|took| !took.is_empty()),
        };
        claims.push(NeedClaim { path, what, took });
    }
    Ok(claims)
}

/// The schema of the `needs` property, the same on every tool that takes it.
pub(super) fn property() -> Value {
    json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The repository-relative file you need something on \
                        and others may too, such as the file that lists migrations or \
                        holds a version.",
                },
                "what": {
                    "type": "string",
                    "description": "What you need there, in your own words: a new \
                        migration, a minor.",
                },
                "took": {
                    "type": "string",
                    "description": "Once you have chosen it, the value you took, such as \
                        V95. Call again with it. Leave it out until then.",
                },
            },
            "required": ["path", "what"],
            "additionalProperties": false,
        },
        "description": "Optional. What you need on a file where another Job may need \
            something at the same time. Declare it before you choose the number or name: \
            the first to declare goes first, you are told who is ahead of you and what \
            they took, and you take the value after theirs. Your work is held at the \
            merge until every need ahead of yours has landed. Leave it out where you \
            need nothing like that.",
    })
}

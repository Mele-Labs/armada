//! The fix tool: a Drone says a test it hit is already broken on main, and asks
//! for the fix to be drafted. #999.
//!
//! **Not a report, and not named like one.** `submit_evidence` is the one tool a
//! Drone reports through, and spike 6 found a Drone reaching for reporting-shaped
//! tools it was not meant to use. **The Drone's claim is a question, not a
//! verdict**: Fleet runs just that test against main before anything is drafted,
//! and nothing here passes the Drone's own gate.

use serde_json::{json, Map, Value};

use super::tools::{closed, filled, list, NotAnArgument};

/// A list that names at least one thing, every entry filled.
fn named(
    arguments: &Map<String, Value>,
    field: &'static str,
) -> Result<Vec<String>, NotAnArgument> {
    let listed = list(arguments, field)?;
    if listed.is_empty() || listed.iter().any(|entry| entry.trim().is_empty()) {
        return Err(NotAnArgument::Blank { field });
    }
    Ok(listed)
}

/// The fix tool's name, bare.
pub const FIX_TOOL: &str = "draft_fix";

/// The fields the fix tool takes. Public for `EVIDENCE_FIELDS`' reason.
pub const FIX_FIELDS: &[&str] = &[
    "check",
    "test",
    "files",
    "failure",
    "title",
    "workflow",
    "brief",
    "acceptance_criteria",
];

/// A test a Drone says is broken on main, and the fix it asks for.
///
/// **The last four are `dispatch_job`'s**, for its reason: a drafted Job is
/// judged against its frozen workflow, so the Drone names one rather than Fleet
/// guessing the nearest fit. Every text field is required and none may be blank;
/// `acceptance_criteria` may be empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftFix {
    /// The Check the test failed under, as this step's Checks name it.
    pub check: String,
    /// The failing test's name, copied from the Check's output.
    pub test: String,
    /// The files the test lives in, as paths in the repository. **Required and
    /// never empty from a Drone**: while the fix is worked they are outside the
    /// write scope of every Job that hit the test, so Fleet has to know which
    /// they are. #1673. Empty only on a fix Fleet spotted itself.
    pub files: Vec<String>,
    /// What the output said about the failure, in a line or two.
    pub failure: String,
    pub title: String,
    pub workflow: String,
    pub brief: String,
    pub acceptance_criteria: Vec<String>,
}

/// Read one call's arguments, or say which field is wrong.
pub(super) fn asked_for(arguments: &Map<String, Value>) -> Result<DraftFix, NotAnArgument> {
    closed(arguments, FIX_TOOL, FIX_FIELDS)?;
    Ok(DraftFix {
        check: filled(arguments, "check")?,
        test: filled(arguments, "test")?,
        files: named(arguments, "files")?,
        failure: filled(arguments, "failure")?,
        title: filled(arguments, "title")?,
        workflow: filled(arguments, "workflow")?,
        brief: filled(arguments, "brief")?,
        acceptance_criteria: list(arguments, "acceptance_criteria")?,
    })
}

/// The fix tool, described by each of its outcomes and by what it is not.
pub(super) fn fix_tool() -> Value {
    json!({
        "name": FIX_TOOL,
        "description":
            "Say that a test your Check failed on is already broken on main, not \
             by your change, and draft the Job that fixes it. Fleet runs just that \
             test against main first. If it fails there too, a fix Job is drafted \
             and waits for a person's approval; if that test already has a fix \
             drafted, nothing new is drafted and you are told which Job has it. If \
             it passes on main, the failure is in your change and nothing is \
             drafted. The call comes back at once, and what the test came to on \
             main arrives as a later turn, however long it takes: carry on with \
             your part meanwhile. It is not how you \
             report your work, which is submit_evidence, and it never passes your \
             own gate: your step's Checks still decide it.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "check": {
                    "type": "string",
                    "description": "The Check the test failed under, as your step's Checks name it.",
                },
                "test": {
                    "type": "string",
                    "description": "The failing test's name, copied exactly from the Check's output.",
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "description": "The files the failing test lives in, as paths from the \
                                    repository root. Fleet checks each is on main. While the \
                                    fix is worked, no Job that hit the test may change them.",
                },
                "failure": {
                    "type": "string",
                    "description": "What the output said about the failure, in a line or two.",
                },
                "title": {
                    "type": "string",
                    "description": "What the fix Job is called on the Board.",
                },
                "workflow": {
                    "type": "string",
                    "description": "The workflow the fix Job runs, spelled as this repository names it.",
                },
                "brief": {
                    "type": "string",
                    "description": "What the fix Job's Drone is told about the breakage.",
                },
                "acceptance_criteria": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "What the fix is held to. May be empty.",
                },
            },
            "required": FIX_FIELDS,
        },
    })
}

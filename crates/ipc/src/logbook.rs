//! What the Logbook hands out mid-task: a search of the whole of it, and the entries a subagent is
//! given for the paths it works under. `docs/concepts/logbook.md`, *Recall*.

use serde::{Deserialize, Serialize};

/// `recall`: the entries that match, best first. `GET /logbook/recall?query=&section=&manifest_id=`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recalled {
    pub entries: Vec<RecalledEntry>,
}

/// One Logbook entry as `recall` answers it. Faded and candidate entries are in it, and `state`
/// says which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecalledEntry {
    /// `repo`, `habits` or `machine`.
    pub section: String,
    /// `command`, `gotcha`, `convention`, `environment` or `habit`.
    pub kind: String,
    /// `candidate`, `standing` or `faded`.
    pub state: String,
    pub text: String,
    /// How many distinct Sessions bore it out.
    pub confirmed_by: u32,
}

/// `get_logbook_for_subagent`'s body: the Session whose subagent is starting, and the prompt it is
/// given. `POST /logbook/subagent`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentAsk {
    pub session_id: String,
    pub prompt: String,
}

/// What a subagent is handed. `text` is empty where the Logbook holds nothing for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForSubagent {
    pub text: String,
}

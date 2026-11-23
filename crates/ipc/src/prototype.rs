//! `show_prototype`: the Job shows a scene it wrote under its `scenes/` folder, and native draws it live.
//! **Checked here, in the crate where bytes enter**: [`ShowPrototype::checked`] says every reason a call is refused in
//! one sentence an agent can act on. Fleet reads the files; this crate only says which may be named.
//! `crates/ipc/operations/show_prototype.toml`.

use serde::{Deserialize, Serialize};

use crate::sketch::MOST_TITLE_CHARS;
use crate::{ArtifactFor, SessionId, MOST_PART_ID_CHARS};

/// Most bytes a scene file may weigh.
pub const MOST_SCENE_BYTES: usize = 64 * 1024;
/// Most bytes all the fixtures a scene names may weigh together.
pub const MOST_FIXTURE_BYTES: usize = 512 * 1024;
/// Most characters in a state's name.
pub const MOST_STATE_CHARS: usize = 80;
/// The folder under a Job's scratch folder that holds its scenes.
pub const SCENES_DIR: &str = "scenes";

/// `show_prototype`: the Job puts the scene `scene` under the artifact of `title`, as its next revision where it is
/// there already. A part put by this call is kept under the scene's file name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowPrototype {
    /// The artifact's title, as `add_part`'s.
    pub title: String,
    /// A plain file name under `$ARMADA_SCRATCH/scenes/`, such as `ready.html`.
    pub scene: String,
    /// The state to step him to. The scene's first where left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// What the artifact is for, as `add_part`'s `for`. Kept where the call makes the artifact.
    #[serde(rename = "for", default, skip_serializing_if = "Option::is_none")]
    pub for_line: Option<String>,
    /// Which session shows it. **Read only where the connection places no hosted session**, as `AddPart`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// A `show_prototype` that passed every rule. Fleet reads the files it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedPrototype {
    pub title: String,
    pub scene: String,
    pub state: Option<String>,
    pub said: Option<ArtifactFor>,
}

impl ShowPrototype {
    /// The call, or every reason it is refused in one message an agent can act on.
    pub fn checked(&self) -> Result<CheckedPrototype, String> {
        let mut problems = Vec::new();
        let title = self.title.trim().to_string();
        if title.is_empty() {
            problems.push(String::from("`title` is blank. Name the artifact the prototype goes on"));
        } else if title.chars().count() > MOST_TITLE_CHARS {
            problems.push(format!("`title` is longer than {MOST_TITLE_CHARS} characters"));
        }
        let scene = match file_name_checked(&self.scene) {
            Ok(name) => name,
            Err(why) => {
                problems.push(format!("`scene` {why}. Name a file directly under `$ARMADA_SCRATCH/{SCENES_DIR}/`, such as `ready.html`"));
                String::new()
            }
        };
        let state = self.state.as_deref().map(str::trim).filter(|state| !state.is_empty()).map(str::to_string);
        if state.as_ref().is_some_and(|state| state.chars().count() > MOST_STATE_CHARS) {
            problems.push(format!("`state` is longer than {MOST_STATE_CHARS} characters"));
        }
        let said = match ArtifactFor::checked(self.for_line.as_deref(), &[]) {
            Ok(said) => said,
            Err(why) => {
                problems.push(why);
                None
            }
        };
        if !problems.is_empty() {
            return Err(format!("the prototype was not shown: {}. Fix these and call show_prototype again", problems.join("; ")));
        }
        Ok(CheckedPrototype { title, scene, state, said })
    }
}

/// `name` as a plain file name, or what is wrong with it. **A path, `..` and a hidden name are all refused**, so a
/// name can only ever land inside the folder it is joined to.
pub fn file_name_checked(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(String::from("is blank"));
    }
    if name.contains(['/', '\\', '\0']) {
        return Err(format!("is `{name}`, a path"));
    }
    if name.contains("..") || name.starts_with('.') {
        return Err(format!("is `{name}`, which climbs out of the folder or hides"));
    }
    if name.chars().count() > MOST_PART_ID_CHARS {
        return Err(format!("is longer than {MOST_PART_ID_CHARS} characters"));
    }
    Ok(name.to_string())
}

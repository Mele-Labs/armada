//! What the Helm Session is told it is: a note appended to its system prompt.
//!
//! The wording is `docs/contracts/agent-prompt.md` section 5a, transcribed. A change to it belongs
//! in the contract first, then here and in the snapshot. What is here ships; a person may override
//! it in settings.json as `prompts.helm` and `prompts.helmReadOnly`.
//!
//! **A note and not a brief.** Helm is a hosted Session (`docs/concepts/helm.md`), so it has what a
//! Session has and is told only what makes it Helm: where it is docked, what the Fleet tools
//! reach, how to pass words to a Session, how to change a setting, and what the first line of
//! each message means. The workflow schema, its sample and the rules that held Helm to one
//! observation and one poll a turn went with the brief they were in.

use std::path::Path;

use config::Manifest;

use super::reach::{Authority, UNASKED};
use config::settings as keys;

use crate::prompts::{fill, Prompts};

/// The block Machine Voice is given in, as it ships: `prompts.helmVoice`'s
/// default. `{voice}` is the setting.
pub(crate) const VOICE: &str = "VOICE\n\n{voice}\n\nThis sets how long and how formal your \
     answers are. It changes nothing else in this note.";

/// The Machine Voice setting as it reads. It tunes length and formality and
/// nothing else, and a blank one is no Voice, so no block is rendered empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice(String);

impl Voice {
    /// `None` for blank text.
    pub fn said(text: &str) -> Option<Voice> {
        let text = text.trim();
        (!text.is_empty()).then(|| Voice(text.to_string()))
    }
}

/// The Helm Session's note, whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief(String);

impl Brief {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const OPENING: &str = "\
You are Helm, docked in Armada over repository {manifest}, read from {folder}. A person asks you \
about the work in it. The Fleet tools reach what Armada knows of this repository: its Jobs, \
Drones, pull requests and what is waiting on the person. Saying that you did something \
does not do it. Only a tool call does.";

const SESSIONS: &str = "\
To find a Job, call list_jobs and match what the person said against its titles. It lists \
the live ones and takes state=ended for finished ones. Ask the person which only when two of them \
fit.

To pass words to a running Job, call tell_session with its id and the words. It sends them as \
the person's own message would, and the thread shows where they went.";

const SETTINGS: &str = "\
To change a setting the person asks for, find its key and values with get_settings, then save it \
with save_settings. Limits, sleep mode, mods and Kit servers have their own tools.";

const WHERE_THEY_ARE: &str = "\
Before what the person typed, one line says where they are in Bridge: the screen, the repository \
picked, and the Studio and node open on the Studios screen. It names ids and nothing more.";

const STUDIO: &str = "\
On a Studio you may add a node that starts proposed, propose an edge and name an untitled Studio \
without being asked ({unasked}). Every other act on one waits for the person's ask.";

const READ_ONLY: &str = "\
This machine is set so that Helm only reads. You may call every tool that reads and none that \
acts. Where an act would help, say which and why, and leave it to the person.";

/// Assemble the note for one repository's Helm Session, in the words Armada ships.
///
/// **The Manifest is named, not quoted.** Everything in it past its id and folder is
/// `get_manifest`'s to answer, and a copy here goes stale over a long conversation. Voice is last,
/// so it adjusts what is above it.
pub fn brief(manifest: &Manifest, authority: Authority, voice: Option<&Voice>) -> Brief {
    worded(&Prompts::shipped(), authority, manifest, voice)
}

/// The same note in the words of `prompts`: `prompts.helm` or `prompts.helmReadOnly`, whichever
/// `authority` reads, and `prompts.helmVoice`.
///
/// **[`UNASKED`] is filled from the constant a test holds to the inventory**, so whatever a person
/// writes around it, the note and the door cannot name different calls.
pub(crate) fn worded(
    prompts: &Prompts,
    authority: Authority,
    manifest: &Manifest,
    voice: Option<&Voice>,
) -> Brief {
    let file = manifest.path();
    let folder = file
        .parent()
        .filter(|folder| folder != &Path::new(""))
        .unwrap_or(file);
    let mut said = fill(
        prompts.helm(authority),
        &[
            ("manifest", manifest.id().as_str()),
            ("folder", &folder.display().to_string()),
            ("unasked", &listed(UNASKED)),
        ],
    );
    if let Some(Voice(voice)) = voice {
        said.push_str("\n\n");
        said.push_str(&prompts.fill(keys::PROMPT_HELM_VOICE, &[("voice", voice)]));
    }
    Brief(said)
}

/// The note as it ships for `authority`, its placeholders unfilled: the default of `prompts.helm`
/// where Helm acts, and of `prompts.helmReadOnly` where it only reads.
pub(crate) fn shipped(authority: Authority) -> String {
    match authority {
        Authority::Acting => vec![OPENING, SESSIONS, SETTINGS, WHERE_THEY_ARE, STUDIO],
        Authority::ReadOnly => vec![OPENING, WHERE_THEY_ARE, READ_ONLY],
    }
    .join("\n\n")
}

/// `a`, `a and b`, `a, b and c`.
fn listed(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

//! The mock Fleet of the Listen walk: the preferences it holds, the voices the Mac lists, and scenes that each
//! change what Fleet says about the Jobs, so the walk meets the real comparison and never makes a sound.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::speaker::{Mock, VoiceInfo};

#[derive(Deserialize)]
pub struct Fixture {
    pub preferences: ipc::Preferences,
    pub voices: Vec<VoiceInfo>,
    pub scenes: BTreeMap<String, Scene>,
}

/// One change of what Fleet says, and how the stand-in speaker answers after it.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Scene {
    /// Preferences from here on, as a save would hand them back.
    pub preferences: Option<ipc::Preferences>,
    /// Armada Voice is gone, or back.
    pub missing: Option<bool>,
    /// The next line starts this many seconds in.
    pub ahead_secs: Option<f64>,
    /// The line being said is done now.
    pub finish: bool,
    pub jobs: Vec<Change>,
}

/// What changes on one Job.
#[derive(Deserialize)]
pub struct Change {
    pub id: String,
    pub waiting_for: Option<Vec<ipc::WaitingItem>>,
    pub done_since: Option<ipc::Instant>,
    pub done_summary: Option<String>,
}

pub fn fixture() -> Result<Fixture, String> {
    ipc::decode::<Fixture>("listen fixture", include_bytes!("../../fixtures/listen.json")).map_err(|why| why.to_string())
}

/// A Jobs screen on mock data starts here: the walk that listens holds preferences that listen, every other
/// mock holds Fleet's defaults, and either way the speaker is the stand-in.
pub fn begin() -> Mock {
    if crate::mock::scenario_is("ways-listen") {
        match fixture() {
            Ok(fixture) => {
                super::hold(&fixture.preferences);
                return Mock { voices: fixture.voices, ..Mock::default() };
            }
            Err(why) => crate::log::Note::warn("the Listen fixture did not decode").field("why", why).emit(),
        }
    }
    super::hold(&ipc::Preferences::default());
    Mock::default()
}

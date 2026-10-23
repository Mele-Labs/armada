//! The Cockpit's flight controls a person arranged, in `~/.armada/flight-controls.toml`.
//! `docs/concepts/kit.md`, *Flight controls*.
//!
//! **Read when asked, never cached**: a person edits the file by hand, and the next read is what
//! the next press draws. **An absent file is Armada's own; a file that will not read is
//! `not_read` with why**, never the defaults, since a person shown their tiles reset saves over the
//! file that held them. **Fleet does not know the built-in ids** (Bridge's), so a rule that needs
//! them is Bridge's to hold.
//!
//! A repository's own tiles are kept where its runs are: [`repository`]. A shell tile its team
//! committed asks before it runs on this Mac: [`approving`].

pub(crate) mod approving;
pub(crate) mod repository;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FlightControl, FlightControls, FlightControlsRead, WireError, FLIGHT_CONTROLS_VERSION};

use crate::daemon::Fleet;

/// The file's name inside Kit's home.
pub const FILE: &str = "flight-controls.toml";

/// A save that breaks a rule. A 422.
const REFUSED: &str = "fleet.flight_controls_refused";
/// The file could not be written. A 500.
const UNWRITABLE: &str = "fleet.flight_controls_unwritable";

/// One writer at a time, so two saves arriving together cannot interleave their staging file.
static WRITING: Mutex<()> = Mutex::new(());

/// The rules a file keeps, the first broken one named by its field. A hand-edited file is held to
/// them as a save is, so what is read is what could have been saved.
pub(crate) fn check(controls: &FlightControls) -> Result<(), String> {
    if controls.version != FLIGHT_CONTROLS_VERSION {
        return Err(format!(
            "version is {}, and this Fleet reads {FLIGHT_CONTROLS_VERSION}",
            controls.version
        ));
    }
    check_list("job", &controls.job)?;
    check_list("deck", &controls.deck)
}

/// A tile's key: a digit or a letter, or a chord in the keymap's spelling, `⌥R` or `⇧⌘J`: modifiers with at least one of ⌃ ⌥ ⌘, then a key.
/// Bridge holds the rest of the rule (what a chord may be, and which it cannot be), since only it knows the keys it answers.
fn is_key(key: &str) -> bool {
    const MODIFIERS: [char; 4] = ['⌃', '⌥', '⇧', '⌘'];
    let mut chars = key.chars();
    if let (Some(one), None) = (chars.next(), chars.next()) {
        return one.is_ascii_alphanumeric();
    }
    let rest = key.trim_start_matches(MODIFIERS);
    let held = &key[..key.len() - rest.len()];
    held.chars().any(|one| one != '⇧') && !rest.is_empty() && !rest.contains(char::is_whitespace) && !rest.starts_with(MODIFIERS)
}

fn check_list(list: &str, tiles: &[FlightControl]) -> Result<(), String> {
    let mut ids: BTreeMap<&str, usize> = BTreeMap::new();
    let mut keys: BTreeMap<String, usize> = BTreeMap::new();
    for (at, tile) in tiles.iter().enumerate() {
        if tile.id.trim().is_empty() {
            return Err(format!("{list}[{at}].id is blank"));
        }
        if let Some(first) = ids.insert(tile.id.as_str(), at) {
            return Err(format!("{list}[{at}].id `{}` is already {list}[{first}].id", tile.id));
        }
        let Some(key) = &tile.key else { continue };
        if !is_key(key) {
            return Err(format!(
                "{list}[{at}].key `{key}` is not one of 1 to 9, 0, a letter or a chord such as ⌥R"
            ));
        }
        if let Some(first) = keys.insert(key.to_ascii_lowercase(), at) {
            return Err(format!("{list}[{at}].key `{key}` is already {list}[{first}].key"));
        }
    }
    Ok(())
}

/// What the file's text comes to.
pub(crate) fn parse(text: &str) -> Result<FlightControls, String> {
    let controls: FlightControls = toml::from_str(text).map_err(|why| why.to_string())?;
    check(&controls)?;
    Ok(controls)
}

/// The file at `path` as it is now, by `parse`. An absent file is Armada's own.
pub(crate) async fn read_at(path: &Path, parse: fn(&str) -> Result<FlightControls, String>) -> FlightControlsRead {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => match parse(&text) {
            Ok(controls) => FlightControlsRead::Read { controls },
            Err(why) => FlightControlsRead::NotRead { why: format!("{}: {why}", path.display()) },
        },
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {
            FlightControlsRead::Read { controls: FlightControls::default() }
        }
        Err(cause) => FlightControlsRead::NotRead { why: format!("{}: {cause}", path.display()) },
    }
}

/// Write `text` to `<path>.writing` and rename it over `path`.
fn write_over(path: &Path, text: &str) -> std::io::Result<()> {
    let _one_at_a_time = WRITING.lock().unwrap_or_else(|held| held.into_inner());
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut staged = path.as_os_str().to_owned();
    staged.push(".writing");
    let staged = PathBuf::from(staged);
    std::fs::write(&staged, text).and_then(|()| std::fs::rename(&staged, path))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn flight_controls_path(&self) -> PathBuf {
        PathBuf::from(&self.host().kit_home).join(FILE)
    }

    /// `get_flight_controls`: the file as it is now.
    pub(crate) async fn flight_controls_read(&self) -> FlightControlsRead {
        read_at(&self.flight_controls_path(), parse).await
    }

    /// `save_flight_controls`: check, write whole, and answer with the file read again.
    pub(crate) async fn flight_controls_saved(
        &self,
        controls: FlightControls,
    ) -> Result<FlightControlsRead, Refusal> {
        check(&controls)
            .map_err(|why| Refusal::Unacceptable(WireError::raised(REFUSED, why, self.run_id())))?;
        self.flight_controls_written(&self.flight_controls_path(), &controls).await?;
        Ok(self.flight_controls_read().await)
    }

    fn flight_controls_unwritable(&self, named: &str, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(
            UNWRITABLE,
            format!("{named} could not be written: {why}"),
            self.run_id(),
        ))
    }

    /// The file's text; `named` is what an error calls it, a path or an object.
    fn flight_controls_text(&self, named: &str, controls: &FlightControls) -> Result<String, Refusal> {
        toml::to_string(controls).map_err(|why| self.flight_controls_unwritable(named, why))
    }

    /// Write `controls` whole at `path`, already checked.
    pub(crate) async fn flight_controls_written(&self, path: &Path, controls: &FlightControls) -> Result<(), Refusal> {
        let named = path.display().to_string();
        let text = self.flight_controls_text(&named, controls)?;
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || write_over(&target, &text))
            .await
            .map_err(|why| self.flight_controls_unwritable(&named, why))?
            .map_err(|why| self.flight_controls_unwritable(&named, why))
    }
}

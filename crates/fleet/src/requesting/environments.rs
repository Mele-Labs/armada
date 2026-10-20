//! The owner's named base addresses, in `~/.armada/requests.toml`. **The file is the wire**, read
//! when asked and never cached, as `crate::flight_controls`' is: an absent file is empty, and a
//! file that will not read is `not_read` with why, never empty, since a person shown an empty list
//! saves over the file that held theirs.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    RequestEnvironmentsFile, RequestEnvironmentsRead, WireError, REQUEST_ENVIRONMENTS_VERSION,
    SERVER_ENVIRONMENT_PREFIX,
};

use crate::daemon::Fleet;

/// The file's name inside Kit's home.
pub const FILE: &str = "requests.toml";

/// A save that breaks a rule. A 422.
const REFUSED: &str = "fleet.request_environments_refused";
/// The file could not be written. A 500.
const UNWRITABLE: &str = "fleet.request_environments_unwritable";

/// One writer at a time, so two saves cannot interleave their staging file.
static WRITING: Mutex<()> = Mutex::new(());

/// The rules a file keeps, the first broken one named by its field. A hand-edited file is held to
/// them as a save is.
pub fn check(file: &RequestEnvironmentsFile) -> Result<(), String> {
    if file.version != REQUEST_ENVIRONMENTS_VERSION {
        return Err(format!(
            "version is {}, and this Fleet reads {REQUEST_ENVIRONMENTS_VERSION}",
            file.version
        ));
    }
    let mut names = std::collections::BTreeMap::new();
    for (at, one) in file.environment.iter().enumerate() {
        let name = one.name.trim();
        if name.is_empty() {
            return Err(format!("environment[{at}].name is blank"));
        }
        if name.starts_with(SERVER_ENVIRONMENT_PREFIX) {
            return Err(format!(
                "environment[{at}].name `{name}` starts with `{SERVER_ENVIRONMENT_PREFIX}`, which names a server a Job runs"
            ));
        }
        if let Some(first) = names.insert(name, at) {
            return Err(format!("environment[{at}].name `{name}` is already environment[{first}].name"));
        }
        let base = one.base.trim().to_ascii_lowercase();
        if super::host_of(&base).is_err() {
            return Err(format!("environment[{at}].base `{}` is not an http or https address", one.base));
        }
        if base.ends_with('/') {
            return Err(format!("environment[{at}].base `{}` ends in `/`", one.base));
        }
    }
    Ok(())
}

/// What the file's text comes to.
pub fn parse(text: &str) -> Result<RequestEnvironmentsFile, String> {
    let file: RequestEnvironmentsFile = toml::from_str(text).map_err(|why| why.to_string())?;
    check(&file)?;
    Ok(file)
}

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
    fn environments_path(&self) -> PathBuf {
        PathBuf::from(&self.host().kit_home).join(FILE)
    }

    /// The file as it is now.
    pub(crate) async fn environments_read(&self) -> RequestEnvironmentsRead {
        let path = self.environments_path();
        match tokio::fs::read_to_string(&path).await {
            Ok(text) => match parse(&text) {
                Ok(file) => RequestEnvironmentsRead::Read { file },
                Err(why) => RequestEnvironmentsRead::NotRead { why: format!("{}: {why}", path.display()) },
            },
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {
                RequestEnvironmentsRead::Read { file: RequestEnvironmentsFile::default() }
            }
            Err(cause) => RequestEnvironmentsRead::NotRead { why: format!("{}: {cause}", path.display()) },
        }
    }

    /// Check, write whole, and answer with the file read again.
    pub(crate) async fn environments_saved(
        &self,
        file: RequestEnvironmentsFile,
    ) -> Result<RequestEnvironmentsRead, Refusal> {
        check(&file).map_err(|why| Refusal::Unacceptable(WireError::raised(REFUSED, why, self.run_id())))?;
        let path = self.environments_path();
        let unwritable = |why: String| {
            Refusal::Fault(WireError::raised(
                UNWRITABLE,
                format!("{} could not be written: {why}", path.display()),
                self.run_id(),
            ))
        };
        let text = toml::to_string(&file).map_err(|why| unwritable(why.to_string()))?;
        let target = path.clone();
        tokio::task::spawn_blocking(move || write_over(&target, &text))
            .await
            .map_err(|why| unwritable(why.to_string()))?
            .map_err(|why| unwritable(why.to_string()))?;
        Ok(self.environments_read().await)
    }
}

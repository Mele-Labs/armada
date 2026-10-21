//! The headless browser the Playwright tool drives on this Mac. Chrome is not installed here;
//! Playwright's own headless shell is, so a Job's process is told where it is.
//!
//! The shell is looked for at each launch rather than once, so a Job started after the install
//! finishes gets it. `pub` items are what Fleet reads for its start note and its one install.

use std::path::{Path, PathBuf};

use adapter_traits::{Environment, SpawnConfigRefused};

/// What puts a shell on this Mac.
pub const INSTALL_COMMAND: &str = "npx playwright install chromium --only-shell";

/// The folder Playwright keeps its browsers in: its own variable if set, else the Mac's cache.
pub fn browsers_root() -> Option<PathBuf> {
    match std::env::var_os("PLAYWRIGHT_BROWSERS_PATH") {
        Some(path) if !path.is_empty() => Some(PathBuf::from(path)),
        _ => std::env::var_os("HOME").map(|home| Path::new(&home).join("Library/Caches/ms-playwright")),
    }
}

/// The newest installed shell on this Mac, if any.
pub fn found() -> Option<PathBuf> {
    newest_in(&browsers_root()?)
}

/// The executable of the highest-numbered `chromium_headless_shell-<N>` under `root` that has one.
pub fn newest_in(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let number: u64 = entry.file_name().to_str()?.strip_prefix("chromium_headless_shell-")?.parse().ok()?;
            let executable = ["mac-arm64", "mac-x64"]
                .iter()
                .map(|arch| entry.path().join(format!("chrome-headless-shell-{arch}/chrome-headless-shell")))
                .find(|path| path.is_file())?;
            Some((number, executable))
        })
        .max_by_key(|(number, _)| *number)
        .map(|(_, executable)| executable)
}

/// `environment` with the three variables the Playwright tool reads, where there is a shell to
/// name. Where there is none it is returned as it was.
pub fn environment(environment: Environment, shell: Option<&Path>) -> Result<Environment, SpawnConfigRefused> {
    let Some(shell) = shell else {
        return Ok(environment);
    };
    environment
        .and("PLAYWRIGHT_MCP_BROWSER", "chromium")?
        .and("PLAYWRIGHT_MCP_HEADLESS", "true")?
        .and("PLAYWRIGHT_MCP_EXECUTABLE_PATH", &shell.to_string_lossy())
}

/// What the start asked of the install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Install {
    /// A shell was already there, so nothing was run.
    NotNeeded,
    /// The install ran and a shell is there now.
    Installed,
    /// The install ran and left no shell.
    Failed,
}

/// Install a shell with `run` where `root` holds none. `run` is the whole install, and is not
/// called when a shell is found.
pub fn install_if_missing(root: &Path, run: impl FnOnce() -> bool) -> Install {
    if newest_in(root).is_some() {
        return Install::NotNeeded;
    }
    if run() && newest_in(root).is_some() {
        Install::Installed
    } else {
        Install::Failed
    }
}

/// Install a shell on this Mac where it has none, once. Blocks until the install ends, so Fleet
/// calls it off its runtime. `path` is the `PATH` the install runs under, which has to find `npx`.
pub fn ensure_installed(path: &str) -> Install {
    let Some(root) = browsers_root() else {
        return Install::Failed;
    };
    install_if_missing(&root, || {
        std::process::Command::new("npx")
            .args(["-y", "playwright@latest", "install", "chromium", "--only-shell"])
            .env("PATH", path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

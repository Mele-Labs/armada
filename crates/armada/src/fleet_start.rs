//! `armada fleet start`: start the Fleet launchd holds, from whatever state it is in.
//!
//! A Bridge with no Fleet to ask has nothing to press but this. It never boots a loaded job out,
//! so it never restarts a running Fleet: that is `scripts/restart`'s and Bridge's Restart Fleet.
//!
//! | launchd says | Does | Prints |
//! |---|---|---|
//! | the job is not loaded | `launchctl bootstrap` from the plist `scripts/restart` writes | `Fleet was not loaded. Loaded <label> from <plist>.` |
//! | loaded, no pid | `launchctl kickstart` | `Fleet was loaded and not running. Started <label>.` |
//! | loaded, with a pid | nothing | `Fleet is already running as pid <pid>.` |

use std::path::{Path, PathBuf};
use std::process::Command;

/// What `launchctl` is asked, which a test stands in for.
pub trait Launchd {
    /// `launchctl print <service>`: its text, or none when launchd does not hold the job.
    fn print(&self, service: &str) -> Option<String>;
    /// `launchctl bootstrap <domain> <plist>`; launchctl's own error on failure.
    fn bootstrap(&self, domain: &str, plist: &Path) -> Result<(), String>;
    /// `launchctl kickstart <service>`; launchctl's own error on failure.
    fn kickstart(&self, service: &str) -> Result<(), String>;
}

/// What was done, and so what to print.
#[derive(Debug, PartialEq, Eq)]
pub enum Started {
    Loaded { label: String, plist: PathBuf },
    Kickstarted { label: String },
    AlreadyRunning { pid: u32 },
}

impl std::fmt::Display for Started {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Started::Loaded { label, plist } => write!(out, "Fleet was not loaded. Loaded {label} from {}.", plist.display()),
            Started::Kickstarted { label } => write!(out, "Fleet was loaded and not running. Started {label}."),
            Started::AlreadyRunning { pid } => write!(out, "Fleet is already running as pid {pid}."),
        }
    }
}

/// The pid `launchctl print` names for a job that is running; its own line, one tab in.
fn pid_in(printed: &str) -> Option<u32> {
    printed.lines().find_map(|line| line.strip_prefix("\tpid = ")?.trim().parse().ok())
}

/// Start the job `label` in the domain of `uid`, from `plist` when launchd does not hold it.
pub fn start(launchd: &dyn Launchd, uid: u32, label: &str, plist: &Path) -> Result<Started, String> {
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{label}");
    match launchd.print(&service) {
        None => {
            if !plist.is_file() {
                return Err(format!("{label} is not loaded and there is no job file at {}", plist.display()));
            }
            launchd.bootstrap(&domain, plist)?;
            Ok(Started::Loaded { label: label.to_string(), plist: plist.to_path_buf() })
        }
        Some(printed) => match pid_in(&printed) {
            Some(pid) => Ok(Started::AlreadyRunning { pid }),
            None => {
                launchd.kickstart(&service)?;
                Ok(Started::Kickstarted { label: label.to_string() })
            }
        },
    }
}

/// launchd, as `scripts/restart` drives it.
pub struct OnThisMac;

fn ran(args: &[&str]) -> Result<(), String> {
    let out = Command::new("launchctl").args(args).output().map_err(|why| format!("launchctl would not run: {why}"))?;
    match out.status.success() {
        true => Ok(()),
        false => {
            let said = String::from_utf8_lossy(&out.stderr);
            let said = said.trim();
            Err(match said.is_empty() {
                true => format!("launchctl {} exited {}", args.join(" "), out.status),
                false => said.to_string(),
            })
        }
    }
}

impl Launchd for OnThisMac {
    fn print(&self, service: &str) -> Option<String> {
        let out = Command::new("launchctl").args(["print", service]).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn bootstrap(&self, domain: &str, plist: &Path) -> Result<(), String> {
        ran(&["bootstrap", domain, &plist.to_string_lossy()])
    }

    fn kickstart(&self, service: &str) -> Result<(), String> {
        ran(&["kickstart", service])
    }
}

/// The person whose launchd domain the job is in: the owner of their home.
pub fn uid_of(home: &Path) -> Result<u32, String> {
    let held = std::fs::metadata(home).map_err(|why| format!("{}: {why}", home.display()))?;
    Ok(std::os::unix::fs::MetadataExt::uid(&held))
}

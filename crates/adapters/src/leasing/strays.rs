//! What is still running in a slot its session is leaving. A session's agent
//! starts servers, Checks and subagents that outlive its own process, and a
//! slot handed on with them running gives the next tenant a port held and a
//! build written under it.
//!
//! **Found by working directory, and only inside the slot.** `lsof` lists every
//! process's cwd; one at the slot's path or below it is ended, and nothing else
//! is. The test is path components, so `slot-10` is not `slot-1`. This process
//! and its ancestors are spared, so a Fleet or a shell run from inside a slot is
//! never the thing that ends. No `unsafe` here: the signal is `kill(1)`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// How long ended processes are given to be gone before the commit that
/// follows reads the tree.
const GONE_WITHIN: Duration = Duration::from_secs(2);

/// A process that was ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stray {
    pub(crate) pid: u32,
    pub(crate) command: String,
}

/// End every process whose working directory is `slot` or inside it, and
/// answer which. An `lsof` that will not answer ends nothing.
pub(crate) fn end_in(slot: &Path) -> Vec<Stray> {
    let slot = slot.canonicalize().unwrap_or_else(|_| slot.to_path_buf());
    let Some(listing) = cwd_listing() else {
        return Vec::new();
    };
    let spared = spared();
    let found: Vec<Stray> = working_directories(&listing)
        .into_iter()
        .filter(|(pid, _, cwd)| inside(cwd, &slot) && !spared.contains(pid))
        .map(|(pid, command, _)| Stray { pid, command })
        .collect();
    if found.is_empty() {
        return found;
    }
    let mut kill = Command::new("kill");
    kill.arg("-KILL");
    kill.args(found.iter().map(|stray| stray.pid.to_string()));
    let _ = kill.output();
    let until = Instant::now() + GONE_WITHIN;
    while found.iter().any(|stray| running(stray.pid)) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(25));
    }
    found
}

/// Whether `cwd` is `slot` or a path below it, by components.
pub(crate) fn inside(cwd: &Path, slot: &Path) -> bool {
    cwd.starts_with(slot)
}

/// `(pid, command, cwd)` for each process in `lsof -Fpcn` output: a `p` line
/// opens a process, `c` names it, and an `n` line is a directory it holds.
pub(crate) fn working_directories(listing: &str) -> Vec<(u32, String, PathBuf)> {
    let mut found = Vec::new();
    let (mut pid, mut command) = (None, String::new());
    for line in listing.lines() {
        let (tag, rest) = line.split_at(line.chars().next().map_or(0, char::len_utf8));
        match tag {
            "p" => {
                pid = rest.parse::<u32>().ok();
                command.clear();
            }
            "c" => command = rest.to_string(),
            "n" => {
                if let Some(pid) = pid {
                    found.push((pid, command.clone(), PathBuf::from(rest)));
                }
            }
            _ => {}
        }
    }
    found
}

/// `(pid, port)` for each TCP listener in `lsof -Fpn` output: a `p` line opens a process and an
/// `n` line names an address such as `*:3000` or `[::1]:3000`. A process's first port stands.
pub(crate) fn listening(listing: &str) -> Vec<(u32, u16)> {
    let mut found: Vec<(u32, u16)> = Vec::new();
    let mut pid = None;
    for line in listing.lines() {
        let (tag, rest) = line.split_at(line.chars().next().map_or(0, char::len_utf8));
        match tag {
            "p" => pid = rest.parse::<u32>().ok(),
            "n" => {
                let port = rest.rsplit_once(':').and_then(|(_, port)| port.parse::<u16>().ok());
                if let (Some(pid), Some(port)) = (pid, port) {
                    if !found.iter().any(|(held, _)| *held == pid) {
                        found.push((pid, port));
                    }
                }
            }
            _ => {}
        }
    }
    found
}

/// Every process listening on a TCP port, with the port. A failing `lsof` is no ports.
pub fn listening_ports() -> Vec<(u32, u16)> {
    let said = Command::new("lsof")
        .args(["-b", "-w", "-nP", "-iTCP", "-sTCP:LISTEN", "-Fpn"])
        .output();
    match said {
        Ok(said) => listening(&String::from_utf8_lossy(&said.stdout)),
        Err(_) => Vec::new(),
    }
}

fn cwd_listing() -> Option<String> {
    // `-b` and `-w`: no kernel calls that can block on a stale mount, and no warnings.
    let said = Command::new("lsof")
        .args(["-b", "-w", "-d", "cwd", "-Fpcn"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&said.stdout).into_owned();
    // The exit status says nothing: 1 is both "found nothing" and "could not read one".
    (!text.is_empty()).then_some(text)
}

/// This process and every ancestor of it.
fn spared() -> Vec<u32> {
    let mut chain = vec![std::process::id()];
    while let Some(parent) = chain.last().and_then(|pid| parent_of(*pid)) {
        if parent <= 1 || chain.contains(&parent) {
            break;
        }
        chain.push(parent);
    }
    chain
}

fn parent_of(pid: u32) -> Option<u32> {
    let said = Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&said.stdout).trim().parse().ok()
}

/// Whether `pid` is a live process; a zombie waiting to be reaped is not.
fn running(pid: u32) -> bool {
    let said = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output();
    match said {
        Ok(said) => {
            let stat = String::from_utf8_lossy(&said.stdout);
            let stat = stat.trim();
            !stat.is_empty() && !stat.starts_with('Z')
        }
        Err(_) => false,
    }
}

/// End `pids`: `TERM`, then `KILL` for any still running after `grace`. Never this process or an
/// ancestor of it.
pub fn end_pids(pids: &[u32], grace: Duration) {
    let spared = spared();
    let pids: Vec<u32> = pids.iter().copied().filter(|pid| !spared.contains(pid)).collect();
    if pids.is_empty() {
        return;
    }
    signal("-TERM", &pids);
    let until = Instant::now() + grace;
    while pids.iter().any(|pid| running(*pid)) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(25));
    }
    let left: Vec<u32> = pids.into_iter().filter(|pid| running(*pid)).collect();
    if !left.is_empty() {
        signal("-KILL", &left);
    }
}

fn signal(which: &str, pids: &[u32]) {
    let mut kill = Command::new("kill");
    kill.arg(which);
    kill.args(pids.iter().map(u32::to_string));
    let _ = kill.output();
}

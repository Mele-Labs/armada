//! Drones whose Fleet is gone, and ending them. Adoption owns a Fleet's own.
//!
//! **Positive identification only.** `Detached::spawn` puts `ARMADA_SPAWNED_BY=
//! <pid>:<start>` on every process: the spawning Fleet's pid and start time. A
//! group is ended when it leads itself, carries a mark, and the marked process
//! is absent or started at another time. This process, another live Fleet, an
//! unmarked process, an unreadable probe and a recorded Drone's pid are left.
//!
//! **One older road, for Drones spawned before the mark.** A parent-1 process
//! whose argv names `<temp>/armada-fleet-<pid>-<n>/` for a dead `<pid>` is a
//! Fleet test's fake Drone. Nothing else unmarked is touched.
//!
//! **Reading the mark differs by platform.** macOS: `ps eww` appends each
//! process's environment to its row, and hides that of Apple's binaries, so a
//! `/bin/sh` fake Drone is unmarked there; the agent CLI is not. Linux: `ps`
//! does not print the environment in a custom column list, so a group
//! leader's `/proc/<pid>/environ` is read instead.

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::num::NonZeroU32;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::task::JoinHandle;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;
use crate::group::{ask_the_group_to_end, end_the_group};
use crate::process::{holder_of, Holder};

/// The variable [`crate::Detached::spawn`] sets on every process it starts.
pub(crate) const MARK: &str = "ARMADA_SPAWNED_BY";

/// How often the sweep runs after the one at start.
pub const EVERY: Duration = Duration::from_secs(60);

/// Between the polite signal and the other one.
const GRACE: Duration = Duration::from_secs(2);

/// What a process started by this one carries. `None` where the start cannot be
/// read: such a process goes unmarked and is never swept.
pub(crate) fn own_mark() -> Option<String> {
    static OWN: OnceLock<Option<String>> = OnceLock::new();
    OWN.get_or_init(|| match holder_of(std::process::id()) {
        Ok(Holder::Held(started)) => Some(mark_of(std::process::id(), started.as_str())),
        _ => None,
    })
    .clone()
}

/// No whitespace, so the value is one word in `ps`'s output.
pub(crate) fn mark_of(pid: u32, started: &str) -> String {
    let started = started.split_whitespace().collect::<Vec<_>>().join("_");
    format!("{pid}:{started}")
}

/// Sweep at start and then each `every`. `held` is the pids this Fleet's store
/// records a Drone at, which are adoption's (`None` where unreadable); `said`
/// gets one line per group.
pub fn keep_ending_orphans<F>(
    every: Duration,
    held: impl Fn() -> F + Send + 'static,
    said: impl Fn(&str) + Send + 'static,
) -> JoinHandle<()>
where
    F: Future<Output = Option<Vec<u32>>> + Send,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            // A record that will not read is not an empty one: skip the round.
            let Some(held) = held().await else { continue };
            let Ok(ended) = tokio::task::spawn_blocking(move || {
                let found = find(&held).unwrap_or_default();
                end(found, GRACE, || find(&held).unwrap_or_default())
            })
            .await
            else {
                return;
            };
            for group in ended {
                said(&format!(
                    "a Drone group of a Fleet that is gone was ended: {group}"
                ));
            }
        }
    })
}

/// The groups to end: orphans of a Fleet that is gone, minus `held`.
pub(crate) fn find(held: &[u32]) -> io::Result<Vec<NonZeroU32>> {
    let listing = Command::new("ps")
        .args([LISTING_STYLE, "-axo", "pid=,ppid=,pgid=,command="])
        .output()?;
    let temp = temp_prefixes();
    let mut probed: HashMap<u32, Option<String>> = HashMap::new();
    // `started` is what a mark recorded, `None` where only absence counts.
    let mut gone = |pid: u32, started: Option<&str>| -> bool {
        let now = probed.entry(pid).or_insert_with(|| match holder_of(pid) {
            Ok(Holder::Vacant) => Some(String::new()),
            Ok(Holder::Held(at)) => Some(mark_of(pid, at.as_str())),
            Err(_) => None,
        });
        match (now, started) {
            (None, _) => false,
            (Some(now), None) => now.is_empty(),
            (Some(now), Some(started)) => now.is_empty() || *now != mark_of(pid, started),
        }
    };
    let me = std::process::id();
    let mut orphans = Vec::new();
    for row in String::from_utf8_lossy(&listing.stdout)
        .lines()
        .filter_map(Row::parse)
    {
        let Some(group) = NonZeroU32::new(row.pid) else {
            continue;
        };
        if row.pid != row.pgid || row.pid == me || held.contains(&row.pid) {
            continue;
        }
        let ended = match mark_of_process(&row) {
            Some(Some((by, started))) => by != me && gone(by, Some(&started)),
            Some(None) => false,
            None => row.ppid == 1 && names_a_dead_test(row.command, &temp, &mut gone),
        };
        if ended {
            orphans.push(group);
        }
    }
    Ok(orphans)
}

/// Ask each group to end, and after `grace` end those `still` names.
pub(crate) fn end(
    found: Vec<NonZeroU32>,
    grace: Duration,
    still: impl FnOnce() -> Vec<NonZeroU32>,
) -> Vec<NonZeroU32> {
    if found.is_empty() {
        return found;
    }
    for group in &found {
        ask_the_group_to_end(*group);
    }
    std::thread::sleep(grace);
    // Read again: a group that left and a pid that came round is not named.
    for group in still().into_iter().filter(|group| found.contains(group)) {
        end_the_group(group);
    }
    found
}

struct Row<'a> {
    pid: u32,
    ppid: u32,
    pgid: u32,
    command: &'a str,
}

impl<'a> Row<'a> {
    fn parse(line: &'a str) -> Option<Row<'a>> {
        let mut rest = line.trim_start();
        let mut next = || {
            let end = rest.find(char::is_whitespace)?;
            let word = &rest[..end];
            rest = rest[end..].trim_start();
            word.parse::<u32>().ok()
        };
        let (pid, ppid, pgid) = (next()?, next()?, next()?);
        Some(Row {
            pid,
            ppid,
            pgid,
            command: rest,
        })
    }
}

/// `e` appends the environment to each row; only macOS's `ps` honours it
/// beside a column list.
#[cfg(target_os = "macos")]
const LISTING_STYLE: &str = "eww";
#[cfg(not(target_os = "macos"))]
const LISTING_STYLE: &str = "ww";

/// The mark a process carries. `None` is no mark, or an environment that cannot
/// be read; the inner `None` is something there that is not a mark.
#[cfg(target_os = "macos")]
fn mark_of_process(row: &Row) -> Option<Option<(u32, String)>> {
    mark_in(row.command)
}

#[cfg(not(target_os = "macos"))]
fn mark_of_process(row: &Row) -> Option<Option<(u32, String)>> {
    let environment = std::fs::read(format!("/proc/{}/environ", row.pid)).ok()?;
    mark_in_environment(&environment)
}

/// The last mark on the line, since the environment follows argv.
#[cfg(any(target_os = "macos", test))]
fn mark_in(command: &str) -> Option<Option<(u32, String)>> {
    let at = command.rfind(&format!(" {MARK}="))? + MARK.len() + 2;
    Some(parse_mark(
        command[at..].split_whitespace().next().unwrap_or(""),
    ))
}

/// The mark in a NUL-separated environment block, as `/proc/<pid>/environ` has it.
#[cfg(any(not(target_os = "macos"), test))]
pub(crate) fn mark_in_environment(environment: &[u8]) -> Option<Option<(u32, String)>> {
    let prefix = format!("{MARK}=");
    let value = environment
        .split(|byte| *byte == 0)
        .filter_map(|entry| entry.strip_prefix(prefix.as_bytes()))
        .next_back()?;
    Some(parse_mark(&String::from_utf8_lossy(value)))
}

fn parse_mark(value: &str) -> Option<(u32, String)> {
    let (pid, started) = value.split_once(':')?;
    Some((pid.parse().ok()?, started.to_string()))
}

fn temp_prefixes() -> Vec<String> {
    let temp = std::env::temp_dir();
    let mut found = vec![temp.to_string_lossy().trim_end_matches('/').to_string()];
    if let Ok(real) = std::fs::canonicalize(&temp) {
        found.push(real.to_string_lossy().trim_end_matches('/').to_string());
    }
    found
}

/// An argument `<temp>/armada-fleet-<pid>-<n>/..` whose `<pid>` is vacant.
pub(crate) fn names_a_dead_test(
    command: &str,
    temp: &[String],
    gone: &mut impl FnMut(u32, Option<&str>) -> bool,
) -> bool {
    command.split_whitespace().any(|word| {
        let named = temp.iter().find_map(|temp| {
            let rest = word
                .strip_prefix(temp.as_str())?
                .strip_prefix("/armada-fleet-")?;
            let (pid, after) = rest.split_once('-')?;
            let (count, _) = after.split_once('/')?;
            let all_digits = !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit());
            all_digits.then(|| pid.parse::<u32>().ok()).flatten()
        });
        named.is_some_and(|pid| pid != std::process::id() && gone(pid, None))
    })
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
    /// The pids a Drone is recorded at, which adoption owns.
    pub async fn recorded_drone_pids(&self) -> Option<Vec<u32>> {
        let held = self.store().lock().await.drone_processes().ok()?;
        Some(held.into_iter().map(|process| process.pid).collect())
    }
}

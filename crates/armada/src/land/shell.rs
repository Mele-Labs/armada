//! Running one of the swappable binaries — `$ARMADA_LAND_GH`,
//! `$ARMADA_LAND_ARMADA`, `$ARMADA_LAND_FOUNDATIONS` — and turning a spawn
//! failure into a [`Stopped`], the way `scripts/land`'s own `run()` does.
//! `gh`'s JSON answer is decoded through [`ipc::decode`], the same doorway
//! every state file under `land/` already goes through, rather than parsing
//! it directly here.

use std::collections::BTreeSet;
use std::io::{Read, Write as _};
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::stop::Stopped;

/// One command's answer, kept whole rather than split into stdout/stderr at
/// the call site — most callers here read either half depending on what
/// went wrong, exactly as `scripts/land`'s own `run()` result does.
pub struct Ran {
    output: Output,
}

impl Ran {
    pub fn success(&self) -> bool {
        self.output.status.success()
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    /// stdout, falling back to stderr — `not_installed`'s own reading needs
    /// both halves of what a Check printed.
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout(), self.stderr())
    }

    /// The process's own exit code, or `-1` for one that ended by signal.
    pub fn status_code(&self) -> i32 {
        self.output.status.code().unwrap_or(-1)
    }
}

/// Run `argv[0] argv[1..]` in `cwd`, piping `stdin` in if given and
/// appending stdout and stderr to `log` if given. A spawn failure is a
/// [`Stopped`] naming the program and the directory — the way a relative
/// `ARMADA_LAND_ARMADA` is caught in practice.
pub fn run(
    argv: &[&str],
    cwd: &Path,
    stdin: Option<&str>,
    log: Option<&Path>,
) -> Result<Ran, Stopped> {
    let mut command = Command::new(argv[0]);
    command
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let spawn_failed = |cause: std::io::Error| {
        Stopped::stopped(format!(
            "`{}` could not be run from {}: {cause}",
            argv[0],
            cwd.display()
        ))
    };
    let mut child = command.spawn().map_err(spawn_failed)?;
    if let Some(input) = stdin {
        if let Some(mut pipe) = child.stdin.take() {
            let _ = pipe.write_all(input.as_bytes());
        }
    }
    let output = child.wait_with_output().map_err(spawn_failed)?;
    let ran = Ran { output };
    if let Some(path) = log {
        append_log(path, argv, &ran);
    }
    Ok(ran)
}

/// [`run_limited`]'s answer: what ran, and whether `limit` ended it.
pub struct Limited {
    pub ran: Ran,
    pub timed_out: bool,
}

/// How long the output of a run that has ended is waited for: a descendant
/// still holding the pipe would otherwise hold the turn.
const DRAINED_WITHIN: Duration = Duration::from_secs(5);

/// [`run`] with no stdin, killed past `limit` along with every process group
/// its descendants lead: `armada check` puts a Check's command in a group of
/// its own, which killing the child's group alone would miss.
pub fn run_limited(
    argv: &[&str],
    cwd: &Path,
    log: &Path,
    limit: Duration,
) -> Result<Limited, Stopped> {
    let spawn_failed = |cause: std::io::Error| {
        Stopped::stopped(format!(
            "`{}` could not be run from {}: {cause}",
            argv[0],
            cwd.display()
        ))
    };
    let mut child = Command::new(argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(spawn_failed)?;
    let stdout = drained(child.stdout.take());
    let stderr = drained(child.stderr.take());
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(spawn_failed)? {
            break status;
        }
        if started.elapsed() >= limit {
            timed_out = true;
            end_the_tree(child.id());
            break child.wait().map_err(spawn_failed)?;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let until = Instant::now().max(started + limit) + DRAINED_WITHIN;
    let read = |pipe: mpsc::Receiver<Vec<u8>>| {
        pipe.recv_timeout(until.saturating_duration_since(Instant::now()))
            .unwrap_or_default()
    };
    let ran = Ran {
        output: Output {
            status,
            stdout: read(stdout),
            stderr: read(stderr),
        },
    };
    append_log(log, argv, &ran);
    if timed_out {
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(log) {
            let _ = writeln!(file, "[killed at its limit of {}]", spoken(limit));
        }
    }
    Ok(Limited { ran, timed_out })
}

/// A limit as a turn's outcome says it: "15 minutes", "2 seconds".
pub fn spoken(limit: Duration) -> String {
    let seconds = limit.as_secs();
    let (count, unit) = if seconds >= 60 && seconds % 60 == 0 {
        (seconds / 60, "minute")
    } else {
        (seconds, "second")
    };
    format!("{count} {unit}{}", if count == 1 { "" } else { "s" })
}

fn drained(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            let _ = sender.send(bytes);
        });
    }
    receiver
}

/// `SIGKILL` to `root`'s group and every group a descendant of it leads,
/// read from one `ps` listing. Through `kill(1)` because this crate forbids
/// `unsafe`, and never to this process's own group.
fn end_the_tree(root: u32) {
    let listed = Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,pgid="])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
        .unwrap_or_default();
    let rows: Vec<[u32; 3]> = listed
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace().map(str::parse::<u32>);
            Some([
                fields.next()?.ok()?,
                fields.next()?.ok()?,
                fields.next()?.ok()?,
            ])
        })
        .collect();
    let mut tree = vec![root];
    let mut groups = BTreeSet::from([root]);
    let mut at = 0;
    while let Some(&parent) = tree.get(at) {
        for [pid, ppid, pgid] in &rows {
            if *ppid == parent && !tree.contains(pid) {
                tree.push(*pid);
                groups.insert(*pgid);
            }
        }
        at += 1;
    }
    let own = std::process::id();
    for [pid, _, pgid] in &rows {
        if *pid == own {
            groups.remove(pgid);
        }
    }
    groups.retain(|group| *group > 1);
    let _ = Command::new("kill")
        .arg("-KILL")
        .arg("--")
        .args(groups.iter().map(|group| format!("-{group}")))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn append_log(path: &Path, argv: &[&str], ran: &Ran) {
    use std::fs::OpenOptions;
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "$ {}", argv.join(" "));
        let _ = file.write_all(&ran.output.stdout);
        let _ = file.write_all(&ran.output.stderr);
        let _ = writeln!(file, "[exit {}]", ran.status_code());
    }
}

/// What `gh pr view` may answer, across every field list a call site here
/// asks for. **One struct, every field optional**: `gh --json` prints only
/// the fields asked for, so a field this call did not request decodes as
/// `None` rather than failing the whole read.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct GhPullRequest {
    pub number: Option<u64>,
    pub state: Option<String>,
    #[serde(rename = "baseRefName")]
    pub base_ref_name: Option<String>,
    #[serde(rename = "headRefOid")]
    pub head_ref_oid: Option<String>,
    #[serde(rename = "mergeCommit")]
    pub merge_commit: Option<GhMergeCommit>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GhMergeCommit {
    pub oid: String,
}

/// `$ARMADA_LAND_GH pr view <pull_request> --json <fields>`, decoded through
/// [`ipc::decode`] — the one JSON boundary this module crosses, kept to the
/// doorway every state file under `land/` already uses, rather than reading
/// the bytes directly here.
pub fn gh_view(gh: &str, cwd: &Path, pull_request: &str, fields: &str) -> Option<GhPullRequest> {
    let ran = run(
        &[gh, "pr", "view", pull_request, "--json", fields],
        cwd,
        None,
        None,
    )
    .ok()?;
    if !ran.success() {
        return None;
    }
    ipc::decode("gh pr view", ran.stdout().as_bytes()).ok()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{run_limited, spoken};
    use crate::tests::TempDir;

    #[test]
    fn a_run_past_its_limit_is_killed_with_every_group_under_it() {
        let dir = TempDir::new();
        let pid_file = dir.path().join("grandchild.pid");
        let log = dir.path().join("hung.log");
        let script = format!(
            "python3 -c 'import os, time; os.setpgid(0, 0); \
             open(\"{pid}\", \"w\").write(str(os.getpid())); time.sleep(60)' &\n\
             while [ ! -s {pid} ]; do sleep 0.05; done\nsleep 60\n",
            pid = pid_file.display(),
        );
        let started = Instant::now();
        let limited = run_limited(
            &["sh", "-c", &script],
            dir.path(),
            &log,
            Duration::from_secs(2),
        )
        .expect("sh runs");
        assert!(limited.timed_out);
        assert!(!limited.ran.success());
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "{:?}",
            started.elapsed()
        );
        let grandchild = std::fs::read_to_string(&pid_file).expect("the pid");
        assert!(
            gone(grandchild.trim()),
            "a group the child's descendant led outlived the limit"
        );
        let logged = std::fs::read_to_string(&log).expect("the log");
        assert!(
            logged.contains("[killed at its limit of 2 seconds]"),
            "{logged}"
        );
    }

    #[test]
    fn a_run_inside_its_limit_answers_as_run_does() {
        let dir = TempDir::new();
        let limited = run_limited(
            &["sh", "-c", "echo out; echo err >&2; exit 3"],
            dir.path(),
            &dir.path().join("quick.log"),
            Duration::from_secs(30),
        )
        .expect("sh runs");
        assert!(!limited.timed_out);
        assert_eq!(limited.ran.status_code(), 3);
        assert_eq!(limited.ran.combined(), "out\nerr\n");
    }

    #[test]
    fn a_limit_is_said_in_whole_minutes_where_it_is_one() {
        assert_eq!(spoken(Duration::from_secs(15 * 60)), "15 minutes");
        assert_eq!(spoken(Duration::from_secs(90)), "90 seconds");
        assert_eq!(spoken(Duration::from_secs(1)), "1 second");
    }

    fn gone(pid: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let alive = std::process::Command::new("kill")
                .args(["-0", pid])
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if !alive {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }
}

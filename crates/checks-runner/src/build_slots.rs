//! One pool of compiler slots for the whole machine, handed to every build
//! Fleet starts. `docs/concepts/manifest.md`, *How wide a Check runs*.
//!
//! **A GNU make jobserver on a named fifo.** Cargo and rustc take a token from
//! it before each job, so five builds share the pool instead of each starting
//! one job per core. A fifo, not an inherited pipe, because Fleet starts agents
//! detached and a Drone's own builds begin hours after its spawn.
//!
//! **A build is one implicit job plus what it takes from the fifo**, so the
//! fifo holds one fewer than the machine's cores and a lone build gets them
//! all. Each further build adds its implicit job: `k` builds peak at
//! `cores - 1 + k`.
//!
//! **A killed build loses its tokens for good**, which is measured: `kill -9`
//! on a cargo holding all four of four left an empty fifo. Tokens are anonymous
//! bytes, so the only repair is [`BuildSlots::settle`]: when the pool is short
//! and no compiler is running, what is missing was lost, and it is written back.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

use crate::Priority;

/// What a process names itself when it holds tokens: cargo takes one per job
/// and rustc takes them for its own codegen threads.
const COMPILERS: [&str; 3] = ["cargo", "rustc", "rustdoc"];

/// Consecutive looks at a short pool with no compiler running before the
/// shortfall is called lost. One look would write tokens back in the gap
/// between a build's `cargo` exiting and its tokens landing in the fifo.
const LOOKS_BEFORE_LOST: u32 = 2;

static OFFERED: OnceLock<String> = OnceLock::new();

/// Hand the pool to every build this process starts from here on: `flags` is
/// what [`BuildSlots::flags`] returns. Once per process; a second offer is
/// ignored, so a Fleet's pool cannot be swapped under a running build.
pub fn offer(flags: String) {
    let _ = OFFERED.set(flags);
}

/// The pool this process offers, as the value of `MAKEFLAGS`.
pub fn offered() -> Option<&'static str> {
    OFFERED.get().map(String::as_str)
}

/// `CARGO_BUILD_JOBS` for a build that has no pool, and nothing for one that
/// does. **Cargo ignores the variable and `-j` when it inherits a jobserver**
/// (`-j` with a warning), so this is the fallback and never a ceiling.
pub fn build_cap(width: crate::CheckWidth) -> Option<(String, String)> {
    let inherited = std::env::var("MAKEFLAGS").is_ok_and(|flags| flags.contains("--jobserver-auth"));
    if offered().is_some() || inherited {
        return None;
    }
    Some((
        "CARGO_BUILD_JOBS".to_string(),
        width.get().to_string(),
    ))
}

/// What one [`BuildSlots::settle`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    /// Tokens in the fifo when looked at.
    pub free: u32,
    /// Written back because a build that held them was gone.
    pub restored: u32,
    /// Dropped because a late return had put the pool over its size.
    pub trimmed: u32,
    /// The fifo had to be made again.
    pub recreated: bool,
}

pub struct BuildSlots {
    path: PathBuf,
    tokens: u32,
    // Held open read-write so the fifo always has a writer and a reader, and a
    // read on it returns "empty" instead of end-of-file.
    held: File,
    short_looks: AtomicU32,
}

impl BuildSlots {
    /// The pool at `path`, sized so a lone build gets `cores` jobs.
    ///
    /// **A fifo already there is kept**, with what builds still running hold
    /// of it: a Fleet that crashed leaves its Drones building, and a fresh pool
    /// would hand their share out twice. Anything else at the path is replaced.
    ///
    /// **A fifo nobody has open loses what it held**, so a Fleet that died with
    /// no build running leaves an empty one. Where no compiler runs, nothing
    /// holds a token and the pool is filled whatever it found.
    pub fn open(path: &Path, cores: u32) -> io::Result<BuildSlots> {
        BuildSlots::open_asking(path, cores, compiler_running)
    }

    pub(crate) fn open_asking(
        path: &Path,
        cores: u32,
        compiler_running: impl FnOnce() -> bool,
    ) -> io::Result<BuildSlots> {
        if path.to_string_lossy().contains(char::is_whitespace) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a jobserver path with a space cannot be written into MAKEFLAGS",
            ));
        }
        let tokens = cores.max(2) - 1;
        let reused = make_fifo(path)?;
        let held = open_held(path)?;
        let slots = BuildSlots {
            path: path.to_path_buf(),
            tokens,
            held,
            short_looks: AtomicU32::new(0),
        };
        if !reused || !compiler_running() {
            slots.take_all()?;
            slots.put(tokens)?;
        }
        Ok(slots)
    }

    /// The value for `MAKEFLAGS`.
    pub fn flags(&self) -> String {
        format!("-j --jobserver-auth=fifo:{}", self.path.display())
    }

    pub fn size(&self) -> u32 {
        self.tokens
    }

    /// Look at the pool and repair it. `compiler_running` is asked only when
    /// the pool is short, and a shortfall is only called lost after
    /// [`LOOKS_BEFORE_LOST`] looks in a row that found no compiler.
    pub fn settle(&mut self, compiler_running: impl FnOnce() -> bool) -> io::Result<Settled> {
        let recreated = !is_fifo(&self.path);
        if recreated {
            make_fifo(&self.path)?;
            self.held = open_held(&self.path)?;
        }
        let free = self.take_all()?;
        let (keep, restored, trimmed) = if free > self.tokens {
            self.short_looks.store(0, Ordering::Relaxed);
            (self.tokens, 0, free - self.tokens)
        } else if free < self.tokens {
            let idle = !compiler_running();
            let looks = if idle {
                self.short_looks.fetch_add(1, Ordering::Relaxed) + 1
            } else {
                self.short_looks.store(0, Ordering::Relaxed);
                0
            };
            if recreated || looks >= LOOKS_BEFORE_LOST {
                self.short_looks.store(0, Ordering::Relaxed);
                (self.tokens, self.tokens - free, 0)
            } else {
                (free, 0, 0)
            }
        } else {
            self.short_looks.store(0, Ordering::Relaxed);
            (free, 0, 0)
        };
        self.put(keep)?;
        Ok(Settled {
            free,
            restored,
            trimmed,
            recreated,
        })
    }

    /// Read every token that is there. A build waiting on the fifo waits the
    /// microseconds until [`BuildSlots::put`] returns them.
    fn take_all(&self) -> io::Result<u32> {
        let mut taken = 0;
        let mut buffer = [0u8; 64];
        loop {
            match (&self.held).read(&mut buffer) {
                Ok(0) => return Ok(taken),
                Ok(n) => taken += n as u32,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(taken),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }

    fn put(&self, count: u32) -> io::Result<()> {
        (&self.held).write_all(&vec![b'+'; count as usize])
    }
}

/// Settle `slots` every `every` until the runtime ends, telling `said` what a
/// look repaired. The pool is owned by the task, so nothing else can write to
/// it between looks.
pub fn keep(
    mut slots: BuildSlots,
    every: std::time::Duration,
    said: impl Fn(Settled) + Send + 'static,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(every).await;
            let Ok((held, settled)) = tokio::task::spawn_blocking(move || {
                let settled = slots.settle(compiler_running);
                (slots, settled)
            })
            .await
            else {
                return;
            };
            slots = held;
            if let Ok(done) = settled {
                if done.restored + done.trimmed > 0 || done.recreated {
                    said(done);
                }
            }
        }
    })
}

/// Whether a compiler is running anywhere on this machine, by name.
///
/// **Any compiler, not only Fleet's.** A build started outside Fleet holds no
/// token of this pool, and counting it only delays a repair that a quiet
/// machine makes a moment later. When the table cannot be read, one is assumed
/// running: tokens are written back late rather than twice.
pub fn compiler_running() -> bool {
    let (program, args) = Priority::Low.command(
        "ps".to_string(),
        vec!["-axo".to_string(), "comm=".to_string()],
        Path::new("/"),
    );
    let Ok(listing) = Command::new(program).args(args).output() else {
        return true;
    };
    if !listing.status.success() {
        return true;
    }
    names_a_compiler(&String::from_utf8_lossy(&listing.stdout))
}

/// Whether a `ps -o comm=` listing names a compiler, by the last part of its path.
fn names_a_compiler(listing: &str) -> bool {
    listing
        .lines()
        .filter_map(|line| line.trim().rsplit('/').next())
        .any(|name| COMPILERS.contains(&name))
}

#[cfg(test)]
pub(crate) fn names_a_compiler_for_test(listing: &str) -> bool {
    names_a_compiler(listing)
}

fn is_fifo(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|held| held.file_type().is_fifo())
}

/// Make the fifo, or find it already made. `true` when it was already there.
fn make_fifo(path: &Path) -> io::Result<bool> {
    if is_fifo(path) {
        return Ok(true);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if std::fs::symlink_metadata(path).is_ok() {
        std::fs::remove_file(path)?;
    }
    let made = Command::new("/usr/bin/mkfifo")
        .args(["-m", "600"])
        .arg(path)
        .status()?;
    if made.success() {
        Ok(false)
    } else {
        Err(io::Error::other(format!("mkfifo exited {made}")))
    }
}

fn open_held(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
}

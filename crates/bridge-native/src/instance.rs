//! One native Bridge at a time, on the build launchd keeps current.
//!
//! The live app takes an exclusive lock on `<support>/bridge-native.lock` and writes its pid, executable and
//! whether launchd started it into that file. A second start reads the holder's record and either hands the
//! person over to the holder or, when launchd's own start finds a stray from some other bundle, asks that
//! stray to quit. The lock is the OS's `flock`, so it frees itself when the holder dies, however it dies.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

use crate::log;

/// What the holder wrote about itself.
#[derive(Debug, PartialEq, Eq)]
pub struct Holder {
    pub pid: i32,
    pub exe: PathBuf,
    /// Started by launchd through `scripts/bridge-launch`, which sets `ARMADA_BRIDGE_QUIT_MARK`.
    pub launchd: bool,
}

impl Holder {
    fn record(&self) -> String {
        format!("{}\n{}\n{}\n", self.pid, self.exe.display(), self.launchd)
    }

    fn parse(text: &str) -> Option<Holder> {
        let mut lines = text.lines();
        let pid = lines.next()?.parse().ok()?;
        let exe = PathBuf::from(lines.next()?);
        let launchd = lines.next()?.parse().ok()?;
        Some(Holder { pid, exe, launchd })
    }
}

/// What a start that finds the lock held does about it.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Bring the holder forward and leave.
    HandOff,
    /// Ask the holder to quit and take its place.
    Replace,
}

/// A start from launchd that finds a holder launchd did not start has found a stray, usually an old bundle
/// Launch Services opened; every other pair leaves the holder where it is.
pub fn verdict(newcomer_launchd: bool, holder_launchd: bool) -> Verdict {
    if newcomer_launchd && !holder_launchd {
        Verdict::Replace
    } else {
        Verdict::HandOff
    }
}

/// How long a start waits for a holder it asked to quit.
const QUIT_WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(100);

/// Where the lock lives: beside the app's other files.
pub fn lock_file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("bridge-native.lock"))
}

/// The lock taken and the record written, or the holder that has it.
pub fn take(path: &Path, me: &Holder) -> std::io::Result<Result<File, Option<Holder>>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
    match file.try_lock() {
        Ok(()) => {
            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(me.record().as_bytes())?;
            file.flush()?;
            Ok(Ok(file))
        }
        Err(TryLockError::WouldBlock) => {
            // The holder may have the lock and not yet its record; a moment later it does.
            for _ in 0..20 {
                let mut text = String::new();
                file.seek(SeekFrom::Start(0))?;
                file.read_to_string(&mut text)?;
                if let Some(holder) = Holder::parse(&text) {
                    return Ok(Err(Some(holder)));
                }
                std::thread::sleep(POLL);
            }
            Ok(Err(None))
        }
        Err(TryLockError::Error(why)) => Err(why),
    }
}

/// This process as the lock records it.
fn myself(quit_mark: &Option<PathBuf>) -> Holder {
    let exe = std::env::current_exe().and_then(|exe| exe.canonicalize()).unwrap_or_default();
    Holder { pid: std::process::id() as i32, exe, launchd: quit_mark.is_some() }
}

/// What `main` does once the lock is settled.
pub enum Claim {
    /// This process is the instance; the file keeps the lock until the process ends.
    Instance(Option<File>),
    /// Another instance stands; leave with success.
    Leave,
}

/// Take the lock, or settle with the holder. `quit_mark` is `ARMADA_BRIDGE_QUIT_MARK`.
pub fn claim(quit_mark: Option<PathBuf>) -> Claim {
    let Some(path) = lock_file() else { return Claim::Instance(None) };
    let me = myself(&quit_mark);
    let mut taken = take(&path, &me);
    let leave = |holder: Option<&Holder>| {
        // launchd reopens a process that exits with no mark, so a start that leaves says it meant to.
        if let Some(mark) = &quit_mark {
            let _ = std::fs::write(mark, "");
        }
        if let Some(holder) = holder {
            bring_forward(holder.pid);
        }
        Claim::Leave
    };
    match taken {
        Err(why) => {
            log::Note::warn("the instance lock could not be taken").field("why", why.to_string()).emit();
            Claim::Instance(None)
        }
        Ok(Ok(file)) => Claim::Instance(Some(file)),
        Ok(Err(None)) => {
            log::Note::warn("another instance holds the lock and has not said who it is").emit();
            leave(None)
        }
        Ok(Err(Some(holder))) => match verdict(me.launchd, holder.launchd) {
            Verdict::HandOff => {
                log::Note::info("another instance stands, so it is brought forward").field("pid", holder.pid as u32).field("exe", holder.exe.display().to_string()).emit();
                leave(Some(&holder))
            }
            Verdict::Replace => {
                log::Note::info("a stray instance holds the lock, so it is asked to quit").field("pid", holder.pid as u32).field("exe", holder.exe.display().to_string()).emit();
                ask_to_quit(holder.pid);
                let until = Instant::now() + QUIT_WAIT;
                loop {
                    taken = take(&path, &me);
                    if !matches!(taken, Ok(Err(_))) || Instant::now() >= until {
                        break;
                    }
                    std::thread::sleep(POLL);
                }
                match taken {
                    Ok(Ok(file)) => Claim::Instance(Some(file)),
                    Ok(Err(_)) | Err(_) => {
                        log::Note::warn("a stray instance did not quit, so this start leaves it").field("pid", holder.pid as u32).emit();
                        leave(None)
                    }
                }
            }
        },
    }
}

fn bring_forward(pid: i32) {
    if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
    }
}

fn ask_to_quit(pid: i32) {
    if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        app.terminate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holder(pid: i32, launchd: bool) -> Holder {
        Holder { pid, exe: PathBuf::from("/x/builds/s/Armada.app/Contents/MacOS/Armada"), launchd }
    }

    fn lock_in_temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("armada-instance-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("bridge-native.lock")
    }

    #[test]
    fn only_launchds_own_start_replaces_a_holder_launchd_did_not_start() {
        assert_eq!(verdict(true, false), Verdict::Replace);
        assert_eq!(verdict(true, true), Verdict::HandOff);
        assert_eq!(verdict(false, true), Verdict::HandOff);
        assert_eq!(verdict(false, false), Verdict::HandOff);
    }

    #[test]
    fn a_holders_record_is_read_back_as_written() {
        let me = holder(4242, true);
        assert_eq!(Holder::parse(&me.record()), Some(me));
        assert_eq!(Holder::parse(""), None);
        assert_eq!(Holder::parse("4242\n/x\nmaybe\n"), None);
    }

    #[test]
    fn a_second_start_finds_the_first_and_a_start_after_it_ends_takes_the_lock() {
        let path = lock_in_temp("lock");
        let first = holder(111, false);
        let held = take(&path, &first).unwrap().expect("the first start takes the lock");

        let seen = take(&path, &holder(222, true)).unwrap().expect_err("the lock is held");
        assert_eq!(seen, Some(first), "the second start reads who holds it, and leaves the record as it was");

        drop(held);
        let third = holder(333, true);
        let _taken = take(&path, &third).unwrap().expect("a lock its holder let go is free");
        let record = std::fs::read_to_string(&path).unwrap();
        assert_eq!(Holder::parse(&record), Some(third), "the new holder's record replaces the old");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

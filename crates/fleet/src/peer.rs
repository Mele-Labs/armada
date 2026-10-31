//! Which process holds a connection, and what it started.
//!
//! **The pair, never the port.** A local port is not unique on a host, so a
//! lookup keyed on the peer's port alone names the wrong pid whenever it meets
//! the impostor first — deterministically, not as a race, which is why a test
//! that passes against it proves nothing. Matching `insi_lport` *and*
//! `insi_fport` was right in every ordering.
//! `docs/spikes/012-peer-identity-under-concurrency.md` holds that
//! reproduction, the four routes it timed — `proc_pidfdinfo` at 22µs against
//! `lsof` at 64ms, and it is the only one that can match a pair — and the 384
//! connections engineered to lose their peer, of which **none came back naming
//! another process**.
//!
//! Absent is therefore the only failure, and a refusal is not a guess. It is
//! why the lookup runs on the call rather than at accept — a call being served
//! is a connection that is open.

use api::Caller;

/// Whether a process holds a particular TCP connection.
///
/// **A trait so the matching can be tested against something other than the
/// kernel**, and implemented once — [`Kernel`] — because there is one right
/// answer and it is a syscall.
pub trait PeerOf: Send + Sync {
    /// Whether `pid` holds a TCP socket whose local port is `from` and whose
    /// foreign port is `to`.
    ///
    /// **Both, never one.** A local port is not unique on a host; see this
    /// module's header for the measurement, and for what matching on one alone
    /// gets wrong.
    fn holds(&self, pid: u32, from: u16, to: u16) -> bool;

    /// The processes `pid` started that are still running. Empty on any
    /// failure, the direction [`held_within`] may fail in.
    fn children(&self, pid: u32) -> Vec<u32>;

    /// The file `pid` is running. `None` where it cannot be said, which reads as not one of the
    /// session's own.
    fn executable(&self, _pid: u32) -> Option<String> {
        None
    }

    /// Whether `pid` was started with `key=value` in its environment. `false` where it cannot be
    /// said, as for a platform binary such as `/bin/zsh`, whose environment the kernel withholds.
    fn carries(&self, _pid: u32, _key: &str, _value: &str) -> bool {
        false
    }
}

/// Whether `pid` runs a file under one of `directories`. **A daemon leaves its parent's tree**
/// (a dev Fleet is `setsid`ed and reparented to launchd), so a session that started one can no
/// longer reach it by descent; the binary it built in its own slot is what still says whose it is.
pub fn runs_from(pid: u32, directories: &[String], peers: &dyn PeerOf) -> bool {
    let Some(file) = peers.executable(pid) else {
        return false;
    };
    let file = std::fs::canonicalize(&file).unwrap_or_else(|_| file.into());
    directories.iter().any(|directory| {
        let directory = std::fs::canonicalize(directory).unwrap_or_else(|_| directory.into());
        file.starts_with(directory)
    })
}

/// How deep and how wide [`held_within`] walks. A Helm session's relay is its
/// agent's child; the bound is there so a runaway tree cannot stall a call.
const DEEPEST: usize = 6;
const MOST: usize = 256;

/// Whether a call arrived on a connection held by one of `roots` or by a
/// process one of them started. `#941`.
///
/// **Descendants, because a Helm session reaches the door through a relay its
/// agent started**, and that relay holds the socket. A process outside the tree
/// cannot join it: a parent is the kernel's record, not a process's claim.
pub fn held_within(caller: &Caller, served_on: u16, roots: &[u32], peers: &dyn PeerOf) -> bool {
    let Some(from) = caller.port() else {
        return false;
    };
    let mut seen: Vec<u32> = Vec::new();
    let mut level = roots.to_vec();
    for _ in 0..DEEPEST {
        let mut below = Vec::new();
        for pid in level {
            if pid == 0 || seen.contains(&pid) || seen.len() >= MOST {
                continue;
            }
            if peers.holds(pid, from, served_on) {
                return true;
            }
            seen.push(pid);
            below.extend(peers.children(pid));
        }
        if below.is_empty() {
            break;
        }
        level = below;
    }
    false
}

/// Whether `pid` is one of `roots` or a process one of them started, however
/// deep. **The same walk as [`held_within`]**, for a lease record's holder
/// rather than a connection's.
pub fn started_within(roots: &[u32], pid: u32, peers: &dyn PeerOf) -> bool {
    let mut seen: Vec<u32> = Vec::new();
    let mut level = roots.to_vec();
    for _ in 0..DEEPEST {
        let mut below = Vec::new();
        for each in level {
            if each == 0 || seen.contains(&each) || seen.len() >= MOST {
                continue;
            }
            if each == pid {
                return true;
            }
            seen.push(each);
            below.extend(peers.children(each));
        }
        if below.is_empty() {
            break;
        }
        level = below;
    }
    false
}

pub use self::kernel::Kernel;

#[cfg(target_vendor = "apple")]
mod kernel;

#[cfg(not(target_vendor = "apple"))]
mod kernel {
    /// Nothing to ask. Armada is a macOS application — see
    /// `docs/contracts/system-architecture.md` — and this exists so the
    /// workspace still builds where somebody is reading it rather than running
    /// it. **It answers `false` to everything**, which makes every call
    /// unattributable, which is the refusal rather than a wrong Job.
    #[derive(Debug, Default)]
    pub struct Kernel;

    impl super::PeerOf for Kernel {
        fn holds(&self, _pid: u32, _from: u16, _to: u16) -> bool {
            false
        }

        fn children(&self, _pid: u32) -> Vec<u32> {
            Vec::new()
        }
    }
}

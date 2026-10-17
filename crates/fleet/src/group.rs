//! The process group a Drone was detached into: reading it, and ending it.
//!
//! **The mirror of `crate::detach`.** That file makes the group — one
//! `libc::setsid()` between fork and exec, so a Drone leads a session and a
//! process group whose id is its own pid. This one is the only thing that
//! speaks to that group afterwards, and it exists as a file of its own so the
//! exemption the gate grants is the two calls below rather than everything
//! `crate::session` does.
//!
//! **Both calls take a [`NonZeroU32`], and that is the safety argument at the
//! call site.** Zero is the *caller's* own group to a group-directed call —
//! Fleet, and every process Fleet has not detached — so the number that would
//! turn one Drone's ending into a fleet-wide kill cannot be spelled. The pid
//! being non-zero is not enough on its own: it also has to still be this
//! Drone's, and **the two callers prove that two different ways.**
//!
//! [`DroneSession::group`](crate::session::DroneSession) proves it by holding
//! an uncollected child on the pid: while the child has not been waited on the
//! operating system cannot hand the number to anybody else. `crate::adopting`
//! cannot — an adopted Drone is not this process's child and there is nothing
//! to hold — so it proves it by asking [`crate::holder_of`] when the process at
//! that pid started and comparing the answer against what was recorded at the
//! spawn. That is a weaker claim by exactly the width of `ps`'s second
//! resolution, which is stated where it bites at
//! [`Holder::Held`](crate::Holder).

use std::num::NonZeroU32;

/// libc exposes `si_pid` as an unsafe method on Linux and a field on macOS.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
unsafe fn si_pid_of(reading: &libc::siginfo_t) -> libc::pid_t {
    reading.si_pid()
}

/// `SIGKILL` to a whole process group.
///
/// `SIGKILL` rather than `SIGTERM`, which is `checks-runner`'s reasoning in the
/// other place this workspace signals a group: the decision to end has already
/// been taken, and a graceful window would be a second one nobody configured.
///
/// **A group that is already gone is not an error.** `killpg` answers `ESRCH`
/// for a Drone that exited between the reading above and this line, which is
/// the ordinary case rather than a fault — so nothing is returned and nothing
/// is logged. What happened to the Drone itself is on the `wait` its caller
/// does next.
#[allow(unsafe_code)]
pub(crate) fn end_the_group(group: NonZeroU32) {
    // A pid the platform cannot express is not a group, and `try_from` says so
    // rather than a cast quietly making one up.
    let Ok(group) = libc::pid_t::try_from(group.get()) else {
        return;
    };
    // SAFETY: `killpg` is a plain system call over two integers, and this one
    // is the pid `libc::setsid()` in `crate::detach` made a group leader — so
    // the signal reaches that Drone and what it spawned, and stops there. Both
    // Drone callers hold an uncollected child on the pid, which is what stops
    // it naming anything else. The third, `crate::servers::left`, holds none:
    // it signals only after `crate::process::holder_of` finds the process at
    // the pid started when its record says, so a reused pid is never named.
    // `crate::orphans` signals a group only after reading the mark of a Fleet
    // that is gone from the process itself. Under test there is another,
    // `crate::tests::tmp::TempDir`, ending what a test spawned in its
    // directory; it holds no proof, and says why there.
    unsafe {
        libc::killpg(group, libc::SIGKILL);
    }
}

/// `SIGTERM` to a whole process group: the first of the two asks
/// `crate::orphans` makes of a group whose Fleet is gone, the second being
/// [`end_the_group`] once a grace has passed.
///
/// **The caller proves the group is an orphan's**, by the mark it carries or the
/// directory its argv names, read immediately before — never by a pid alone.
#[allow(unsafe_code)]
pub(crate) fn ask_the_group_to_end(group: NonZeroU32) {
    let Ok(group) = libc::pid_t::try_from(group.get()) else {
        return;
    };
    // SAFETY: as `end_the_group`: two integers, non-zero, naming one group.
    unsafe {
        libc::killpg(group, libc::SIGTERM);
    }
}

/// `SIGINT` to a whole process group: **asking a scout to stop, not ending
/// it.** Spike 017 measured the agent CLI ending its turn on an interrupt and
/// reporting its cost, where `SIGTERM` left no report at all. The caller holds
/// the uncollected child, for [`end_the_group`]'s reason, and ends the group
/// that way if the interrupt is not heeded.
#[allow(unsafe_code)]
pub(crate) fn interrupt_the_group(group: NonZeroU32) {
    let Ok(group) = libc::pid_t::try_from(group.get()) else {
        return;
    };
    // SAFETY: as `end_the_group`: two integers, and a group leader the caller
    // still holds as an uncollected child, so the number is still its own.
    unsafe {
        libc::killpg(group, libc::SIGINT);
    }
}

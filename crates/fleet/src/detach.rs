//! Spawning a child that outlives this process, and no other kind of spawn.
//!
//! **A type rather than a helper function.** Every Drone is spawned into its
//! own session, because a supervisor signals a job's whole process tree and a
//! Drone spawned as a plain child dies at every Fleet restart — silently,
//! mid-Job, burning tokens against a real repository. That is the opposite of
//! the killing Fleet does do, which is deliberate, at a cap, on a Drone burning
//! without converging. A helper a caller remembers to use is the convention
//! failure this codebase exists to remove, so there is no `Command` in this
//! crate's public surface at all: [`Detached`] is the only way to start a
//! process from Fleet and applies the call in its constructor. A caller cannot
//! spawn an attached child because the call does not exist.
//!
//! **No `process_group` on this type**, and no way to reach the underlying
//! `Command` and set one. Not tidiness: setting a process group and creating a
//! session are mutually exclusive, and a spawn asking for both fails at
//! `pre_exec` with `Operation not permitted` — the caller having become a
//! process-group leader is precisely why the session call then refuses. v1
//! measured that. One flag, and it is not optional, so the pair cannot be asked
//! for.
//!
//! **`libc` and not a shell**, because macOS ships no `/usr/bin/setsid`: a
//! library call between fork and exec rather than a wrapper program — one of
//! the `unsafe` sites `xtask/src/rules_unsafe.rs` names, and long since not the
//! only one. The group it makes is ended in `crate::group`.

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::Stdio;

use adapter_traits::{Environment, Launch};
use tokio::process::{Child, Command};

/// A child that will be spawned into a session of its own.
///
/// Cannot be built any other way, and cannot be turned back into a plain
/// command. The surface below is what Fleet needs to start a Drone and nothing
/// more — a new requirement is a new named method, never a raw argv escape
/// hatch, for the same reason `DroneSpawnConfig` has none.
pub struct Detached {
    command: Command,
    /// Leave `crate::orphans`' mark off: see [`Detached::outliving_fleet`].
    unmarked: bool,
    /// Where a test's child runs, so the test's own directory can end it.
    /// `crate::tests::tmp` holds the reason; a build that ships has no field.
    #[cfg(test)]
    directory: Option<std::path::PathBuf>,
    /// A test's stand-in for another Fleet's mark, in place of this process's.
    #[cfg(test)]
    marked_by: Option<String>,
}

impl Detached {
    /// A command that will detach when it is spawned.
    ///
    /// The session call is attached here rather than in [`Detached::spawn`], so
    /// there is no window in which a partially configured command exists
    /// without it.
    pub fn program(program: impl AsRef<OsStr>) -> Detached {
        let mut command = Command::new(program);
        detach(&mut command);
        // Stdin is null unless a caller asks for it, rather than inherited
        // unless a caller remembers to close it. v1's Drone got the same, and
        // it is the one part of v1's Drone spawn that needed no change.
        command.stdin(Stdio::null());
        Detached {
            command,
            unmarked: false,
            #[cfg(test)]
            directory: None,
            #[cfg(test)]
            marked_by: None,
        }
    }

    /// Carry the mark of a Fleet that is not this process, so a test can spawn
    /// a Drone whose Fleet is gone, or another live one's.
    #[cfg(test)]
    pub(crate) fn marked_by(mut self, mark: &str) -> Detached {
        self.marked_by = Some(mark.to_string());
        self
    }

    /// Everything a harness rendered, in one call.
    ///
    /// **The four halves go on together or not at all.** A caller that set the
    /// program and the arguments and forgot the directory would put a Drone in
    /// Fleet's own working directory; one that forgot the environment would
    /// hand it Fleet's. Taking a [`Launch`] means neither is a thing to
    /// remember, and a `Launch` can only be built from a spawn config.
    pub fn launching(launch: &Launch) -> Detached {
        Detached::program(launch.program())
            .args(launch.args())
            .in_directory(launch.directory())
            .in_environment(launch.environment())
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Detached {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Detached
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    /// The worktree the child runs in.
    pub fn in_directory(mut self, directory: impl AsRef<Path>) -> Detached {
        #[cfg(test)]
        {
            self.directory = Some(directory.as_ref().to_path_buf());
        }
        self.command.current_dir(directory);
        self
    }

    /// The child's whole environment.
    ///
    /// **Clears first, always, and there is no method that adds one variable to
    /// an inherited environment.** `Command::env` layers over the parent's, so
    /// a builder offering it would make wholesale inheritance the default and
    /// clearing the thing a caller has to remember — which is exactly what v1
    /// did, in the one place its Drone spawn was worse than its check spawn: a
    /// token exported in the operator's shell reached every Drone it started.
    ///
    /// The type of the argument is what carries the guarantee. An
    /// [`Environment`] is built from [`Environment::nothing`] up, so what
    /// arrives here was named a variable at a time by whoever built it.
    pub fn in_environment(mut self, environment: &Environment) -> Detached {
        self.command.env_clear();
        for (name, value) in environment.vars() {
            self.command.env(name, value);
        }
        self
    }

    /// Hold the child's input open, so the caller can write to it.
    ///
    /// The default is [`Stdio::null`], set in the constructor: a child that
    /// inherited Fleet's own stdin could read whatever Fleet was given, and a
    /// detached child has no terminal for it to be anyway.
    pub fn piping_input(mut self) -> Detached {
        self.command.stdin(Stdio::piped());
        self
    }

    /// Carry no mark of this Fleet, so `crate::orphans` does not end the child
    /// when this Fleet is gone. **Only a session's keeper**
    /// (`crate::session_host::keeper`), whose whole job is to outlive Fleet and
    /// be found again by the next one. It ends itself when it has no Fleet for
    /// too long. What it starts is marked with *its* pid, so the agent beneath
    /// it is swept if the keeper dies.
    pub fn outliving_fleet(mut self) -> Detached {
        self.unmarked = true;
        self
    }

    /// Stdout and stderr go nowhere, so a caller waiting for Fleet's output
    /// pipe to close (a launcher script capturing it) is not held by the child.
    pub fn ignoring_output(mut self) -> Detached {
        self.command.stdout(Stdio::null()).stderr(Stdio::null());
        self
    }

    /// Stdout and stderr both appended to `file`, for a child nobody reads
    /// from but whose last words someone may want.
    pub fn writing_output_to(mut self, file: &std::fs::File) -> io::Result<Detached> {
        self.command.stdout(Stdio::from(file.try_clone()?));
        self.command.stderr(Stdio::from(file.try_clone()?));
        Ok(self)
    }

    pub fn capturing_output(mut self) -> Detached {
        self.command.stdout(Stdio::piped()).stderr(Stdio::piped());
        self
    }

    /// Start it. The child is a session leader before its program is running,
    /// so nothing signalled at Fleet's process group reaches it.
    ///
    /// **Under test, the child is also handed to the directory it runs in**, so
    /// a test cannot leave one behind: `crate::tests::tmp::TempDir` ends every
    /// group spawned inside it when it drops. A detached Drone outliving its
    /// Fleet is the point in a build that ships and a leak in a test.
    pub fn spawn(mut self) -> io::Result<Child> {
        // After `in_environment` cleared and filled it, so no caller's
        // environment can leave the mark out: `crate::orphans` reads it.
        #[cfg(test)]
        let mark = self.marked_by.take().or_else(crate::orphans::own_mark);
        #[cfg(not(test))]
        let mark = crate::orphans::own_mark();
        if let (false, Some(mark)) = (self.unmarked, mark) {
            self.command.env(crate::orphans::MARK, mark);
        }
        let child = self.command.spawn()?;
        #[cfg(test)]
        if let (Some(pid), Some(directory)) = (
            child.id().and_then(std::num::NonZeroU32::new),
            self.directory.as_deref(),
        ) {
            crate::tests::tmp::spawned_in(pid, directory);
        }
        Ok(child)
    }
}

/// The fork-to-exec step. `setsid` puts the child in a new session and a new
/// process group, both led by the child, which is what makes a group-directed
/// signal at Fleet stop at Fleet.
///
/// It fails only when the caller is already a process-group leader, which the
/// forked child never is. The error is returned rather than ignored: a child
/// that silently stayed attached is the failure mode this whole module exists
/// to remove, so it must not be able to look like a successful spawn.
///
/// **One of the sites the gate names, and it says why here.** `pre_exec` is
/// unsafe because its closure runs in the forked child, between fork and exec,
/// where only async-signal-safe calls are legal — and the closure below makes
/// exactly one and touches nothing else. `crates/fleet/Cargo.toml` therefore
/// sets `unsafe_code = "deny"` instead of inheriting the workspace's `forbid`,
/// so a listed site can carry an `allow` while every other site in the crate
/// still fails to compile. The deviation is one attribute wide and greppable.
#[allow(unsafe_code)]
fn detach(command: &mut Command) {
    // SAFETY: the closure runs in the forked child before exec, where only
    // async-signal-safe calls are permitted. `setsid` is one, it allocates
    // nothing, and it is the only call made here.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

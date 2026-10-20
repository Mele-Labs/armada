//! Fleet's own `git` and `gh`, run at utility priority.
//!
//! **Fleet's launchd job is `Interactive`**, so a run the owner starts from
//! Bridge goes first, and everything Fleet starts for itself inherits that
//! priority with it. Measured on a 12-core M2 Max: `git status` in
//! a large work repository's slots reached 71% of a core each, at priority 31.
//! What Fleet reads on a timer or on a poll is not what the owner is waiting
//! for, so it takes the clamp `checks_runner::Priority` gives agent work.
//!
//! **A read goes through here and an act does not.** A merge, a push or a
//! lease is something the owner or a Job asked for and stays at normal; the
//! helpers they share (`crate::delivery::run_in`, `crate::leasing::git::git`)
//! are the acts', and `read_in` and `read` are the polling's.

use std::process::Command;

use checks_runner::Priority;

/// `program`, at utility priority, for arguments, a directory and an
/// environment to be added as to any other command.
pub(crate) fn command(program: &str) -> Command {
    Priority::Low.spawning(program)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    /// The scheduler priority `ps` reports for a shell started as `command`.
    fn priority_of(mut command: Command) -> u32 {
        let printed = command
            .args(["-c", "ps -o pri= -p $$"])
            .output()
            .expect("sh runs");
        String::from_utf8_lossy(&printed.stdout)
            .trim()
            .parse()
            .expect("ps printed a number")
    }

    /// Utility QoS's band; normal work is 31.
    const UTILITY: u32 = 20;

    fn this_process() -> u32 {
        let printed = Command::new("ps")
            .args(["-o", "pri=", "-p", &std::process::id().to_string()])
            .output()
            .expect("ps runs");
        String::from_utf8_lossy(&printed.stdout)
            .trim()
            .parse()
            .expect("ps printed a number")
    }

    #[test]
    fn a_background_command_runs_clamped_to_utility() {
        assert!(priority_of(command("/bin/sh")) <= UTILITY);
    }

    #[test]
    fn a_read_runs_clamped_and_an_act_runs_at_the_priority_of_fleet() {
        let ps = ["-c", "ps -o pri= -p $$"];
        let priority = |said: std::io::Result<std::process::Output>| -> u32 {
            String::from_utf8_lossy(&said.expect("sh runs").stdout)
                .trim()
                .parse()
                .expect("ps printed a number")
        };
        assert!(priority(crate::delivery::read_in("/", "/bin/sh", &ps)) <= UTILITY);
        // Equal to this process's own, which is 31 under Fleet and under the
        // merge line, and 20 inside an agent's Check, where nothing differs.
        assert_eq!(
            priority(crate::delivery::run_in("/", "/bin/sh", &ps)),
            this_process()
        );
    }
}

//! The person's login shell's environment, read when a watch starts, is tried, or first runs after
//! Fleet starts, and given to its command over Fleet's own.
//!
//! **Why read it at all.** Fleet runs under launchd, whose environment has neither the shell's
//! `PATH` nor anything a person exports in their rc files, so `gh`, a version manager's shims and a
//! provider's token were all missing from a watch's command.
//!
//! **A marker on each side, and only what lies between is read**, because an interactive shell's
//! prompt (powerlevel10k and the like) prints escapes before and after a `-c` command. `env -0`
//! ends each variable with a NUL, so a value holding a newline reads whole.
//!
//! **A read that fails gives Fleet's own environment, and says why**, so a watch still runs.

use std::path::Path;
use std::time::Duration;

use checks_runner::{Priority, Writing};
use verification::Exit;

/// How long the shell may take. Measured at 7 to 8 seconds for an interactive zsh login on a
/// machine at a load of 30, and well under a second idle.
pub(crate) const READ_WITHIN: Duration = Duration::from_secs(15);

/// What the read shell sets for itself and no command should inherit.
const DROPPED: &[&str] = &["PWD", "OLDPWD", "SHLVL", "_"];

/// The environment a watch's command is given before its own additions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoginEnvironment {
    /// Each variable the shell held, laid over Fleet's own. Empty where it fell back.
    pub(crate) variables: Vec<(String, String)>,
    /// Why Fleet's own environment stands in, where it does.
    pub(crate) fell_back: Option<String>,
}

impl LoginEnvironment {
    pub(crate) fn fallen_back(why: String) -> LoginEnvironment {
        LoginEnvironment { variables: Vec::new(), fell_back: Some(why) }
    }

    pub(crate) fn wire(&self) -> ipc::LoginShell {
        match &self.fell_back {
            Some(why) => ipc::LoginShell::FellBack { why: why.clone() },
            None => ipc::LoginShell::Read,
        }
    }

    /// The `PATH` a command given this environment sees.
    pub(crate) fn path(&self) -> String {
        self.variables
            .iter()
            .find(|(name, _)| name == "PATH")
            .map(|(_, value)| value.clone())
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default()
    }
}

/// Read `shell`'s environment as a login, interactive shell started in `folder`. `None` reads
/// nothing and falls back, which is what Fleet's own tests ask for.
pub(crate) async fn read(shell: Option<&str>, folder: &Path) -> LoginEnvironment {
    let Some(shell) = shell else {
        return LoginEnvironment::fallen_back(String::from("This Fleet reads no login shell."));
    };
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|at| at.as_nanos()).unwrap_or(0);
    let marker = format!("ARMADA-LOGIN-{}-{nanos}", std::process::id());
    let script = format!("printf '%s' '{marker}<'; /usr/bin/env -0; printf '%s' '>{marker}'");
    let mut env = Vec::new();
    if let Some(mark) = crate::orphans::own_mark() {
        env.push((crate::orphans::MARK.to_string(), mark));
    }
    let attempt = checks_runner::run_words(
        shell.to_string(),
        vec![String::from("-lic"), script],
        folder,
        READ_WITHIN,
        Writing::Nowhere,
        &env,
        std::future::pending(),
        Priority::Normal,
        checks_runner::GRACE,
    )
    .await;
    let failed = match &attempt.exit {
        Exit::TimedOut { .. } => Some(format!("{shell} took longer than {}s to start.", READ_WITHIN.as_secs())),
        Exit::NeverRan(_) => Some(format!("{shell} would not start.")),
        _ => None,
    };
    if let Some(why) = failed {
        return LoginEnvironment::fallen_back(why);
    }
    match parsed(&attempt.output.stdout, &marker) {
        Some(variables) => LoginEnvironment { variables, fell_back: None },
        None => LoginEnvironment::fallen_back(format!("{shell} printed no environment Fleet could read.")),
    }
}

/// The variables between `marker`'s two ends, the last pair printed. `None` where either is
/// missing.
pub(crate) fn parsed(stdout: &str, marker: &str) -> Option<Vec<(String, String)>> {
    let end = stdout.rfind(&format!(">{marker}"))?;
    let opened = format!("{marker}<");
    let start = stdout[..end].rfind(&opened)? + opened.len();
    let variables = stdout[start..end]
        .split('\0')
        .filter_map(|one| one.split_once('='))
        .filter(|(name, _)| !name.is_empty() && !DROPPED.contains(name))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    Some(variables)
}

//! The environment an agent's process is started with.
//!
//! **Fleet builds the environment; nobody inherits one.** [`environment`] is
//! the whole of what an agent gets and every variable in it was named there.
//! Nothing here reads `std::env`: the values an agent needs are parameters
//! resolved by the composition root, for the same reason a timestamp is — a
//! function reading its own inputs from the process cannot be tested or replayed.
//!
//! **No credential of any kind is on that list**: no `SSH_AUTH_SOCK`, no
//! `GIT_ASKPASS`, no forge token.
//!
//! **The environment is not a hiding place.** `ps -Eww` prints a same-uid
//! process's environment on darwin 27, measured in
//! `docs/spikes/011-what-can-one-drone-reach.md`.

use adapter_traits::{Environment, SpawnConfigRefused};

/// The two host facts a Drone needs, named by whoever resolved them.
///
/// **Not read from this process.** The composition root reads them once and
/// hands them down, so a test can plant something else and every other caller
/// gets the same two values rather than whatever its own shell had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostPaths<'a> {
    /// Where the Drone looks for programs. **Fleet's choice, not Fleet's own
    /// `PATH`** — a Drone that inherited it would find whatever the operator
    /// happened to have in front, which is a different toolchain on two
    /// machines and a different one again after a shell profile changes.
    pub path: &'a str,
    /// Who the operator is, as the agent CLI insists on being told.
    ///
    /// Not decoration and not provenance: the CLI refuses to authenticate
    /// without it, however readable its credentials are. See `environment`.
    pub user: &'a str,
    /// The home directory the agent CLI reads its own credentials from.
    ///
    /// **This is the confinement's known floor and it is written down as one.**
    /// Passing it is what lets the Drone authenticate at all; it is also what
    /// lets the CLI look in the operator's home. `--strict-mcp-config` bounds
    /// MCP servers and nothing else. The Drone reads their user settings, mod
    /// included, by the owner's choice (spike 023); the mod stays quiet on
    /// `ARMADA_DRONE`.
    pub home: &'a str,
    /// How many build jobs the agent's own `cargo` may start: a Check's width
    /// on this machine. See `environment`.
    pub width: checks_runner::CheckWidth,
}

/// Everything a Drone's process will hold.
///
/// Six variables, and each one is here because something breaks without it.
/// Adding a fifth is a deliberate edit to this function, which is the point:
/// the list is a diff, not a default.
///
/// `env_clear` appears nowhere in v1's production code, so a token exported in
/// the operator's shell reached every Drone v1 started — while v1's own handoff
/// table rejected "Armada's own environment" as a channel for exactly that
/// reason. It is the only place v1's Drone spawn was worse than its check
/// spawn.
pub fn environment(
    host: HostPaths<'_>,
    ports: &[(String, String)],
) -> Result<Environment, SpawnConfigRefused> {
    let mut env = Environment::nothing()
        .and("PATH", host.path)?
        .and("HOME", host.home)?
        // Deterministic text handling. Without it the child takes the C locale
        // on one machine and the operator's on another, and a diff of the same
        // work differs by how a byte was collated.
        .and("LANG", "en_US.UTF-8")?
        // The CLI renders progress differently into a pipe than into a
        // terminal. Saying so once here beats a heuristic reading the stream.
        .and("TERM", "dumb")?
        // **Without this the agent CLI is not logged in**, and says so on its
        // first turn instead of working. Its credentials are in the macOS
        // Keychain, and the Keychain is readable without this — the same
        // `security` read succeeds either way, so nothing here is an access
        // problem. The CLI simply will not authenticate with no `USER`.
        //
        // Measured against a real Drone: same binary, same `HOME`, `USER` the
        // only difference. `LOGNAME` does not substitute.
        .and("USER", host.user)?;
    // **A build the agent runs itself draws from the machine's one pool of
    // compiler jobs** (`checks_runner::BuildSlots`), as a Check and a seed
    // warm-up do: five builds at 3 jobs each were 15 rustc on 12 cores at load
    // 28. Cargo ignores `CARGO_BUILD_JOBS` and `-j` once it inherits a
    // jobserver, so the width stays only as the cap for a Fleet with no pool.
    env = match checks_runner::offered() {
        Some(flags) => env.and("MAKEFLAGS", flags)?,
        None => env.and("CARGO_BUILD_JOBS", &host.width.get().to_string())?,
    };
    // **A seventh and eighth kind, added when `crate::ports` has something to
    // add.** Empty for every repository that declares no `ports:`, which is
    // most of them — see that module for `ARMADA_PORT_<NAME>` and why a
    // Command's own environment is the guaranteed channel a command string is
    // not.
    for (name, value) in ports {
        env = env.and(name, value)?;
    }
    Ok(env)
}

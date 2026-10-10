//! The composition root, and the only one.
//!
//! Kept thin so linking stays the only slow build step. There is no `api-bin`
//! and no thirteenth crate.
//!
//! It will also carry `armada doctor --json`, the short-lived probe process
//! Bridge spawns on demand — how Doctor sees what a long-running daemon cannot
//! report about itself. **Not built**: `doctor` is not among `cli`'s verbs
//! yet. `docs/concepts/doctor.md` specifies it.
//!
//! # Where the composition actually is
//!
//! [`serve`](mod@serve), which is the one verb that needs a port, a store and a
//! process. `src/main.rs` reads the command line and dispatches; everything a
//! verb does is a module here, so it can be driven by a test that starts
//! nothing.
//!
//! [`setup`](mod@setup) reads a repository's `armada.yml` and its one workflow
//! and resolves the second against the first — the part of starting Fleet that
//! can be wrong on disk. [`agent`](mod@agent) is the same shape for the
//! machine: which binary a Drone is started as, and which model.
//! [`declared`](mod@declared) and [`clean`](mod@clean) are the verbs that
//! need no daemon at all. [`mcp`](mod@mcp) is the verb and the one an
//! agent runs rather than a person: it publishes the agent door into a
//! repository and, started by a client, relays a session to it over
//! [`loopback`](mod@loopback). [`watching`](mod@watching) is what makes the
//! Manifest's live keys live — `#430` — and it is here because the composition
//! root owns the runtime and nothing below it may spawn a task.
//! [`locating`](mod@locating) reads a folder a person adds into one more
//! repository Fleet serves, and holds every watch.
//!
//! [`leasing`](mod@leasing) is `armada worktree`, the pool of warm
//! worktrees agents lease.
//!
//! [`need`](mod@need) is `armada need`: what a branch needs on a path, and who
//! is ahead of it there.

pub mod agent;
pub mod authoring;
mod booting;
pub mod clean;
pub mod cli;
pub mod declared;
pub mod leasing;
pub mod locating;
pub mod loopback;
pub mod manifests;
pub mod mcp;
pub mod need;
pub mod pocketing;
pub mod reaching;
pub mod say;
pub mod serve;
pub mod settings;
pub mod setup;
mod trigger_authoring;
pub mod watching;

#[cfg(test)]
mod tests;

pub use agent::{agent_binary, model_choices, NoSuchAgent, AGENT_BINARY, MODEL};
pub use setup::{Setup, SetupRefused};

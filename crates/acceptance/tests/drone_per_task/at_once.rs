//! Slice 5: **tasks the planner marked as safe together run at once, within
//! this Job's cap and the machine's; I can watch, message or stop any one of
//! their Drones without touching the others; and where two of them edited one
//! file, the group does not commit.**
//!
//! A module of `drone_per_task` rather than more of it, because the file is
//! near the gate's line limit and the claim is one milestone's either way.
//!
//! | Not proved here | Why not |
//! |---|---|
//! | Fleet spawning a second Drone beside the first, and ending one while the other goes on | Hermetic: nothing here spawns. `crates/fleet/src/tests/at_once.rs` drives the fake harness |
//! | A redirect or a kill refused for a Drone that is not live | Needs a Fleet holding Drones; `crates/fleet/src/tests/at_once.rs` |
//! | Reading a path off an edit call in a real Drone's stream | The vendor's tool names are `adapters`' alone; `crates/adapters/src/tests/edited.rs` |











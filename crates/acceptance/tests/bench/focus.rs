//! The apparatus Focus's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `mod.rs` is separate from the test
//! files: M1's bench answers "did this Job pass its gates", and Focus asks a
//! different question of the same machinery — which Drone was on the step, and
//! which definition of the step it was measured against. Two subjects, two
//! files, and `mod.rs` stays under the length the gate warns at.
//!
//! It reaches [`Bench`]'s private fields because it is a child module of the
//! one that declares it. That is deliberate: the seams a gate is called with
//! are the bench's own, and a second copy of them here is how the two would
//! come to disagree about the same call.

use core_model::{DroneId, Timestamp, Ulid};
use fleet::Clock;

use super::Bench;

/// A Drone id a test can write down, in the shape [`Ulid`] admits.
///
/// **Minted by the test rather than by the bench**, because which Drone is on a
/// step is the thing being asserted: a bench that handed them out would be
/// answering the question.
pub fn drone(n: u32) -> DroneId {
    DroneId::carried(Ulid::carried(format!("01DRONE{n:019}")))
}

/// The bench's planted clock, which a test needs directly because a Drone
/// arriving and leaving is timed by the caller rather than by a gate.
pub fn now(bench: &Bench) -> Timestamp {
    bench.clock.now()
}



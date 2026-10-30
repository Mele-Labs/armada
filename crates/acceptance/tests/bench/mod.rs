//! The apparatus each claim is asserted against. None of it asserts anything.
//!
//! # Nothing here reaches the world
//!
//! No process, no repository, no network, no file. Each module holds the
//! values one test file needs and nothing it does not.

/// Autopilot's own apparatus: a backlog, a catalogue and a scripted model.
pub mod autopilot;
/// Lifecycles and rules' own apparatus: two Workspaces' ticket lifecycles, held as text. See its
/// own header.
pub mod lifecycles;
/// Reach's own apparatus: a Manifest and a workflow definition for a
/// repository that is not this one, held as text. See its own header.
pub mod reach;
/// Studio's own apparatus: the Studios a case reads back across the wire. See
/// its own header.
pub mod studio;

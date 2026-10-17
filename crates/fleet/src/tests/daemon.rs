//! A Fleet assembled over fakes: the fixtures the suite's other modules reach through
//! `crate::tests::daemon`.
//!
//! [`mod@workflows`] is what a case is run against and [`mod@fleets`] is what runs
//! it. Two files because the fixtures are the whole suite's — most modules reach
//! them through the re-exports below.

mod fleets;
mod workflows;

pub use fleets::{a_fleet, fitted_with, fittings};
pub use workflows::{manifest, one, workflow_named};

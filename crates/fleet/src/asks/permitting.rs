//! The shipped hold on one call a hosted Job puts to a person. `#1389`.

use std::time::Duration;

/// The shipped hold: fourteen minutes.
///
/// **Chosen against three bounds and it is the smallest.** A reply's whole
/// process ends at `super::hosting::REPLY_BUDGET`, fifteen minutes, so a hold
/// of fourteen ends before the reply does; spike 15
/// measured a stdio permission tool abandoned near thirty minutes, and the door
/// a hosted Job holds is stdio; and spike 19 measured a two-minute hold
/// surviving intact. Beyond that it is a person's patience rather than a
/// process's. It was five minutes until 13 Oct 2026, when the owner found a
/// question lapsing before he came back to it.
pub const SHIPPED_ASK_HOLD: Duration = Duration::from_secs(14 * 60);

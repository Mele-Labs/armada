//! A sweep's by-number reads are spread, so a repository with many followed pull requests does
//! not send them to the forge in one second.

use std::time::Duration;

use crate::pull_notices::by_number_gap;

#[test]
fn the_first_read_of_a_sweep_waits_for_nothing_and_each_later_one_waits_a_little() {
    assert_eq!(by_number_gap(0), Duration::ZERO);
    assert_eq!(by_number_gap(1), Duration::from_millis(250));
    assert_eq!(by_number_gap(40), Duration::from_millis(250));
}

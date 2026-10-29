//! The ports a machine listens on, read from `lsof -Fpn`, and ending the processes behind them.

use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::time::Duration;

use crate::leasing::strays::listening;
use crate::leasing::end_pids;

#[test]
fn a_listener_is_read_as_its_pid_and_port_whether_the_address_is_v4_v6_or_any() {
    let listing = "p100\nn127.0.0.1:3000\np200\nn[::1]:5501\np300\nn*:8080\n";
    assert_eq!(listening(listing), [(100, 3000), (200, 5501), (300, 8080)]);
}

#[test]
fn a_process_with_no_listen_line_is_left_out_and_its_first_port_stands() {
    let listing = "p100\np200\nn*:4000\nn*:4001\np300\n";
    assert_eq!(listening(listing), [(200, 4000)]);
}

#[test]
fn lines_that_are_not_lsof_output_read_as_no_listener() {
    assert!(listening("").is_empty());
    assert!(listening("lsof: WARNING\nn*:3000\n").is_empty(), "a port before any pid belongs to nobody");
    assert!(listening("pabc\nn*:3000\n").is_empty(), "a pid that is not a number");
    assert!(listening("p100\nn*:http\nn*:99999\nnnocolon\n").is_empty(), "a port that is not one");
}

#[test]
fn ending_a_pid_sends_term_to_a_child_and_spares_this_process() {
    let mut child = Command::new("sleep").arg("60").spawn().expect("a sleep to end");
    end_pids(&[child.id(), std::process::id()], Duration::from_secs(2));
    let ended = child.wait().expect("reaped");
    assert_eq!(ended.signal(), Some(15), "TERM, not KILL: {ended:?}");
}

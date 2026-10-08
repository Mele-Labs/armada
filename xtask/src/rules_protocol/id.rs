//! Fleet and Bridge hash the wire surface the same way.
//!
//! Two implementations take the ID: `crates/ipc/build.rs` for Fleet and
//! `apps/desktop/codegen/protocol-id.mjs` for Bridge. They share a file list,
//! a comment rule and a hash by being written twice, and a difference between
//! them is two builds from one commit that refuse each other. This rule runs
//! both on the working tree and compares.
//!
//! The Rust half is `crates/ipc/wire_hash.rs`, the same file `build.rs`
//! includes, so what is compared is the code Fleet is built with.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::Report;

include!("../../../crates/ipc/wire_hash.rs");

const NODE: &str = "apps/desktop/codegen/protocol-id.mjs";

/// Rule: Bridge's protocol ID is Fleet's.
pub fn bridge_hashes_the_wire_the_way_fleet_does(root: &Path) -> Report {
    let mut report = Report::new("Bridge hashes the wire surface the way Fleet does");
    let run = Command::new("node").arg(NODE).current_dir(root).output();
    let bridge = match run {
        Ok(run) if run.status.success() => String::from_utf8_lossy(&run.stdout).trim().to_string(),
        Ok(run) => {
            report.fail(format!(
                "{NODE} stopped: {}",
                String::from_utf8_lossy(&run.stderr).trim()
            ));
            return report;
        }
        Err(e) => {
            report.fail(format!("{NODE} — cannot run `node`: {e}"));
            return report;
        }
    };
    let fleet = protocol_id(root);
    if bridge != fleet {
        report.fail(format!(
            "Fleet is built with protocol ID {fleet} and Bridge with {bridge} from one tree. \
             `crates/ipc/wire_hash.rs` and {NODE} take the same file list, drop the same \
             comment lines and hash the same bytes; one of them no longer does"
        ));
    }
    report
}

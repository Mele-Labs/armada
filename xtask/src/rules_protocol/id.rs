//! Fleet and `scripts/restart` hash the wire surface the same way.
//!
//! Two implementations take the ID: `crates/ipc/build.rs` for the binaries, and
//! `scripts/protocol-id` for a restart that must know the ID of a tree before it has
//! built it. They share a file list, a comment rule and a hash by being written twice,
//! and a difference between them is a restart that builds one wire and expects
//! another. This rule runs both on the working tree and compares.
//!
//! The Rust half is `crates/ipc/wire_hash.rs`, the same file `build.rs`
//! includes, so what is compared is the code Fleet is built with.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::Report;

include!("../../../crates/ipc/wire_hash.rs");

const SCRIPT: &str = "scripts/protocol-id";

/// Rule: the restart script's protocol ID is Fleet's.
pub fn the_restart_script_hashes_the_wire_the_way_fleet_does(root: &Path) -> Report {
    let mut report = Report::new("the restart script hashes the wire surface the way Fleet does");
    let run = Command::new("python3").arg(SCRIPT).current_dir(root).output();
    let script = match run {
        Ok(run) if run.status.success() => String::from_utf8_lossy(&run.stdout).trim().to_string(),
        Ok(run) => {
            report.fail(format!(
                "{SCRIPT} stopped: {}",
                String::from_utf8_lossy(&run.stderr).trim()
            ));
            return report;
        }
        Err(e) => {
            report.fail(format!("{SCRIPT} — cannot run `python3`: {e}"));
            return report;
        }
    };
    let fleet = protocol_id(root);
    if script != fleet {
        report.fail(format!(
            "Fleet is built with protocol ID {fleet} and {SCRIPT} prints {script} from one tree. \
             `crates/ipc/wire_hash.rs` and {SCRIPT} take the same file list, drop the same \
             comment lines and hash the same bytes; one of them no longer does"
        ));
    }
    report
}

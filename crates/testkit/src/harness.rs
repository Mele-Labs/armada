//! An `AgentHarness` that reads what a test scripts.
//!
//! **Not faithful: reading.** It does not decode — turning a line of a vendor's
//! stream into an event is that vendor's schema, and belongs to `adapters`,
//! the only crate permitted to read untyped bytes on this path.
//! [`FakeHarness::read`] answers with the line as prose unless a test scripted
//! something, which is enough for a caller that only needs *an* event.

use std::collections::BTreeMap;
use std::convert::Infallible;

use adapter_traits::{AgentHarness, DroneEvent, Reach, Speaker, ToolUse};

/// A harness that reads lines as scripted.
#[derive(Debug, Default)]
pub struct FakeHarness {
    scripted: BTreeMap<String, Vec<DroneEvent>>,
}

/// How a turn Fleet wrote looks coming back, as far as the fake reads it.
///
/// **`fleet::session::Turn`'s own serialisation**, which puts `type` first
/// because the struct does. The real decoder asks the same question of a parsed
/// object; this asks it of the bytes, which is the whole difference between a
/// fixture and `adapters::transcript`.
pub const REPLAYED: &str = "{\"type\":\"user\"";

impl FakeHarness {
    /// A harness that reads every line as prose.
    pub fn that_listens() -> FakeHarness {
        FakeHarness::default()
    }

    /// The same, for a test that names a program it never starts.
    pub fn running(_program: &str, _args: &[&str]) -> FakeHarness {
        FakeHarness::default()
    }

    /// What one line means, for a test that needs a specific event out of it.
    pub fn reading(mut self, line: &str, as_events: Vec<DroneEvent>) -> FakeHarness {
        self.scripted.insert(String::from(line), as_events);
        self
    }
}

impl AgentHarness for FakeHarness {
    type Error = Infallible;

    fn read(&self, line: &str) -> Vec<DroneEvent> {
        match self.scripted.get(line) {
            Some(events) => events.clone(),
            // **A turn coming back is Armada's, and `/bin/cat` is what makes
            // that real.** The agent CLI runs with `--replay-user-messages`,
            // which re-emits an injected turn when the Drone is handed it — so
            // `that_listens` echoing Fleet's own line is the fixture's whole
            // model of delivery, and attributing it to the Drone made every
            // fake Drone one that was never told anything.
            //
            // **A prefix rather than a parse**, because the gate keeps
            // untyped JSON reads inside `store` and `ipc`. It is exactly what
            // `fleet::session::Turn` serialises to, and
            // `a_turn_fleet_writes_is_read_back_as_armadas` pins the two
            // together so this cannot drift quietly.
            None if line.starts_with(REPLAYED) => vec![DroneEvent::Said {
                text: String::from(line),
                by: Speaker::Armada,
            }],
            // The Drone's, because a scripted line stands in for something a
            // Drone wrote. A test that wants a different event scripts it.
            None => vec![DroneEvent::Said {
                text: String::from(line),
                by: Speaker::Drone,
            }],
        }
    }

    /// A call to [`EDITS`] names the file in its detail. **Not the real
    /// harness's tool names**, which are `adapters`' vocabulary: a test that
    /// scripts an edit call scripts one of these.
    fn edited(&self, event: &DroneEvent) -> Option<String> {
        match event {
            DroneEvent::Called { tool, detail, .. } if tool == EDITS => {
                Some(String::from(detail.shown()))
            }
            _ => None,
        }
    }

    /// A call to [`READS`] names the file in its detail, and one to [`RUNS`] the command.
    fn used(&self, event: &DroneEvent) -> Option<ToolUse> {
        match event {
            DroneEvent::Called { tool, detail, .. } if tool == READS => {
                Some(ToolUse::Read(String::from(detail.shown())))
            }
            DroneEvent::Called { tool, detail, .. } if tool == RUNS => {
                Some(ToolUse::Ran(String::from(detail.shown())))
            }
            // The detail is the line, and the service is the first word of it.
            DroneEvent::Called { tool, detail, .. } if tool == REACHES => {
                let (service, line) = detail.shown().split_once(' ').unwrap_or((detail.shown(), ""));
                Some(ToolUse::Reached(Reach {
                    service: String::from(service),
                    name: String::from(service),
                    line: String::from(line),
                }))
            }
            _ => None,
        }
    }
}

/// The tool [`FakeHarness::edited`] reads as an edit call.
pub const EDITS: &str = "edits_a_file";

/// The tool [`FakeHarness::used`] reads as a file read.
pub const READS: &str = "reads_a_file";

/// The tool [`FakeHarness::used`] reads as a call outside the repository: its detail is the service, then the line.
pub const REACHES: &str = "reaches_a_service";

/// The tool [`FakeHarness::used`] reads as a shell command.
pub const RUNS: &str = "runs_a_command";

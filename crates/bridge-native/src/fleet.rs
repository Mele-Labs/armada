//! The Fleet connection as an entity: the one owner of what Fleet has said.
//!
//! Views read from this entity and none opens a connection of its own. A task drains the link's
//! channel, folds **a whole batch** into [`Held`] and notifies once; the Electron Bridge's slowness
//! was redraw volume, one per event (#3356).

use std::collections::BTreeMap;
use std::io::{self, Write};

use chrono::Local;
use gpui::{Context, Task};

use ipc::{Cursor, Event, SessionRecord, StreamMessage};

use crate::link::{self, LinkState, Signal};

/// What Fleet has said, folded. Plain data, so a fold is tested without a window.
pub struct Held {
    pub state: LinkState,
    pub jobs: Vec<SessionRecord>,
    pub cursor: Option<Cursor>,
}

impl Held {
    fn new() -> Self {
        Held { state: LinkState::NotRunning("not yet read".into()), jobs: Vec::new(), cursor: None }
    }

    /// Fold one batch; the lines say what it held, one for the stream and one for each other fact.
    pub fn fold(&mut self, batch: Vec<Signal>) -> Vec<String> {
        let mut lines = Vec::new();
        let (mut events, mut resyncs, mut missed) = (0usize, 0usize, 0u64);
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for signal in batch {
            match signal {
                Signal::State(state) => {
                    if state != self.state {
                        lines.push(describe(&state));
                        self.state = state;
                    }
                }
                Signal::Jobs(jobs) => {
                    lines.push(format!("jobs: {} on connect", jobs.len()));
                    self.jobs = jobs;
                }
                Signal::JobsFailed(why) => lines.push(format!("jobs: could not be read: {why}")),
                Signal::Stream(StreamMessage::Resync(resync)) => {
                    resyncs += 1;
                    self.cursor = Some(resync.cursor);
                }
                Signal::Stream(StreamMessage::Missed(lost)) => missed += lost.dropped,
                Signal::Stream(StreamMessage::Event(delivered)) => {
                    events += 1;
                    self.cursor = Some(delivered.cursor);
                    *kinds.entry(delivered.event.kind()).or_default() += 1;
                    // A row arrives whole and replaces the one held, never patches it.
                    if let Event::SessionChanged(record) = delivered.event {
                        match self.jobs.iter_mut().find(|held| held.id == record.id) {
                            Some(held) => *held = record,
                            None => self.jobs.push(record),
                        }
                    }
                }
            }
        }
        if events + resyncs > 0 || missed > 0 {
            let kinds: Vec<String> = kinds.iter().map(|(kind, n)| format!("{kind} x{n}")).collect();
            let cursor = self.cursor.map_or("none".to_string(), |at| at.position().to_string());
            lines.push(format!(
                "events: {events} in batch [{}], resync {resyncs}, missed {missed}, cursor {cursor}, jobs held {}",
                kinds.join(", "),
                self.jobs.len()
            ));
        }
        lines
    }
}

fn describe(state: &LinkState) -> String {
    match state {
        LinkState::NotRunning(why) => format!("link: Fleet is not running ({why})"),
        LinkState::Refused(why) => format!("link: runtime file refused ({why})"),
        LinkState::ProtocolMismatch { fleet, ours } => {
            format!("link: protocol mismatch, Fleet speaks {fleet}, this build speaks {ours}; not connecting")
        }
        LinkState::Connecting { port, pid } => format!("link: connecting to Fleet on port {port} (pid {pid})"),
        LinkState::Starting => "link: Fleet has not answered yet and is still starting".to_string(),
        LinkState::Unreachable(why) => format!("link: unreachable ({why})"),
        LinkState::Connected => "link: connected".to_string(),
    }
}

/// The line goes to stderr, which is where this app reports until a screen can.
fn log(line: &str) {
    let _ = writeln!(io::stderr().lock(), "bridge-native {} {line}", Local::now().format("%T%.3f"));
}

pub struct FleetConnection {
    held: Held,
    /// Kept, or the drain is cancelled with it.
    _drain: Task<()>,
}

impl FleetConnection {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = async_channel::unbounded();
        let home = std::env::var("HOME").ok();
        log(&format!("protocol {}", ipc::ProtocolId::current()));
        // Its own thread and runtime: nothing here waits on the network. The thread ends when the
        // drain below is dropped and the channel closes.
        let spawned = std::thread::Builder::new().name("fleet-link".into()).spawn(move || {
            match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime.block_on(link::run(tx, home)),
                Err(why) => log(&format!("link: could not start its runtime: {why}")),
            }
        });
        if let Err(why) = spawned {
            log(&format!("link: could not start its thread: {why}"));
        }
        let drain = cx.spawn(async move |this, cx| {
            while let Ok(first) = rx.recv().await {
                let mut batch = vec![first];
                while let Ok(more) = rx.try_recv() {
                    batch.push(more);
                }
                let folded = this.update(cx, |this, cx| {
                    let lines = this.held.fold(batch);
                    cx.notify();
                    lines
                });
                match folded {
                    Ok(lines) => lines.iter().for_each(|line| log(line)),
                    Err(_) => return,
                }
            }
        });
        FleetConnection { held: Held::new(), _drain: drain }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{Missed, Resync};

    fn event(cursor: u64) -> Signal {
        let wire = format!(
            r#"{{"message":"event","cursor":{cursor},"event":{{"kind":"repositories.changed","repositories":[]}}}}"#
        );
        Signal::Stream(crate::frames::decode(&wire).expect("an event"))
    }

    #[test]
    fn a_batch_of_events_is_one_line_and_moves_the_cursor() {
        let mut held = Held::new();
        let lines = held.fold(vec![event(1), event(2), event(3)]);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("3 in batch [repositories.changed x3]"), "{}", lines[0]);
        assert_eq!(held.cursor, Some(Cursor::at(3)));
    }

    #[test]
    fn a_state_that_did_not_change_says_nothing() {
        let mut held = Held::new();
        assert_eq!(held.fold(vec![Signal::State(LinkState::Connected)]).len(), 1);
        assert!(held.fold(vec![Signal::State(LinkState::Connected)]).is_empty());
    }

    #[test]
    fn resync_and_missed_are_counted_in_the_batch_line() {
        let mut held = Held::new();
        let resync = Signal::Stream(StreamMessage::Resync(Resync {
            protocol_id: ipc::ProtocolId::current(),
            cursor: Cursor::at(9),
        }));
        let lost = Signal::Stream(StreamMessage::Missed(Missed { dropped: 4 }));
        let lines = held.fold(vec![lost, resync]);
        assert!(lines[0].contains("resync 1, missed 4, cursor 9"), "{}", lines[0]);
    }

    #[test]
    fn jobs_on_connect_replace_what_was_held() {
        let mut held = Held::new();
        let lines = held.fold(vec![Signal::Jobs(Vec::new())]);
        assert_eq!(lines, vec!["jobs: 0 on connect".to_string()]);
    }
}

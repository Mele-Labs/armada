//! Telling Fleet that `armada check` ran in a Session's slot. `ipc::session_checks`.
//!
//! **Best effort and silent.** A Check run by a person, with no Fleet up, or in a directory no
//! Session holds, behaves exactly as it did: Fleet answers no run, or nothing answers, and the
//! Check goes on. Fleet places the Session by the directory, so nothing here names one.

use std::path::Path;
use std::time::Duration;

use fleet::runtime;
use ipc::{SessionCheckAct, SessionCheckAnswer, SessionCheckCall, SessionCheckState};

use crate::declared::Ran;
use crate::loopback::Loopback;

const ROUTE: &str = "/sessions/checks";

/// A run Fleet has begun a row for, to be ended with the Check's result.
pub struct Told {
    fleet: Loopback,
    cwd: String,
    name: String,
    run: u64,
}

fn say(fleet: &Loopback, call: &SessionCheckCall) -> Option<SessionCheckAnswer> {
    let body = ipc::encode(call).ok()?;
    let answer = fleet.post(ROUTE, body.as_bytes()).ok()?;
    (answer.status == 200)
        .then(|| ipc::decode("a Session Check answer", &answer.body).ok())
        .flatten()
}

/// Tell Fleet a Check `name` is starting in `cwd`. `None` where no Session holds that slot.
pub fn begin(cwd: &Path, name: &str) -> Option<Told> {
    let at = runtime::machine_path().ok()?;
    let fleet = Loopback::at(crate::mcp::listening(runtime::read(&at), &at).ok()?);
    let cwd = cwd.to_str()?.to_string();
    let call = SessionCheckCall {
        act: SessionCheckAct::Start,
        cwd: cwd.clone(),
        name: name.to_string(),
        run: None,
        outcome: None,
        took_ms: None,
        log: None,
    };
    let run = say(&fleet, &call)?.run?;
    Some(Told {
        fleet,
        cwd,
        name: name.to_string(),
        run,
    })
}

impl Told {
    /// Tell Fleet how it ended, with what it printed.
    pub fn end(self, ran: &Ran, took: Duration) {
        let output = &ran.attempt.output;
        let log = format!("$ {}\n{}{}", ran.command, output.stdout, output.stderr);
        self.ended(ran.status() == 0, took, log);
    }

    /// The Check could not be run at all, and Fleet is told that, so the row does not stay out.
    pub fn refused(self, why: &str, took: Duration) {
        self.ended(false, took, why.to_string());
    }

    fn ended(self, passed: bool, took: Duration, log: String) {
        let call = SessionCheckCall {
            act: SessionCheckAct::End,
            cwd: self.cwd,
            name: self.name,
            run: Some(self.run),
            outcome: Some(match passed {
                true => SessionCheckState::Passed,
                false => SessionCheckState::Failed,
            }),
            took_ms: Some(u64::try_from(took.as_millis()).unwrap_or(u64::MAX)),
            log: Some(log),
        };
        say(&self.fleet, &call);
    }
}

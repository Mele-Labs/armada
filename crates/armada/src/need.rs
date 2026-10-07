//! `armada need`: a branch says what it needs on a path, and is told who is
//! ahead of it there. `docs/capabilities/needs.md`; the decision is
//! `decisions/2026-10-02-a-plan-leases-its-numbers.md`. This is the half for
//! agents working outside Fleet.
//!
//! **Fleet keeps the needs**, as rows on its session ledger, and this verb asks
//! it: a Job and a session on one machine stand in one order because there is
//! one table. **Where Fleet is not running this says so and does nothing else.**
//! A list of its own, kept here, would be a second order that Fleet's merge never
//! reads, which is what the files under `.git/armada-needs/` were.
//!
//! **A need is a path and what is needed there, in the declarer's words**, and
//! the repository declares no kinds up front: the file is the resource. First
//! to declare goes first, and **nothing expires by time**: a stalled need is
//! given back by a person, `--release`.
//!
//! **`armada land` still reads the files** ([`Needs`], re-exported for it). It is
//! being retired for pull requests and is not moved onto the ledger, so a need
//! declared through Fleet is not one it holds a branch behind.

use std::path::Path;

pub use adapters::needs::{waiting_text, Need, Needs};
use fleet::runtime;
use ipc::{NeedAct as Act, NeedAnswer, NeedCall, NeedLine, NeedList, WireError};

use crate::land::git::checked;
use crate::loopback::Loopback;

/// Where Fleet serves the act and the read.
const NEEDS: &str = "/needs";

/// What `armada need` prints after declaring.
pub fn declared_text(
    done: &NeedAnswer,
    branch: &str,
    path: &str,
    what: &str,
    late: bool,
) -> String {
    let mut out = String::new();
    let verb = if done.already {
        "already recorded"
    } else {
        "recorded"
    };
    out.push_str(&format!("need {verb}: {branch} on {path}: {what}\n"));
    if done.ahead.is_empty() {
        out.push_str(&format!("nothing is ahead of you on {path}\n"));
        return out;
    }
    out.push_str(&format!("ahead of you on {path}:\n"));
    for ahead in &done.ahead {
        out.push_str(&format!("  {}\n", describe(ahead)));
    }
    out.push_str(
        "pick the value after theirs, then `armada need --took <path> \"<value>\"`. \
         Fleet holds this branch's merge until they have landed or been given back.\n",
    );
    if late {
        out.push_str(&format!(
            "this branch already changes {path}: if it took a number there before declaring, \
             search comments and docs for the old number and change every mention.\n",
        ));
    }
    out
}

/// `who: what`, and what it took where it said.
fn describe(need: &NeedLine) -> String {
    match &need.took {
        Some(took) => format!("{}: {}, took {took}", need.held_by, need.what),
        None => format!("{}: {}, took nothing yet", need.held_by, need.what),
    }
}

/// `armada need --status`.
pub fn status_text(needs: &[NeedLine]) -> String {
    if needs.is_empty() {
        return "no needs standing\n".to_string();
    }
    let mut out = String::new();
    let mut at: Option<&str> = None;
    let mut n = 0;
    for need in needs {
        if at != Some(need.path.as_str()) {
            at = Some(need.path.as_str());
            n = 0;
            out.push_str(&format!("{}\n", need.path));
        }
        n += 1;
        out.push_str(&format!("  {n}. {}\n", describe(need)));
    }
    out
}

/// Whether the checkout at `cwd` differs from the base at `path`, committed or
/// not. Unanswerable reads as no.
pub(crate) fn changes(cwd: &Path, path: &str) -> bool {
    let env = crate::land::Env::read();
    let base = format!("{}/{}", env.remote, env.base);
    let Ok(point) = crate::land::merge_base(cwd, "HEAD", &base) else {
        return false;
    };
    checked(cwd, &["diff", "--name-only", &point, "--", path])
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false)
}

/// The Fleet to ask, or the sentence saying there is not one.
fn fleet_here() -> Result<Loopback, String> {
    let at = runtime::machine_path().map_err(|why| why.to_string())?;
    crate::mcp::listening(runtime::read(&at), &at)
        .map(Loopback::at)
        .map_err(|why| {
            format!("{why} `armada need` asks Fleet and keeps no list of its own, so nothing was recorded.")
        })
}

/// Fleet's refusal, as the sentence it carried.
fn refusal(status: u16, body: &[u8]) -> String {
    match ipc::decode::<WireError>("an error", body) {
        Ok(error) => error.message,
        Err(_) => format!("Fleet answered {status}"),
    }
}

fn asked<T: serde::de::DeserializeOwned>(
    answer: Result<crate::loopback::Answer, crate::loopback::Unreachable>,
    what: &'static str,
) -> Result<T, String> {
    let answer = answer.map_err(|why| format!("Fleet did not answer: {why}"))?;
    if answer.status != 200 {
        return Err(refusal(answer.status, &answer.body));
    }
    ipc::decode(what, &answer.body)
        .map_err(|why| format!("what Fleet answered could not be read: {why}"))
}

/// Run one form of `armada need` from the checkout at `cwd`; what it says.
pub fn run(cwd: &Path, act: crate::cli::NeedAct) -> Result<String, String> {
    use crate::cli::NeedAct;
    let manifest = crate::mcp::standing_in(cwd)?.id().to_string();
    let fleet = fleet_here()?;
    if let NeedAct::Status = act {
        let path = format!("{NEEDS}?manifest_id={}", ipc::door::encoded(&manifest));
        let list: NeedList = asked(fleet.get(&path), "a need list")?;
        return Ok(status_text(&list.needs));
    }
    let branch = crate::land::current_branch(cwd)
        .ok_or("no branch is checked out here, and a need belongs to a branch")?;
    let call = |act, path: &str, what: Option<&str>, value: Option<&str>| NeedCall {
        act,
        manifest_id: Some(ipc::ManifestId::carried(&manifest)),
        branch: branch.clone(),
        path: path.to_string(),
        what: what.map(str::to_string),
        value: value.map(str::to_string),
    };
    let send = |call: NeedCall| -> Result<NeedAnswer, String> {
        let body = ipc::encode(&call).map_err(|why| why.to_string())?;
        asked(fleet.post(NEEDS, body.as_bytes()), "a need answer")
    };
    match act {
        NeedAct::Declare { path, what } => {
            let done = send(call(Act::Declare, &path, Some(&what), None))?;
            let path = adapters::needs::clean_path(&path);
            let late = !done.ahead.is_empty() && changes(cwd, &path);
            Ok(declared_text(&done, &branch, &path, &what, late))
        }
        NeedAct::Took { path, value } => {
            let done = send(call(Act::Took, &path, None, Some(&value)))?;
            let path = done.mine.map(|need| need.path).unwrap_or(path);
            Ok(format!("recorded: {branch} took {value} on {path}\n"))
        }
        NeedAct::Release { path } => {
            let done = send(call(Act::Release, &path, None, None))?;
            Ok(match done.gave_back {
                true => format!("gave back {branch}'s need on {path}\n"),
                false => format!("{branch} had no need on {path}, so nothing was given back\n"),
            })
        }
        NeedAct::Status => unreachable!("answered above"),
    }
}

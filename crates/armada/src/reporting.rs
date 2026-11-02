//! `armada report`: the text of Copy debug report, to paste to whoever helps. Nothing is uploaded.
//!
//! **Fleet's answer where Fleet answers, and what this process can read itself where it does not.**
//! Both are laid out by `ipc::debug_report`. A report is a report either way, so the verb exits 0:
//! Bridge runs it when Fleet is down and reads its standard output.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::AgentHarness;
use fleet::{runtime, Clock};
use ipc::debug_report::{self, Facts};

use crate::loopback::Loopback;

const ROUTE: &str = "/debug_report";

/// How long Fleet has to answer: it may ask the CLI's version for two seconds itself.
const FLEET_WITHIN: Duration = Duration::from_secs(8);

/// How long the CLI is asked its version, here.
const VERSION_WITHIN: Duration = Duration::from_secs(2);

/// The folder holding `.git` at or above `from`: the repository the caller stands in.
fn repository_above(from: &Path) -> Option<PathBuf> {
    from.ancestors().find(|folder| folder.join(".git").exists()).map(Path::to_path_buf)
}

/// A value in a query: every byte but the unreserved ones as `%XX`.
fn encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// What the Fleet on this machine says, where it answers.
fn fleet_says(home: &str, root: Option<&Path>) -> Option<String> {
    let at = runtime::support_dir(home).join(runtime::FILE_NAME);
    let port = crate::mcp::listening(runtime::read(&at), &at).ok()?;
    let path = match root {
        Some(root) => format!("{ROUTE}?root={}", encoded(&root.to_string_lossy())),
        None => ROUTE.to_string(),
    };
    let answer = Loopback::at(port).get_within(&path, FLEET_WITHIN).ok()?;
    let report: ipc::DebugReport = (answer.status == 200).then(|| ipc::decode("a debug report", &answer.body).ok()).flatten()?;
    Some(report.text)
}

/// What this process can read without Fleet: the install and the log, and a line saying Fleet did not
/// answer.
pub fn silent(home: &str, path: &str, taken: &str) -> String {
    let claude_code = adapters::HeadlessAgent::on_path().version_within(path, VERSION_WITHIN);
    debug_report::render(&Facts {
        home: home.to_string(),
        install: adapters::debug_facts::install_of(path, claude_code),
        fleet: None,
        trouble: adapters::debug_facts::recent_log(Path::new(home), taken),
        taken: taken.to_string(),
    })
}

/// The report for the directory `cwd`.
pub fn report(cwd: &Path) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let root = repository_above(cwd);
    if let Some(text) = fleet_says(&home, root.as_deref()) {
        return text;
    }
    let taken = debug_report::to_the_second(fleet::SystemClock::new().now().as_str());
    silent(&home, &std::env::var("PATH").unwrap_or_default(), &taken)
}

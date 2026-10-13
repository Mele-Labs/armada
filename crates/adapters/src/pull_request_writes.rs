//! What a person writes on a pull request: a comment, an approval, a request
//! for changes.
//!
//! The body goes to the tool on its standard input and never in its arguments,
//! so a long one is not an argv and a body that looks like a flag is a body.

use std::io::Write;
use std::process::{Command, Stdio};

use adapter_traits::PullRequestWrite;

use crate::delivery::{said, FORGE};

/// The arguments after `pr`, and the body for standard input where there is one.
/// A blank body is none, so a comment or a request for changes with nothing in
/// it is refused here as well as where it is asked.
pub(crate) fn arguments<'a>(
    pull_request: &'a str,
    write: &'a PullRequestWrite,
) -> Result<(Vec<&'a str>, Option<&'a str>), String> {
    let present = |body: &'a str| (!body.trim().is_empty()).then_some(body);
    match write {
        PullRequestWrite::Comment { body } => {
            let body = present(body).ok_or("a comment needs something to say")?;
            Ok((vec!["comment", pull_request, "--body-file", "-"], Some(body)))
        }
        PullRequestWrite::Approve { body } => match body.as_deref().and_then(present) {
            Some(body) => Ok((
                vec!["review", pull_request, "--approve", "--body-file", "-"],
                Some(body),
            )),
            None => Ok((vec!["review", pull_request, "--approve"], None)),
        },
        PullRequestWrite::RequestChanges { body } => {
            let body = present(body).ok_or("a request for changes needs a reason")?;
            Ok((
                vec!["review", pull_request, "--request-changes", "--body-file", "-"],
                Some(body),
            ))
        }
    }
}

/// Write it. **A write to the forge**, so the refusal is its own sentence and
/// nothing here retries.
pub(crate) fn write(
    in_repo: &str,
    pull_request: &str,
    write: &PullRequestWrite,
) -> Result<(), String> {
    let (args, body) = arguments(pull_request, write)?;
    let mut child = Command::new(FORGE)
        .arg("pr")
        .args(&args)
        .current_dir(in_repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(if body.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    if let (Some(body), Some(mut stdin)) = (body, child.stdin.take()) {
        stdin
            .write_all(body.as_bytes())
            .map_err(|why| format!("`{FORGE}` would not take the text: {why}"))?;
    }
    let run = child
        .wait_with_output()
        .map_err(|why| format!("`{FORGE}` did not finish: {why}"))?;
    match run.status.success() {
        true => Ok(()),
        false => Err(said(&run)),
    }
}

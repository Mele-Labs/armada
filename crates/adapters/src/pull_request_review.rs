//! A review that carries comments, posted as one: opened pending, filled one comment a call, then
//! submitted, so the forge sends a single notification at the end and nothing before it.
//!
//! **One `api graphql` call a step.** Text goes to the tool on its standard input (`-F body=@-`)
//! and never in its arguments; every other value is a `-f` string, so a path or a commit that
//! reads as a number or `true` is still the string it was. A step the forge refuses after the
//! review is open deletes the pending review, best effort, and returns the forge's sentence:
//! nothing is left half posted. The steps are values ([`Step`]) handed to a runner, so a test
//! drives the whole sequence without a forge.

use std::io::Write;
use std::process::{Command, Stdio};

use adapter_traits::{DiffSide, ReviewComment, ReviewSaid};

use crate::delivery::{last_line, run_in, said, FORGE};

const ADD_REVIEW: &str = "mutation($pr:ID!,$commit:GitObjectID){addPullRequestReview(\
    input:{pullRequestId:$pr,commitOID:$commit}){pullRequestReview{id}}}";
const ADD_LINE: &str = "mutation($review:ID!,$path:String!,$line:Int!,$side:DiffSide!,$body:String!){\
    addPullRequestReviewThread(input:{pullRequestReviewId:$review,path:$path,line:$line,side:$side,\
    body:$body}){thread{id}}}";
const ADD_FILE: &str = "mutation($review:ID!,$path:String!,$body:String!){\
    addPullRequestReviewThread(input:{pullRequestReviewId:$review,path:$path,subjectType:FILE,\
    body:$body}){thread{id}}}";
const ADD_REPLY: &str = "mutation($review:ID!,$thread:ID!,$body:String!){\
    addPullRequestReviewThreadReply(input:{pullRequestReviewId:$review,\
    pullRequestReviewThreadId:$thread,body:$body}){comment{id}}}";
const SUBMIT: &str = "mutation($review:ID!,$event:PullRequestReviewEvent!,$body:String){\
    submitPullRequestReview(input:{pullRequestReviewId:$review,event:$event,body:$body})\
    {pullRequestReview{id}}}";
const DELETE: &str = "mutation($review:ID!){deletePullRequestReview(\
    input:{pullRequestReviewId:$review}){pullRequestReview{id}}}";

/// One call to the forge: a mutation, its string values, an integer, and the text for standard
/// input. `picked` is the jq path of the id it answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) query: &'static str,
    pub(crate) strings: Vec<(&'static str, String)>,
    pub(crate) line: Option<u64>,
    /// Sent as `body` on standard input. Absent sends no `body` at all.
    pub(crate) text: Option<String>,
    picked: &'static str,
}

impl Step {
    fn new(query: &'static str, picked: &'static str) -> Step {
        Step {
            query,
            strings: Vec::new(),
            line: None,
            text: None,
            picked,
        }
    }

    fn with(mut self, name: &'static str, value: &str) -> Step {
        self.strings.push((name, value.to_string()));
        self
    }

    fn saying(mut self, text: &str) -> Step {
        self.text = Some(text.to_string());
        self
    }

    /// What follows `gh`. **The text is never in it.**
    pub(crate) fn arguments(&self) -> Vec<String> {
        let mut args = vec!["api".to_string(), "graphql".to_string(), "-f".to_string()];
        args.push(format!("query={}", self.query));
        for (name, value) in &self.strings {
            args.push("-f".to_string());
            args.push(format!("{name}={value}"));
        }
        if let Some(line) = self.line {
            args.push("-F".to_string());
            args.push(format!("line={line}"));
        }
        if self.text.is_some() {
            args.push("-F".to_string());
            args.push("body=@-".to_string());
        }
        args.push("--jq".to_string());
        args.push(self.picked.to_string());
        args
    }
}

/// How a step is taken. The real one runs the tool; a test's records.
pub(crate) type Runner<'a> = dyn FnMut(&Step) -> Result<String, String> + 'a;

fn open(pull_request_id: &str, commit: &str) -> Step {
    Step::new(ADD_REVIEW, ".data.addPullRequestReview.pullRequestReview.id")
        .with("pr", pull_request_id)
        .with("commit", commit)
}

fn comment(review: &str, comment: &ReviewComment) -> Step {
    match comment {
        ReviewComment::Line { path, line, side, body } => {
            let mut step = Step::new(ADD_LINE, ".data.addPullRequestReviewThread.thread.id")
                .with("review", review)
                .with("path", path)
                .with(
                    "side",
                    match side {
                        DiffSide::Left => "LEFT",
                        DiffSide::Right => "RIGHT",
                    },
                )
                .saying(body);
            step.line = Some(*line);
            step
        }
        ReviewComment::File { path, body } => {
            Step::new(ADD_FILE, ".data.addPullRequestReviewThread.thread.id")
                .with("review", review)
                .with("path", path)
                .saying(body)
        }
        ReviewComment::Reply { thread, body } => {
            Step::new(ADD_REPLY, ".data.addPullRequestReviewThreadReply.comment.id")
                .with("review", review)
                .with("thread", thread)
                .saying(body)
        }
    }
}

fn submit(review: &str, verdict: ReviewSaid, body: Option<&str>) -> Step {
    let event = match verdict {
        ReviewSaid::Approved => "APPROVE",
        ReviewSaid::ChangesRequested => "REQUEST_CHANGES",
        ReviewSaid::Commented => "COMMENT",
    };
    let step = Step::new(SUBMIT, ".data.submitPullRequestReview.pullRequestReview.id")
        .with("review", review)
        .with("event", event);
    match body.filter(|body| !body.trim().is_empty()) {
        Some(body) => step.saying(body),
        None => step,
    }
}

fn delete(review: &str) -> Step {
    Step::new(DELETE, ".data.deletePullRequestReview.pullRequestReview.id").with("review", review)
}

/// A step that answered with no id did not do what it was asked.
fn taken(run: &mut Runner<'_>, step: &Step) -> Result<String, String> {
    let id = run(step)?;
    match id.trim().is_empty() {
        true => Err("the forge took the step and gave nothing back".to_string()),
        false => Ok(id.trim().to_string()),
    }
}

/// The whole review through `run`, given the pull request's node id and the commit its branch is
/// at. `commit` is the one the person read where they said.
pub(crate) fn post_with(
    run: &mut Runner<'_>,
    (pull_request_id, head): (&str, &str),
    verdict: ReviewSaid,
    body: Option<&str>,
    comments: &[ReviewComment],
    commit: Option<&str>,
) -> Result<(), String> {
    // A refusal here leaves no review to delete.
    let review = taken(run, &open(pull_request_id, commit.unwrap_or(head)))?;
    let filled = comments
        .iter()
        .try_for_each(|one| taken(run, &comment(&review, one)).map(drop))
        .and_then(|()| taken(run, &submit(&review, verdict, body)).map(drop));
    if filled.is_err() {
        // Best effort: the sentence the person reads is the step's, not the clean-up's.
        let _ = run(&delete(&review));
    }
    filled
}

/// Post it. **A write to the forge**, taken only from a person's ask.
pub(crate) fn post(
    in_repo: &str,
    pull_request: &str,
    verdict: ReviewSaid,
    body: Option<&str>,
    comments: &[ReviewComment],
    commit: Option<&str>,
) -> Result<(), String> {
    let (id, head) = identified(in_repo, pull_request)?;
    post_with(
        &mut |step| ran(in_repo, step),
        (&id, &head),
        verdict,
        body,
        comments,
        commit,
    )
}

/// The pull request's node id and the commit its branch is at.
fn identified(in_repo: &str, pull_request: &str) -> Result<(String, String), String> {
    let run = run_in(
        in_repo,
        FORGE,
        &["pr", "view", pull_request, "--json", "id,headRefOid", "--jq", "[.id,.headRefOid]|@tsv"],
    )
    .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    if !run.status.success() {
        return Err(said(&run));
    }
    let line = last_line(&run);
    match line.split_once('\t') {
        Some((id, head)) if !id.is_empty() && !head.is_empty() => Ok((id.to_string(), head.to_string())),
        _ => Err(format!("the forge did not say what pull request {pull_request} is")),
    }
}

/// Take a step with the real tool: the id it answered with, or the forge's sentence.
fn ran(in_repo: &str, step: &Step) -> Result<String, String> {
    let mut child = Command::new(FORGE)
        .args(step.arguments())
        .current_dir(in_repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(if step.text.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    if let (Some(text), Some(mut stdin)) = (&step.text, child.stdin.take()) {
        stdin
            .write_all(text.as_bytes())
            .map_err(|why| format!("`{FORGE}` would not take the text: {why}"))?;
    }
    let run = child
        .wait_with_output()
        .map_err(|why| format!("`{FORGE}` did not finish: {why}"))?;
    match run.status.success() {
        true => Ok(last_line(&run)),
        false => Err(said(&run)),
    }
}

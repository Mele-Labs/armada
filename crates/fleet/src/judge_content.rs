//! Each kind of artifact of a Job, read into the words a Judge is shown. `docs/concepts/judge.md`.
//!
//! **Where Fleet holds nothing readable for a kind, the reading fails with Fleet's reason** and does not
//! guess: an artifact the Judge was shown nothing of is not one it read, and a clean answer about it
//! would say it was.
//!
//! | Kind | Read from |
//! |---|---|
//! | `job` | the plan, and the Job's diff against its base |
//! | `plan` | the plan's words, when its version is still the current one |
//! | `pull_request` | the pull request's branch against the base, from the checkout |
//! | `commit` | the commit against its first parent, from the checkout; the id is its hash |
//! | `file` | the file as it stands, and its part of the Job's diff |
//! | `page` | the address and the walk's steps the ledger keeps |
//! | `doc` | the newest revision of the page, as text |
//! | `sketch` | the scene the ledger keeps |
//! | `command` | the thread's row of the command and what it printed |
//! | `subagent` | the thread's row of the subagent and what it last said |
//! | `issue` | the title and body the ledger keeps |
//! | `note` | the note's words |

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use ipc::{JudgeArtifact, JudgeKind, SessionRow};
use store::{Holder, KeptAttachment, KeptHosting, KeptSession};

use crate::daemon::Fleet;
use crate::judge::Content;
use crate::repositories::Served;

/// The most of a diff, a file or a page a reading is shown, in characters. Cut here, it says so.
const AT_MOST: usize = 120_000;

fn cut(text: &str) -> String {
    match text.char_indices().nth(AT_MOST) {
        Some((at, _)) => format!("{}\n[the rest is not shown]", &text[..at]),
        None => text.to_string(),
    }
}

fn detail<'a>(held: &'a KeptAttachment, key: &str) -> Option<&'a str> {
    held.detail.get(key).map(String::as_str).map(str::trim).filter(|said| !said.is_empty())
}

fn form<'a>(held: &'a KeptAttachment) -> Option<&'a str> {
    detail(held, "form")
}

/// The part of a patch that is one file's, as `diff --git` headers divide it.
fn file_section(patch: &str, path: &str) -> String {
    let header = format!("diff --git a/{path} b/{path}");
    let mut kept = String::new();
    let mut inside = false;
    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            inside = line == header;
        }
        if inside {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    kept
}

/// A page's text: block ends become blank lines, tags go, and the few entities a page uses are read.
pub(crate) fn text_of(html: &str) -> String {
    let mut plain = String::with_capacity(html.len());
    let (mut tag, mut skipping): (String, Option<&str>) = (String::new(), None);
    let mut in_tag = false;
    for letter in html.chars() {
        match (in_tag, letter) {
            (false, '<') => (in_tag, tag) = (true, String::new()),
            (true, '>') => {
                in_tag = false;
                let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_lowercase();
                let closing = tag.starts_with('/');
                match (skipping, name.as_str()) {
                    (Some(inside), found) if closing && found == inside => skipping = None,
                    (Some(_), _) => {}
                    (None, "script") if !closing => skipping = Some("script"),
                    (None, "style") if !closing => skipping = Some("style"),
                    (None, "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr" | "pre" | "blockquote" | "section" | "div") => {
                        if closing {
                            plain.push_str("\n\n");
                        }
                    }
                    (None, "br") => plain.push('\n'),
                    _ => {}
                }
            }
            (true, letter) => tag.push(letter),
            (false, letter) if skipping.is_none() => plain.push(letter),
            (false, _) => {}
        }
    }
    let plain = plain
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    let mut out = String::new();
    for paragraph in plain.split("\n\n") {
        let words = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
        if !words.is_empty() {
            out.push_str(&words);
            out.push_str("\n\n");
        }
    }
    out.trim_end().to_string()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The Job's work against its base, or why it cannot be read. Empty where the Job holds no branch.
    fn judge_diff(&self, served: &Served, session: &KeptSession, hosting: &KeptHosting) -> Result<String, String> {
        match hosting.lease_branch.as_deref() {
            Some(branch) => {
                let worktree = self.based(served, Worktree::at(session.cwd.clone(), branch));
                self.work().patch(&worktree).map(|patch| cut(patch.as_str())).map_err(|why| format!("the Job's diff would not read: {why}"))
            }
            None => Ok(String::new()),
        }
    }

    /// A branch against the base, from the checkout and no worktree.
    fn judge_branch_diff(&self, served: &Served, session: &KeptSession, hosting: &KeptHosting, branch: &str) -> Result<String, String> {
        if hosting.lease_branch.as_deref() == Some(branch) {
            return self.judge_diff(served, session, hosting);
        }
        let base = served
            .manifest()
            .base()
            .ok_or_else(|| format!("the repository names no base branch to read {branch} against"))?;
        self.work()
            .branch_work(served.root(), base, branch)
            .map(|work| cut(work.patch.as_str()))
            .map_err(|why| format!("the diff of {branch} would not read: {why}"))
    }

    /// The words of one artifact of the Job, or Fleet's reason it has none to show.
    pub(crate) async fn judge_content(
        &self,
        id: &str,
        session: &KeptSession,
        hosting: &KeptHosting,
        served: &Served,
        rows: &[SessionRow],
        artifact: &JudgeArtifact,
    ) -> Result<Content, String> {
        let held = self
            .store()
            .lock()
            .await
            .attachments_of(&Holder::session(id))
            .map_err(|why| format!("the Job's ledger would not read: {why}"))?;
        let title = artifact.title.trim().to_string();
        let named = |text: String| Content { kind: artifact.kind, title: title.clone(), plan: None, text };
        let unheld = |what: &str| format!("Fleet holds no {what} named {}", artifact.id);
        match artifact.kind {
            JudgeKind::Job => {
                let plan = self.store().lock().await.session_plan(id).ok().flatten().map(|kept| crate::plan_record::wire(&kept));
                let diff = self.judge_diff(served, session, hosting)?;
                if plan.is_none() && diff.trim().is_empty() {
                    return Err("the Job has no plan and no diff yet, so there is nothing of its work to read".into());
                }
                Ok(Content { kind: artifact.kind, title, plan, text: diff })
            }
            JudgeKind::Plan => {
                let plan = self.store().lock().await.session_plan(id).ok().flatten();
                let plan = plan.ok_or_else(|| "the Job has recorded no plan".to_string())?;
                if plan.version.to_string() != artifact.id.trim() {
                    return Err("the plan was recorded again, so the version asked about is not the one Fleet holds".into());
                }
                Ok(Content { kind: artifact.kind, title, plan: Some(crate::plan_record::wire(&plan)), text: String::new() })
            }
            JudgeKind::PullRequest => {
                let pull = held
                    .iter()
                    .find(|one| one.kind == "pr" && one.target == artifact.id.trim())
                    .ok_or_else(|| unheld("pull request"))?;
                let diff = match detail(pull, "branch") {
                    Some(branch) => self.judge_branch_diff(served, session, hosting, branch)?,
                    None => self.judge_diff(served, session, hosting)?,
                };
                if diff.trim().is_empty() {
                    return Err("the pull request's branch changes nothing against the base".into());
                }
                Ok(named(diff))
            }
            JudgeKind::Commit => {
                let sha = artifact.id.trim();
                let diff = self
                    .work()
                    .commit_work(served.root(), sha)
                    .map(|work| cut(work.patch.as_str()))
                    .map_err(|why| format!("the commit {sha} would not read: {why}"))?;
                if diff.trim().is_empty() {
                    return Err(format!("the commit {sha} changes nothing"));
                }
                Ok(named(diff))
            }
            JudgeKind::File => {
                let path = artifact.id.trim();
                let absolute = if Path::new(path).is_absolute() { path.to_string() } else { format!("{}/{path}", session.cwd.trim_end_matches('/')) };
                let content = std::fs::read_to_string(&absolute).map_err(|why| format!("the file {path} would not read: {why}"))?;
                let relative = absolute.strip_prefix(&format!("{}/", session.cwd.trim_end_matches('/'))).unwrap_or(path).to_string();
                let section = file_section(&self.judge_diff(served, session, hosting)?, &relative);
                let diff = if section.is_empty() { String::from("(the Job's diff does not touch it)") } else { section };
                Ok(named(format!("{}\n\nIts diff against the base:\n\n{diff}", cut(&content))))
            }
            JudgeKind::Page => {
                let page = held
                    .iter()
                    .find(|one| one.kind == "artifact" && form(one) == Some("window") && one.target == artifact.id.trim())
                    .ok_or_else(|| unheld("page"))?;
                let steps = detail(page, "steps").map(|steps| format!("\nThe walk's steps:\n{steps}\n")).unwrap_or_default();
                Ok(named(format!("Address: {}{steps}", page.target)))
            }
            JudgeKind::Doc => {
                let page = held
                    .iter()
                    .find(|one| one.kind == "artifact" && form(one) == Some("document") && one.target == artifact.id.trim())
                    .ok_or_else(|| unheld("document"))?;
                let newest = detail(page, "revisions")
                    .and_then(|revisions| ipc::decode::<Vec<String>>("a document's revisions", revisions.as_bytes()).ok())
                    .and_then(|revisions| revisions.last().cloned());
                let html = newest
                    .and_then(|path| std::fs::read_to_string(path).ok())
                    .or_else(|| std::fs::read_to_string(&page.target).ok())
                    .ok_or_else(|| "the document's page would not read".to_string())?;
                Ok(named(cut(&text_of(&html))))
            }
            JudgeKind::Sketch => {
                let sketch = held
                    .iter()
                    .find(|one| {
                        one.kind == "artifact"
                            && form(one) == Some("sketch")
                            && (one.target == artifact.id.trim() || detail(one, "title") == Some(artifact.id.trim()))
                    })
                    .ok_or_else(|| unheld("sketch"))?;
                let scene = detail(sketch, "scene").ok_or_else(|| "the sketch holds no scene".to_string())?;
                Ok(named(cut(scene)))
            }
            JudgeKind::Command => {
                let ran = rows.iter().find_map(|row| match row {
                    SessionRow::Ran { id: row, command, code, tail, .. } if *row == artifact.id.trim() || command.trim() == artifact.id.trim() => {
                        Some((command, code, tail))
                    }
                    _ => None,
                });
                let (command, code, tail) = ran.ok_or_else(|| unheld("command"))?;
                let code = code.map(|code| format!("exit code {code}")).unwrap_or_else(|| String::from("no exit code"));
                Ok(named(format!("Command: {}\n{code}\n\nWhat it printed, the end of it:\n{}", command.trim(), cut(tail))))
            }
            JudgeKind::Subagent => {
                let subagent = rows.iter().find_map(|row| match row {
                    SessionRow::Tool { id: row, subagent: Some(subagent), .. } if *row == artifact.id.trim() || subagent.id == artifact.id.trim() => {
                        Some(subagent)
                    }
                    _ => None,
                });
                let subagent = subagent.ok_or_else(|| unheld("subagent"))?;
                let report = subagent.latest.as_deref().map(str::trim).filter(|said| !said.is_empty());
                let report = report.ok_or_else(|| "Fleet holds no report from the subagent yet".to_string())?;
                Ok(named(format!("Its task: {}\n\nWhat it reported:\n{}", subagent.task.trim(), cut(report))))
            }
            JudgeKind::Issue => {
                let issue = held
                    .iter()
                    .find(|one| one.kind == "issue" && one.target == artifact.id.trim())
                    .ok_or_else(|| unheld("issue"))?;
                let body = detail(issue, "body");
                let said = detail(issue, "title").into_iter().chain(body).collect::<Vec<_>>().join("\n\n");
                if said.is_empty() {
                    return Err(format!("Fleet holds only the number of issue {}, not its words", issue.target));
                }
                Ok(named(cut(&said)))
            }
            JudgeKind::Note => {
                let note = held
                    .iter()
                    .find(|one| one.kind == "artifact" && form(one) == Some("note") && (one.target == artifact.id.trim() || detail(one, "note") == Some(artifact.id.trim())))
                    .ok_or_else(|| unheld("note"))?;
                let text = detail(note, "text").ok_or_else(|| "the note holds no words".to_string())?;
                Ok(named(text.to_string()))
            }
        }
    }
}

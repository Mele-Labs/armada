//! What Fleet writes for one message to the Operator to read: a file for each live Job and one of the open
//! issues, as plain text, his newest answers and his settings, and the journal of this week and last week. `looking` makes the folder and hands it to the call.
//!
//! **Read the way the rest of the box is**: the plan, the open tasks and the thread come from what Fleet
//! already keeps, and the issues from the list it shows Bridge, held for five minutes. A Job's Needs you
//! items are written in full. What will not read is left out of its file rather than failing the message.

use std::fmt::Write as _;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{SessionRow, SessionVoice};
use store::KeptPlan;

use super::looking::ToRead;
use super::{LiveJob, NeedsYou};
use crate::daemon::Fleet;

/// How many of the last messages of one Job's thread its file holds.
pub(super) const THREAD_MESSAGES: usize = 40;
/// The most characters of one message a file holds.
const MESSAGE_CHARS: usize = 3000;
/// The file the open issues are in.
pub const ISSUES_FILE: &str = "issues.txt";
/// The file the open tasks of his projects are in, those of his other Macs' projects among them.
pub const TASKS_FILE: &str = "tasks.txt";
/// The most tasks `tasks.txt` lists; the rest are counted.
const TASKS_LISTED: usize = 300;
/// The most characters of one task's line.
const TASK_LINE_CHARS: usize = 300;

/// The file name of the Job `id`.
pub fn job_file(id: &str) -> String {
    format!("job-{id}.txt")
}

fn clipped(text: &str) -> String {
    match text.char_indices().nth(MESSAGE_CHARS) {
        Some((at, _)) => format!("{}...", &text[..at]),
        None => text.to_string(),
    }
}

pub(super) fn plan_written(out: &mut String, plan: &KeptPlan) {
    let approved = if plan.approved_at.is_some() { "approved" } else { "not yet approved" };
    let _ = writeln!(out, "\nIts plan, version {}, {approved}:\n{}", plan.version, plan.summary.trim());
    for group in &plan.groups {
        let _ = writeln!(out, "  Group {}: {}", group.id, group.title);
        if let Some(gives) = group.gives.as_deref().filter(|gives| !gives.trim().is_empty()) {
            let _ = writeln!(out, "    Gives: {gives}");
        }
        if !group.paths.is_empty() {
            let _ = writeln!(out, "    Paths: {}", group.paths.join(", "));
        }
        if let Some(issue) = &group.issue {
            let _ = writeln!(out, "    Issue: {issue}");
        }
        for task in &group.tasks {
            let state = match (&task.completed_at, &task.listed) {
                (Some(_), _) => "done",
                (None, Some(listed)) => listed.as_str(),
                (None, None) => "open",
            };
            let _ = writeln!(out, "    Task ({state}): {}", task.text);
        }
    }
}

/// `text` cut to `chars` characters, with `...` where it was cut.
fn cut_to(text: &str, chars: usize) -> String {
    match text.char_indices().nth(chars) {
        Some((at, _)) => format!("{}...", &text[..at]),
        None => text.to_string(),
    }
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
    /// The files a message may read, written from `live` and the items waiting on the person, and where
    /// to make them.
    pub(crate) async fn operator_to_read(&self, live: &[LiveJob], needs: &[NeedsYou]) -> ToRead {
        let mut files = Vec::new();
        {
            let store = self.store().lock().await;
            for job in live {
                let mut out = format!("Job {}: \"{}\"\nRepository: {}\n", job.session_id, job.title, job.manifest_id);
                if let Ok(Some(plan)) = store.session_plan(&job.session_id) {
                    plan_written(&mut out, &plan);
                }
                let tasks: Vec<String> = store
                    .session_build_tasks(&job.session_id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|task| task.status != "completed")
                    .map(|task| task.text)
                    .collect();
                out.push_str("\nTasks still open on its task list:\n");
                for task in &tasks {
                    let _ = writeln!(out, "  {task}");
                }
                out.push_str("\nWaiting on the person, written in full:\n");
                for item in needs.iter().filter(|item| item.session_id == job.session_id) {
                    let _ = writeln!(out, "  Item {}: {}", item.id, item.text);
                }
                let mut thread: Vec<String> = store
                    .session_rows(&job.session_id)
                    .unwrap_or_default()
                    .iter()
                    .rev()
                    .filter_map(|body| match ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok()? {
                        SessionRow::Message { from, text, .. } if !text.trim().is_empty() => {
                            let who = match from {
                                SessionVoice::You => "The person".to_string(),
                                SessionVoice::Agent => "Its agent".to_string(),
                                SessionVoice::Session { id, title, .. } => format!("Job {id} \"{title}\""),
                            };
                            Some(format!("{who}: {}", clipped(text.trim())))
                        }
                        _ => None,
                    })
                    .take(THREAD_MESSAGES)
                    .collect();
                thread.reverse();
                let _ = writeln!(out, "\nThe last {THREAD_MESSAGES} messages of its thread, oldest first:");
                for message in thread {
                    let _ = writeln!(out, "\n{message}");
                }
                files.push((job_file(&job.session_id), out));
            }
        }
        let mut repositories = Vec::new();
        let mut issues = String::from("The issues open in each repository's tracker.\n");
        for served in self.repositories().hosted() {
            let id = served.manifest().id().clone();
            let root = served.root().to_string();
            let name = std::path::Path::new(&root).file_name().map_or_else(|| root.clone(), |name| name.to_string_lossy().to_string());
            let _ = writeln!(issues, "\nRepository {name} (id {}):", id.as_str());
            match self.open_issues(&ipc::ManifestId::carried(id.as_str())).await {
                Ok(open) => {
                    for one in open.iter().filter(|one| !one.closed) {
                        let _ = writeln!(issues, "\n  {} {}", one.reference, one.title.as_written());
                        let labels: Vec<&str> = one.labels.iter().map(|label| label.as_written()).collect();
                        if !labels.is_empty() {
                            let _ = writeln!(issues, "    Labels: {}", labels.join(", "));
                        }
                        let excerpt = one.excerpt.as_written().trim();
                        if !excerpt.is_empty() {
                            let _ = writeln!(issues, "    {excerpt}");
                        }
                    }
                }
                Err(_) => issues.push_str("  The tracker could not be read just now.\n"),
            }
            repositories.push((id.as_str().to_string(), name, root));
        }
        files.push((ISSUES_FILE.to_string(), issues));
        files.push((TASKS_FILE.to_string(), self.operator_tasks_file(&repositories).await));
        files.push((super::precedent::ANSWERS_FILE.to_string(), self.operator_answers_file().await));
        files.push((super::precedent::SETTINGS_FILE.to_string(), self.operator_settings_file().await));
        files.extend(self.operator_journal_files().await);
        ToRead { under: self.scouts().host().runtime_dir().join("operator-desk"), files, repositories }
    }
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
    /// The open tasks (not done) of every project, this Mac's and the copies of his other Macs', a line each:
    /// the title, the project, the Mac it is on where another, the repository, and the Job holding it. **The
    /// Operator only reads it**: no task is closed by what it says.
    async fn operator_tasks_file(&self, repositories: &[(String, String, String)]) -> String {
        let mut out = String::from("The open tasks of his projects, one a line.\n\n");
        let Ok(projects) = api::Projects::list_projects(self).await else {
            out.push_str("The projects could not be read just now.\n");
            return out;
        };
        let name_of = |id: &str| -> String {
            repositories.iter().find(|(known, _, _)| known == id).map_or_else(|| id.to_string(), |(_, name, _)| name.clone())
        };
        let mut listed = 0usize;
        let mut more = 0usize;
        for project in &projects.projects {
            for row in &project.rows {
                let ipc::ProjectRow::Task { title, manifest_id, state, held_by, .. } = row else { continue };
                if matches!(state, ipc::ProjectTaskState::Done) {
                    continue;
                }
                if listed == TASKS_LISTED {
                    more += 1;
                    continue;
                }
                listed += 1;
                let mut line = format!("{} (project {}", cut_to(title.trim(), TASK_LINE_CHARS), project.name);
                if let Some(mac) = &project.mac {
                    let _ = write!(line, ", on Mac {mac}");
                }
                let _ = write!(line, ", repository {}", name_of(manifest_id.as_str()));
                if let Some(holder) = held_by {
                    let _ = write!(line, ", held by Job {}", holder.session_id);
                }
                let _ = writeln!(out, "{line})");
            }
        }
        if more > 0 {
            let _ = writeln!(out, "\n...and {more} more.");
        }
        out
    }
}

//! What a tool-call row opens, as a tab of the right column's top pane: the file a Read, Edit or Write
//! named, read-only, and the Check run a call started. Each tab draws the body its panel draws, from an
//! entity of its own, so a press on the same row brings the open tab forward and opens no second one.

use gpui::{div, prelude::*, Context, Role, SharedString, Task, WeakEntity, Window};
use ipc::CheckOutput;

use super::panes::{CheckTab, FileRead, Refusal};
use super::rows::mono;
use super::view_tabs::ViewTab;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// A running Check's log is read again this often, as the panel does.
const EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// The id of the tab holding the file at `path`.
fn file_id(path: &str) -> String {
    format!("call-file:{path}")
}

/// The id of the tab holding Check run `run`.
fn check_id(run: u64) -> String {
    format!("call-check:{run}")
}

/// The file tab's name: the last part of its path.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The file at `path` of a Job's repository, as Fleet reads it, or as the mock holds it.
struct FileTab {
    path: String,
    read: Option<FileRead>,
    _read: Option<Task<()>>,
}

impl FileTab {
    fn new(job: &str, path: String, mock: bool, port: Option<u16>, cx: &mut Context<Self>) -> Self {
        if mock {
            let read = crate::mock::repository_file(&path).map(|read| read.map_or_else(|why| FileRead::Refused(Refusal::from_wire(&why)), FileRead::Text));
            return FileTab { path, read, _read: None };
        }
        let Some(port) = port else { return FileTab { read: Some(FileRead::Refused(Refusal::Unreadable)), path, _read: None } };
        let ask = format!("/sessions/repository_file?session_id={}&path={}", super::wire::encoded(job), super::wire::encoded(&path));
        let task = cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| (why.to_string(), None))?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &ask)).map_err(|why| (why, None))?;
                    if (200..300).contains(&status) {
                        Ok(bytes)
                    } else {
                        Err(super::wire::refusal(status, &bytes))
                    }
                })
                .await;
            let read = match answer {
                Ok(body) => match ipc::decode::<ipc::RepositoryFile>("RepositoryFile", &body) {
                    Ok(ipc::RepositoryFile::Text { text, .. }) => FileRead::Text(text),
                    Ok(ipc::RepositoryFile::Picture { .. }) => FileRead::Refused(Refusal::Binary),
                    Err(_) => FileRead::Refused(Refusal::Unreadable),
                },
                Err((_, code)) => {
                    let code = code.unwrap_or_default();
                    FileRead::Refused(Refusal::from_wire(code.strip_prefix("fleet.repository_file_").unwrap_or(&code)))
                }
            };
            let _ = this.update(cx, |this, cx| {
                this.read = Some(read);
                cx.notify();
            });
        });
        FileTab { path, read: None, _read: Some(task) }
    }
}

impl Render for FileTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let look = &look;
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(labelled(
                div()
                    .id("call-file-path")
                    .flex_none()
                    .truncate()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(self.path.clone())),
                Role::Group,
                self.path.clone(),
            ))
            .child(JobView::file_body(self.read.as_ref(), look))
    }
}

/// Check run `run` of the Check `name` in a Job: its runs, and the run's log, read again while it runs.
struct CheckTabView {
    job: SharedString,
    name: String,
    run: u64,
    tab: CheckTab,
    output: Option<Result<CheckOutput, String>>,
    view: WeakEntity<JobView>,
    _poll: Option<Task<()>>,
}

impl CheckTabView {
    fn new(job: SharedString, name: String, run: u64, view: WeakEntity<JobView>, port: Option<u16>, cx: &mut Context<Self>) -> Self {
        let mut tab = CheckTabView { job, name, run, tab: CheckTab::Runs, output: None, view, _poll: None };
        let Some(port) = port else {
            tab.output = crate::mock::check_output(run).map(Ok);
            return tab;
        };
        let path = format!("/sessions/checks/{run}/output");
        tab._poll = Some(cx.spawn(async move |this, cx| loop {
            let path = path.clone();
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<CheckOutput>("CheckOutput", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let running = this.update(cx, |this, cx| {
                this.output = Some(answer);
                cx.notify();
                this.running(cx)
            });
            if !running.unwrap_or(false) {
                return;
            }
            cx.background_executor().timer(EVERY).await;
        }));
        tab
    }

    /// Whether Fleet still has this run running.
    fn running(&self, cx: &gpui::App) -> bool {
        let Some(view) = self.view.upgrade() else { return false };
        view.read(cx).check_rows(&self.job, cx).iter().any(|row| matches!(row, ipc::SessionRow::Check { run, state: ipc::SessionCheckState::Running, .. } if *run == self.run))
    }
}

impl Render for CheckTabView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let look = &look;
        let rows = self.view.upgrade().map(|view| view.read(cx).check_rows(&self.job, cx)).unwrap_or_default();
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(JobView::check_body(&self.name, self.run, self.tab, self.output.as_ref(), &JobView::runs_of(rows, &self.name), look, |which| {
                cx.listener(move |this, _, _, cx| {
                    this.tab = which;
                    cx.notify();
                })
            }))
    }
}

impl JobView {
    /// The file a call named, in a tab of the top pane; one already open comes forward.
    pub(super) fn open_file_tab(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let id = file_id(&path);
        if self.front_view_tab(&job, &id, cx) {
            return;
        }
        let (mock, port) = (self.fleet.is_none(), self.port(cx));
        let title = SharedString::from(file_name(&path).to_string());
        let view = cx.new(|cx| FileTab::new(&job, path, mock, port, cx));
        self.open_view_tab(&job, ViewTab { id, title, glyph: "file", view: view.into(), menu: Vec::new(), closing: None, closed: None, renamed: None }, true, cx);
    }

    /// The Check run a call started, in a tab of the top pane; one already open comes forward.
    pub(super) fn open_check_tab(&mut self, name: String, run: u64, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let id = check_id(run);
        if self.front_view_tab(&job, &id, cx) {
            return;
        }
        let port = self.port(cx);
        let title = SharedString::from(format!("Check {name}"));
        let me = cx.entity().downgrade();
        let view = cx.new(|cx| CheckTabView::new(job.clone(), name, run, me, port, cx));
        self.open_view_tab(&job, ViewTab { id, title, glyph: "shield-check", view: view.into(), menu: Vec::new(), closing: None, closed: None, renamed: None }, true, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_and_a_check_run_each_keep_one_tab_by_their_own_id() {
        assert_eq!(file_id("crates/x/src/router.rs"), "call-file:crates/x/src/router.rs");
        assert_eq!(check_id(7), "call-check:7");
        assert_eq!(file_name("crates/x/src/router.rs"), "router.rs");
        assert_eq!(file_name("router.rs"), "router.rs");
    }
}

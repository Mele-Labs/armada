//! A pull request panel's Files tab (`PullRequestFiles`): the file list floating beside the panel,
//! then one section a file, its head over its lines, with the judge's notes and the review threads on
//! that file under the head. A judge note that names no line stands at the top, above every file.
//!
//! **The native diff has no line numbers yet**, so a note or a thread stands under its file's head
//! rather than between its lines, and a line cannot be pressed to write a note of his own.

use chrono::Local;
use gpui::{div, point, prelude::*, px, AnyElement, Context, FontWeight, Pixels, Role, SharedString, Window};
use ipc::{JudgeKind, JudgeNote, PullRequestFileStatus, PullRequestFilePatches, PullRequestPage, PullReviewThread};

use super::pr_panel::{diff_size, mark, stand_ins, PullPanel, Tone};
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::diff;
use crate::labels::labelled;
use crate::look::{token, Look};

/// A file's status as its glyph, word and hue (`STATUS`).
fn status_look(status: PullRequestFileStatus) -> (&'static str, &'static str, Option<Tone>) {
    match status {
        PullRequestFileStatus::Added => ("square-plus", "Added", Some(Tone::Success)),
        PullRequestFileStatus::Modified => ("file-pen-line", "Modified", Some(Tone::Running)),
        PullRequestFileStatus::Removed => ("trash-2", "Removed", Some(Tone::Failed)),
        PullRequestFileStatus::Renamed => ("corner-up-right", "Renamed", None),
    }
}

/// A judge note's file and line, where it names both (`path:line`, a range read from its first line).
pub(super) fn line_of(place: &str) -> Option<(&str, u64)> {
    let (path, tail) = place.rsplit_once(':')?;
    let first = tail.split('-').next()?;
    (!path.is_empty() && !first.is_empty() && first.chars().all(|one| one.is_ascii_digit())).then(|| (path, first.parse().ok())).and_then(|(path, line)| Some((path, line?)))
}

/// One row of the file list: a folder, its one-child folders folded into its name, or a file.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Branch {
    pub name: String,
    /// The file's place in the page's `files`; None for a folder.
    pub file: Option<usize>,
    pub under: Vec<Branch>,
}

/// The files as a tree: folders first, each kind by name, a folder holding one folder alone folded
/// into it (`treeOf`).
pub(super) fn tree_of(paths: &[&str]) -> Vec<Branch> {
    let mut root: Vec<Branch> = Vec::new();
    for (at, path) in paths.iter().enumerate() {
        let parts: Vec<&str> = path.split('/').collect();
        let mut level = &mut root;
        for (depth, part) in parts.iter().enumerate() {
            let last = depth + 1 == parts.len();
            let found = level.iter().position(|one| one.name == *part && one.file.is_none() != last);
            let ix = match found {
                Some(ix) => ix,
                None => {
                    level.push(Branch { name: (*part).to_string(), file: last.then_some(at), under: Vec::new() });
                    level.len() - 1
                }
            };
            level = &mut level[ix].under;
        }
    }
    fn settle(level: &mut Vec<Branch>) {
        for one in level.iter_mut() {
            while one.file.is_none() && one.under.len() == 1 && one.under[0].file.is_none() {
                let only = one.under.remove(0);
                one.name = format!("{}/{}", one.name, only.name);
                one.under = only.under;
            }
            settle(&mut one.under);
        }
        level.sort_by(|a, b| a.file.is_some().cmp(&b.file.is_some()).then_with(|| a.name.cmp(&b.name)));
    }
    settle(&mut root);
    root
}

impl JobView {
    /// Fleet's patches for the open pull request, read once and kept as the Changes pane keeps them.
    pub(super) fn want_pull_patches(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let Some(key) = panel.patches_key() else { return };
        if self.panes.patches.contains_key(&key) {
            return;
        }
        let (Some(manifest), number) = (panel.pull.manifest.clone(), panel.pull.number) else { return };
        if self.fleet.is_none() {
            if let Some(patches) = crate::mock::patches(&key) {
                self.panes.patches.insert(key, Ok(patches));
            }
            return;
        }
        self.get(
            format!("/pull_request_patches/{}/{number}", super::wire::encoded(&manifest)),
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<PullRequestFilePatches>("PullRequestFilePatches", &bytes).map_err(|why| why.to_string()));
                this.panes.patches.insert(key, read);
                cx.notify();
            }),
        );
    }

    /// The patch of one file of the open pull request, where Fleet has read it.
    fn pull_patch<'a>(&'a self, panel: &PullPanel, path: &str) -> Option<&'a ipc::PullRequestFilePatch> {
        let key = panel.patches_key()?;
        self.panes.patches.get(&key)?.as_ref().ok()?.files.iter().find(|one| one.path == path)
    }

    /// The lines a thread is about, under its head in Conversation.
    pub(super) fn thread_excerpt(&self, panel: &PullPanel, thread: &PullReviewThread, look: &Look) -> Option<AnyElement> {
        let line = thread.line?;
        let patch = self.pull_patch(panel, &thread.path)?.patch.as_deref()?;
        let lines = diff::excerpt(patch, line, thread.side == ipc::DiffSide::Left)?;
        Some(div().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(diff::lines(format!("pull-thread-code:{}", thread.id), &lines, look)).into_any_element())
    }

    /// A file pressed in the list: it is marked, and the body scrolls to its section.
    fn pull_file_pressed(&mut self, at: usize, path: String, cx: &mut Context<Self>) {
        if let Some(panel) = self.pull_panel_mut() {
            panel.current = Some(path);
            if let Some(item) = panel.sections.bounds_for_item(at) {
                let body = panel.scroll.bounds();
                let offset = panel.scroll.offset();
                let y = offset.y - (item.origin.y - body.origin.y);
                panel.scroll.set_offset(point(offset.x, y.min(px(0.))));
            }
        }
        cx.notify();
    }

    /// Files: the judge's notes that name no line, then each file's section. No files, nothing drawn.
    pub(super) fn pull_files(&self, panel: &PullPanel, page: &PullRequestPage, tree_inside: bool, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if page.files.is_empty() {
            return None;
        }
        let number = panel.pull.number;
        let key = format!("ledger:pr:{number}");
        let job = self.shown.clone()?;
        let job_page = self.pages.get(&job)?;
        let reading = self.record(&job, cx).and_then(|record| super::judge::reading_of(record, JudgeKind::PullRequest, &number.to_string()));
        let pinned = reading.and_then(|reading| super::judge::Pinned::whole_job(super::acts::SHEET, &key, reading));
        let notes: Vec<&JudgeNote> = pinned.as_ref().map(|at| at.whole()).unwrap_or_default();
        let on_file = |path: &str| notes.iter().copied().filter(|note| note.r#where.as_deref().and_then(line_of).is_some_and(|(at, _)| at == path)).collect::<Vec<_>>();
        let lineless: Vec<&JudgeNote> = notes
            .iter()
            .copied()
            .filter(|note| note.r#where.as_deref().and_then(line_of).is_none_or(|(path, _)| !page.files.iter().any(|file| file.path == path)))
            .collect();
        let top = pinned.as_ref().filter(|_| !lineless.is_empty()).and_then(|at| self.notes_list(at, lineless, "Notes from the judge".into(), job_page, look, cx));
        let resolved = page.threads.iter().filter(|thread| thread.resolved).count();
        let toggle = (resolved > 0).then(|| {
            let shown = panel.show_resolved;
            let said = if shown { "Hide resolved".to_string() } else { format!("Show resolved ({resolved})") };
            labelled(
                super::pr_panel::small_button(look, "pull-files-resolved", &said, false, true).on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(panel) = this.pull_panel_mut() {
                        panel.show_resolved = !shown;
                    }
                    cx.notify();
                })),
                Role::Button,
                said.clone(),
            )
        });
        let now = Local::now();
        let read = panel.patches_key().and_then(|key| self.panes.patches.get(&key));
        let sections: Vec<AnyElement> = page
            .files
            .iter()
            .enumerate()
            .map(|(at, file)| {
                let (icon, said, tone) = status_look(file.status);
                let current = panel.current.as_deref() == Some(file.path.as_str());
                let head = div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_w_0()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .bg(look.color(token::BG_RAISED))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(mark(look, format!("pull-file-status:{at}"), icon, said, tone))
                    .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(file.path.clone())))
                    .child(diff_size(look, format!("pull-file-size:{at}"), file.additions, file.deletions));
                let file_notes = pinned.as_ref().and_then(|at_| {
                    let mine = on_file(&file.path);
                    (!mine.is_empty()).then(|| self.notes_list(at_, mine, format!("Notes from the judge on {}", file.path), job_page, look, cx)).flatten()
                });
                let threads: Vec<AnyElement> = page
                    .threads
                    .iter()
                    .filter(|thread| thread.path == file.path && (panel.show_resolved || !thread.resolved))
                    .map(|thread| self.pull_thread(panel, &panel.pull, thread, look, now, cx))
                    .collect();
                let under = (file_notes.is_some() || !threads.is_empty())
                    .then(|| div().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_2)).border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).children(file_notes).children(threads));
                let body: AnyElement = match read {
                    None => div().p(look.length(token::SPACE_3)).child(stand_ins(look, &format!("pull-file-reading:{at}"), &[0.9, 0.7, 0.8])).into_any_element(),
                    Some(Err(why)) => quiet(look, why),
                    Some(Ok(patches)) => {
                        let one = patches.files.iter().find(|one| one.path == file.path);
                        let drawn = diff::file_diff_of(&file.path, one.and_then(|one| one.patch.as_deref()), one.and_then(|one| one.said.as_deref()));
                        match drawn.files.first() {
                            Some(lines) => div().flex().flex_col().children(drawn.cut.clone().map(|cut| quiet(look, &cut))).child(diff::lines(format!("pull-file-lines:{at}"), &lines.lines, look)).into_any_element(),
                            None => quiet(look, drawn.empty.as_deref().unwrap_or("")),
                        }
                    }
                };
                labelled(
                    div()
                        .id(SharedString::from(format!("pull-file:{at}")))
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .overflow_hidden()
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(if current { look.color(token::ACCENT) } else { look.color(token::BORDER_SUBTLE) })
                        .child(head)
                        .children(under)
                        .child(body),
                    Role::Group,
                    file.path.clone(),
                )
                .into_any_element()
            })
            .collect();
        // Where the window has no room for the list beside the panel, it stands above the files.
        let tree = tree_inside.then(|| {
            let paths: Vec<&str> = page.files.iter().map(|file| file.path.as_str()).collect();
            let mut rows: Vec<AnyElement> = Vec::new();
            self.file_rows(panel, page, &tree_of(&paths), 0, look, cx, &mut rows);
            labelled(
                div().id("pull-file-tree").flex().flex_col().py(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_MD)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).children(rows),
                Role::Tree,
                "Files",
            )
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .children(top)
                .children(tree)
                .children(toggle.map(|toggle| div().flex().child(toggle)))
                .child(div().id("pull-files").track_scroll(&panel.sections).flex().flex_col().gap(look.length(token::SPACE_3)).children(sections))
                .into_any_element(),
        )
    }

    /// The file list, floating beside the panel while Files is in front (`PullRequestFileList`).
    pub(super) fn pull_file_list(&self, panel: &PullPanel, page: &PullRequestPage, wide: Pixels, high: Pixels, look: &Look, cx: &Context<Self>) -> AnyElement {
        let paths: Vec<&str> = page.files.iter().map(|file| file.path.as_str()).collect();
        let mut rows: Vec<AnyElement> = Vec::new();
        self.file_rows(panel, page, &tree_of(&paths), 0, look, cx, &mut rows);
        labelled(
            div()
                .id("pull-file-list")
                .occlude()
                .flex_none()
                .w(wide)
                .max_h(high)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(
                    div()
                        .flex_none()
                        .px(look.length(token::SPACE_4))
                        .py(look.length(token::SPACE_3))
                        .border_b(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .text_size(look.length(token::TEXT_SM))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .child("Files"),
                )
                .child(labelled(div().id("pull-file-tree").flex_1().min_h_0().overflow_y_scroll().py(look.length(token::SPACE_2)).flex().flex_col().children(rows), Role::Tree, "Files")),
            Role::Complementary,
            "Files",
        )
        .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn file_rows(&self, panel: &PullPanel, page: &PullRequestPage, level: &[Branch], depth: usize, look: &Look, cx: &Context<Self>, out: &mut Vec<AnyElement>) {
        let indent = look.length(token::SPACE_3) + look.length(token::SPACE_4) * depth as f32;
        for one in level {
            match one.file {
                None => {
                    out.push(
                        labelled(
                            div()
                                .id(SharedString::from(format!("pull-folder:{depth}:{}", one.name)))
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .pl(indent)
                                .pr(look.length(token::SPACE_3))
                                .py(px(3.))
                                .text_size(look.length(token::TEXT_XS))
                                .text_color(look.color(token::FG_MUTED))
                                .child(glyph(look, "folder", Size::Small, look.color(token::FG_SUBTLE)))
                                .child(div().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(one.name.clone()))),
                            Role::TreeItem,
                            one.name.clone(),
                        )
                        .into_any_element(),
                    );
                    self.file_rows(panel, page, &one.under, depth + 1, look, cx, out);
                }
                Some(at) => {
                    let file = &page.files[at];
                    let (icon, said, tone) = status_look(file.status);
                    let current = panel.current.as_deref() == Some(file.path.as_str());
                    let path = file.path.clone();
                    let hover = look.color(token::BG_HOVER);
                    out.push(
                        labelled(
                            div()
                                .id(SharedString::from(format!("pull-file-row:{at}")))
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .pl(indent)
                                .pr(look.length(token::SPACE_3))
                                .py(px(3.))
                                .text_size(look.length(token::TEXT_XS))
                                .cursor_pointer()
                                .when(current, |row| row.bg(look.color(token::ACCENT).opacity(0.14)))
                                .hover(move |style| style.bg(hover))
                                .tooltip(tip(file.path.clone()))
                                .on_click(cx.listener(move |this, _, _, cx| this.pull_file_pressed(at, path.clone(), cx)))
                                .child(mark(look, format!("pull-file-row-status:{at}"), icon, said, tone))
                                .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(one.name.clone()))),
                            Role::TreeItem,
                            file.path.clone(),
                        )
                        .into_any_element(),
                    );
                }
            }
        }
    }
}

/// A sentence in the place of a file's lines: why it has none, or where they were cut.
fn quiet(look: &Look, said: &str) -> AnyElement {
    div()
        .p(look.length(token::SPACE_3))
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(said.to_string()))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_names_a_line_only_with_digits_after_its_last_colon() {
        assert_eq!(line_of("src/a.rs:40"), Some(("src/a.rs", 40)));
        assert_eq!(line_of("src/a.rs:40-44"), Some(("src/a.rs", 40)));
        assert_eq!(line_of("src/a.rs"), None);
        assert_eq!(line_of("the whole change"), None);
    }

    #[test]
    fn the_tree_puts_folders_first_and_folds_a_lone_folder() {
        let tree = tree_of(&["crates/app/src/main.rs", "crates/app/src/lib.rs", "README.md"]);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].name, "crates/app/src");
        assert_eq!(tree[0].under.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), vec!["lib.rs", "main.rs"]);
        assert_eq!(tree[1].name, "README.md");
        assert_eq!(tree[1].file, Some(2));
    }
}

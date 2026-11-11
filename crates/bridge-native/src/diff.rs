//! A unified diff, as Electron's `UnifiedDiff` draws it: one box per file under its path, each line
//! a `+`, `-`, context or hunk row in mono, no line numbers and no wrap. A Job's Changes pane draws
//! it now; the thread's diffs and Reviews take the same one later.
//!
//! **The split is `drawn-patch.ts` ported as it is**, so both apps cut a long patch at the same line.

use gpui::{div, prelude::*, AnyElement, App, ClipboardItem, ElementId, Role, SharedString};

use crate::labels::labelled;
use crate::look::{token, Look};

/// The most lines drawn; the forge holds the rest.
pub const DRAWN_LINES: usize = 2000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineKind {
    Hunk,
    Added,
    Removed,
    Context,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DiffLine {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DiffFile {
    pub path: String,
    /// The mode, rename and binary lines git writes above the hunks, joined by ` · `.
    pub meta: Option<String>,
    pub lines: Vec<DiffLine>,
}

/// What one diff draws: its files, the sentence a cut patch says, or the sentence where it has none.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Drawn {
    pub files: Vec<DiffFile>,
    pub cut: Option<String>,
    pub empty: Option<String>,
}

const FILE_HEADER: &str = "diff --git ";
const META: [&str; 9] = ["new file mode ", "deleted file mode ", "old mode ", "new mode ", "rename from ", "rename to ", "copy from ", "copy to ", "Binary files "];
const UNNAMED: &str = "(the patch names no file)";
const NO_CHANGE: &str = "No lines changed in this file.";

/// A patch split into files, cut at [`DRAWN_LINES`] with `cut_said(drawn, total)`.
pub fn drawn_of(patch: &str, cut_said: impl Fn(usize, usize) -> String) -> Drawn {
    let mut files: Vec<DiffFile> = Vec::new();
    let mut in_hunk = false;
    let (mut drawn, mut total, mut cut) = (0, 0, false);
    for row in patch.split('\n') {
        if let Some(rest) = row.strip_prefix(FILE_HEADER) {
            files.push(DiffFile { path: path_of(rest).unwrap_or_else(|| UNNAMED.to_string()), meta: None, lines: Vec::new() });
            in_hunk = false;
            continue;
        }
        if !in_hunk {
            if row.starts_with("+++ ") || row.starts_with("--- ") {
                if let (Some(open), Some(named)) = (files.last_mut(), stripped(&row[4..])) {
                    open.path = named;
                }
                continue;
            }
            if let Some(open) = files.last_mut().filter(|_| META.iter().any(|prefix| row.starts_with(prefix))) {
                open.meta = Some(match open.meta.take() {
                    None => row.to_string(),
                    Some(before) => format!("{before} \u{b7} {row}"),
                });
                continue;
            }
        }
        let Some(kind) = kind_of(row) else { continue };
        if kind == LineKind::Hunk {
            in_hunk = true;
        }
        total += 1;
        if drawn >= DRAWN_LINES {
            cut = true;
            continue;
        }
        if files.is_empty() {
            files.push(DiffFile { path: UNNAMED.to_string(), meta: None, lines: Vec::new() });
        }
        if let Some(open) = files.last_mut() {
            open.lines.push(DiffLine { kind, text: row.to_string() });
        }
        drawn += 1;
    }
    files.retain(|file| !file.lines.is_empty());
    Drawn { files, cut: cut.then(|| cut_said(drawn, total)), empty: None }
}

/// One file of a pull request, as the forge gave its patch (`fileDiffOf`). The forge's patch has no
/// `diff --git` line, so one is written for the split to name the file by; a file with no patch says
/// Fleet's sentence.
pub fn file_diff_of(path: &str, patch: Option<&str>, said: Option<&str>) -> Drawn {
    let Some(patch) = patch else {
        return Drawn { empty: Some(said.unwrap_or(NO_CHANGE).to_string()), ..Drawn::default() };
    };
    let drawn = drawn_of(&format!("diff --git a/{path} b/{path}\n{patch}"), |shown, total| {
        format!("The first {} of {} lines. Open the pull request in the browser for the rest.", grouped(shown), grouped(total))
    });
    if drawn.files.is_empty() {
        return Drawn { empty: Some(NO_CHANGE.to_string()), ..Drawn::default() };
    }
    drawn
}

fn kind_of(row: &str) -> Option<LineKind> {
    if row.starts_with("@@") {
        Some(LineKind::Hunk)
    } else if row.starts_with('+') {
        Some(LineKind::Added)
    } else if row.starts_with('-') {
        Some(LineKind::Removed)
    } else if row.starts_with(' ') || row.starts_with('\\') {
        Some(LineKind::Context)
    } else {
        None
    }
}

fn path_of(rest: &str) -> Option<String> {
    let half = rest.len() / 2;
    if rest.is_char_boundary(half) && rest.as_bytes().get(half) == Some(&b' ') {
        return stripped(&rest[half + 1..]);
    }
    rest.find(" b/").and_then(|cut| stripped(&rest[cut + 1..]))
}

fn stripped(value: &str) -> Option<String> {
    let path = value.trim();
    if path.is_empty() || path == "/dev/null" {
        return None;
    }
    Some(path.strip_prefix("a/").or_else(|| path.strip_prefix("b/")).unwrap_or(path).to_string())
}

/// A count with its thousands marked, as `toLocaleString` writes it.
fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (ix, digit) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn mono(look: &Look) -> SharedString {
    look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()
}

/// The diff drawn: the cut first, then each file, or the empty sentence alone. `id` keeps each file's
/// sideways scroll apart from another diff's.
pub fn unified(id: impl Into<SharedString>, drawn: &Drawn, look: &Look) -> AnyElement {
    let id: SharedString = id.into();
    if drawn.files.is_empty() {
        return match &drawn.empty {
            None => div().into_any_element(),
            Some(said) => labelled(
                div().id(ElementId::Name(format!("{id}:empty").into()))
                    .p(look.length(token::SPACE_4))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(said.clone())),
                Role::Note,
                said.clone(),
            )
            .into_any_element(),
        };
    }
    let font = mono(look);
    let cut = drawn.cut.clone().map(|said| {
        labelled(
            div().id(ElementId::Name(format!("{id}:cut").into()))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::STATUS_COMPLETED_FAILED))
                .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(said.clone())),
            Role::Note,
            said,
        )
    });
    let files = drawn.files.iter().enumerate().map(|(ix, file)| {
        let path = file.path.clone();
        let accent = look.color(token::ACCENT);
        let head = div()
            .flex()
            .items_baseline()
            .gap(look.length(token::SPACE_3))
            .min_w_0()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::BG_RAISED))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                div()
                    .id(ElementId::Name(format!("{id}:path:{ix}").into()))
                    .min_w_0()
                    .truncate()
                    .font_family(font.clone())
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .cursor_pointer()
                    .hover(move |style| style.text_color(accent))
                    .on_click(move |_, _, cx: &mut App| cx.write_to_clipboard(ClipboardItem::new_string(path.clone())))
                    .child(SharedString::from(file.path.clone())),
            )
            .children(file.meta.clone().map(|meta| {
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .font_family(font.clone())
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(meta))
            }));
        let lines = file.lines.iter().map(|line| {
            let (ground, ink) = match line.kind {
                LineKind::Added => (Some(token::DIFF_ADD_BG), token::DIFF_ADD_FG),
                LineKind::Removed => (Some(token::DIFF_DEL_BG), token::DIFF_DEL_FG),
                LineKind::Hunk => (Some(token::BG_RAISED), token::FG_SUBTLE),
                LineKind::Context => (None, token::DIFF_CONTEXT),
            };
            div()
                .px(look.length(token::SPACE_3))
                .when(line.kind == LineKind::Hunk, |row| row.py(look.length(token::SPACE_1)))
                .whitespace_nowrap()
                .when_some(ground, |row, ground| row.bg(look.color(ground)))
                .text_color(look.color(ink))
                .child(SharedString::from(line.text.clone()))
        });
        labelled(
            div().id(ElementId::Name(format!("{id}:file:{ix}").into()))
                .flex()
                .flex_col()
                .min_w_0()
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .overflow_hidden()
                .child(head)
                .child(
                    div()
                        .id(ElementId::Name(format!("{id}:lines:{ix}").into()))
                        .min_w_0()
                        .overflow_x_scroll()
                        .bg(look.color(token::BG_BASE))
                        .font_family(font.clone())
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .child(div().flex().flex_col().min_w_full().children(lines)),
                ),
            Role::Group,
            file.path.clone(),
        )
    });
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_3))
        .min_w_0()
        .p(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(token::BG_SUNKEN))
        .children(cut)
        .children(files)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_forge_patch_is_named_by_the_written_header() {
        let drawn = file_diff_of("src/a.rs", Some("@@ -1,2 +1,2 @@\n-old\n+new\n same"), None);
        assert_eq!(drawn.files.len(), 1);
        assert_eq!(drawn.files[0].path, "src/a.rs");
        let kinds: Vec<_> = drawn.files[0].lines.iter().map(|line| line.kind).collect();
        assert_eq!(kinds, vec![LineKind::Hunk, LineKind::Removed, LineKind::Added, LineKind::Context]);
        assert_eq!(drawn.cut, None);
    }

    #[test]
    fn no_patch_says_fleets_sentence_or_no_change() {
        assert_eq!(file_diff_of("a", None, Some("Binary file")).empty.as_deref(), Some("Binary file"));
        assert_eq!(file_diff_of("a", None, None).empty.as_deref(), Some(NO_CHANGE));
        assert_eq!(file_diff_of("a", Some(""), None).empty.as_deref(), Some(NO_CHANGE));
    }

    #[test]
    fn a_long_patch_is_cut_and_says_how_much() {
        let body: String = (0..2500).map(|n| format!("+line {n}\n")).collect();
        let drawn = file_diff_of("a", Some(&format!("@@ -0,0 +1,2500 @@\n{body}")), None);
        assert_eq!(drawn.files[0].lines.len(), DRAWN_LINES);
        assert_eq!(drawn.cut.as_deref(), Some("The first 2,000 of 2,501 lines. Open the pull request in the browser for the rest."));
    }

    #[test]
    fn mode_lines_join_as_meta_and_dev_null_keeps_the_header_path() {
        let drawn = drawn_of("diff --git a/x b/x\nnew file mode 100644\n--- /dev/null\n+++ b/x\n@@ -0,0 +1 @@\n+hi", |_, _| String::new());
        assert_eq!(drawn.files[0].path, "x");
        assert_eq!(drawn.files[0].meta.as_deref(), Some("new file mode 100644"));
    }

    #[test]
    fn counts_take_thousands_marks() {
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(2000), "2,000");
        assert_eq!(grouped(1234567), "1,234,567");
    }
}

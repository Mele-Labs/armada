//! A unified diff, as Electron's `UnifiedDiff` draws it: one box per file under its path, each line
//! a `+`, `-`, context or hunk row in mono, no line numbers and no wrap. A Job's Changes pane draws
//! it now; the thread's diffs and Reviews take the same one later.
//!
//! **The split is `drawn-patch.ts` ported as it is**, so both apps cut a long patch at the same line.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, ClipboardItem, ElementId, KeyDownEvent, Role, SharedString, Window};

use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, tip};

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
    /// A hunk that collides with another side's change says with whom (`collides`).
    pub collides: Option<String>,
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
            open.lines.push(DiffLine { kind, text: row.to_string(), collides: None });
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

/// The press each changed line's gutter makes (`onLine`): the words on its tip, each line's own name,
/// and the press with whether Shift was held.
#[derive(Clone)]
pub struct Gutter {
    pub label: SharedString,
    pub name: Option<Rc<dyn Fn(usize) -> String>>,
    pub press: Rc<dyn Fn(usize, bool, &mut Window, &mut App)>,
}

/// What a host puts on a file's lines beside the diff: the gutter, the line picked for a note
/// (`markOf`), and what stands under a line (`under`), each by the line's place in the file.
#[derive(Default)]
pub struct Slots {
    pub gutter: Option<Gutter>,
    pub picked: Option<usize>,
    pub under: HashMap<usize, AnyElement>,
}

/// A file's lines alone, with no path above them: what a pull request's Files section and a thread's
/// excerpt draw under their own heads.
pub fn lines(id: impl Into<SharedString>, lines: &[DiffLine], look: &Look) -> AnyElement {
    lines_with(id, lines, look, Slots::default())
}

/// A file's lines with what the host puts on them (`UnifiedDiff`'s `onLine`, `markOf` and `under`). The
/// gutter is a button over the line's start that shows while the pointer is on the line or the key is
/// on it (`.armada-diff__gutter`), so Tab reaches each one and Enter or Space presses it.
pub fn lines_with(id: impl Into<SharedString>, lines: &[DiffLine], look: &Look, mut slots: Slots) -> AnyElement {
    let id: SharedString = id.into();
    let accent = look.color(token::ACCENT);
    let ring = look.color(token::ACCENT);
    let collides = look.color(token::STATUS_AWAITING_REVIEW);
    let rows: Vec<AnyElement> = lines
        .iter()
        .enumerate()
        .map(|(at, line)| {
            let (ground, ink) = match line.kind {
                LineKind::Added => (Some(token::DIFF_ADD_BG), token::DIFF_ADD_FG),
                LineKind::Removed => (Some(token::DIFF_DEL_BG), token::DIFF_DEL_FG),
                LineKind::Hunk => (Some(token::BG_RAISED), token::FG_SUBTLE),
                LineKind::Context => (None, token::DIFF_CONTEXT),
            };
            let picked = slots.picked == Some(at);
            let group: SharedString = format!("{id}:line:{at}").into();
            let gutter = slots.gutter.as_ref().filter(|_| line.kind != LineKind::Hunk).map(|gutter| {
                let name = gutter.name.as_ref().map_or_else(|| gutter.label.to_string(), |name| name(at));
                let (press, keyed) = (gutter.press.clone(), gutter.press.clone());
                labelled(
                    div()
                        .id(ElementId::Name(format!("{id}:gutter:{at}").into()))
                        .absolute()
                        .left_0()
                        .top_0()
                        .w(look.length(token::SPACE_4))
                        .h(look.length(token::LEADING_2XS))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(gpui::transparent_black())
                        .bg(accent)
                        .cursor_pointer()
                        .when(!picked, |button| button.opacity(0.).group_hover(group.clone(), |style| style.opacity(1.)))
                        .focusable()
                        .tab_stop(true)
                        .focus_visible(move |style| style.opacity(1.).border_color(ring))
                        .tooltip(tip(gutter.label.clone()))
                        .child(glyph("message-square", px(12.), look.color(token::TEXT_ON_ACCENT)))
                        .on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            press(at, event.modifiers().shift, window, cx);
                        })
                        .on_key_down(move |event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                                cx.stop_propagation();
                                keyed(at, false, window, cx);
                            }
                        }),
                    Role::Button,
                    name,
                )
            });
            let swords = line.collides.clone().map(|said| {
                labelled(
                    div()
                        .id(ElementId::Name(format!("{id}:collides:{at}").into()))
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .right(look.length(token::SPACE_2))
                        .flex()
                        .items_center()
                        .tooltip(tip(said.clone()))
                        .child(glyph("swords", px(12.), collides)),
                    Role::Image,
                    said,
                )
            });
            let text = div()
                .relative()
                .px(look.length(token::SPACE_3))
                .when(line.kind == LineKind::Hunk, |row| row.py(look.length(token::SPACE_1)))
                .whitespace_nowrap()
                .when_some(ground, |row, ground| row.bg(look.color(ground)))
                .text_color(if line.collides.is_some() { collides } else { look.color(ink) })
                .children(picked.then(|| div().absolute().inset_0().bg(accent.opacity(0.16))))
                .children(picked.then(|| div().absolute().left_0().top_0().bottom_0().w(look.length(token::SPACE_1)).bg(accent)))
                .when(line.collides.is_some(), |row| row.child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(collides)))
                .child(div().relative().child(SharedString::from(line.text.clone())))
                .children(gutter)
                .children(swords);
            let under = slots.under.remove(&at).map(|under| {
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .my(look.length(token::SPACE_2))
                    .mx(look.length(token::SPACE_3))
                    .whitespace_normal()
                    .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(under)
            });
            div().group(group).relative().flex().flex_col().child(text).children(under).into_any_element()
        })
        .collect();
    div()
        .id(ElementId::Name(id.into()))
        .min_w_0()
        .overflow_x_scroll()
        .bg(look.color(token::BG_BASE))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .child(div().flex().flex_col().min_w_full().children(rows))
        .into_any_element()
}

/// The lines a review thread is about: up to four, ending at its line on its side, under their hunk's
/// head (`excerptOf`). None where the patch does not reach the line.
pub fn excerpt(patch: &str, line: u64, left: bool) -> Option<Vec<DiffLine>> {
    let (mut old, mut new) = (0u64, 0u64);
    let mut hunk: Option<DiffLine> = None;
    let mut run: Vec<DiffLine> = Vec::new();
    for row in patch.split('\n') {
        let Some(kind) = kind_of(row) else { continue };
        if kind == LineKind::Hunk {
            let (from, to) = hunk_starts(row)?;
            (old, new) = (from, to);
            hunk = Some(DiffLine { kind, text: row.to_string(), collides: None });
            run.clear();
            continue;
        }
        let at = match kind {
            LineKind::Added => {
                new += 1;
                (!left).then_some(new)
            }
            LineKind::Removed => {
                old += 1;
                left.then_some(old)
            }
            _ => {
                old += 1;
                new += 1;
                Some(if left { old } else { new })
            }
        };
        run.push(DiffLine { kind, text: row.to_string(), collides: None });
        if at == Some(line) {
            let from = run.len().saturating_sub(4);
            return Some(hunk.into_iter().chain(run.drain(from..)).collect());
        }
    }
    None
}

/// Each line's number as the forge names it, and whether it is on the removed side, by its place in the
/// file (`counted` and `placeOfLine`, `pr-review/patch.ts`): a line with a number in the new file is on
/// the right, a removed line on the left. A hunk's head and git's `\` lines have none.
pub fn places(lines: &[DiffLine]) -> Vec<Option<(u64, bool)>> {
    let (mut old, mut new) = (0u64, 0u64);
    lines
        .iter()
        .map(|line| match line.kind {
            LineKind::Hunk => {
                if let Some((from, to)) = hunk_starts(&line.text) {
                    (old, new) = (from, to);
                }
                None
            }
            LineKind::Context if line.text.starts_with('\\') => None,
            LineKind::Removed => {
                old += 1;
                Some((old, true))
            }
            LineKind::Added => {
                new += 1;
                Some((new, false))
            }
            LineKind::Context => {
                old += 1;
                new += 1;
                Some((new, false))
            }
        })
        .collect()
}

/// The lines of the file as it was that a hunk header changes, as `[from, to)`; a count of nought is a
/// point (`rangeOf`).
fn range_of(header: &str) -> Option<(u64, u64)> {
    let old = header.strip_prefix("@@ -")?.split(' ').next()?;
    let mut parts = old.split(',');
    let from = parts.next()?.parse::<u64>().ok()?;
    let count = match parts.next() {
        None => 1,
        Some(count) => count.parse::<u64>().ok()?,
    };
    Some((from, from + count.max(1)))
}

/// Each hunk of `file` that changes lines a hunk of `other` changes says it collides, in `said`
/// (`markCollisions`). Both patches run from where the two sides split, so their old-side ranges count
/// the same file's lines.
pub fn mark_collisions(file: &mut Drawn, other: &Drawn, said: &str) {
    let theirs: Vec<(u64, u64)> = other.files.iter().flat_map(|one| one.lines.iter()).filter(|line| line.kind == LineKind::Hunk).filter_map(|line| range_of(&line.text)).collect();
    for line in file.files.iter_mut().flat_map(|one| one.lines.iter_mut()).filter(|line| line.kind == LineKind::Hunk) {
        if range_of(&line.text).is_some_and(|(a0, a1)| theirs.iter().any(|&(b0, b1)| a0 < b1 && b0 < a1)) {
            line.collides = Some(said.to_string());
        }
    }
}

/// `@@ -a,b +c,d @@` → the line before each side's first, so the first row counts as `a` and `c`.
fn hunk_starts(row: &str) -> Option<(u64, u64)> {
    let mut parts = row.split_whitespace().skip(1);
    let start = |part: &str| part.get(1..)?.split(',').next()?.parse::<u64>().ok();
    let old = start(parts.next()?)?;
    let new = start(parts.next()?)?;
    Some((old.saturating_sub(1), new.saturating_sub(1)))
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
                .child(lines(format!("{id}:lines:{ix}"), &file.lines, look)),
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
    fn an_excerpt_ends_at_the_threads_line_on_its_side() {
        let patch = "@@ -10,6 +10,7 @@\n a\n b\n-c\n+C\n+D\n d\n e\n f";
        let new: Vec<_> = excerpt(patch, 13, false).unwrap().into_iter().map(|line| line.text).collect();
        assert_eq!(new, vec!["@@ -10,6 +10,7 @@", " b", "-c", "+C", "+D"]);
        let old: Vec<_> = excerpt(patch, 12, true).unwrap().into_iter().map(|line| line.text).collect();
        assert_eq!(old, vec!["@@ -10,6 +10,7 @@", " a", " b", "-c"]);
        assert!(excerpt(patch, 40, false).is_none());
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

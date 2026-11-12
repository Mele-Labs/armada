//! The grid a shell's bytes are read into, as xterm.js holds it in Electron's `XtermScreen`: the
//! escape parser, the screen and its 10,000 lines of scrollback, the selection, and the replies a
//! program's queries ask for. Fleet holds the shell; this only reads what it printed.

use std::cell::RefCell;
use std::rc::Rc;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Processor, Rgb};

/// Lines kept above the screen (`scrollback: 10_000`, `XtermScreen.tsx:120`).
pub const SCROLLBACK: usize = 10_000;

/// A screen's size in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cells {
    pub cols: u16,
    pub rows: u16,
}

impl Cells {
    /// Fleet's size for a shell opened before its pane is measured (`terminals.rs:23`).
    pub const OPENED: Cells = Cells { cols: 80, rows: 24 };
}

impl Dimensions for Cells {
    fn total_lines(&self) -> usize {
        self.rows as usize
    }

    fn screen_lines(&self) -> usize {
        self.rows as usize
    }

    fn columns(&self) -> usize {
        self.cols as usize
    }
}

/// What the parser asked of the screen's owner while it read: replies for the shell, and a title.
#[derive(Clone, Default)]
pub struct Heard(Rc<RefCell<Vec<Event>>>);

impl EventListener for Heard {
    fn send_event(&self, event: Event) {
        self.0.borrow_mut().push(event);
    }
}

/// How a selection grows from where the press landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grain {
    /// One press: cell by cell.
    Cells,
    /// Two presses: word by word.
    Words,
    /// Three presses: line by line.
    Lines,
}

pub struct Screen {
    term: Term<Heard>,
    parser: Processor,
    heard: Heard,
    size: Cells,
    /// The cell's size in points, which a size report answers with.
    cell: (u16, u16),
    title: Option<String>,
}

impl Screen {
    pub fn new(size: Cells) -> Screen {
        let heard = Heard::default();
        let config = Config { scrolling_history: SCROLLBACK, ..Config::default() };
        let term = Term::new(config, &size, heard.clone());
        Screen { term, parser: Processor::new(), heard, size, cell: (8, 16), title: None }
    }

    /// Read what the shell printed. Answers the bytes the shell asked for while it was read, such as
    /// the cursor's place: none while `replaying`, since history was answered when it was first printed
    /// and a reply now would be typed into the shell (`XtermScreen.tsx:91`).
    pub fn feed(&mut self, bytes: &[u8], replaying: bool) -> Vec<u8> {
        self.parser.advance(&mut self.term, bytes);
        let heard: Vec<Event> = std::mem::take(&mut *self.heard.0.borrow_mut());
        let mut replies = Vec::new();
        for event in heard {
            match event {
                Event::PtyWrite(text) if !replaying => replies.extend_from_slice(text.as_bytes()),
                Event::ColorRequest(index, answer) if !replaying => {
                    let rgb = self.term.colors()[index].unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                    replies.extend_from_slice(answer(rgb).as_bytes());
                }
                Event::TextAreaSizeRequest(answer) if !replaying => {
                    let size = WindowSize { num_lines: self.size.rows, num_cols: self.size.cols, cell_width: self.cell.0, cell_height: self.cell.1 };
                    replies.extend_from_slice(answer(size).as_bytes());
                }
                Event::Title(title) => self.title = Some(title),
                Event::ResetTitle => self.title = None,
                _ => {}
            }
        }
        replies
    }

    pub fn size(&self) -> Cells {
        self.size
    }

    /// Fit the grid to `size`; the cell's points are what a size report answers. Whether the size
    /// moved, so Fleet is told only then (`XtermScreen.tsx:143-161`).
    pub fn resize(&mut self, size: Cells, cell: (u16, u16)) -> bool {
        self.cell = cell;
        if size == self.size || size.cols == 0 || size.rows == 0 {
            return false;
        }
        self.size = size;
        self.term.resize(size);
        true
    }

    pub fn mode(&self) -> TermMode {
        *self.term.mode()
    }

    pub fn term(&self) -> &Term<Heard> {
        &self.term
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// How many lines the screen is scrolled up from the end.
    pub fn scrolled(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// Scroll the history by `lines`, up where positive.
    pub fn scroll(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(lines));
    }

    pub fn scroll_page(&mut self, up: bool) {
        self.term.scroll_display(if up { Scroll::PageUp } else { Scroll::PageDown });
    }

    /// Back to the end, as typing brings it.
    pub fn scroll_to_end(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    /// The cell at `row` and `col` of the screen as drawn, scrolled or not.
    pub fn point(&self, row: usize, col: usize) -> Point {
        let line = row.min(self.size.rows.saturating_sub(1) as usize) as i32 - self.scrolled() as i32;
        Point::new(Line(line), Column(col.min(self.size.cols.saturating_sub(1) as usize)))
    }

    pub fn select_from(&mut self, at: Point, side: Side, grain: Grain) {
        let ty = match grain {
            Grain::Cells => SelectionType::Simple,
            Grain::Words => SelectionType::Semantic,
            Grain::Lines => SelectionType::Lines,
        };
        self.term.selection = Some(Selection::new(ty, at, side));
    }

    pub fn select_to(&mut self, at: Point, side: Side) {
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(at, side);
        }
    }

    pub fn unselect(&mut self) {
        self.term.selection = None;
    }

    pub fn select_all(&mut self) {
        let top = self.term.topmost_line();
        let bottom = self.term.bottommost_line();
        let mut selection = Selection::new(SelectionType::Lines, Point::new(top, Column(0)), Side::Left);
        selection.update(Point::new(bottom, self.term.last_column()), Side::Right);
        self.term.selection = Some(selection);
    }

    /// The words selected, or None where nothing is.
    pub fn selected(&self) -> Option<String> {
        let selection = self.term.selection.as_ref()?;
        if selection.is_empty() {
            return None;
        }
        self.term.selection_to_string().filter(|text| !text.is_empty())
    }

    /// The screen's lines as text, trailing blanks cut: what a test reads.
    #[cfg(test)]
    pub fn lines(&self) -> Vec<String> {
        let grid = self.term.grid();
        (0..self.size.rows as i32)
            .map(|line| {
                let row = &grid[Line(line)];
                let text: String = (0..self.size.cols as usize).map(|col| row[Column(col)].c).collect();
                text.trim_end().to_string()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Screen {
        Screen::new(Cells { cols: 20, rows: 4 })
    }

    #[test]
    fn printed_text_lands_on_the_grid_and_a_colour_escape_is_not_text() {
        let mut screen = screen();
        screen.feed(b"$ ls\r\n\x1b[32mok\x1b[0m done", false);
        assert_eq!(screen.lines(), vec!["$ ls", "ok done", "", ""]);
    }

    #[test]
    fn a_cursor_query_is_answered_live_and_not_on_replay() {
        let mut screen = screen();
        screen.feed(b"ab", false);
        assert_eq!(screen.feed(b"\x1b[6n", false), b"\x1b[1;3R".to_vec());
        assert!(screen.feed(b"\x1b[6n", true).is_empty());
    }

    #[test]
    fn a_resize_is_told_once_and_reflows_the_grid() {
        let mut screen = screen();
        assert!(!screen.resize(Cells { cols: 20, rows: 4 }, (8, 16)));
        assert!(screen.resize(Cells { cols: 10, rows: 6 }, (8, 16)));
        assert_eq!(screen.size(), Cells { cols: 10, rows: 6 });
        assert!(!screen.resize(Cells { cols: 0, rows: 6 }, (8, 16)));
    }

    #[test]
    fn scrollback_keeps_what_left_the_screen() {
        let mut screen = screen();
        for line in 0..10 {
            screen.feed(format!("line {line}\r\n").as_bytes(), false);
        }
        assert_eq!(screen.lines()[0], "line 7");
        screen.scroll(3);
        assert_eq!(screen.scrolled(), 3);
        assert_eq!(screen.lines()[0], "line 7", "the grid's own lines do not move; the drawn rows do");
        assert_eq!(screen.point(0, 0).line, Line(-3));
        screen.scroll_to_end();
        assert_eq!(screen.scrolled(), 0);
    }

    #[test]
    fn a_selection_reads_back_as_its_words() {
        let mut screen = screen();
        screen.feed(b"hello world", false);
        screen.select_from(screen.point(0, 6), Side::Left, Grain::Cells);
        screen.select_to(screen.point(0, 10), Side::Right);
        assert_eq!(screen.selected().as_deref(), Some("world"));
        screen.select_from(screen.point(0, 1), Side::Left, Grain::Words);
        assert_eq!(screen.selected().as_deref(), Some("hello"));
        screen.unselect();
        assert_eq!(screen.selected(), None);
    }

    #[test]
    fn a_title_escape_names_the_screen() {
        let mut screen = screen();
        screen.feed(b"\x1b]0;vim notes.md\x07", false);
        assert_eq!(screen.title(), Some("vim notes.md"));
    }
}

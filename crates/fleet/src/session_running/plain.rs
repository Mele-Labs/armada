//! A terminal's output as the plain lines a person would have read on its screen.

use std::iter::Peekable;
use std::str::Chars;

/// The last `count` lines of `output` as the screen drew them. Escape sequences are dropped, a
/// carriage return or a backspace writes over what was there, and erase-in-line is honoured, so a
/// progress bar that redrew itself fifty times reads as its last line.
pub(crate) fn last_lines(output: &str, count: usize) -> String {
    let mut done: Vec<String> = Vec::new();
    let mut line: Vec<char> = Vec::new();
    let mut col: usize = 0;
    let mut chars = output.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => {
                done.push(finished(&mut line));
                col = 0;
            }
            '\r' => col = 0,
            '\u{8}' => col = col.saturating_sub(1),
            '\t' => {
                for _ in 0..8 - col % 8 {
                    write(&mut line, &mut col, ' ');
                }
            }
            '\u{1b}' => escape(&mut chars, &mut line, &mut col),
            c if c.is_control() => {}
            c => write(&mut line, &mut col, c),
        }
    }
    done.push(finished(&mut line));
    while done.last().is_some_and(String::is_empty) {
        done.pop();
    }
    let from = done.len().saturating_sub(count);
    done[from..].join("\n")
}

/// `output`'s printable characters in the order they arrived, with escapes, controls and spaces
/// taken out and nothing written over. A line a shell wrapped by returning to its start reads whole
/// here, where [`last_lines`] would read it overwritten.
pub(crate) fn printed_through(output: &str) -> String {
    let (mut scratch, mut col) = (Vec::new(), 0);
    let mut kept = String::new();
    let mut chars = output.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => escape(&mut chars, &mut scratch, &mut col),
            c if c.is_control() || c.is_whitespace() => {}
            c => kept.push(c),
        }
    }
    kept
}

fn finished(line: &mut Vec<char>) -> String {
    let text: String = line.iter().collect();
    line.clear();
    text.trim_end().to_string()
}

fn write(line: &mut Vec<char>, col: &mut usize, c: char) {
    if *col < line.len() {
        line[*col] = c;
    } else {
        line.resize(*col, ' ');
        line.push(c);
    }
    *col += 1;
}

/// Consume the sequence after an ESC. Only the ones that move the cursor along the line or erase
/// part of it change what is read; the rest are dropped whole.
fn escape(chars: &mut Peekable<Chars<'_>>, line: &mut Vec<char>, col: &mut usize) {
    match chars.next() {
        Some('[') => {
            let mut params = String::new();
            let last = loop {
                match chars.next() {
                    Some(c) if ('\u{40}'..='\u{7e}').contains(&c) => break Some(c),
                    Some(c) => params.push(c),
                    None => break None,
                }
            };
            let first = params.split(';').next().and_then(|n| n.parse::<usize>().ok());
            match last {
                Some('K') => match first.unwrap_or(0) {
                    0 => line.truncate(*col),
                    1 => line.iter_mut().take(*col).for_each(|c| *c = ' '),
                    _ => line.clear(),
                },
                Some('G') => *col = first.unwrap_or(1).max(1) - 1,
                Some('D') => *col = col.saturating_sub(first.unwrap_or(1).max(1)),
                Some('C') => *col += first.unwrap_or(1).max(1),
                _ => {}
            }
        }
        // A string sequence: ends at BEL, or at ESC and a backslash.
        Some(']' | 'P' | 'X' | '^' | '_') => {
            while let Some(c) = chars.next() {
                if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                    break;
                }
            }
        }
        // Intermediate bytes, such as the `(` of choosing a character set, then a final byte.
        Some('\u{20}'..='\u{2f}') => {
            chars.next();
        }
        _ => {}
    }
}

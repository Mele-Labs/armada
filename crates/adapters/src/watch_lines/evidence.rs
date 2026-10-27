//! What a failed part's own log says: its exact error lines and its end, for the person to send on.
//!
//! **Lines as printed, cleaned only of what a terminal draws**: colour escapes, a provider's
//! timestamp marks, and a line redrawn over itself, which keeps what it last read. Nothing is
//! reworded, since the point is the exact error.

/// The most error lines a part carries, the last ones found.
pub(crate) const ERROR_LINES: usize = 20;
/// How many lines from the end of a log a part carries.
pub(crate) const TAIL_LINES: usize = 50;
/// The most characters a line keeps, so one minified line cannot fill a reading.
const LINE_AT_MOST: usize = 500;

/// Words that mark a line as an error.
const MARKS: &[&str] = &["error", "failed", "failure", "fatal", "exception", "panicked", "traceback", "denied", "not found"];

/// `log` without colour escapes and timestamp marks, each line as it was last drawn.
pub(crate) fn cleaned(log: &str) -> Vec<String> {
    let mut plain = String::with_capacity(log.len());
    let mut chars = log.chars().peekable();
    while let Some(one) = chars.next() {
        if one != '\u{1b}' {
            plain.push(one);
            continue;
        }
        match chars.peek() {
            // A control sequence: up to its final letter.
            Some('[') => {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // An application command, such as a timestamp mark: up to its bell or string end.
            Some('_' | ']') => {
                chars.next();
                while let Some(next) = chars.next() {
                    if next == '\u{7}' {
                        break;
                    }
                    if next == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    plain
        .lines()
        .map(|line| line.rsplit('\r').find(|drawn| !drawn.trim().is_empty()).unwrap_or("").trim_end())
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take(LINE_AT_MOST).collect())
        .collect()
}

/// Whether `line` reads as an error.
fn marks_an_error(line: &str) -> bool {
    let lower = line.to_lowercase();
    MARKS.iter().any(|mark| lower.contains(mark))
}

/// `log`'s exact error lines, the last [`ERROR_LINES`] found, and its last [`TAIL_LINES`] lines.
/// Either is `None` where the log holds nothing of it.
pub(crate) fn of_log(log: &str) -> (Option<String>, Option<String>) {
    let lines = cleaned(log);
    let errors: Vec<&String> = lines.iter().filter(|line| marks_an_error(line)).collect();
    let errors = &errors[errors.len().saturating_sub(ERROR_LINES)..];
    let tail = &lines[lines.len().saturating_sub(TAIL_LINES)..];
    let joined = |lines: &[&String]| (!lines.is_empty()).then(|| lines.iter().map(|line| line.as_str()).collect::<Vec<_>>().join("\n"));
    (joined(errors), joined(&tail.iter().collect::<Vec<_>>()))
}

/// The last [`TAIL_LINES`] lines of `text`, cleaned. `None` where it holds none.
pub(crate) fn tail_of(text: &str) -> Option<String> {
    let lines = cleaned(text);
    let tail = &lines[lines.len().saturating_sub(TAIL_LINES)..];
    (!tail.is_empty()).then(|| tail.join("\n"))
}

/// The first [`ERROR_LINES`] lines of `text`, cleaned. `None` where it holds none.
pub(crate) fn head_of(text: &str) -> Option<String> {
    let lines = cleaned(text);
    let head = &lines[..lines.len().min(ERROR_LINES)];
    (!head.is_empty()).then(|| head.join("\n"))
}

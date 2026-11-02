//! Whether a shell line is one `armada job` command and nothing else.
//!
//! **`armada job` never asks permission**: the door it speaks to already refuses what a Job may not
//! reach, so the command confers nothing the Job did not have, and a prompt on every call would be
//! an interruption with no decision in it. That holds only for the command alone. A line that does
//! anything besides, even a harmless-looking second word after a `;`, is the shell's and goes to the
//! person like any other.
//!
//! The check is a reading of the line, not a match on its start. Quotes are followed, because a
//! JSON body is full of `;` and `|` that are text. What shell would act on is refused: an unquoted
//! `; | & < > $` backtick, backslash, parenthesis or newline, and a `$`, backtick or backslash
//! inside double quotes. One heredoc with a single-quoted delimiter is the exception, since its
//! body is literal and it is how a long body reaches stdin: `armada job <act> - <<'TAG'`.

/// Whether `line` is exactly one `armada job` command, with at most one literal heredoc on stdin.
pub(crate) fn is_one_armada_job(line: &str) -> bool {
    let Some((words, heredoc)) = read(line) else {
        return false;
    };
    let named = matches!(words.as_slice(), [program, verb, ..] if program == "armada" && verb == "job");
    // The body is the act's, so the act is told to read stdin.
    named && (!heredoc || words.last().is_some_and(|last| last == "-"))
}

/// The words of the command and its heredoc, or `None` where the line holds anything else.
fn read(line: &str) -> Option<(Vec<String>, bool)> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = line.chars();
    let mut heredoc = false;
    while let Some(next) = chars.next() {
        match next {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        literal => word.push(literal),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '$' | '`' | '\\' => return None,
                        literal => word.push(literal),
                    }
                }
            }
            ' ' | '\t' => {
                if std::mem::take(&mut in_word) {
                    words.push(std::mem::take(&mut word));
                }
            }
            '<' => {
                // Only `<<'TAG'`, then the end of the line, then the body.
                let rest = chars.as_str();
                let (tag, after) = rest.strip_prefix("<'")?.split_once('\'')?;
                if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return None;
                }
                let after = after.trim_start_matches([' ', '\t']);
                let body = after.strip_prefix('\n')?;
                if !terminated(body, tag) {
                    return None;
                }
                if std::mem::take(&mut in_word) {
                    words.push(std::mem::take(&mut word));
                }
                heredoc = true;
                break;
            }
            ';' | '|' | '&' | '>' | '$' | '`' | '\\' | '\n' | '\r' | '(' | ')' => return None,
            other => {
                in_word = true;
                word.push(other);
            }
        }
    }
    if in_word {
        words.push(word);
    }
    Some((words, heredoc))
}

/// Whether the body ends at a line that is the delimiter alone, with nothing but blank space after.
fn terminated(body: &str, tag: &str) -> bool {
    let mut lines = body.split('\n');
    lines.any(|line| line == tag) && lines.all(|line| line.trim().is_empty())
}

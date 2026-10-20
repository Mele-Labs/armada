//! The commands a shell said it does not have: `zsh: command not found: timeout`,
//! `bash: gtimeout: command not found`, `sh: jq: command not found`.

use super::Record;

/// Each command named as not found in what a Job's shell printed, with the time of the call that
/// printed it. The newest time is kept for a name seen twice.
pub fn missing_commands(records: &[Record]) -> Vec<(String, u64)> {
    let mut times = std::collections::HashMap::new();
    for record in records {
        if let Record::Call { id, at_ms, .. } = record {
            times.insert(id.as_str(), *at_ms);
        }
    }
    let mut found: Vec<(String, u64)> = Vec::new();
    for record in records {
        let Record::Done { id, said, .. } = record else {
            continue;
        };
        let at = times.get(id.as_str()).copied().unwrap_or(0);
        for name in named_in(said) {
            match found.iter_mut().find(|(held, _)| *held == name) {
                Some((_, seen)) => *seen = (*seen).max(at),
                None => found.push((name, at)),
            }
        }
    }
    found
}

/// The commands a printed result names as not found, in order.
pub(super) fn named_in(said: &str) -> Vec<String> {
    said.lines().filter_map(|line| named(line.trim())).collect()
}

fn named(line: &str) -> Option<String> {
    let name = if let Some(rest) = line.strip_prefix("zsh: command not found: ") {
        rest.trim()
    } else {
        let rest = line.strip_prefix("bash: ").or_else(|| line.strip_prefix("sh: "))?;
        // A script's shell adds where it was: `bash: line 3: jq: command not found`.
        let rest = match rest.strip_prefix("line ") {
            Some(after) => after.split_once(": ")?.1,
            None => rest,
        };
        rest.strip_suffix(": command not found")?
    };
    let plain = !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'));
    plain.then(|| name.to_string())
}

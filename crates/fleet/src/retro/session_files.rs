//! What the agent CLI kept of a Session on this machine: its transcript and each subagent's.

use std::path::Path;

use ipc::SessionRow;

/// The agent CLI's transcript of a Session and of each of its subagents, as
/// lines, and the thread drawn from the first.
#[derive(Default)]
pub(super) struct Files {
    pub(super) main: Vec<String>,
    pub(super) subagents: Vec<(String, Vec<String>)>,
    pub(super) drawn: Vec<SessionRow>,
}

fn lines_of(file: &Path) -> Vec<String> {
    std::fs::read_to_string(file)
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

pub(super) fn transcripts_of(home: &str, id: &str) -> Files {
    let Some(file) = adapters::terminal_thread::find(home, id) else {
        return Files::default();
    };
    let drawn = adapters::terminal_thread::read_from(&file, 0)
        .map(|thread| thread.rows)
        .unwrap_or_default();
    let mut subagents = Vec::new();
    if let Ok(entries) = std::fs::read_dir(file.with_extension("").join("subagents")) {
        let mut named: Vec<_> = entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let stem = path
                    .file_stem()?
                    .to_str()?
                    .strip_prefix("agent-")?
                    .to_string();
                (path.extension()? == "jsonl").then_some((stem, path))
            })
            .collect();
        named.sort();
        subagents = named
            .into_iter()
            .map(|(agent, path)| (agent, lines_of(&path)))
            .collect();
    }
    Files {
        main: lines_of(&file),
        subagents,
        drawn,
    }
}

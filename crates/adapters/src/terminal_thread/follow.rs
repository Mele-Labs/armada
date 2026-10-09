//! A transcript kept drawn in memory, so reading it again costs a `stat` and what was appended.
//!
//! **The rows are what `read_from(file, 0)` would draw**, because a line draws the same rows
//! wherever the read began: `drawn` looks at one line and nothing before it, and a subagent's
//! `done` is carried here from one catch-up to the next.

use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use ipc::SessionRow;

use super::{drawn, fold_subagent, whole_lines};

pub struct Followed {
    file: PathBuf,
    /// Device and inode, so a file replaced at the same path is read again from the start.
    identity: (u64, u64),
    /// The end of the last whole line drawn.
    next: u64,
    rows: Vec<SessionRow>,
    inside_agent: bool,
    done: Option<Option<String>>,
}

impl Followed {
    /// A session's thread, drawn as far as the file reaches.
    pub fn thread(file: &Path) -> std::io::Result<Self> {
        Self::start(file, false)
    }

    /// A subagent's own thread, drawn as `read_subagent` draws it.
    pub fn subagent(file: &Path) -> std::io::Result<Self> {
        Self::start(file, true)
    }

    fn start(file: &Path, inside_agent: bool) -> std::io::Result<Self> {
        let mut one = Followed {
            file: file.to_path_buf(),
            identity: (0, 0),
            next: 0,
            rows: Vec::new(),
            inside_agent,
            done: None,
        };
        one.catch_up()?;
        Ok(one)
    }

    pub fn follows(&self, file: &Path) -> bool {
        self.file == file
    }

    pub fn rows(&self) -> &[SessionRow] {
        &self.rows
    }

    pub fn next(&self) -> u64 {
        self.next
    }

    /// Whether a subagent's last turn ended, and what it said last.
    pub fn finished(&self) -> (bool, Option<String>) {
        match &self.done {
            Some(report) => (true, report.clone()),
            None => (false, None),
        }
    }

    /// Draw what the file gained since the last call. **A file that is shorter than what was
    /// drawn, or is another file, is drawn again from the start.**
    pub fn catch_up(&mut self) -> std::io::Result<()> {
        let meta = std::fs::metadata(&self.file)?;
        let identity = (meta.dev(), meta.ino());
        if identity != self.identity || meta.len() < self.next {
            self.restart(identity);
        } else if meta.len() == self.next {
            return Ok(());
        }
        let (text, from, next) = whole_lines(&self.file, self.next)?;
        if from != self.next {
            // Cut short between the `stat` and the read: the read began again at 0.
            self.restart(identity);
        }
        if self.inside_agent {
            fold_subagent(&text, &mut self.rows, &mut self.done);
        } else {
            self.rows.extend(text.lines().flat_map(|line| drawn(line, false)));
        }
        self.next = next;
        Ok(())
    }

    fn restart(&mut self, identity: (u64, u64)) {
        self.identity = identity;
        self.next = 0;
        self.rows.clear();
        self.done = None;
    }
}

//! Discussion reading before it answers: a message to the Operator is put to a call that holds the three
//! read tools, Read, Grep and Glob, and nothing else. A ruling is not: it stays on [`Calling`], a fresh call
//! with none.
//!
//! **What it may read is a folder of plain text Fleet writes for that one message**, a file for each live
//! Job and one of the open issues, and the checkouts of the repositories Fleet hosts. The folder is gone
//! when the message is answered, however it ended. No Fleet tool is given: whatever the call may know, it
//! was written there by Fleet, so the call can neither change a Job nor ask for more than Fleet chose to show.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Model;

use super::calling::{Answering, Calling};
use crate::judging::CallFailed;
use crate::scout::ScoutHost;

/// The most turns one message takes, its reads and its answer together.
pub const MOST_TURNS: u32 = 16;
/// How long one message may take before it is a failed call.
pub const MOST_TIME: Duration = Duration::from_secs(150);

/// What Fleet puts in front of a message to read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToRead {
    /// Where the folder is made: Fleet's own runtime home, never a repository.
    pub under: PathBuf,
    /// A file name and its text, for each file of the folder.
    pub files: Vec<(String, String)>,
    /// The id, the name and the checkout's root of each repository Fleet hosts.
    pub repositories: Vec<(String, String, String)>,
}

/// One question put to a call that may read, and what it said last.
pub trait Looking: Send + Sync {
    /// `question`, put to a call that may read `folder` and the code under each of `roots`.
    fn look<'a>(&'a self, question: String, folder: &'a Path, roots: &'a [String]) -> Answering<'a>;
}

/// A [`Looking`] that reads nothing: it puts the question to the call that holds no tools. For an
/// Operator made with only that call.
pub struct Blind(pub Arc<dyn Calling>);

impl Looking for Blind {
    fn look<'a>(&'a self, question: String, _folder: &'a Path, _roots: &'a [String]) -> Answering<'a> {
        self.0.ask(question)
    }
}

/// The call through a scout's process: the Operator model as the setting reads now, in the folder, with the
/// repositories beside it.
pub struct ReadingScout {
    host: Arc<ScoutHost>,
    model: Box<dyn Fn() -> Option<Model> + Send + Sync>,
}

impl ReadingScout {
    pub fn new(host: Arc<ScoutHost>, model: impl Fn() -> Option<Model> + Send + Sync + 'static) -> ReadingScout {
        ReadingScout { host, model: Box::new(model) }
    }
}

impl Looking for ReadingScout {
    fn look<'a>(&'a self, question: String, folder: &'a Path, roots: &'a [String]) -> Answering<'a> {
        Box::pin(async move {
            let model = (self.model)().ok_or(CallFailed::NothingToAsk)?;
            let directory = folder.to_string_lossy();
            self.host.answer_reading(model.as_str(), &directory, roots, MOST_TURNS, &question, MOST_TIME).await
        })
    }
}

/// The folder one message reads, removed when this is dropped.
pub struct Folder {
    path: PathBuf,
}

/// Folders made by this Fleet, so two messages at once do not share one.
static MADE: AtomicU64 = AtomicU64::new(0);

impl Folder {
    /// Make a folder under `under` holding `files`.
    pub fn made(under: &Path, files: &[(String, String)]) -> io::Result<Folder> {
        let path = under.join(format!("talk-{}-{}", std::process::id(), MADE.fetch_add(1, Ordering::SeqCst)));
        fs::create_dir_all(&path)?;
        // Held from here, so a file that will not write still removes the folder.
        let folder = Folder { path };
        for (name, text) in files {
            fs::write(folder.path.join(name), text)?;
        }
        Ok(folder)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

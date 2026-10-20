//! The review tools a repository has, and writing one. `docs/concepts/pull-review.md`,
//! *Review tools*.
//!
//! **Read when asked, never cached**: a definition is a small file a person edits, and a run
//! reads it again when it starts, so what the owner saved is what the next press asks. The merge
//! and what a file must hold are `config::review_tools`'; this is the two folders and the write.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::review_tools::{self as tools, Def, ToolKind, ToolSource, Written};
use ipc::{
    ReviewTool, ReviewToolKind, ReviewToolLeftOut, ReviewToolScope, ReviewToolSource, ReviewTools,
    SaveReviewTool,
};

use crate::daemon::Fleet;

/// The folder under Kit's home, and under a repository's `.armada/`.
pub(crate) const FOLDER: &str = "review-tools";
/// A blank name or prompt. A 422.
const BLANK: &str = "fleet.review_tool_blank";
/// A name that has no words in it, so no id. A 422.
const NOT_A_NAME: &str = "fleet.review_tool_not_a_name";
/// The id is already defined in that scope and `overwrite` was not asked for. A 422.
const EXISTS: &str = "fleet.review_tool_exists";
/// The file could not be written. A 500.
const UNWRITABLE: &str = "fleet.review_tool_unwritable";

fn source_of(source: ToolSource) -> ReviewToolSource {
    match source {
        ToolSource::Armada => ReviewToolSource::Armada,
        ToolSource::Kit => ReviewToolSource::Kit,
        ToolSource::Repository => ReviewToolSource::Repository,
    }
}

fn kind_of(kind: ReviewToolKind) -> ToolKind {
    match kind {
        ReviewToolKind::Tool => ToolKind::Tool,
        ReviewToolKind::Explanation => ToolKind::Explanation,
    }
}

fn tool_of(def: &Def) -> ReviewTool {
    ReviewTool {
        id: def.id.clone(),
        kind: match def.kind {
            ToolKind::Tool => ReviewToolKind::Tool,
            ToolKind::Explanation => ReviewToolKind::Explanation,
        },
        name: def.name.clone(),
        prompt: def.prompt.clone(),
        source: source_of(def.source),
        replaces: def.replaces.map(source_of),
        hides: def.hides.map(source_of),
    }
}

/// Every `*.md` in `folder`, by name. A folder that is not there holds none; a file that will not
/// read is skipped here, since a person cannot be told what is not readable except as a file
/// that did not fit, and that sentence needs text.
fn files_in(folder: &Path) -> Vec<(PathBuf, String)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md") && path.is_file())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| std::fs::read_to_string(&path).ok().map(|text| (path, text)))
        .collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    pub(crate) fn review_tool_folder(&self, scope: ReviewToolScope, root: &str) -> PathBuf {
        match scope {
            ReviewToolScope::Kit => PathBuf::from(&self.host().kit_home).join(FOLDER),
            ReviewToolScope::Repository => Path::new(root).join(".armada").join(FOLDER),
        }
    }

    /// The three places merged: Armada's, then Kit's, then the repository's.
    pub(crate) fn review_tools_resolved(&self, root: &str) -> tools::Resolved {
        let mut all = tools::carried();
        for (path, text) in files_in(&self.review_tool_folder(ReviewToolScope::Kit, root)) {
            all.push(Written::in_kit(path, text));
        }
        for (path, text) in files_in(&self.review_tool_folder(ReviewToolScope::Repository, root)) {
            all.push(Written::in_repository(path, text));
        }
        tools::resolve(all)
    }

    fn review_tools_listed(&self, root: &str) -> ReviewTools {
        let resolved = self.review_tools_resolved(root);
        ReviewTools {
            tools: resolved.defs.iter().map(tool_of).collect(),
            left_out: resolved
                .left_out
                .into_iter()
                .map(|one| ReviewToolLeftOut {
                    file: one.path.display().to_string(),
                    source: source_of(one.source),
                    said: one.said,
                })
                .collect(),
        }
    }

    pub(crate) async fn review_tools_read(&self, manifest_id: ipc::ManifestId) -> Result<ReviewTools, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        Ok(self.review_tools_listed(served.root()))
    }

    /// Write one definition in the scope named, then answer what the three places come to.
    pub(crate) async fn review_tool_saved(
        &self,
        manifest_id: ipc::ManifestId,
        save: SaveReviewTool,
    ) -> Result<ReviewTools, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let blank = |said: &str| Refusal::Unacceptable(self.pr_refusal(BLANK, said.to_string()));
        if save.name.trim().is_empty() {
            return Err(blank("a review tool needs a name"));
        }
        if save.prompt.trim().is_empty() {
            return Err(blank("a review tool needs a prompt"));
        }
        let kind = kind_of(save.kind);
        let id = tools::fit(kind, &save.name, &save.prompt)
            .map_err(|said| Refusal::Unacceptable(self.pr_refusal(NOT_A_NAME, said)))?;
        let folder = self.review_tool_folder(save.scope, served.root());
        // The file that holds this id in this scope is the one replaced, whatever it is called.
        let held = self.review_tool_held(&folder, save.scope, &id);
        if let (Some(path), false) = (&held, save.overwrite) {
            return Err(Refusal::Unacceptable(self.pr_refusal(
                EXISTS,
                format!("{} already defines `{id}`", path.display()),
            )));
        }
        let path = held.unwrap_or_else(|| folder.join(format!("{id}.md")));
        let unwritable = |why: std::io::Error| {
            Refusal::Fault(self.pr_refusal(UNWRITABLE, format!("{} could not be written: {why}", path.display())))
        };
        std::fs::create_dir_all(&folder).map_err(unwritable)?;
        let staged = path.with_extension("md.writing");
        std::fs::write(&staged, tools::file_of(kind, &save.name, &save.prompt))
            .and_then(|()| std::fs::rename(&staged, &path))
            .map_err(unwritable)?;
        Ok(self.review_tools_listed(served.root()))
    }

    /// The file in `folder` that defines `id`, if one does.
    pub(crate) fn review_tool_held(&self, folder: &Path, scope: ReviewToolScope, id: &str) -> Option<PathBuf> {
        // A file of the name this would take that does not fit is still in the way.
        review_tool_defined(folder, scope, id)
            .map(|(path, _)| path)
            .or_else(|| Some(folder.join(format!("{id}.md"))).filter(|path| path.exists()))
    }
}

/// The file in `folder` that defines `id` and fits, with its text.
pub(crate) fn review_tool_defined(folder: &Path, scope: ReviewToolScope, id: &str) -> Option<(PathBuf, String)> {
    let wrap = |path: PathBuf, text: String| match scope {
        ReviewToolScope::Kit => Written::in_kit(path, text),
        ReviewToolScope::Repository => Written::in_repository(path, text),
    };
    files_in(folder).into_iter().find(|(path, text)| {
        let resolved = tools::resolve([wrap(path.clone(), text.clone())]);
        resolved.defs.iter().any(|def| def.id == id)
    })
}

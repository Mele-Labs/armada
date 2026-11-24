//! One folder of a repository at `HEAD`, for the code tree Bridge opens a level at a time.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SystemFolder, WireError};

use crate::daemon::Fleet;

const UNKNOWN: &str = "fleet.system_folder_unknown";

/// The names directly in `folder` at `HEAD` of the checkout at `root`, folders first and each ending in `/`. `None` where
/// git would not list it. **Blocking**.
fn listed(root: &std::path::Path, folder: &str) -> Option<Vec<String>> {
    let mut git = crate::files::git_in(root);
    git.args(["ls-tree", "-z", "HEAD"]);
    if !folder.is_empty() {
        git.arg(format!("{folder}/"));
    }
    let out = git.output().ok().filter(|out| out.status.success())?;
    let (mut folders, mut files) = (Vec::new(), Vec::new());
    for line in out.stdout.split(|byte| *byte == 0).filter(|line| !line.is_empty()) {
        // `<mode> <type> <object>\t<path>`
        let line = String::from_utf8_lossy(line);
        let Some((head, path)) = line.split_once('\t') else { continue };
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        if head.split_whitespace().nth(1) == Some("tree") {
            folders.push(format!("{name}/"));
        } else {
            files.push(name);
        }
    }
    folders.sort();
    files.sort();
    folders.extend(files);
    Some(folders)
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
    /// `list_system_folder`. Where the project holds several repositories the path starts with the repository's folder name.
    pub(crate) async fn system_folder_read(&self, project: String, path: String) -> Result<SystemFolder, Refusal> {
        let members = self.system_members(&project).await?;
        let wanted = path.trim_matches('/').to_string();
        if wanted.split('/').any(|segment| segment == ".." || segment == ".") {
            return Err(Refusal::Unacceptable(WireError::raised(UNKNOWN, "a folder is named from the repository's root, with nothing above it", self.run_id())));
        }
        let several = members.len() > 1;
        let chosen = members.into_iter().find_map(|member| {
            if !several {
                return Some((member.root, wanted.clone()));
            }
            let own = member.root.file_name().map(|name| name.to_string_lossy().into_owned())?;
            let rest = if wanted == own { Some("") } else { wanted.strip_prefix(&format!("{own}/")) };
            rest.map(|rest| (member.root.clone(), rest.to_string()))
        });
        let unknown = || Refusal::NotFound(WireError::raised(UNKNOWN, format!("the repository holds no folder `{wanted}` at its head"), self.run_id()));
        let (root, folder) = chosen.ok_or_else(unknown)?;
        let entries = tokio::task::spawn_blocking(move || listed(&root, &folder)).await.ok().flatten().filter(|entries| !entries.is_empty() || wanted.is_empty());
        Ok(SystemFolder { project, path: wanted.clone(), entries: entries.ok_or_else(unknown)? })
    }
}

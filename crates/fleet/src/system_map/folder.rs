//! One folder of a repository at `HEAD`, for the code tree Bridge opens a level at a time.

use std::collections::BTreeSet;

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
    /// **Where the project picked folders of a repository, a folder above them lists only the way down to them and a folder
    /// outside them is refused** as one the repository does not hold.
    pub(crate) async fn system_folder_read(&self, project: String, path: String) -> Result<SystemFolder, Refusal> {
        let members = self.system_members(&project).await?;
        let wanted = path.trim_matches('/').to_string();
        if wanted.split('/').any(|segment| segment == ".." || segment == ".") {
            return Err(Refusal::Unacceptable(WireError::raised(UNKNOWN, "a folder is named from the repository's root, with nothing above it", self.run_id())));
        }
        let several = members.len() > 1;
        let chosen = members.into_iter().find_map(|member| {
            if !several {
                return Some((member.root, wanted.clone(), member.folders));
            }
            let own = member.root.file_name().map(|name| name.to_string_lossy().into_owned())?;
            let rest = if wanted == own { Some("") } else { wanted.strip_prefix(&format!("{own}/")) };
            rest.map(|rest| (member.root.clone(), rest.to_string(), member.folders.clone()))
        });
        let unknown = || Refusal::NotFound(WireError::raised(UNKNOWN, format!("the repository holds no folder `{wanted}` at its head"), self.run_id()));
        let (root, folder, picked) = chosen.ok_or_else(unknown)?;
        // Above the picked folders: the first name on the way down to each.
        let ways: Option<BTreeSet<String>> = (!picked.is_empty() && !super::repo::inside(&picked, &folder)).then(|| {
            picked
                .iter()
                .filter_map(|one| if folder.is_empty() { Some(one.as_str()) } else { one.strip_prefix(&format!("{folder}/")) })
                .map(|rest| format!("{}/", rest.split('/').next().unwrap_or(rest)))
                .collect()
        });
        if ways.as_ref().is_some_and(BTreeSet::is_empty) {
            return Err(unknown());
        }
        let entries = tokio::task::spawn_blocking(move || listed(&root, &folder)).await.ok().flatten().filter(|entries| !entries.is_empty() || wanted.is_empty());
        let mut entries = entries.ok_or_else(unknown)?;
        if let Some(ways) = ways {
            entries.retain(|entry| ways.contains(entry));
        }
        Ok(SystemFolder { project, path: wanted.clone(), entries })
    }
}

//! Moving his Macs onto one build, keeping running Jobs: `POST /macs/update`. This Mac restarts as Refresh
//! preview does, with `--adopt`; another Mac is asked over the link, the one call a frozen link carries.
//! `docs/concepts/pocket.md`, *Builds that differ*.
//!
//! **A preview is pushed, so both Macs build the same commit.** A preview refresh on a Mac with another linked
//! passes `--push-as <this Mac>` to `scripts/restart-build`, which pushes the merged commit to
//! `preview/<this Mac>` on origin. The other Mac is sent that commit and branch and builds exactly it, merging
//! nothing of its own.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{BuildSource, ChangeFleetBuild, FleetBuildChanging, UpdateMac, WireError};

use crate::daemon::Fleet;
use crate::kept_builds;
use crate::other_macs::MAC_UNREACHABLE;

/// The body names a Mac and another to move like, a Mac where another Mac sent it, or a commit or branch
/// that is not one. A 422.
const UPDATE_UNACCEPTABLE: &str = "fleet.mac_update_unacceptable";
/// No Mac by that id is paired, or it never said what it runs. A 404.
const MAC_UNKNOWN: &str = "fleet.mac_update_unknown";

/// A commit as git prints it: 7 to 40 hex digits.
fn a_commit(text: &str) -> bool {
    (7..=40).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `preview/<mac id>`, and nothing else. **The other Mac sends it**, and it reaches `git fetch` as an argument.
fn a_preview_branch(text: &str) -> bool {
    text.strip_prefix("preview/").is_some_and(|id| !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The branch a Mac's preview is pushed to.
pub(crate) fn preview_branch(mac: &str) -> String {
    format!("preview/{mac}")
}

/// What one Mac moves onto: main, or the preview, at a pushed commit where one is named.
struct Onto {
    build: BuildSource,
    at: Option<(String, String)>,
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
    fn update_refused(&self, why: impl Into<String>) -> Refusal {
        Refusal::Unacceptable(WireError::raised(UPDATE_UNACCEPTABLE, why.into(), self.run_id()))
    }

    /// `update_mac`.
    pub(crate) async fn mac_updated(&self, asked: UpdateMac) -> Result<FleetBuildChanging, Refusal> {
        let from_a_mac = api::via().is_some_and(|via| via.as_wire() == api::MAC);
        if from_a_mac && (asked.mac.is_some() || asked.like.is_some() || asked.this_mac_too) {
            return Err(self.update_refused("Another Mac can only ask this one to update itself."));
        }
        if asked.mac.is_some() && asked.like.is_some() {
            return Err(self.update_refused("Name the Mac to update, or the Mac to update like, not both."));
        }
        if asked.commit.as_deref().is_some_and(|commit| !a_commit(commit)) {
            return Err(self.update_refused("The commit is not one git would print."));
        }
        if asked.reference.as_deref().is_some_and(|branch| !a_preview_branch(branch)) {
            return Err(self.update_refused("Only a preview branch, preview/<Mac>, is fetched for an update."));
        }
        if let Some(like) = asked.like.as_deref() {
            let onto = self.onto_like(like).await?;
            return self.updated_here(onto, None).await;
        }
        let Some(mac) = asked.mac.clone() else {
            let at = asked.commit.zip(asked.reference);
            return self.updated_here(Onto { build: asked.build.unwrap_or(BuildSource::Main), at }, None).await;
        };
        let mac = self.mac_named(&mac)?;
        if !asked.this_mac_too {
            let onto = self.onto_this_build().await;
            return self.other_mac_updated(&mac, onto).await;
        }
        match asked.build.unwrap_or(BuildSource::Main) {
            // Each Mac fetches main for itself.
            BuildSource::Main => {
                self.other_mac_updated(&mac, Onto { build: BuildSource::Main, at: None }).await?;
                self.updated_here(Onto { build: BuildSource::Main, at: None }, None).await
            }
            // The commit is not merged yet: the wrapper merges it, pushes it and asks the other Mac.
            BuildSource::Preview => self.updated_here(Onto { build: BuildSource::Preview, at: None }, Some(&mac)).await,
        }
    }

    /// `change_fleet_build`: a preview refresh on a Mac with another linked pushes its commit, so an Update on the
    /// other Mac can build the same one.
    pub(crate) async fn fleet_build_changed(&self, asked: &ChangeFleetBuild) -> Result<FleetBuildChanging, Refusal> {
        let extra = match asked.build {
            BuildSource::Preview => self.push_as().await.map(|me| vec![String::from("--push-as"), me]).unwrap_or_default(),
            BuildSource::Main => Vec::new(),
        };
        self.change_fleet_build_now(asked, &extra)
    }

    /// This Mac's id, where another Mac is linked to share a preview with.
    async fn push_as(&self) -> Option<String> {
        let store = self.store().lock().await;
        let linked = store.paired_macs().ok()?.iter().any(|mac| mac.state == store::MacState::Linked);
        linked.then(|| store.this_mac().ok().flatten().map(|me| me.id)).flatten()
    }

    /// This Mac restarted onto `onto`, keeping its Jobs. `tell` is the other Mac the wrapper asks to build the
    /// preview it merges.
    async fn updated_here(&self, onto: Onto, tell: Option<&str>) -> Result<FleetBuildChanging, Refusal> {
        let mut extra = Vec::new();
        match (&onto.build, onto.at) {
            (BuildSource::Preview, Some((commit, branch))) => extra.extend([String::from("--at"), commit, String::from("--ref"), branch]),
            (BuildSource::Preview, None) => {
                let me = self.push_as().await;
                if let Some(me) = me {
                    extra.extend([String::from("--push-as"), me]);
                }
                if let Some(other) = tell {
                    if extra.is_empty() {
                        return Err(self.update_refused("This Mac is linked to no other Mac to share its preview with."));
                    }
                    extra.extend([String::from("--tell"), other.to_string()]);
                }
            }
            (BuildSource::Main, _) => {}
        }
        self.change_fleet_build_now(&ChangeFleetBuild { build: onto.build, adopt: true }, &extra)
    }

    /// The build this Mac runs, as another Mac would build it: main, or this Mac's pushed preview.
    async fn onto_this_build(&self) -> Onto {
        let support = crate::runtime::support_dir(&self.host().home);
        let running = kept_builds::kept_at(&support.join(kept_builds::BUILDS), kept_builds::CURRENT);
        let me = self.store().lock().await.this_mac().ok().flatten();
        match (running, me) {
            (Some(kept), Some(me)) if kept.source == "preview" => {
                Onto { build: BuildSource::Preview, at: Some((kept.commit, preview_branch(&me.id))) }
            }
            _ => Onto { build: BuildSource::Main, at: None },
        }
    }

    /// The build Mac `like` runs, as its last hello said.
    async fn onto_like(&self, like: &str) -> Result<Onto, Refusal> {
        let row = self.store().lock().await.paired_mac(like).map_err(|why| self.ledger_fault(store::WriteError::Database(why)))?;
        let unknown = || Refusal::NotFound(WireError::raised(MAC_UNKNOWN, format!("No Mac {like} is paired with this one."), self.run_id()));
        let row = row.ok_or_else(unknown)?;
        match (row.source.as_deref(), row.build) {
            (Some("preview"), Some(commit)) if a_commit(&commit) => {
                Ok(Onto { build: BuildSource::Preview, at: Some((commit, preview_branch(&row.id))) })
            }
            (Some("preview"), _) => Err(Refusal::NotFound(WireError::raised(
                MAC_UNKNOWN,
                format!("{} runs a preview it did not say the commit of.", row.name),
                self.run_id(),
            ))),
            _ => Ok(Onto { build: BuildSource::Main, at: None }),
        }
    }

    /// Mac `mac` asked over the link to move onto `onto`. **What it refuses is passed on as it said it.**
    async fn other_mac_updated(&self, mac: &str, onto: Onto) -> Result<FleetBuildChanging, Refusal> {
        let (commit, reference) = onto.at.unzip();
        let sent = UpdateMac { build: Some(onto.build), commit, reference, ..UpdateMac::default() };
        let body = self.encoded(&sent)?.into_bytes();
        let answered = self.phone_switch()?.send_other_mac(mac, "/other_mac/update", body).await;
        let (status, bytes) = answered.map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREACHABLE, why, self.run_id())))?;
        if !(200..300).contains(&status) {
            return Err(self.refusal_passed_on(mac, status, &bytes));
        }
        ipc::decode("an update from another Mac", &bytes).map_err(|why| {
            Refusal::Fault(WireError::raised(
                crate::other_macs::MAC_UNREADABLE,
                format!("Mac {mac} answered its update in a way this build cannot read: {why}"),
                self.run_id(),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_hex_commit_and_a_preview_branch_reach_git() {
        assert!(a_commit("126b4bb") && a_commit(&"a".repeat(40)));
        assert!(!a_commit("12ab") && !a_commit("--upload-pack=x") && !a_commit(&"a".repeat(41)));
        assert!(a_preview_branch("preview/ab12"));
        for refused in ["main", "preview/", "preview/--upload-pack=x", "preview/ab/../main", "refs/heads/preview/ab"] {
            assert!(!a_preview_branch(refused), "{refused}");
        }
    }
}

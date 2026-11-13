//! `list_forge_repositories`: what the person's forge account can see, to pick one to clone,
//! each marked with the served repository that is it.
//!
//! **One forge read a minute.** The whole listing is kept for [`LISTING_KEPT`] and every `q`
//! narrows the kept one, so typing asks the forge nothing. A refusal is not kept.
//!
//! **Cloned is what Fleet serves**, matched by remote: the served roots' remotes are read, and
//! no other folder on the disk.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::{AccountRepositories, AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::ForgeAddress;
use api::Refusal;
use ipc::{ForgeRepositories, ForgeRepository, WireError};

use crate::daemon::Fleet;

/// How long a listing answers every ask before the forge is asked again.
pub const LISTING_KEPT: Duration = Duration::from_secs(60);
/// The most an answer lists. More that match is `cut`.
pub const MOST_LISTED: usize = 200;
/// The most the forge is asked for, newest pushed first, which `q` narrows.
pub const MOST_READ: usize = 1000;
/// The forge would not list them: no token, no network, a refusal. A 422 in the forge's words.
const FORGE_REPOSITORIES_UNREADABLE: &str = "fleet.forge_repositories_unreadable";

/// The last listing and when it was read. **Held across the read**, so asks that arrive while
/// the forge is answering wait for that answer rather than asking again.
#[derive(Default)]
pub struct ForgeListing(tokio::sync::Mutex<Option<(Instant, Arc<AccountRepositories>)>>);

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
    /// `q` narrows by words, each a case-blind substring of `owner/name`.
    pub(crate) async fn forge_repositories(&self, q: Option<String>) -> Result<ForgeRepositories, Refusal> {
        let listed = self.account_listing().await?;
        let words: Vec<String> = q.unwrap_or_default().split_whitespace().map(str::to_lowercase).collect();
        let roots: Vec<String> = self.repositories().served().iter().map(|one| one.root().to_string()).collect();
        let marked = tokio::task::spawn_blocking(move || narrowed(&listed, &words, &roots)).await;
        Ok(marked.expect("marking the forge's repositories panicked"))
    }

    async fn account_listing(&self) -> Result<Arc<AccountRepositories>, Refusal> {
        let mut kept = self.repositories().forge_listing().0.lock().await;
        if let Some((at, listed)) = kept.as_ref() {
            if at.elapsed() < LISTING_KEPT {
                return Ok(Arc::clone(listed));
            }
        }
        let vcs = Arc::clone(self.vcs());
        let listed = tokio::task::spawn_blocking(move || vcs.account_repositories(MOST_READ))
            .await
            .expect("listing the forge's repositories panicked")
            .map_err(|said| Refusal::Unacceptable(WireError::raised(FORGE_REPOSITORIES_UNREADABLE, said, self.run_id())))?;
        let listed = Arc::new(listed);
        *kept = Some((Instant::now(), Arc::clone(&listed)));
        Ok(listed)
    }
}

/// The listing narrowed to `words` and capped, each marked with the served root that is it.
/// Reads each served root's remote, so off the async workers.
fn narrowed(listed: &AccountRepositories, words: &[String], roots: &[String]) -> ForgeRepositories {
    let served: Vec<(&String, ForgeAddress)> =
        roots.iter().filter_map(|root| Some((root, adapters::remote_of(Path::new(root))?))).collect();
    let mut matching = listed.repositories.iter().filter(|one| {
        let named = format!("{}/{}", one.owner, one.name).to_lowercase();
        words.iter().all(|word| named.contains(word.as_str()))
    });
    let repositories = matching
        .by_ref()
        .take(MOST_LISTED)
        .map(|one| {
            let address = ForgeAddress::of_remote(&one.url);
            let local = served.iter().find(|(_, remote)| Some(remote) == address.as_ref());
            ForgeRepository {
                owner: one.owner.clone(),
                name: one.name.clone(),
                url: one.url.clone(),
                private: one.private,
                pushed_at: one.pushed_at.as_deref().map(ipc::Instant::carried),
                local_path: local.map(|(root, _)| root.to_string()),
            }
        })
        .collect();
    let cut = listed.cut || matching.next().is_some();
    ForgeRepositories { repositories, cut }
}

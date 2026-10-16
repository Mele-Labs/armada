//! Whose pull requests Fleet watches. `docs/concepts/fleet.md`, *Whose pull requests Fleet reads*.
//!
//! **Mine is the author's login being one of two**: the account Armada is signed in as for the
//! repository (`SigningIn::login_for`) or the one the forge's command line is signed in as on
//! this machine. Case does not matter. **Where neither is known every pull request is mine**, and
//! so is one whose forge named no author, because a rule that cannot be read must not go quiet.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;
use crate::repository_profiles::Repo;

/// The logins that are the person's. `None` is none known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mine(Option<Vec<String>>);

impl Mine {
    pub(crate) fn of(logins: impl IntoIterator<Item = Option<String>>) -> Mine {
        let known: Vec<String> = logins
            .into_iter()
            .flatten()
            .map(|login| login.trim().to_lowercase())
            .filter(|login| !login.is_empty())
            .collect();
        Mine((!known.is_empty()).then_some(known))
    }

    pub(crate) fn owns(&self, author: Option<&str>) -> bool {
        match (&self.0, author.map(str::trim).filter(|it| !it.is_empty())) {
            (Some(known), Some(author)) => known.iter().any(|login| login.eq_ignore_ascii_case(author)),
            _ => true,
        }
    }
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
    /// Who the repository at `root`'s pull requests are mine by.
    pub(crate) async fn mine_in(&self, root: &str) -> Mine {
        let (reaching, signing, at) = (Arc::clone(self.reaching()), self.signing_in().clone(), root.to_string());
        let signed = tokio::task::spawn_blocking(move || {
            let slug = reaching.slug(&at);
            signing.login_for(&Repo::new(&at, slug))
        })
        .await
        .ok()
        .flatten();
        Mine::of([signed, self.local_login(root).await])
    }

    /// The forge command line's login, **asked until it answers and then kept**: a signed-out `gh`
    /// is asked again at the next reading, and a signed-in one never twice.
    async fn local_login(&self, root: &str) -> Option<String> {
        if let Some(kept) = self.sweeping().lock().await.local_login.clone() {
            return Some(kept);
        }
        let asked = self.forge_asked(root, |vcs: &V, root: &str| vcs.forge_login(root)).await;
        if let Some(login) = &asked {
            self.sweeping().lock().await.local_login = Some(login.clone());
        }
        asked
    }
}

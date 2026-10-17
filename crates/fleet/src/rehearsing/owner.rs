//! Whose tree a run is in: the main checkout.
//!
//! **One run at a time is per owner**: two runs in one tree fight over one
//! build directory, which is what [`Owner`] keys
//! [`Rehearsals`](super::in_flight::Rehearsals) on.
//!
//! **The checkout's runs are kept under `main`**, the name `crate::servers`
//! already keeps the main checkout's servers under.

use std::path::PathBuf;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::WireError;

use super::workspace::Within;
use crate::daemon::Fleet;
use crate::repositories::{Repository, Served};

/// A route naming its repository by both `manifest_id` and `repository`. A 422.
const REPOSITORY_NAMED_TWICE: &str = "fleet.repository_named_twice";

/// A repository's main checkout. **Its root Manifest may be absent**, so a
/// workspace's own file can be verified before the root has one.
#[derive(Clone)]
pub(crate) struct Checkout {
    repository: Arc<Repository>,
    served: Option<Served>,
}

impl Checkout {
    pub(crate) fn root(&self) -> &str {
        self.repository.root()
    }

    pub(crate) fn records_root(&self) -> &str {
        self.repository.records_root()
    }

    /// The repository with its root Manifest, where it has one.
    pub(crate) fn served(&self) -> Option<&Served> {
        self.served.as_ref()
    }
}

impl From<Served> for Checkout {
    fn from(served: Served) -> Checkout {
        Checkout {
            repository: Arc::clone(served.repository()),
            served: Some(served),
        }
    }
}

/// The directory the main checkout's runs are kept under.
pub(crate) const CHECKOUT_HANDLE: &str = "main";

/// The key one run at a time is held per.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Owner {
    /// The main checkout of the repository at this root.
    Checkout(String),
}

/// A tree a run happens in, as a path and — where there is one — as what
/// `WorkProduct` reads a diff from.
pub(crate) struct Tree {
    pub(crate) path: PathBuf,
    /// Where commands run, relative to `path`, and the ports they draw: a
    /// workspace's own in a Verify of its file. The snapshot and Undo stay on
    /// `path`.
    pub(crate) within: Option<Within>,
}

impl Tree {
    /// The directory a command runs in.
    pub(crate) fn commands_in(&self) -> PathBuf {
        match &self.within {
            Some(within) => self.path.join(&within.dir),
            None => self.path.clone(),
        }
    }
}

/// An owner resolved: what a run of its needs that the owner alone decides.
pub(crate) struct Place {
    pub(crate) owner: Owner,
    /// The directory under `.armada/runs/` this owner's runs are kept in.
    pub(crate) handle: String,
    /// The checkout of the repository the tree is in.
    pub(crate) checkout: Checkout,
}

impl Place {

    pub(crate) fn of_checkout(checkout: impl Into<Checkout>) -> Place {
        let checkout = checkout.into();
        Place {
            owner: Owner::Checkout(checkout.root().to_string()),
            handle: String::from(CHECKOUT_HANDLE),
            checkout,
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
    /// The tree this owner's runs happen in. **The main checkout is always
    /// there**: it is the repository Fleet is running on, and Fleet answering
    /// at all is the proof.
    pub(crate) fn tree_at(&self, place: &Place) -> Tree {
        Tree {
            path: PathBuf::from(place.checkout.root()),
            within: None,
        }
    }

    /// The Manifest this owner's entries are read from: the one Fleet holds.
    /// `None` where the checkout's root has no Manifest.
    pub(crate) fn manifest_at<'a>(&self, place: &'a Place) -> Option<&'a config::Manifest> {
        Some(place.checkout.served()?.manifest())
    }

    /// The checkout a route names, by `?manifest_id=` or by `?repository=` root,
    /// which also reaches a repository with no root Manifest. Both is refused
    /// rather than one quietly chosen.
    pub(crate) fn checkout_named(
        &self,
        manifest_id: Option<&ipc::ManifestId>,
        root: Option<&str>,
    ) -> Result<Checkout, api::Refusal> {
        let Some(root) = root else {
            return self.served_named(manifest_id).map(Checkout::from);
        };
        if manifest_id.is_some() {
            return Err(api::Refusal::Unacceptable(WireError::raised(
                REPOSITORY_NAMED_TWICE,
                String::from("name the repository by `manifest_id` or by `repository`, not both"),
                self.run_id(),
            )));
        }
        let repository = self.repository_named(Some(root))?;
        let served = self
            .repositories()
            .served()
            .into_iter()
            .find(|one| one.root() == repository.root());
        Ok(Checkout { repository, served })
    }

}

//! The claims stores this Mac joined, as the Gateway reaches them to pair with his other devices with no code.
//! The composition root hands it in, as it does the Keychain's keys, so this crate depends on neither Fleet
//! nor the store's client. `docs/concepts/pocket.md`, *Through his store*.

use adapter_traits::StorePeer;

/// A store this Mac joined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinedStore {
    pub address: String,
    /// The project that names it names a team too, so it is not his alone.
    pub team: bool,
}

/// Blocking: every call but [`HisStores::joined`] and [`HisStores::client_id`] is a request to a store.
pub trait HisStores: Send + Sync + 'static {
    /// Every store this Mac joined, in project order, each once.
    fn joined(&self) -> Vec<JoinedStore>;
    /// The other devices of this Mac's own account on `store`.
    fn peers(&self, store: &str) -> Result<Vec<StorePeer>, String>;
    /// Say on `store` that this is a Mac, reached at `address`.
    fn put_presence(&self, store: &str, address: &str) -> Result<(), String>;
    /// The four `X-Armada-*` headers for a request from this Mac to another of his devices, signed with this
    /// Mac's key for `store` as a request to the store is.
    fn sign(&self, store: &str, method: &str, target: &str, body: &[u8]) -> Result<Vec<(&'static str, String)>, String>;
    /// The sign-in app's client id, where one is set.
    fn client_id(&self) -> Option<String>;
}

/// The store a phone signs in to for pairing through his store: the first joined that names no team.
pub(crate) fn his_own(stores: &dyn HisStores) -> Option<String> {
    stores.joined().into_iter().find(|store| !store.team).map(|store| store.address)
}

//! Registering the app Armada signs in with, from a manifest: the person's
//! browser posts a form to the forge, the forge sends it back with a code, and
//! Fleet converts the code into the app. `docs/concepts/account.md`, *Sign-in*.
//!
//! **What comes back holds no secret, because no field could hold one.** The
//! forge answers a conversion with the app's client secret, private key and
//! webhook secret beside its client id. [`RegisteredApp`] names the client id
//! and the page its settings are on and nothing else, so an implementation
//! reads the secrets into no value and has nowhere to hand them.

use alloc::string::String;

use crate::sign_in::SignInError;

/// The form a browser posts to the forge to register an app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppForm {
    /// Where the form is posted, the state already on it.
    pub action: String,
    /// The one field's name.
    pub field: String,
    /// The field's value: the app as the forge reads it.
    pub manifest: String,
}

/// An app the forge registered, as Armada keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredApp {
    /// What a sign-in names the app by. Not a secret.
    pub client_id: String,
    /// The forge's page for the app's settings, where its device flow is turned on.
    pub settings_page: String,
}

/// The forge's app-manifest flow, as Fleet uses it.
pub trait SignInApp {
    /// The form that registers an app named `name`, private, asking for no
    /// permission and no event, with the forge sending the browser back to
    /// `back_to` with `state`. Builds a value and reaches nothing.
    fn form(&self, name: &str, back_to: &str, state: &str) -> AppForm;

    /// Turn the code the forge sent the browser back with into the app.
    /// **A network round trip, blocking**: call it from a thread, not an async
    /// worker. The forge takes a code once, and for an hour.
    fn convert(&self, code: &str) -> Result<RegisteredApp, SignInError>;
}

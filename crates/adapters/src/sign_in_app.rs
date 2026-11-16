//! Registering Armada's sign-in app on GitHub from a manifest: the form the
//! person's browser posts to `github.com/settings/apps/new`, and the one call
//! that turns the code GitHub sends back into the app. `docs/concepts/account.md`,
//! *Sign-in*.
//!
//! **The conversion's secrets are never read.** GitHub answers with the app's
//! `client_secret`, `pem` and `webhook_secret` beside its `client_id`;
//! [`Converted`] names only the two fields kept, so the decoder skips the rest
//! and the body they arrived in is dropped with the reply. A refusal is read
//! for its `message` alone, never repeated whole.
//!
//! **No manifest field turns the device flow on**, so the app is made with it
//! off and the settings page answered is where the person turns it on.

use adapter_traits::{AppForm, RegisteredApp, SignInApp, SignInError};
use serde::{Deserialize, Serialize};

use crate::sign_in::{ForgeSignIn, Reply};

const NEW_APP_URL: &str = "https://github.com/settings/apps/new";
const CONVERSIONS_URL: &str = "https://api.github.com/app-manifests";
const APPS_URL: &str = "https://github.com/settings/apps";
/// What the app says it is for: GitHub requires a homepage, and the app has none of its own.
const HOMEPAGE: &str = "https://github.com";
/// The form field GitHub reads the manifest from.
const FIELD: &str = "manifest";
/// The longest refusal kept.
const SAID_AT_MOST: usize = 200;

/// Where each half of the flow is, a field so a test's socket stands in for GitHub.
#[derive(Clone, Debug)]
pub(crate) struct AppUrls {
    pub(crate) new_app: String,
    pub(crate) conversions: String,
    pub(crate) apps: String,
}

impl AppUrls {
    pub(crate) fn github() -> AppUrls {
        AppUrls { new_app: NEW_APP_URL.to_string(), conversions: CONVERSIONS_URL.to_string(), apps: APPS_URL.to_string() }
    }
}

#[derive(Serialize)]
struct Hook<'a> {
    url: &'a str,
    active: bool,
}

/// No permission, no event and no webhook: the app exists to be signed in with.
#[derive(Serialize)]
struct Manifest<'a> {
    name: &'a str,
    url: &'a str,
    redirect_url: &'a str,
    public: bool,
    hook_attributes: Hook<'a>,
    default_permissions: std::collections::BTreeMap<String, String>,
    default_events: Vec<String>,
}

/// The two fields of a conversion kept. **The secrets have no field here.**
#[derive(Deserialize)]
struct Converted {
    client_id: String,
    slug: String,
}

#[derive(Deserialize)]
struct Refusal {
    message: Option<String>,
}

pub(crate) fn manifest_of(name: &str, back_to: &str) -> String {
    let manifest = Manifest {
        name,
        url: HOMEPAGE,
        redirect_url: back_to,
        public: false,
        hook_attributes: Hook { url: HOMEPAGE, active: false },
        default_permissions: std::collections::BTreeMap::new(),
        default_events: Vec::new(),
    };
    // A struct of strings and empty collections always encodes.
    ipc::encode(&manifest).unwrap_or_default()
}

/// A code is GitHub's short token, put in a path. Anything else is refused before it is sent.
fn fits_a_path(text: &str) -> bool {
    !text.is_empty() && text.len() <= 100 && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The app a conversion reply names, read off the reply's two kept fields.
pub(crate) fn registered_of(reply: &Reply, apps: &str) -> Result<RegisteredApp, SignInError> {
    if reply.status >= 500 {
        return Err(SignInError::Unreachable(format!("status {}", reply.status)));
    }
    if reply.status != 201 && reply.status != 200 {
        let words = ipc::decode::<Refusal>("a refusal", &reply.body).ok().and_then(|refusal| refusal.message);
        let words: String = words.unwrap_or_else(|| format!("status {}", reply.status)).chars().take(SAID_AT_MOST).collect();
        return Err(SignInError::Refused(words));
    }
    let converted: Converted = ipc::decode("a registered app", &reply.body)
        .map_err(|_| SignInError::Refused("GitHub answered with no app Armada could read.".to_string()))?;
    if converted.client_id.is_empty() || !fits_a_path(&converted.slug) {
        return Err(SignInError::Refused("GitHub answered with no app Armada could read.".to_string()));
    }
    Ok(RegisteredApp { client_id: converted.client_id, settings_page: format!("{apps}/{}", converted.slug) })
}

impl SignInApp for ForgeSignIn {
    fn form(&self, name: &str, back_to: &str, state: &str) -> AppForm {
        let action = reqwest::Url::parse_with_params(&self.apps.new_app, [("state", state)])
            .map(String::from)
            .unwrap_or_else(|_| self.apps.new_app.clone());
        AppForm { action, field: FIELD.to_string(), manifest: manifest_of(name, back_to) }
    }

    fn convert(&self, code: &str) -> Result<RegisteredApp, SignInError> {
        if !fits_a_path(code) {
            return Err(SignInError::Refused("The code GitHub sent back is not one it gives.".to_string()));
        }
        let url = format!("{}/{code}/conversions", self.apps.conversions);
        registered_of(&self.send(|client| client.post(&url))?, &self.apps.apps)
    }
}

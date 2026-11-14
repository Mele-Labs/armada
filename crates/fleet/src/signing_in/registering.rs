//! Registering the app sign-in uses, from a manifest, in one press: a state
//! issued here, a page that posts the manifest to the forge with it, and the
//! code the forge sends back converted into the app's client id.
//! `docs/concepts/account.md`, *Sign-in*.
//!
//! **A state is good once, for an hour, and only in memory.** The page leaves
//! it live; the way back uses it up before the forge is asked, so a second
//! visit is refused whatever the first came to. A restart forgets every one.
//!
//! **The forge's secrets never reach this crate.** [`RegisteredApp`] has no
//! field for them, so the client id and the settings page are all there is.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use adapter_traits::{AppForm, RegisteredApp, SignInApp, SignInError};

use super::{Refused, SigningIn};

/// How long a state stays live: the hour the forge gives a code.
const LIVE_FOR_MILLIS: i64 = 60 * 60 * 1000;
/// What every app is called before its six random characters, which keep the
/// name unique on the forge within its 34-character limit.
const NAME: &str = "Armada sign-in";

/// The forge's half, and the states issued with when each was.
pub(super) struct Registering {
    apps: Arc<dyn SignInApp + Send + Sync>,
    issued: Mutex<HashMap<String, i64>>,
}

impl Registering {
    pub(super) fn through(apps: Arc<dyn SignInApp + Send + Sync>) -> Registering {
        Registering { apps, issued: Mutex::new(HashMap::new()) }
    }

    fn issued(&self) -> MutexGuard<'_, HashMap<String, i64>> {
        self.issued.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `text` safe inside an element or a quoted attribute.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// `scheme://host[:port]` of an address, which is all a policy names.
fn origin_of(address: &str) -> &str {
    let after_scheme = address.find("://").map_or(0, |at| at + 3);
    match address[after_scheme..].find(['/', '?', '#']) {
        Some(at) => &address[..after_scheme + at],
        None => address,
    }
}

/// The page: one form, posted at once by the one script its policy allows, and
/// a button for a browser with scripts off.
fn page(form: &AppForm, nonce: &str) -> String {
    let policy = format!(
        "default-src 'none'; script-src 'nonce-{nonce}'; style-src 'unsafe-inline'; form-action {}; base-uri 'none'",
        origin_of(&form.action)
    );
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
         <meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\">\
         <title>Armada sign-in app</title>\
         <style>body{{font:15px -apple-system,sans-serif;margin:4em auto;max-width:32em;color:#333}}</style></head>\
         <body><form method=\"post\" action=\"{action}\">\
         <p>Taking you to the forge to create the app Armada signs in with.</p>\
         <input type=\"hidden\" name=\"{field}\" value=\"{manifest}\">\
         <button type=\"submit\">Continue</button></form>\
         <script nonce=\"{nonce}\">document.forms[0].submit();</script></body></html>\n",
        action = escaped(&form.action),
        field = escaped(&form.field),
        manifest = escaped(&form.manifest),
    )
}

impl SigningIn {
    fn now_millis(&self) -> i64 {
        self.inner.clock.now().epoch_millis().unwrap_or(i64::MAX)
    }

    fn live(&self, issued: i64) -> bool {
        self.now_millis().saturating_sub(issued) < LIVE_FOR_MILLIS
    }

    /// A fresh state, live for the hour. The lapsed are forgotten on the way.
    pub fn issue_app_state(&self) -> String {
        let state = hex(&rand::random::<[u8; 32]>());
        let now = self.now_millis();
        let mut issued = self.inner.registering.issued();
        issued.retain(|_, at| now.saturating_sub(*at) < LIVE_FOR_MILLIS);
        issued.insert(state.clone(), now);
        state
    }

    /// The page that posts the manifest for `state`, the forge sending the browser
    /// back to `back_to`. **Leaves the state live.**
    pub fn app_form(&self, state: &str, back_to: &str) -> Result<String, Refused> {
        let issued = self.inner.registering.issued().get(state).copied();
        if !issued.is_some_and(|at| self.live(at)) {
            return Err(Refused::AppStateUnknown);
        }
        let name = format!("{NAME} {}", hex(&rand::random::<[u8; 3]>()));
        let form = self.inner.registering.apps.form(&name, back_to, state);
        Ok(page(&form, &hex(&rand::random::<[u8; 16]>())))
    }

    /// Use `state` up, then convert `code` with the forge. **Blocks** on the forge.
    pub fn register_app(&self, code: &str, state: &str) -> Result<RegisteredApp, Refused> {
        let issued = self.inner.registering.issued().remove(state);
        if !issued.is_some_and(|at| self.live(at)) {
            return Err(Refused::AppStateUnknown);
        }
        self.inner.registering.apps.convert(code).map_err(|why| {
            Refused::AppNotRegistered(match why {
                SignInError::Unreachable(why) => format!("The forge could not be reached, so no app was kept: {why}"),
                SignInError::Refused(why) => format!("The forge would not register the app, so none was kept: {why}"),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_policy_names_only_the_origin_the_form_posts_to() {
        assert_eq!(origin_of("https://forge.test/settings/apps/new?state=1"), "https://forge.test");
        assert_eq!(origin_of("http://127.0.0.1:9/a"), "http://127.0.0.1:9");
        assert_eq!(origin_of("https://forge.test"), "https://forge.test");
    }

    #[test]
    fn the_manifest_cannot_close_its_attribute() {
        let form = AppForm { action: "https://forge.test/new?state=1".into(), field: "manifest".into(), manifest: r#"{"name":"a\"><script>"}"#.into() };
        let html = page(&form, "n");
        assert!(html.contains("value=\"{&quot;name&quot;:&quot;a\\&quot;&gt;&lt;script&gt;&quot;}\""), "{html}");
        assert_eq!(html.matches("<script").count(), 1);
    }
}

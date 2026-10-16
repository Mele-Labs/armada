//! The addresses a session signs in with, from `show_window` until Bridge opens the window.
//!
//! **A secret, held in memory only**: a magic link or a token address is a login. Nothing here is
//! stored, published, entered on the timeline or put on a thread, and a restart forgets it. Bridge
//! takes it once. What nobody takes is held until the session shows another window or ends, so a
//! Bridge that connects late still signs in.

use std::collections::BTreeMap;
use std::sync::Mutex;

struct Held {
    url: String,
    sign_in: Vec<String>,
}

/// The sign-in addresses held, one set per session: a session has one window.
#[derive(Default)]
pub(crate) struct SignIns {
    held: Mutex<BTreeMap<String, Held>>,
}

impl SignIns {
    /// Hold `sign_in` for the window on `url`, in place of any held for the session. An empty list
    /// holds nothing, so a window shown again without one clears the last.
    pub(crate) fn hold(&self, session: &str, url: &str, sign_in: Vec<String>) {
        let Ok(mut held) = self.held.lock() else { return };
        held.remove(session);
        if !sign_in.is_empty() {
            held.insert(session.to_string(), Held { url: url.to_string(), sign_in });
        }
    }

    /// Let go of what is held for a session that ended.
    pub(crate) fn forget(&self, session: &str) {
        if let Ok(mut held) = self.held.lock() {
            held.remove(session);
        }
    }

    /// The addresses held for the window on `url`, once. A window on another address is no match.
    pub(crate) fn take(&self, session: &str, url: &str) -> Vec<String> {
        let Ok(mut held) = self.held.lock() else { return Vec::new() };
        match held.get(session) {
            Some(one) if one.url == url => held.remove(session).map(|one| one.sign_in).unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "https://staging.example.com/";

    fn links() -> Vec<String> {
        vec!["https://staging.example.com/dev/login?token=abc".to_string()]
    }

    #[test]
    fn what_is_held_is_taken_once() {
        let held = SignIns::default();
        held.hold("s", PAGE, links());
        assert_eq!(held.take("s", PAGE), links());
        assert!(held.take("s", PAGE).is_empty(), "a second take finds nothing");
    }

    #[test]
    fn another_address_or_session_takes_nothing_and_leaves_it_held() {
        let held = SignIns::default();
        held.hold("s", PAGE, links());
        assert!(held.take("s", "https://other.example.com/").is_empty());
        assert!(held.take("t", PAGE).is_empty());
        assert_eq!(held.take("s", PAGE), links());
    }

    #[test]
    fn showing_again_without_addresses_clears_the_last() {
        let held = SignIns::default();
        held.hold("s", PAGE, links());
        held.hold("s", PAGE, Vec::new());
        assert!(held.take("s", PAGE).is_empty());
    }

    #[test]
    fn what_nobody_takes_is_held_until_the_session_ends() {
        let held = SignIns::default();
        held.hold("s", PAGE, links());
        held.hold("t", PAGE, links());
        held.forget("s");
        assert!(held.take("s", PAGE).is_empty(), "an ended session holds nothing");
        assert_eq!(held.take("t", PAGE), links(), "another session's is left");
    }

    #[test]
    fn showing_another_window_replaces_what_was_held() {
        let held = SignIns::default();
        let other = "https://other.example.com/";
        held.hold("s", PAGE, links());
        held.hold("s", other, vec!["https://other.example.com/login?token=xyz".to_string()]);
        assert!(held.take("s", PAGE).is_empty());
        assert_eq!(held.take("s", other).len(), 1);
    }
}

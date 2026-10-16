//! The addresses a session signs in with, from `show_window` until Bridge opens the window.
//!
//! **A secret, held in memory only**: a magic link or a token address is a login. Nothing here is
//! stored, published, entered on the timeline or put on a thread, and a restart forgets it. Bridge
//! takes it once, and what nobody takes is dropped after [`HELD_FOR`].

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Long enough for Bridge to hear the window row and ask. A Bridge that is not connected opens nothing.
const HELD_FOR: Duration = Duration::from_secs(120);

struct Held {
    url: String,
    sign_in: Vec<String>,
    at: Instant,
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
        self.hold_at(Instant::now(), session, url, sign_in);
    }

    /// The addresses held for the window on `url`, once. A window on another address is no match.
    pub(crate) fn take(&self, session: &str, url: &str) -> Vec<String> {
        self.take_at(Instant::now(), session, url)
    }

    fn hold_at(&self, now: Instant, session: &str, url: &str, sign_in: Vec<String>) {
        let Ok(mut held) = self.held.lock() else { return };
        held.retain(|_, one| now.saturating_duration_since(one.at) < HELD_FOR);
        held.remove(session);
        if !sign_in.is_empty() {
            held.insert(session.to_string(), Held { url: url.to_string(), sign_in, at: now });
        }
    }

    fn take_at(&self, now: Instant, session: &str, url: &str) -> Vec<String> {
        let Ok(mut held) = self.held.lock() else { return Vec::new() };
        held.retain(|_, one| now.saturating_duration_since(one.at) < HELD_FOR);
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
    fn what_nobody_takes_is_dropped() {
        let held = SignIns::default();
        let start = Instant::now();
        held.hold_at(start, "s", PAGE, links());
        assert!(held.take_at(start + HELD_FOR, "s", PAGE).is_empty());
    }
}

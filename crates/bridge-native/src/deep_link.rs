//! `armada://pull/<owner>/<name>/<number>`: what macOS hands the app to open, read the way
//! Electron's `DeepLinks` reads it (`apps/desktop/src/main/deep-link.ts`, `shared/pull-link.ts`).
//!
//! **Built and not claimed.** The owner decided links stay with Electron (Armada Classic) until
//! switch day, so the app's `Info.plist` carries no `CFBundleURLTypes` and the app is not set as
//! the scheme's default handler: macOS keeps sending links to Classic. The handler `main`
//! registers with `on_open_urls` runs only where something hands this app a link directly.
//! Switch day adds the claim, in `scripts/keep-build`'s `Info.plist`, and the screen a link lands on.
//!
//! For now a link that arrives is read and logged. There is no review page to land on.
//! Electron also holds a link that arrives before Fleet has said whether `features.deepLinks` is
//! on, and ignores one while it is off; the native app reads no settings yet, so the switch is
//! switch day's too.

use gpui::App;

use crate::log::Note;

/// The scheme, as `DEEP_LINK_SCHEME` spells it.
pub const SCHEME: &str = "armada";

/// A pull request named by a link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullLink {
    pub owner: String,
    pub name: String,
    pub number: u64,
}

/// The largest integer JavaScript holds exactly, which is where Electron's `Number.isSafeInteger` stops.
const LARGEST_NUMBER: u64 = (1 << 53) - 1;

fn number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok().filter(|found| (1..=LARGEST_NUMBER).contains(found))
}

/// The pull request an `armada://pull/<owner>/<name>/<number>` address names, or none. The host is
/// `pull`, and what follows the number, a query or a fragment is not read. Surrounding spaces from
/// a paste are ignored, as they are in `parsePullLink`.
pub fn parse_pull_link(address: &str) -> Option<PullLink> {
    let rest = address.trim().strip_prefix(SCHEME)?.strip_prefix("://")?;
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if host != "pull" {
        return None;
    }
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let (owner, name, at) = (parts.next()?, parts.next()?, parts.next()?);
    Some(PullLink { owner: owner.to_string(), name: name.to_string(), number: number(at)? })
}

/// What the OS handed over. A link is Armada's when it starts `armada:` and names a pull request;
/// anything else is not this app's to open.
pub fn arrive(address: &str) -> Option<PullLink> {
    if !address.starts_with(&format!("{SCHEME}:")) {
        return None;
    }
    parse_pull_link(address)
}

/// Take a link to the window. **There is none to take it to yet**, so it is logged as asked.
pub fn deliver(link: &PullLink, _cx: &mut App) {
    Note::info("a pull request link was opened")
        .field("owner", link.owner.as_str())
        .field("name", link.name.as_str())
        .field("number", link.number)
        .emit();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull() -> PullLink {
        PullLink { owner: "Mele-Labs".into(), name: "armada".into(), number: 2899 }
    }

    #[test]
    fn a_pull_request_link_names_its_owner_repository_and_number() {
        assert_eq!(arrive("armada://pull/Mele-Labs/armada/2899"), Some(pull()));
    }

    #[test]
    fn a_query_a_fragment_a_trailing_slash_and_surrounding_spaces_are_not_read() {
        for address in [
            "armada://pull/Mele-Labs/armada/2899/",
            "armada://pull/Mele-Labs/armada/2899?tab=files",
            "armada://pull/Mele-Labs/armada/2899#files",
            "armada://pull/Mele-Labs/armada/2899/files",
        ] {
            assert_eq!(arrive(address), Some(pull()), "{address}");
        }
        assert_eq!(parse_pull_link("  armada://pull/Mele-Labs/armada/2899\n"), Some(pull()));
    }

    #[test]
    fn an_address_that_names_no_pull_request_is_not_taken() {
        for address in [
            "",
            "armada",
            "armada:",
            "armada://",
            "armada://pull",
            "armada://pull/Mele-Labs/2899",
            "armada://pull/Mele-Labs/armada/abc",
            "armada://pull/Mele-Labs/armada/0",
            "armada://pull/Mele-Labs/armada/-3",
            "armada://pull/Mele-Labs/armada/9007199254740992",
            "armada://issue/Mele-Labs/armada/2899",
            "armada://pullx/Mele-Labs/armada/2899",
            "armada:pull/Mele-Labs/armada/2899",
            "https://forge.example/Mele-Labs/armada/pull/2899",
            "javascript:alert(1)",
            "file:///Mele-Labs/armada/pull/2899",
        ] {
            assert_eq!(arrive(address), None, "{address}");
        }
    }

    #[test]
    fn a_link_with_leading_spaces_is_not_armadas_to_take() {
        // `DeepLinks.arrive` tests the prefix before it trims, so a paste is not a link the OS handed over.
        assert_eq!(arrive(" armada://pull/Mele-Labs/armada/2899"), None);
    }
}

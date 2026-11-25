//! `armada://pull/<owner>/<name>/<number>`: what macOS hands the app to open, read the way
//! Electron's `DeepLinks` reads it (`classic-final:apps/desktop/src/main/deep-link.ts`, `shared/pull-link.ts`).
//!
//! **Claimed.** `scripts/native-app` writes `CFBundleURLTypes` into the app's `Info.plist`, so macOS
//! sends the scheme here. A link is read, then held until Fleet has said whether `features.deepLinks`
//! is on and which repositories it serves, and then it opens that pull request in Reviews
//! ([`decide`]). A link is ignored while the switch is off. One naming a repository Fleet has no
//! Manifest for is read through an unlisted stub Fleet makes (`link_repository`), so it opens too.

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

/// What a link comes to, given what Fleet has said so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Fleet has not yet said whether links are on, or which repositories it serves.
    Hold,
    /// `features.deepLinks` is off.
    Ignore,
    /// Reviews opens this pull request.
    Open(ipc::ReviewAsked),
    /// No Manifest serves the repository.
    Unserved,
}

/// What a link comes to. `deep_links` and `repositories` are `None` until Fleet has answered. The
/// repository is matched by its forge name and never guessed (`manifestOfLink`).
pub fn decide(link: &PullLink, deep_links: Option<bool>, repositories: Option<&[ipc::RepositorySummary]>) -> Fate {
    match (deep_links, repositories) {
        (Some(false), _) => Fate::Ignore,
        (None, _) | (_, None) => Fate::Hold,
        (Some(true), Some(served)) => {
            let repository = format!("{}/{}", link.owner, link.name);
            match crate::job_view::pulls::manifest_named(served, &repository) {
                Some(manifest) => Fate::Open(asked(link, &manifest)),
                None => Fate::Unserved,
            }
        }
    }
}

/// The review a link asks for in the repository `manifest` names.
fn asked(link: &PullLink, manifest: &str) -> ipc::ReviewAsked {
    let repository = format!("{}/{}", link.owner, link.name);
    ipc::ReviewAsked {
        manifest_id: ipc::ManifestId::carried(manifest),
        address: format!("https://github.com/{repository}/pull/{}", link.number),
        repository,
        number: link.number,
        title: String::new(),
        author: String::new(),
    }
}

/// What a repository Fleet linked is read through: its Manifest's id where it is served for real, else the one Fleet gives a repository with none.
fn manifest_of(summary: &ipc::RepositorySummary) -> Option<String> {
    summary.manifest.as_ref().map(|manifest| manifest.id.as_str().to_string()).or_else(|| summary.implicit_id.as_ref().map(|id| id.as_str().to_string()))
}

/// The link waiting for Fleet to answer, or for the window to be up.
#[derive(Default)]
struct Waiting(Option<PullLink>);

impl gpui::Global for Waiting {}

/// A link to a repository no Manifest serves: Fleet reads it through an unlisted stub (`POST /repositories/link`),
/// and the review opens on that; a refusal is said in a toast.
fn link_through_fleet(link: PullLink, cx: &mut App) {
    let Some(fleet) = crate::operator::screen::link_fleet(cx) else { return };
    let task = crate::new_project::wire::Wire::Live(fleet).link_repository(link.owner.clone(), link.name.clone(), cx);
    cx.spawn(async move |cx| match task.await {
        Ok(summary) => match manifest_of(&summary) {
            Some(manifest) => cx.update(|cx| crate::operator::screen::open_asked(Some(asked(&link, &manifest)), cx)),
            None => cx.update(|cx| crate::toasts::tell(format!("Armada could not read {}/{}.", link.owner, link.name), cx)),
        },
        Err(said) => cx.update(|cx| crate::toasts::tell(said, cx)),
    })
    .detach();
}

/// Open a pull request a screen of the app names, in Reviews. It is the link's own way in with the switch for links not asked,
/// since a press inside the app is not a link from outside: nothing happens until Fleet has said which repositories it serves.
pub fn open(link: &PullLink, cx: &mut App) {
    let Some((_, repositories)) = crate::operator::screen::link_state(cx) else { return };
    match decide(link, Some(true), repositories.as_deref()) {
        Fate::Open(asked) => crate::operator::screen::open_asked(Some(asked), cx),
        Fate::Unserved => link_through_fleet(link.clone(), cx),
        Fate::Hold | Fate::Ignore => {}
    }
}

/// Take a link to the window: held until there is something to decide it with.
pub fn deliver(link: &PullLink, cx: &mut App) {
    Note::info("a pull request link was opened")
        .field("owner", link.owner.as_str())
        .field("name", link.name.as_str())
        .field("number", link.number)
        .emit();
    cx.set_global(Waiting(Some(link.clone())));
    settle(cx);
}

/// Decide the held link if Fleet has said enough. Called when a link arrives and whenever Fleet's answers move.
pub fn settle(cx: &mut App) {
    let Some(link) = cx.try_global::<Waiting>().and_then(|waiting| waiting.0.clone()) else { return };
    let Some((deep_links, repositories)) = crate::operator::screen::link_state(cx) else { return };
    match decide(&link, deep_links, repositories.as_deref()) {
        Fate::Hold => {}
        Fate::Ignore => {
            cx.set_global(Waiting(None));
            Note::info("a pull request link was ignored: features.deepLinks is off").emit();
        }
        Fate::Open(asked) => {
            cx.set_global(Waiting(None));
            crate::operator::screen::open_asked(Some(asked), cx);
        }
        Fate::Unserved => {
            cx.set_global(Waiting(None));
            link_through_fleet(link, cx);
        }
    }
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

    fn served() -> Vec<ipc::RepositorySummary> {
        crate::mock::repositories()
    }

    fn sample() -> PullLink {
        PullLink { owner: "acme-labs".into(), name: "sample-api".into(), number: 41 }
    }

    #[test]
    fn a_link_is_held_until_fleet_has_answered_both_the_switch_and_the_repositories() {
        let served = served();
        assert_eq!(decide(&sample(), None, Some(&served)), Fate::Hold);
        assert_eq!(decide(&sample(), Some(true), None), Fate::Hold);
    }

    #[test]
    fn a_link_is_ignored_while_the_switch_is_off() {
        let served = served();
        assert_eq!(decide(&sample(), Some(false), Some(&served)), Fate::Ignore);
        assert_eq!(decide(&sample(), Some(false), None), Fate::Ignore);
    }

    #[test]
    fn a_link_to_a_served_repository_opens_its_pull_request() {
        let served = served();
        let Fate::Open(asked) = decide(&sample(), Some(true), Some(&served)) else { panic!("it opens") };
        assert_eq!((asked.number, asked.repository.as_str()), (41, "acme-labs/sample-api"));
    }

    #[test]
    fn a_link_to_a_repository_no_manifest_serves_is_unserved() {
        let served = served();
        let link = PullLink { owner: "acme-labs".into(), name: "not-served".into(), number: 9 };
        assert_eq!(decide(&link, Some(true), Some(&served)), Fate::Unserved);
    }

    #[test]
    fn a_link_with_leading_spaces_is_not_armadas_to_take() {
        // `DeepLinks.arrive` tests the prefix before it trims, so a paste is not a link the OS handed over.
        assert_eq!(arrive(" armada://pull/Mele-Labs/armada/2899"), None);
    }
}

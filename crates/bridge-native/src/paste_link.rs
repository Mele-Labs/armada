//! ⌘V on a forge pull request link, anywhere outside a text box, opens that pull request in Reviews, as
//! Classic's `link-routes` did. A paste into a field stays the field's.

/// The review a pasted text asks to open: its whole is one pull request's address and some Manifest is read
/// from its repository. Anything else is left to whoever else takes the paste.
pub fn asked_by(text: &str, repositories: &[ipc::RepositorySummary]) -> Option<ipc::ReviewAsked> {
    let address = up_to_number(text.trim())?;
    let address = address.as_str();
    let (repository, number) = crate::job_view::pulls::pull_address(address)?;
    let manifest = crate::job_view::pulls::manifest_named(repositories, repository)?;
    Some(ipc::ReviewAsked {
        manifest_id: ipc::ManifestId::carried(&manifest),
        repository: repository.to_string(),
        number,
        title: String::new(),
        author: String::new(),
        address: address.to_string(),
    })
}

/// A pull request's address cut after its number, as Classic's `parsePullLink` reads it: the forge's tab
/// (`/changes`, `/files`), a query or an anchor after the number names the same pull request.
fn up_to_number(address: &str) -> Option<String> {
    let address = &address[..address.find(['?', '#']).unwrap_or(address.len())];
    let at = address.find("/pull/")? + "/pull/".len();
    let rest = &address[at..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let after = &rest[digits..];
    (digits > 0 && (after.is_empty() || after.starts_with('/'))).then(|| address[..at + digits].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pull_request_link_of_a_served_repository_is_asked_for() {
        let served = crate::mock::repositories();
        let asked = asked_by("  https://forge.example/acme-labs/sample-api/pull/41\n", &served).expect("a pull request link");
        assert_eq!(asked.number, 41);
        assert_eq!(asked.repository, "acme-labs/sample-api");
        assert_eq!(asked.address, "https://forge.example/acme-labs/sample-api/pull/41");
        assert!(asked.title.is_empty(), "a link carries no title");
    }

    #[test]
    fn a_pull_request_link_past_its_number_opens_that_pull_request() {
        let served = crate::mock::repositories();
        for text in [
            "https://forge.example/acme-labs/sample-api/pull/41/changes",
            "https://forge.example/acme-labs/sample-api/pull/41/files",
            "https://forge.example/acme-labs/sample-api/pull/41/",
            "https://forge.example/acme-labs/sample-api/pull/41?w=1",
            "https://forge.example/acme-labs/sample-api/pull/41#discussion_r7",
            "https://forge.example/acme-labs/sample-api/pull/41/files#diff-3a",
        ] {
            let asked = asked_by(text, &served).unwrap_or_else(|| panic!("{text:?} names pull request 41"));
            assert_eq!(asked.number, 41, "{text:?}");
            assert_eq!(asked.address, "https://forge.example/acme-labs/sample-api/pull/41", "{text:?}");
        }
    }

    #[test]
    fn a_text_that_is_no_pull_request_link_is_left_alone() {
        let served = crate::mock::repositories();
        for text in [
            "",
            "some words",
            "https://forge.example/acme-labs/sample-api/issues/41",
            "https://forge.example/acme-labs/sample-api/pull/41a",
            "https://forge.example/acme-labs/sample-api/pull/",
            "look at https://forge.example/acme-labs/sample-api/pull/41",
        ] {
            assert_eq!(asked_by(text, &served), None, "{text:?}");
        }
    }

    #[test]
    fn a_link_whose_repository_no_manifest_serves_is_left_alone() {
        let served = crate::mock::repositories();
        assert_eq!(asked_by("https://forge.example/acme-labs/not-served/pull/9", &served), None);
    }
}

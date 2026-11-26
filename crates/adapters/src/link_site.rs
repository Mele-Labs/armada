//! What a web link on a Job points at, read from its address: a Claude artifact, a document on a site
//! people write and draw in, or any other page. Bridge lists each under its own heading in the Job's
//! Artifacts tab. The sites are named here because each is a vendor.

/// Where a link points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkSite {
    /// An artifact published from Claude.
    Claude,
    /// A document, design or recording on a site of that name.
    External(&'static str),
    /// Any other page.
    Web,
}

/// The sites a link counts as an external artifact on, by host, and the name each is shown with.
/// A host matches itself and any host under it.
const EXTERNAL: &[(&str, &str)] = &[
    ("drive.google.com", "Google Drive"),
    ("figma.com", "Figma"),
    ("notion.so", "Notion"),
    ("notion.site", "Notion"),
    ("loom.com", "Loom"),
    ("miro.com", "Miro"),
    ("canva.com", "Canva"),
    ("excalidraw.com", "Excalidraw"),
    ("sharepoint.com", "SharePoint"),
    ("onedrive.live.com", "OneDrive"),
    ("dropbox.com", "Dropbox"),
    ("coda.io", "Coda"),
    ("airtable.com", "Airtable"),
    ("lucid.app", "Lucid"),
    ("whimsical.com", "Whimsical"),
    ("pitch.com", "Pitch"),
    ("gist.github.com", "Gist"),
];

/// Where `url` points. An address that is not http or https is any other page.
pub fn link_site(url: &str) -> LinkSite {
    let Some((scheme, rest)) = url.split_once("://") else { return LinkSite::Web };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return LinkSite::Web;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or(authority).split(':').next().unwrap_or_default().to_ascii_lowercase();
    let path = rest[authority.len()..].split(['?', '#']).next().unwrap_or_default();
    let under = |site: &str| host == site || host.ends_with(&format!(".{site}"));
    if under("claude.site") || (under("claude.ai") && path.contains("/artifacts/")) {
        return LinkSite::Claude;
    }
    if host == "docs.google.com" {
        return LinkSite::External(match path.split('/').nth(1).unwrap_or_default() {
            "spreadsheets" => "Google Sheets",
            "presentation" => "Google Slides",
            "forms" => "Google Forms",
            _ => "Google Docs",
        });
    }
    EXTERNAL.iter().find(|(site, _)| under(site)).map_or(LinkSite::Web, |(_, name)| LinkSite::External(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_claude_artifact_link_is_claude() {
        assert_eq!(link_site("https://claude.ai/public/artifacts/3f2a-b1"), LinkSite::Claude);
        assert_eq!(link_site("https://claude.site/artifacts/3f2a"), LinkSite::Claude);
    }

    #[test]
    fn a_claude_page_that_is_no_artifact_is_any_other_page() {
        assert_eq!(link_site("https://claude.ai/settings"), LinkSite::Web);
    }

    #[test]
    fn a_document_site_is_external_by_its_name() {
        assert_eq!(link_site("https://docs.google.com/document/d/1abc/edit"), LinkSite::External("Google Docs"));
        assert_eq!(link_site("https://docs.google.com/spreadsheets/d/1abc"), LinkSite::External("Google Sheets"));
        assert_eq!(link_site("https://www.figma.com/design/abc/Onboarding"), LinkSite::External("Figma"));
        assert_eq!(link_site("https://acme.notion.site/Roadmap-123"), LinkSite::External("Notion"));
    }

    #[test]
    fn any_other_address_is_a_plain_link() {
        assert_eq!(link_site("https://docs.rs/gpui"), LinkSite::Web);
        assert_eq!(link_site("https://notfigma.com/x"), LinkSite::Web);
        assert_eq!(link_site("not a url"), LinkSite::Web);
    }
}

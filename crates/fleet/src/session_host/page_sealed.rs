//! `get_artifact_page`: one kept revision of a Job's page served sealed, one HTML answer a web view opens with no
//! network. **Only the kit's script runs**: the page's own scripts carry no nonce, so the policy refuses them, and
//! the kit draws the page's actions and votes and hands each press and vote to the host by `postMessage`. For a host
//! that draws the kit itself ([`hosted`]) the page's own inline scripts run instead, each allowed by its hash.
//! `crates/ipc/operations/get_artifact_page.toml` is the contract, `docs/practices/protocol/a-pages-sealed-serving.md`
//! the seam. A scan and not a parse, as `document_page` reads the page. **A page may also link the repository's own
//! stylesheets**: Fleet keeps them when it keeps the page (`page_styles`), and the first such `<link>` becomes a
//! `<style>` of their rules, their fonts carried as `data:` addresses, since the sealed page may fetch nothing.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, SealedPage};
use base64::Engine as _;
use ipc::{ArtifactForm, KeptStyles, PagePress, VoteCast};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::archiving::ARTIFACT_NOT_HELD;
use super::document_page::{open_tag, tag_end};
use super::documents::revisions_named;
use super::page_actions::presses_standing;
use super::page_repo_css::css_of;
use super::page_votes::votes_standing;
use super::styling::applying::PAGE_STYLES_KEY;
use crate::daemon::Fleet;

pub(super) const ARTIFACT_REVISION_UNKNOWN: &str = "fleet.artifact_revision_unknown";

/// The kit's script: plain JS, run with the nonce and nothing else.
const KIT_SCRIPT: &str = include_str!("page_kit.js");
/// The look of what the kit's script draws, on the page's tokens.
const KIT_LOOK: &str = include_str!("page_kit.css");
/// Bridge's `armada-kit.css`, which a page links by a relative href, read out of the one place it is written.
const KIT_SOURCE: &str = include_str!("../../../../packages/screens/src/document-kit.ts");
/// The kit's stylesheet, which a page links by this href.
const KIT_HREF: &str = "armada-kit.css";

/// Whether a stylesheet `href` names a file of the repository: not the kit's, and relative.
pub(super) fn repo_href(href: &str) -> bool {
    let href = href.trim();
    !href.is_empty() && href != KIT_HREF && !href.starts_with("//") && relative(href)
}

/// Attributes that name an address a browser fetches or navigates to.
const ADDRESSED: &[&str] = &["src", "href", "srcset", "poster", "action", "formaction", "data", "xlink:href", "background", "ping"];

/// Attributes that give a script an address to load it from.
const SCRIPT_ADDRESSED: &[&str] = &["src", "href", "xlink:href"];

/// The page's policy, its one script source the nonce.
pub(crate) fn policy(nonce: &str) -> String {
    policy_allowing(&format!("'nonce-{nonce}'"))
}

/// The policy with `scripts` as its one script source; every other wall is the same.
fn policy_allowing(scripts: &str) -> String {
    format!(
        "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; script-src {scripts}; connect-src 'none'; form-action 'none'; base-uri 'none'"
    )
}

/// The rules of `armada-kit.css`, as `packages/screens/src/document-kit.ts` writes them.
fn kit_css() -> &'static str {
    let from = KIT_SOURCE.find("KIT_CSS = `").map_or(KIT_SOURCE.len(), |at| at + "KIT_CSS = `".len());
    let to = KIT_SOURCE[from..].find('`').map_or(KIT_SOURCE.len(), |at| from + at);
    &KIT_SOURCE[from..to]
}

/// Every design token at its shipped value, so the kit's rules read here as they read in Bridge. One that names an
/// address is left out.
fn tokens_css() -> String {
    let declared: String = crate::teaching::tokens::all()
        .iter()
        .filter(|token| !token.value.contains("url("))
        .map(|token| format!("{}:{};", token.name, token.value))
        .collect();
    format!(":root{{{declared}}}")
}

/// What the kit's script reads for the revision served: the presses and the newest cast per vote that still stand on it.
#[derive(Serialize)]
struct PageRecord<'a> {
    artifact: &'a str,
    revision: u32,
    presses: Vec<PagePress>,
    votes: Vec<VoteCast>,
}

/// `html` sealed: the policy first in its `<head>`, the tokens and the kit's look after it, `armada-kit.css` inlined
/// where it was linked, the first link to a repository stylesheet replaced by the rules of `repo`, every other link,
/// `<base>` and `http-equiv` dropped, every relative address and every `nonce` the page wrote taken off its tag, and
/// the record and the kit's script, both with `nonce`, closing the body.
pub(crate) fn sealed(html: &str, nonce: &str, record: &str, repo: Option<&KeptStyles>) -> String {
    let page = rewritten(html, None, repo);
    let record = format!("<script type=\"application/json\" id=\"armada-page-record\" nonce=\"{nonce}\">{}</script>", escaped(record));
    closed(&page, &policy(nonce), &format!("{record}<script nonce=\"{nonce}\">{KIT_SCRIPT}</script>"))
}

/// `html` sealed for a host that draws the kit itself, with its policy: as [`sealed`], but each inline script the page
/// wrote is allowed by its hash and the kit's script is left out. **No nonce anywhere**, since a page script that read
/// one could give it to a `<script src>` of its own and fetch. The record is data, so it needs none.
pub(crate) fn hosted(html: &str, record: &str, repo: Option<&KeptStyles>) -> (String, String) {
    let mut hashes = Vec::new();
    let page = rewritten(html, Some(&mut hashes), repo);
    let scripts = match hashes.is_empty() {
        true => "'none'".to_string(),
        false => hashes.iter().map(|hash| format!("'sha256-{hash}'")).collect::<Vec<_>>().join(" "),
    };
    let policy = policy_allowing(&scripts);
    let record = format!("<script type=\"application/json\" id=\"armada-page-record\">{}</script>", escaped(record));
    (closed(&page, &policy, &record), policy)
}

/// `<` written as an escape, so no text in the record closes its script.
fn escaped(record: &str) -> String {
    record.replace('<', "\\u003c")
}

/// The rewritten `page` with `policy`, the tokens and the kit's look first in its `<head>`, and `scripts` closing its body.
fn closed(page: &str, policy: &str, scripts: &str) -> String {
    let lower = page.to_ascii_lowercase();
    let head = format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\"><style>{}\n{KIT_LOOK}</style>", tokens_css());
    let mut sealed = match open_tag(&lower, "head", 0).or_else(|| open_tag(&lower, "html", 0)) {
        Some((tag, after)) if lower[tag..].starts_with("<head") => format!("{}{head}{}", &page[..after], &page[after..]),
        Some((_, after)) => format!("{}<head>{head}</head>{}", &page[..after], &page[after..]),
        None => format!("<head>{head}</head>{page}"),
    };
    match sealed.to_ascii_lowercase().rfind("</body") {
        Some(at) => sealed.insert_str(at, scripts),
        None => sealed.push_str(scripts),
    }
    sealed
}

/// The hash a browser checks an inline script against: its text after the parser turns every CRLF and CR into LF.
fn script_hash(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    base64::engine::general_purpose::STANDARD.encode(Sha256::digest(text.as_bytes()))
}

/// The page with each tag sealed. The text of a script, a style, a title and a textarea is kept as written. With
/// `hashes`, the hash of each script with no address is added to it. With `repo`, the first link to a repository
/// stylesheet is its rules, where it stood.
fn rewritten(html: &str, mut hashes: Option<&mut Vec<String>>, repo: Option<&KeptStyles>) -> String {
    let mut repo_css = repo.map(css_of);
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find('<') {
        let start = at + found;
        out.push_str(&html[at..start]);
        if lower[start..].starts_with("<!--") {
            let end = lower[start + 4..].find("-->").map_or(lower.len(), |close| start + 4 + close + 3);
            out.push_str(&html[start..end]);
            at = end;
            continue;
        }
        let name_end = start + 1 + lower[start + 1..].bytes().take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'-').count();
        if name_end == start + 1 || !lower.as_bytes()[start + 1].is_ascii_alphabetic() {
            out.push('<');
            at = start + 1;
            continue;
        }
        let name = &lower[start + 1..name_end];
        let end = tag_end(&lower, name_end);
        match repo_css.take_if(|_| name == "link" && repo_sheet(&html[start..end], name_end - start)) {
            Some(css) => out.push_str(&format!("<style>{css}</style>")),
            None => out.push_str(&sealed_tag(name, &html[start..end], name_end - start)),
        }
        at = end;
        if ["script", "style", "title", "textarea"].contains(&name) {
            let close = lower[end..].find(&format!("</{name}")).map_or(lower.len(), |close| end + close);
            out.push_str(&html[end..close]);
            // Judged on the tag as written: a relative `src` is gone from what is served, and still never runs.
            let inline = || !attributes_of(&html[start..end], name_end - start).iter().any(|one| SCRIPT_ADDRESSED.contains(&one.name.as_str()));
            if let Some(hashes) = hashes.as_deref_mut().filter(|_| name == "script" && inline()) {
                hashes.push(script_hash(&html[end..close]));
            }
            at = close;
        }
    }
    out.push_str(&html[at..]);
    out
}

/// Whether the opening `tag` links a repository stylesheet.
fn repo_sheet(tag: &str, name_len: usize) -> bool {
    let attributes = attributes_of(tag, name_len);
    let said = |wanted: &str| attributes.iter().find(|one| one.name == wanted).map(|one| one.value.as_str());
    said("rel").is_some_and(|rel| rel.split_whitespace().any(|one| one.eq_ignore_ascii_case("stylesheet"))) && said("href").is_some_and(repo_href)
}

/// One opening tag, sealed. `name` is in lower case and `name_len` is where the tag's attributes start.
fn sealed_tag(name: &str, tag: &str, name_len: usize) -> String {
    let attributes = attributes_of(tag, name_len);
    let said = |wanted: &str| attributes.iter().find(|one| one.name == wanted).map(|one| one.value.trim().to_string());
    match name {
        "link" => {
            let stylesheet = said("rel").is_some_and(|rel| rel.split_whitespace().any(|one| one.eq_ignore_ascii_case("stylesheet")));
            match stylesheet && said("href").as_deref() == Some(KIT_HREF) {
                true => format!("<style>{}</style>", kit_css()),
                false => String::new(),
            }
        }
        "base" => String::new(),
        "meta" if said("http-equiv").is_some() => String::new(),
        _ => {
            let dropped = |one: &Attribute| one.name == "nonce" || (ADDRESSED.contains(&one.name.as_str()) && relative(&one.value));
            if !attributes.iter().any(dropped) {
                return tag.to_string();
            }
            let mut kept = tag[..name_len].to_string();
            let mut from = name_len;
            for one in &attributes {
                kept.push_str(&tag[from..one.start]);
                if !dropped(one) {
                    kept.push_str(&tag[one.start..one.end]);
                }
                from = one.end;
            }
            kept.push_str(&tag[from..]);
            kept
        }
    }
}

/// An address read against the page's own, which here is Fleet's: no scheme, and not a fragment.
fn relative(value: &str) -> bool {
    let value = value.trim();
    if value.starts_with('#') {
        return false;
    }
    let scheme = value.split(':').next().unwrap_or_default();
    let schemed = value.contains(':')
        && scheme.chars().next().is_some_and(|first| first.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    !schemed
}

/// One attribute of a tag: its name in lower case, its value as written, and where it sits, from the space before it.
struct Attribute {
    name: String,
    value: String,
    start: usize,
    end: usize,
}

/// Every attribute of the opening tag `tag`, read from `from`.
fn attributes_of(tag: &str, from: usize) -> Vec<Attribute> {
    let bytes = tag.as_bytes();
    let mut found = Vec::new();
    let mut at = from;
    loop {
        let start = at;
        while at < bytes.len() && (bytes[at].is_ascii_whitespace() || bytes[at] == b'/') {
            at += 1;
        }
        if at >= bytes.len() || bytes[at] == b'>' {
            return found;
        }
        let name_from = at;
        while at < bytes.len() && !bytes[at].is_ascii_whitespace() && !matches!(bytes[at], b'=' | b'>' | b'/') {
            at += 1;
        }
        let name = tag[name_from..at].to_ascii_lowercase();
        let mut look = at;
        while look < bytes.len() && bytes[look].is_ascii_whitespace() {
            look += 1;
        }
        let mut value = String::new();
        if bytes.get(look) == Some(&b'=') {
            at = look + 1;
            while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                at += 1;
            }
            match bytes.get(at) {
                Some(&quote @ (b'"' | b'\'')) => {
                    let close = tag[at + 1..].find(quote as char).map_or(bytes.len(), |close| at + 1 + close);
                    value = tag[at + 1..close].to_string();
                    at = (close + 1).min(bytes.len());
                }
                _ => {
                    let value_from = at;
                    while at < bytes.len() && !bytes[at].is_ascii_whitespace() && bytes[at] != b'>' {
                        at += 1;
                    }
                    value = tag[value_from..at].to_string();
                }
            }
        }
        found.push(Attribute { name, value, start, end: at });
    }
}

/// A fresh nonce for one answer.
fn nonce() -> String {
    let bytes: [u8; 16] = rand::random();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_artifact_page`: revision `revision` of the page, the newest where none is named, sealed, with what he
    /// pressed and voted on that revision. **Archived or not**: reading changes nothing.
    async fn page_revision(&self, id: &str, artifact: &str, revision: Option<u32>) -> Result<(ipc::JobArtifact, u32, String), Refusal> {
        let id = id.trim();
        let held = self.artifact_held(id, artifact).await?;
        if held.form != ArtifactForm::Page || held.id.starts_with(super::sketching::TARGET_PREFIX) {
            let form = match held.form {
                _ if held.id.starts_with(super::sketching::TARGET_PREFIX) => "an artifact of parts",
                ArtifactForm::Sketch => "a sketch",
                _ => "a draft",
            };
            return Err(self.hosted_refusal(ARTIFACT_NOT_HELD, &format!("`{}` is {form}, and only a page is served", held.title)));
        }
        let newest = held.revisions.last().map_or(1, |one| one.number);
        let number = revision.unwrap_or(newest);
        let unknown = || self.hosted_refusal(ARTIFACT_REVISION_UNKNOWN, &format!("revision {number} of `{}` is not kept. Kept: 1 to {newest}", held.title));
        let (_, page) = self.document_held(id, &held.id).await?;
        let file = page.map(|page| revisions_named(&page.detail)).and_then(|kept| kept.get((number as usize).checked_sub(1)?).cloned());
        let Some(file) = file else { return Err(unknown()) };
        let html = tokio::task::spawn_blocking(move || std::fs::read_to_string(file)).await.ok().and_then(Result::ok).ok_or_else(unknown)?;
        Ok((held, number, html))
    }

    /// `get_artifact_page_source`: revision `revision` of the page exactly as the Job wrote it, the newest where none is
    /// named, so a client that edits its words keeps the markup and `save_artifact_edit` can check them against it.
    pub(crate) async fn artifact_page_source(&self, id: &str, artifact: &str, revision: Option<u32>) -> Result<String, Refusal> {
        Ok(self.page_revision(id.trim(), artifact, revision).await?.2)
    }

    pub(crate) async fn artifact_page(&self, id: &str, artifact: &str, revision: Option<u32>) -> Result<SealedPage, Refusal> {
        self.artifact_page_as(id, artifact, revision, false).await
    }

    /// `get_artifact_page` with `host_kit`: the page for a host that draws the kit itself ([`hosted`]).
    pub(crate) async fn hosted_artifact_page(&self, id: &str, artifact: &str, revision: Option<u32>) -> Result<SealedPage, Refusal> {
        self.artifact_page_as(id, artifact, revision, true).await
    }

    /// The style set of the repository stylesheets the page links, where Fleet kept it with the page. **The row's
    /// `kept_styles` is the newest revision's, so only the newest is drawn with it**: an older one may have linked others.
    async fn repo_styles(&self, id: &str, held: &ipc::JobArtifact, number: u32) -> Option<KeptStyles> {
        if held.revisions.last().map_or(1, |one| one.number) != number {
            return None;
        }
        let (_, page) = self.document_held(id, &held.id).await.ok()?;
        let kept: std::collections::BTreeMap<String, String> = ipc::decode("kept styles", page?.detail.get("kept_styles")?.as_bytes()).ok()?;
        self.kept_styles_read(kept.get(PAGE_STYLES_KEY)?).await.ok()
    }

    async fn artifact_page_as(&self, id: &str, artifact: &str, revision: Option<u32>, host_kit: bool) -> Result<SealedPage, Refusal> {
        let id = id.trim();
        let (held, number, html) = self.page_revision(id, artifact, revision).await?;
        let repo = self.repo_styles(id, &held, number).await;
        let (presses, votes) = {
            let store = self.store().lock().await;
            (store.page_presses(id).map_err(|why| self.ledger_fault(why))?, store.page_votes(id).map_err(|why| self.ledger_fault(why))?)
        };
        let record = PageRecord {
            artifact: &held.id,
            revision: number,
            presses: presses_standing(&presses, &held.id, &html, number),
            votes: votes_standing(&votes, &held.id, &html, number),
        };
        // Strings, numbers and bools, which always encode.
        let record = ipc::encode(&record).unwrap_or_default();
        if host_kit {
            let (html, policy) = hosted(&html, &record, repo.as_ref());
            return Ok(SealedPage { revision: number, html, policy });
        }
        let nonce = nonce();
        Ok(SealedPage { revision: number, html: sealed(&html, &nonce, &record, repo.as_ref()), policy: policy(&nonce) })
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use sha2::Digest as _;

    use super::{hosted, sealed, KIT_SCRIPT};

    /// The page the native Bridge's mock Fleet serves, before it is sealed: a message, a start, an answer and a refused
    /// answer, a pick and a score.
    const SOURCE: &str = include_str!("../../../bridge-native/fixtures/page-actions-source.html");
    /// The pages the native mock reads as Fleet's answer to `get_artifact_page` with `host_kit`, each from its source and
    /// with the record the mock swaps: the page above, its sketches page, and a page whose own script tries to press.
    const NATIVE: [(&str, &str, &str); 3] = [
        (SOURCE, "page-actions.html", r#"{"artifact":"design","revision":2,"presses":[],"votes":[]}"#),
        (include_str!("../../../bridge-native/fixtures/page-sketches-source.html"), "page-sketches.html", r#"{"artifact":"marks","revision":1,"presses":[],"votes":[]}"#),
        (include_str!("../../../bridge-native/fixtures/page-scripts-source.html"), "page-scripts.html", r#"{"artifact":"design","revision":2,"presses":[],"votes":[]}"#),
    ];

    /// A page with no style of its own reads in the app's sans and its form controls take the accent, never the
    /// browser's serif and grey radios.
    #[test]
    fn a_bare_page_reads_in_the_apps_sans_with_calm_form_controls() {
        let page = sealed("<html><head></head><body><form><fieldset><legend>Pick</legend><label><input type=radio> A</label></fieldset></form></body></html>", "bm9uY2U=", "{}", None);
        assert!(page.contains(":where(body) { color: var(--fg-muted); font-family: var(--font-sans);"), "{page}");
        assert!(page.contains("--font-sans:"), "the token the body names is on the page");
        assert!(page.contains(":where(input, button, select, textarea) { font: inherit; accent-color: var(--accent); }"), "{page}");
        assert!(page.contains(":where(fieldset)"), "{page}");
    }

    /// A browser hashes a script's text after turning CRLF into LF, so a page written on Windows still runs.
    #[test]
    fn a_hosted_scripts_hash_is_of_its_text_as_the_browser_reads_it() {
        let (page, policy) = hosted("<body><script>a();\r\nb();</script><script type=module>\rc();</script></body>", "{}", None);
        let hash = |text: &[u8]| base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(text));
        assert!(policy.contains(&format!("script-src 'sha256-{}' 'sha256-{}';", hash(b"a();\nb();"), hash(b"\nc();"))), "{policy}");
        assert!(!page.contains("nonce"), "{page}");
        let (_, none) = hosted("<body><p>still</p></body>", "{}", None);
        assert!(none.contains("script-src 'none';"), "{none}");
    }

    /// A page that links the repository's sheets gets the first link's place as one `<style>` of their rules, each font as an
    /// `@font-face` with its bytes in a `data:` address, and the policy lets a font load from nowhere else.
    #[test]
    fn a_repository_stylesheet_link_becomes_its_kept_rules_with_fonts_as_data() {
        let font = base64::engine::general_purpose::STANDARD.encode(b"wOF2\0\0\0\0");
        let kept = ipc::KeptStyles {
            sheets: vec![ipc::KeptSheet { path: "src/app.css".into(), css: ".card{color:red}</style><p>".into(), edited: false }],
            fonts: vec![ipc::KeptFont { family: "Inter".into(), weight: "400".into(), style: None, bytes: font.clone() }],
        };
        let page = "<head><link rel=stylesheet href=\"armada-kit.css\"><link rel=\"stylesheet\" href=\"src/app.css\"><link rel=stylesheet href=src/b.css></head><body></body>";
        let (html, policy) = hosted(page, "{}", Some(&kept));
        assert!(policy.contains("img-src data:; font-src data:; script-src"), "{policy}");
        assert!(!html.contains("<link") && !html.contains("src/app.css"), "{html}");
        let face = format!("@font-face{{font-family:\"Inter\";font-weight:400;src:url(data:font/woff2;base64,{font}) format(\"woff2\");}}.card{{color:red}}<\\/style><p>");
        assert!(html.contains(&format!("<style>{face}</style>")), "{html}");
        assert!(html.find("li::marker").unwrap() < html.find("@font-face").unwrap(), "after the kit's rules");
        let (bare, _) = hosted(page, "{}", None);
        assert!(!bare.contains(".card{color:red}") && !bare.contains("<link"), "with none kept, every link is dropped");
    }

    /// **The native mock's pages are sealed by this code, never written by hand**, as the native Bridge asks for them
    /// (`host_kit`), so a native walk runs the page's scripts under the policy Fleet ships and draws the kit itself.
    /// `ARMADA_WRITE_NATIVE_PAGE=1` writes them again; without it, each must be today's.
    #[test]
    fn the_native_mock_page_is_sealed_as_fleet_seals_it() {
        for (source, name, record) in NATIVE {
            let path = format!("{}/../bridge-native/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
            let (page, _) = hosted(source, record, None);
            if std::env::var_os("ARMADA_WRITE_NATIVE_PAGE").is_some() {
                std::fs::write(&path, &page).expect("the native mock's page is written");
            }
            let kept = std::fs::read_to_string(&path).expect("the native mock's page is there");
            assert_eq!(kept, page, "{name} is not sealed as Fleet seals it today: write it again with ARMADA_WRITE_NATIVE_PAGE=1");
            assert!(!kept.contains(KIT_SCRIPT) && kept.contains(record), "{name}: the host draws the kit, and the mock finds the record it swaps");
        }
    }
}

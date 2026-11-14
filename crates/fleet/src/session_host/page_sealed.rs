//! `get_artifact_page`: one kept revision of a Job's page served sealed, one HTML answer a web view opens with no
//! network. **Only the kit's script runs**: the page's own scripts carry no nonce, so the policy refuses them, and
//! the kit draws the page's actions and votes and hands each press and vote to the host by `postMessage`.
//! `crates/ipc/operations/get_artifact_page.toml` is the contract, `docs/practices/protocol/a-pages-sealed-serving.md`
//! the seam. A scan and not a parse, as `document_page` reads the page.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, SealedPage};
use base64::Engine as _;
use ipc::{ArtifactForm, PagePress, VoteCast};
use serde::Serialize;

use super::archiving::ARTIFACT_NOT_HELD;
use super::document_page::{open_tag, tag_end};
use super::documents::revisions_named;
use super::page_actions::presses_of;
use super::page_votes::votes_of;
use crate::daemon::Fleet;

pub(super) const ARTIFACT_REVISION_UNKNOWN: &str = "fleet.artifact_revision_unknown";

/// The kit's script: plain JS, run with the nonce and nothing else.
const KIT_SCRIPT: &str = include_str!("page_kit.js");
/// The look of what the kit's script draws, on the page's tokens.
const KIT_LOOK: &str = include_str!("page_kit.css");
/// Bridge's `armada-kit.css`, which a page links by a relative href, read out of the one place it is written.
const KIT_SOURCE: &str = include_str!("../../../../packages/screens/src/document-kit.ts");
/// The one stylesheet a page may link.
const KIT_HREF: &str = "armada-kit.css";

/// Attributes that name an address a browser fetches or navigates to.
const ADDRESSED: &[&str] = &["src", "href", "srcset", "poster", "action", "formaction", "data", "xlink:href", "background", "ping"];

/// The page's policy, its one script source the nonce.
pub(crate) fn policy(nonce: &str) -> String {
    format!(
        "default-src 'none'; style-src 'unsafe-inline'; img-src data:; script-src 'nonce-{nonce}'; connect-src 'none'; form-action 'none'; base-uri 'none'"
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

/// What the kit's script reads for the revision served: the presses and the newest cast per vote made on it.
#[derive(Serialize)]
struct PageRecord<'a> {
    artifact: &'a str,
    revision: u32,
    presses: Vec<PagePress>,
    votes: Vec<VoteCast>,
}

/// `html` sealed: the policy first in its `<head>`, the tokens and the kit's look after it, `armada-kit.css` inlined
/// where it was linked, every other link, `<base>` and `http-equiv` dropped, every relative address and every
/// `nonce` the page wrote taken off its tag, and the record and the kit's script, both with `nonce`, closing the body.
pub(crate) fn sealed(html: &str, nonce: &str, record: &str) -> String {
    let page = rewritten(html);
    let lower = page.to_ascii_lowercase();
    let head = format!(
        "<meta http-equiv=\"Content-Security-Policy\" content=\"{}\"><style>{}\n{KIT_LOOK}</style>",
        policy(nonce),
        tokens_css()
    );
    let mut sealed = match open_tag(&lower, "head", 0).or_else(|| open_tag(&lower, "html", 0)) {
        Some((tag, after)) if lower[tag..].starts_with("<head") => format!("{}{head}{}", &page[..after], &page[after..]),
        Some((_, after)) => format!("{}<head>{head}</head>{}", &page[..after], &page[after..]),
        None => format!("<head>{head}</head>{page}"),
    };
    // `<` written as an escape, so no text in the record closes its script.
    let record = record.replace('<', "\\u003c");
    let scripts = format!(
        "<script type=\"application/json\" id=\"armada-page-record\" nonce=\"{nonce}\">{record}</script><script nonce=\"{nonce}\">{KIT_SCRIPT}</script>"
    );
    match sealed.to_ascii_lowercase().rfind("</body") {
        Some(at) => sealed.insert_str(at, &scripts),
        None => sealed.push_str(&scripts),
    }
    sealed
}

/// The page with each tag sealed. The text of a script, a style, a title and a textarea is kept as written.
fn rewritten(html: &str) -> String {
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
        out.push_str(&sealed_tag(name, &html[start..end], name_end - start));
        at = end;
        if ["script", "style", "title", "textarea"].contains(&name) {
            let close = lower[end..].find(&format!("</{name}")).map_or(lower.len(), |close| end + close);
            out.push_str(&html[end..close]);
            at = close;
        }
    }
    out.push_str(&html[at..]);
    out
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
    pub(crate) async fn artifact_page(&self, id: &str, artifact: &str, revision: Option<u32>) -> Result<SealedPage, Refusal> {
        let id = id.trim();
        let held = self.artifact_held(id, artifact).await?;
        if held.form != ArtifactForm::Page {
            let form = match held.form {
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
        let (presses, votes) = {
            let store = self.store().lock().await;
            (store.page_presses(id).map_err(|why| self.ledger_fault(why))?, store.page_votes(id).map_err(|why| self.ledger_fault(why))?)
        };
        let votes: Vec<_> = votes.into_iter().filter(|one| one.revision == number).collect();
        let record = PageRecord {
            artifact: &held.id,
            revision: number,
            presses: presses_of(&presses, &held.id).into_iter().filter(|one| one.revision == number).collect(),
            votes: votes_of(&votes, &held.id),
        };
        // Strings, numbers and bools, which always encode.
        let record = ipc::encode(&record).unwrap_or_default();
        let nonce = nonce();
        Ok(SealedPage { revision: number, html: sealed(&html, &nonce, &record), policy: policy(&nonce) })
    }
}

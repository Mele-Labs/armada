//! The Job under the choice: where its checkout is, and what it has changed, read and planned,
//! which is where the walk starts from. One function takes a Job and a `Sketch` and answers the
//! sketch with each `mock` kept; a page's mocks come here too.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{MockNode, SessionRow, Sketch, StyleSources};

use crate::session_host::document_page::sketches_in;

use super::choosing::Styled;
use super::choosing::{relative_to, style_mocks};
use super::walk::sources_near;
use crate::daemon::Fleet;
use crate::session_host::sketching::SKETCH_REFUSED;

const JOB_UNKNOWN: &str = "fleet.style_job_unknown";
const STYLES_UNFINISHED: &str = "fleet.sketch_styles_unfinished";

/// Most files read or changed that the walk is started from.
const FILES_AT_MOST: usize = 40;

/// The Job's checkout and the files the walk starts from, nearest to its work first.
pub(crate) struct Look {
    pub checkout: String,
    /// Changed, then read, then planned. The walk starts from the first with a file in it.
    pub tiers: Vec<Vec<String>>,
    pub attachments: String,
}

/// `get_style_sources` for a Job already read.
pub(super) fn sources_for(look: &Look, named: Option<&str>) -> Result<StyleSources, String> {
    let Ok(root) = std::fs::canonicalize(&look.checkout) else {
        return Ok(StyleSources { sources: Vec::new(), start: named.map(str::to_string), packages: Vec::new() });
    };
    let start = match named {
        None => None,
        Some(path) => Some(relative_to(&root, path).ok_or_else(|| {
            format!("`path` {path} is outside this job's checkout. Name a path from the repository's root, such as src/components/Card.tsx")
        })?),
    };
    let mut found = sources_near(&root, start.as_deref(), &look.tiers);
    found.start = named.map(str::to_string);
    Ok(found)
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
    /// Where the Job works and what the walk starts from. **Reuses what Fleet already keeps**: the
    /// files it wrote (`session_touches`), the calls on its thread, and the paths of its plan.
    pub(crate) async fn job_look(&self, id: &str) -> Result<Look, Refusal> {
        let (cwd, touches, plan) = {
            let store = self.store().lock().await;
            let session = store
                .session(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(JOB_UNKNOWN, "a job Fleet knows"))?;
            let touches = store.session_touches(id).map_err(|why| self.ledger_fault(why))?;
            let plan = store.session_plan(id).map_err(|why| self.ledger_fault(why))?;
            (session.cwd, touches, plan)
        };
        let checkout = self.worktree_of(&cwd);
        let inside = |path: &str| within(&checkout, path);
        let mut changed: Vec<String> = Vec::new();
        for touch in touches.iter().rev().filter(|touch| touch.act != "remove") {
            if let Some(path) = inside(&touch.path).filter(|path| !changed.contains(path)) {
                changed.push(path);
            }
        }
        let mut read: Vec<String> = Vec::new();
        for row in self.rows_of(id).await?.iter().rev() {
            let SessionRow::Tool { text, .. } = row else { continue };
            let Some(path) = text.strip_prefix("Read ").and_then(|rest| rest.split_whitespace().next()) else { continue };
            if let Some(path) = inside(path).filter(|path| !read.contains(path)) {
                read.push(path);
            }
        }
        let planned: Vec<String> = plan
            .iter()
            .flat_map(|plan| plan.groups.iter())
            .flat_map(|group| group.paths.iter())
            .filter_map(|path| inside(path.trim_end_matches('*').trim_end_matches('/')))
            .collect();
        let tiers = [changed, read, planned].map(|mut tier| {
            tier.truncate(FILES_AT_MOST);
            tier
        });
        Ok(Look { checkout, tiers: tiers.into(), attachments: self.host().attachments_dir.clone() })
    }

    /// Each of `mocks` drawn for the Job: the hash of its style set, or why it cannot be drawn.
    pub(crate) async fn mocks_styled(&self, id: &str, mocks: Vec<MockNode>) -> Result<Styled, Refusal> {
        if mocks.is_empty() {
            return Ok(Styled::default());
        }
        let look = self.job_look(id).await?;
        tokio::task::spawn_blocking(move || style_mocks(&look.checkout, &look.tiers, &look.attachments, &mocks))
            .await
            .map_err(|why| self.hosted_fault(STYLES_UNFINISHED, &format!("keeping the styles did not finish: {why}")))
    }

    /// `sketch` with `kept` on each `mock`, or the refusal that tells the agent what to change.
    /// `tool` is the call it is asked to make again.
    pub(crate) async fn sketch_styled(&self, id: &str, sketch: Sketch, tool: &str) -> Result<Sketch, Refusal> {
        let mocks = sketch.mocks();
        if mocks.is_empty() {
            return Ok(sketch);
        }
        let styled = self.mocks_styled(id, mocks).await?;
        if !styled.refused.is_empty() {
            let said: Vec<&str> = styled.refused.values().map(String::as_str).collect();
            return Err(self.hosted_refusal(
                SKETCH_REFUSED,
                &format!("the sketch was not drawn: {}. Fix these and call {tool} again", said.join("; ")),
            ));
        }
        sketch.with_kept(&styled.kept).map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))
    }
}

/// The style sets a page's mocks were drawn with, and the mocks that cannot be.
#[derive(Default)]
pub(crate) struct PageStyles {
    /// `{"<sketch title>#<node id>": "<hash>"}` as text, absent where the page keeps none.
    pub kept: Option<String>,
    /// One sentence for each sketch with a mock that cannot be drawn.
    pub refused: Vec<String>,
}

impl PageStyles {
    /// The notice for a page: what `sketches_that_will_not_draw` said, with the mocks that cannot be drawn.
    pub(crate) fn told(&self, scenes: Option<String>, path: &str) -> Option<String> {
        match (scenes, self.refused.is_empty()) {
            (told, true) => told,
            (Some(told), false) => Some(format!("{told} {}", self.refused.join(" "))),
            (None, false) => Some(format!("In the page {path}, {}", self.refused.join(" "))),
        }
    }
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
    /// Keep the style set of each `mock` of each sketch the page embeds. **The page is not
    /// rewritten**, because `keep` compares its bytes with the newest revision, so the hashes are
    /// returned to ride on the document's attachment.
    pub(crate) async fn page_styles(&self, id: &str, html: &str) -> PageStyles {
        let sketches = sketches_in(html);
        let total = sketches.len();
        let mut styles = PageStyles::default();
        let mut kept = BTreeMap::new();
        for (at, sketch) in sketches.iter().enumerate() {
            let mocks = ipc::page_mocks(&sketch.scene);
            if mocks.is_empty() {
                continue;
            }
            let Ok(styled) = self.mocks_styled(id, mocks).await else { continue };
            let title = sketch.title.clone().unwrap_or_default();
            kept.extend(styled.kept.into_iter().map(|(node, hash)| (format!("{title}#{node}"), hash)));
            if !styled.refused.is_empty() {
                let named = sketch.title.as_ref().map(|title| format!(" (\"{title}\")")).unwrap_or_default();
                let said: Vec<&str> = styled.refused.values().map(String::as_str).collect();
                styles.refused.push(format!("sketch {} of {total}{named} will not draw: {}.", at + 1, said.join("; ")));
            }
        }
        styles.kept = (!kept.is_empty()).then(|| ipc::encode(&kept).ok()).flatten();
        styles
    }
}

/// `path` as the checkout names it: relative as it came, or absolute inside the checkout.
fn within(checkout: &str, path: &str) -> Option<String> {
    let path = path.trim();
    let rest = match path.strip_prefix(&format!("{}/", checkout.trim_end_matches('/'))) {
        Some(rest) => rest,
        None if path.starts_with('/') || path.starts_with('~') => return None,
        None => path,
    };
    let rest = rest.trim_start_matches("./");
    (!rest.is_empty() && !rest.split('/').any(|part| part == "..")).then(|| rest.to_string())
}

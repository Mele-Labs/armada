//! `get_design_catalogue`: the tokens, components, glyphs and patterns a `mock` is drawn with, read
//! once from a checkout and kept until a file it was read from changes. The modules are which files
//! it reads and their fingerprint (`inputs`), tokens by role and theme (`themed`), components and
//! their markup (`components`), and the patterns a repository keeps (`patterns`).
//! `docs/concepts/session/drawing-a-sketch.md`, *The design catalogue*.

pub(crate) mod components;
pub(crate) mod inputs;
pub(crate) mod patterns;
pub(crate) mod themed;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    CatalogueArea, CatalogueAsked, CatalogueComponent, CatalogueContents, CataloguePattern, DesignCatalogue, SessionId, TokenFamily, TokenGroup,
    MOCK_GLYPHS,
};
use serde::Serialize;

use crate::daemon::Fleet;

const CATALOGUE_UNPLACED: &str = "fleet.catalogue_unplaced";
const CATALOGUE_AREA_UNKNOWN: &str = "fleet.catalogue_area_unknown";
const CATALOGUE_UNFINISHED: &str = "fleet.catalogue_unfinished";

/// Most bytes one answer weighs. A sketch is at most 32 KB, so its catalogue is smaller.
pub(crate) const ANSWER_AT_MOST: usize = 24 * 1024;
/// Most checkouts whose catalogue is kept at once.
const KEPT_AT_MOST: usize = 32;

/// Everything read from a checkout, before a call cuts it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Whole {
    pub themes: Vec<String>,
    pub sheets: Vec<String>,
    pub tokens: Vec<TokenGroup>,
    pub components: Vec<CatalogueComponent>,
    pub patterns: Vec<CataloguePattern>,
}

/// Read every input of the checkout at `root`.
pub(crate) fn build(root: &Path, files: &inputs::Inputs) -> Whole {
    let read = themed::read(root, &files.sheets);
    Whole {
        themes: read.themes,
        sheets: read.sheets,
        tokens: read.groups,
        components: components::read(root, &files.components, &files.sheets),
        patterns: files.patterns.as_deref().map(|path| patterns::read(&root.join(path))).unwrap_or_default(),
    }
}

struct Kept {
    fingerprint: String,
    whole: Arc<Whole>,
    /// When it was last asked for, to give back the least recent past [`KEPT_AT_MOST`].
    asked: u64,
}

#[derive(Default)]
struct Shelf {
    kept: HashMap<PathBuf, Kept>,
    clock: u64,
}

fn shelf() -> &'static Mutex<Shelf> {
    static SHELF: OnceLock<Mutex<Shelf>> = OnceLock::new();
    SHELF.get_or_init(|| Mutex::new(Shelf::default()))
}

/// The catalogue of the checkout at `root`, a canonical path. **Read again only when the
/// fingerprint of its inputs moved**: listing them and reading their sizes and times is the cost of
/// a call that finds it unchanged.
pub(crate) fn catalogue_of(root: &Path) -> Arc<Whole> {
    let files = inputs::listed(root);
    let fingerprint = inputs::fingerprint(root, &files);
    {
        let mut shelf = shelf().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        shelf.clock += 1;
        let now = shelf.clock;
        if let Some(kept) = shelf.kept.get_mut(root).filter(|kept| kept.fingerprint == fingerprint) {
            kept.asked = now;
            return kept.whole.clone();
        }
    }
    let whole = Arc::new(build(root, &files));
    let mut shelf = shelf().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    shelf.clock += 1;
    let asked = shelf.clock;
    shelf.kept.insert(root.to_path_buf(), Kept { fingerprint, whole: whole.clone(), asked });
    while shelf.kept.len() > KEPT_AT_MOST {
        let Some(oldest) = shelf.kept.iter().min_by_key(|(_, kept)| kept.asked).map(|(path, _)| path.clone()) else { break };
        shelf.kept.remove(&oldest);
    }
    whole
}

/// `whole` cut to `area`, narrowed by `matching`, and kept under [`ANSWER_AT_MOST`].
pub(crate) fn answer(whole: &Whole, area: CatalogueArea, matching: Option<&str>) -> DesignCatalogue {
    let needle = matching.map(|one| one.trim().to_lowercase()).filter(|one| !one.is_empty());
    let hits = |words: &[&str]| needle.as_ref().is_none_or(|needle| words.iter().any(|word| word.to_lowercase().contains(needle.as_str())));
    let mut answered = DesignCatalogue { area, themes: whole.themes.clone(), sheets: whole.sheets.clone(), ..DesignCatalogue::default() };
    let left = ANSWER_AT_MOST.saturating_sub(weight(&answered));
    let mut cut = None;
    match area {
        CatalogueArea::Contents => {
            let mut names: Vec<String> = Vec::new();
            for one in whole.components.iter().filter(|one| hits(&[&one.name, &one.file])) {
                if !names.contains(&one.name) {
                    names.push(one.name.clone());
                }
            }
            let tokens = whole
                .tokens
                .iter()
                .filter(|group| hits(&[&group.family]) || group.tokens.iter().any(|token| hits(&[&token.name])))
                .map(|group| TokenFamily { role: group.role, family: group.family.clone(), tokens: group.tokens.len() })
                .collect();
            let patterns = whole.patterns.iter().filter(|one| hits(&[&one.name])).map(|one| one.name.clone()).collect();
            let mut contents = CatalogueContents { tokens, components: Vec::new(), patterns, icons: MOCK_GLYPHS.len() };
            let room = left.saturating_sub(weight(&contents));
            let (kept, dropped) = fitted(names, room);
            contents.components = kept;
            if dropped > 0 {
                cut = Some(format!("{dropped} component names left out to stay under {} KB. Call again with match naming a folder or a name", ANSWER_AT_MOST / 1024));
            }
            answered.contents = Some(contents);
        }
        CatalogueArea::Tokens => {
            let groups: Vec<TokenGroup> = whole
                .tokens
                .iter()
                .filter_map(|group| {
                    let family = hits(&[&group.family, role_word(group)]);
                    let tokens: Vec<_> = group.tokens.iter().filter(|token| family || hits(&[&token.name])).cloned().collect();
                    (!tokens.is_empty()).then(|| TokenGroup { tokens, ..group.clone() })
                })
                .collect();
            let (kept, dropped) = fitted(groups, left);
            answered.tokens = kept;
            if dropped > 0 {
                cut = Some(format!(
                    "{dropped} token groups left out to stay under {} KB. Call again with match naming a family such as --space, or a role such as colour",
                    ANSWER_AT_MOST / 1024
                ));
            }
        }
        CatalogueArea::Components => {
            let found: Vec<CatalogueComponent> = whole.components.iter().filter(|one| hits(&[&one.name, &one.file])).cloned().collect();
            let (kept, dropped) = fitted(found, left);
            answered.components = kept;
            if dropped > 0 {
                cut = Some(format!(
                    "{dropped} components left out to stay under {} KB. Call again with match naming one, or the folder it is in",
                    ANSWER_AT_MOST / 1024
                ));
            }
        }
        CatalogueArea::Icons => {
            answered.icons = MOCK_GLYPHS.iter().filter(|name| hits(&[name])).map(|name| name.to_string()).collect();
        }
        CatalogueArea::Patterns => {
            let found: Vec<CataloguePattern> =
                whole.patterns.iter().filter(|one| hits(&[&one.name, one.note.as_deref().unwrap_or_default()])).cloned().collect();
            let (kept, dropped) = fitted(found, left);
            answered.patterns = kept;
            if dropped > 0 {
                cut = Some(format!("{dropped} patterns left out to stay under {} KB. Call again with match naming one", ANSWER_AT_MOST / 1024));
            }
        }
    }
    answered.cut = cut;
    answered
}

fn role_word(group: &TokenGroup) -> &'static str {
    match group.role {
        ipc::TokenRole::Colour => "colour color",
        ipc::TokenRole::Space => "space",
        ipc::TokenRole::Type => "type",
        ipc::TokenRole::Radius => "radius",
        ipc::TokenRole::Shadow => "shadow",
        ipc::TokenRole::Motion => "motion",
        ipc::TokenRole::Other => "other",
    }
}

fn weight<T: Serialize>(value: &T) -> usize {
    ipc::encode(value).map_or(0, |text| text.len())
}

/// The first of `items` that fit in `room` bytes, and how many did not.
fn fitted<T: Serialize>(items: Vec<T>, room: usize) -> (Vec<T>, usize) {
    let total = items.len();
    let mut used = 0;
    let mut kept = Vec::new();
    for item in items {
        let one = weight(&item) + 1;
        if used + one > room {
            break;
        }
        used += one;
        kept.push(item);
    }
    let dropped = total - kept.len();
    (kept, dropped)
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
    /// The catalogue of the Job's checkout, or of the served repository `asked.repository` names.
    pub(crate) async fn design_catalogue_read(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<SessionId>,
        asked: CatalogueAsked,
    ) -> Result<DesignCatalogue, Refusal> {
        let area = CatalogueArea::named(asked.area.as_deref().unwrap_or_default()).ok_or_else(|| {
            self.hosted_refusal(
                CATALOGUE_AREA_UNKNOWN,
                &format!(
                    "`area` {} is not one Fleet keeps. Name contents, tokens, components, icons or patterns, or leave it out for contents",
                    asked.area.as_deref().unwrap_or_default()
                ),
            )
        })?;
        let checkout = match asked.repository.as_deref() {
            Some(repository) => self.run_repository(repository)?,
            None => {
                let id = caller
                    .and_then(|caller| self.session_holding(&caller))
                    .or_else(|| session_id.map(|named| named.as_str().trim().to_string()))
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| {
                        self.hosted_refusal(CATALOGUE_UNPLACED, "the design catalogue is a repository's, and this call names no job and no repository")
                    })?;
                self.job_look(&id).await?.checkout
            }
        };
        let matching = asked.matching;
        let read = tokio::task::spawn_blocking(move || {
            let root = std::fs::canonicalize(&checkout).ok().filter(|root| root.is_dir())?;
            Some(answer(&catalogue_of(&root), area, matching.as_deref()))
        })
        .await
        .map_err(|why| self.hosted_fault(CATALOGUE_UNFINISHED, &format!("reading the catalogue did not finish: {why}")))?;
        Ok(read.unwrap_or(DesignCatalogue { area, ..DesignCatalogue::default() }))
    }
}

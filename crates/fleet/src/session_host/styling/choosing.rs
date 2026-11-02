//! Which style set a `mock` node is drawn with, and the refusals when none can be. **A function of a
//! checkout and some paths**, so it is tested on temporary folders; `applying` stands the Job under it.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use ipc::{KeptStyles, MockNode, StyleKind, StyleSources};

use super::keeping;
use super::walk::{sources_near, usable_kind};

/// What choosing did for a scene's mocks.
#[derive(Default)]
pub(crate) struct Styled {
    /// Node id to the hash of the style set it is drawn with.
    pub kept: BTreeMap<String, String>,
    /// Node id to why it cannot be drawn.
    pub refused: BTreeMap<String, String>,
}

/// Most files a refusal lists of what the walk found.
const LISTED_AT_MOST: usize = 8;

pub(crate) fn style_mocks(checkout: &str, tiers: &[Vec<String>], attachments: &str, mocks: &[MockNode]) -> Styled {
    let root = std::fs::canonicalize(checkout).ok().filter(|root| root.is_dir());
    let mut run = Run {
        root: root.as_deref(),
        tiers,
        folder: keeping::folder(attachments),
        walked: None,
        dirty: None,
        kept: HashMap::new(),
    };
    let mut styled = Styled::default();
    for mock in mocks {
        match run.one(mock) {
            Ok(Some(hash)) => {
                styled.kept.insert(mock.id.clone(), hash);
            }
            Ok(None) => {}
            Err(why) => {
                styled.refused.insert(mock.id.clone(), why);
            }
        }
    }
    styled
}

struct Run<'a> {
    root: Option<&'a Path>,
    tiers: &'a [Vec<String>],
    folder: std::path::PathBuf,
    walked: Option<StyleSources>,
    dirty: Option<BTreeSet<String>>,
    /// One set kept for the same files, however many mocks ask for them.
    kept: HashMap<(bool, Vec<String>), Result<String, String>>,
}

impl Run<'_> {
    fn one(&mut self, mock: &MockNode) -> Result<Option<String>, String> {
        let here = format!("mock `{}`", mock.id);
        match (self.root, mock.styles.is_empty()) {
            (Some(root), false) => self.named(root, mock, &here).map(Some),
            (Some(root), true) => self.found(root, mock, &here),
            (None, false) => Err(format!("{here}: `styles` names files, and this job has no checkout to read them from. Leave `styles` out and draw it with `css`")),
            (None, true) => nothing_usable(mock, &here, "this job has no checkout"),
        }
    }

    fn walk(&mut self, root: &Path) -> &StyleSources {
        self.walked.get_or_insert_with(|| sources_near(root, None, self.tiers))
    }

    fn named(&mut self, root: &Path, mock: &MockNode, here: &str) -> Result<String, String> {
        let mut paths: Vec<String> = Vec::new();
        for given in &mock.styles {
            let rel = relative_to(root, given).ok_or_else(|| {
                format!("{here}: `styles` entry `{given}` is outside this job's checkout. Name paths from the repository's root")
            })?;
            if let Err(why) = usable_kind(root, &rel) {
                let found = found_text(self.walk(root));
                return Err(format!("{here}: `styles` entry `{given}` {why}. {found}"));
            }
            if !paths.contains(&rel) {
                paths.push(rel);
            }
        }
        let set = self.kept_for(root, paths, false);
        set.and_then(|hash| hash.ok_or_else(|| format!("{here}: the stylesheets named hold nothing to keep")))
            .map_err(|why| if why.starts_with(&here.to_string()) { why } else { format!("{here}: {why}") })
    }

    fn found(&mut self, root: &Path, mock: &MockNode, here: &str) -> Result<Option<String>, String> {
        let walked = self.walk(root).clone();
        if walked.sources.is_empty() && walked.packages.len() > 1 {
            return Err(format!(
                "{here}: this repository holds {} packages and the job has no changed, read or planned file to start from, \
                 so Fleet will not pick one: {}. Name the mock's `styles`, or call get_style_sources with a path inside the \
                 package you are working on",
                walked.packages.len(),
                walked.packages.iter().take(LISTED_AT_MOST).cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        let usable = walked.sources.iter().filter(|one| one.usable);
        let reset = usable.clone().find(|one| one.kind == StyleKind::TailwindReset).map(|one| one.path.clone());
        let mut paths: Vec<String> = usable.clone().filter(|one| one.kind != StyleKind::TailwindReset).map(|one| one.path.clone()).collect();
        paths.extend(reset.clone());
        if paths.is_empty() {
            return nothing_usable(mock, here, &found_text(&walked));
        }
        match self.kept_for(root, paths, true) {
            Ok(Some(hash)) => Ok(Some(hash)),
            Ok(None) => nothing_usable(mock, here, &found_text(&walked)),
            Err(why) => Err(format!("{here}: {why}")),
        }
    }

    /// The set of `paths` kept, once however often it is asked. `fit` leaves out a sheet past the cap.
    /// `None` where nothing in `paths` could be kept.
    fn kept_for(&mut self, root: &Path, paths: Vec<String>, fit: bool) -> Result<Option<String>, String> {
        let key = (fit, paths.clone());
        if let Some(done) = self.kept.get(&key) {
            return done.clone().map(Some);
        }
        let dirty = self.dirty.get_or_insert_with(|| dirty_files(root)).clone();
        let reset = paths.iter().find(|one| one.ends_with("tailwindcss/preflight.css") || one.ends_with("tailwindcss/lib/css/preflight.css")).cloned();
        let done = keeping::assemble(root, &paths, &dirty, fit).and_then(|set| {
            if set.sheets.is_empty() {
                return Ok(None);
            }
            // Nearest first was how they were chosen. They apply with the nearest last, after the reset.
            let set = match fit {
                true => applied_last_nearest(set, reset.as_deref()),
                false => set,
            };
            keeping::keep(&self.folder, &set).map(Some).map_err(|why| format!("Fleet could not keep the styles: {why}"))
        });
        match done {
            Ok(Some(hash)) => {
                self.kept.insert(key, Ok(hash.clone()));
                Ok(Some(hash))
            }
            Ok(None) => Ok(None),
            Err(why) => {
                self.kept.insert(key, Err(why.clone()));
                Err(why)
            }
        }
    }
}

fn applied_last_nearest(mut set: KeptStyles, reset: Option<&str>) -> KeptStyles {
    set.sheets.reverse();
    if let Some(at) = reset.and_then(|reset| set.sheets.iter().position(|one| one.path == reset)) {
        let one = set.sheets.remove(at);
        set.sheets.insert(0, one);
    }
    set
}

/// `Ok(None)` where the mock has css of its own to draw with, else the refusal that sends the agent to a `screen`.
fn nothing_usable(mock: &MockNode, here: &str, found: &str) -> Result<Option<String>, String> {
    match mock.has_css {
        true => Ok(None),
        false => Err(format!(
            "{here}: no stylesheet of this repository that a browser can use was found ({found}), and the mock has no `css` of its own. \
             Draw a `screen` node instead, so a generic mock-up does not pass as the app"
        )),
    }
}

/// What the walk from the Job's work found, as a refusal says it.
fn found_text(walked: &StyleSources) -> String {
    if walked.sources.is_empty() {
        return match walked.packages.len() {
            0 | 1 => String::from("the walk from this job's work found no stylesheet"),
            many => format!("the walk found no starting point in a repository of {many} packages"),
        };
    }
    let listed: Vec<String> = walked
        .sources
        .iter()
        .take(LISTED_AT_MOST)
        .map(|one| format!("{} ({}{})", one.path, kind_word(one.kind), if one.usable { "" } else { ", not usable" }))
        .collect();
    format!("the walk from this job's work found {}", listed.join(", "))
}

fn kind_word(kind: StyleKind) -> &'static str {
    match kind {
        StyleKind::Css => "css",
        StyleKind::CssModule => "css_module",
        StyleKind::TailwindOutput => "tailwind_output",
        StyleKind::TailwindReset => "tailwind_reset",
        StyleKind::ThemeCss => "theme_css",
        StyleKind::TailwindConfig => "tailwind_config",
        StyleKind::Scss => "scss",
        StyleKind::Less => "less",
        StyleKind::ThemeModule => "theme_module",
    }
}

/// A path the agent named, from the checkout's root, or `None` where it leaves the checkout.
pub(crate) fn relative_to(root: &Path, given: &str) -> Option<String> {
    let given = given.trim();
    let path = Path::new(given);
    let rel = match path.is_absolute() {
        true => path.strip_prefix(root).ok()?.to_string_lossy().into_owned(),
        false => given.trim_start_matches("./").to_string(),
    };
    (!rel.split('/').any(|part| part == "..")).then_some(rel)
}

/// The paths the checkout holds uncommitted changes to, from its root. Empty where git will not say.
fn dirty_files(root: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["--no-optional-locks", "status", "--porcelain", "-z", "--untracked-files=no"])
        .output();
    let Ok(out) = out.map(|out| out.stdout) else { return BTreeSet::new() };
    let text = String::from_utf8_lossy(&out);
    let mut dirty = BTreeSet::new();
    let mut entries = text.split('\0');
    while let Some(entry) = entries.next() {
        if entry.len() > 3 {
            dirty.insert(entry[3..].to_string());
        }
        // A rename names its old path in the entry after.
        if entry.starts_with('R') || entry.starts_with('C') {
            entries.next();
        }
    }
    dirty
}

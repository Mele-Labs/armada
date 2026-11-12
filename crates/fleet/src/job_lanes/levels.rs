//! The levels a workflow's editor reads its pieces at: the default's, a Workspace's (`workspace:<id>`) and one
//! repository's. A port of `levels.ts` in Bridge's `lifecycle-editing/` and of the level helpers of
//! `LaneCanvas/model.ts`, branch for branch.
//!
//! **Fleet runs a narrower piece whole and merges nothing**, so a level with a piece of its own reads that
//! piece, and one with none reads the nearest above. A repository in two Workspaces has both above it: the one
//! that sets a piece runs, and where both set it at one place neither runs and the repository reads the default's.

use std::collections::HashSet;

use ipc::{AppliedLifecycle, AppliedRule, LifecycleDefinition, LifecycleRule, PieceKept, PieceScope};

/// The level every repository reads until a narrower one sets a piece.
pub(crate) const DEFAULT: &str = "default";

const WORKSPACE: &str = "workspace:";

/// The level a Workspace's pieces are kept at.
pub(crate) fn workspace_level(id: &str) -> String {
    format!("{WORKSPACE}{id}")
}

pub(crate) fn is_workspace_level(level: &str) -> bool {
    level.starts_with(WORKSPACE)
}

/// A Workspace that holds the repository the editor is on: a project it is a member of.
#[derive(Clone, Debug)]
pub(crate) struct WorkspaceRead {
    pub id: String,
    pub name: String,
    /// Manifest ids, each once, in the order the project lists them.
    pub repositories: Vec<String>,
}

/// What one repository applies: `list_lifecycles`' and `list_lifecycle_rules`' lists for it.
#[derive(Clone, Debug)]
pub(crate) struct RepositoryRead {
    pub id: String,
    /// What its tab and its marks call it.
    pub name: String,
    pub lifecycles: Vec<AppliedLifecycle>,
    pub rules: Vec<AppliedRule>,
}

/// Everything read for a workflow's editor: the Workspaces that hold the repository it is on, and that
/// repository's pieces **first**, then those of the other repositories of its Workspaces.
#[derive(Clone, Debug, Default)]
pub(crate) struct PiecesRead {
    pub workspaces: Vec<WorkspaceRead>,
    pub repositories: Vec<RepositoryRead>,
}

/// A piece and where it was kept.
#[derive(Clone, Debug)]
pub(super) struct Layer<T> {
    pub piece: T,
    pub kept: PieceKept,
}

/// The pieces of one id by the level each applies at, in the order the levels were first read.
pub(super) struct Layers<T>(Vec<(String, Layer<T>)>);

impl<T> Layers<T> {
    pub(super) fn get(&self, level: &str) -> Option<&Layer<T>> {
        self.0.iter().find(|(at, _)| at == level).map(|(_, layer)| layer)
    }

    pub(super) fn has(&self, level: &str) -> bool {
        self.get(level).is_some()
    }
}

/// Where Fleet prefers a piece of one id and level from: the person's own over the repository's over Armada's.
fn place(kept: &PieceKept) -> u8 {
    match kept.rank.source.as_str() {
        "kit" => 0,
        "repository" => 1,
        "armada" => 2,
        _ => 9,
    }
}

/// The level a piece's scope names: a Workspace's by its id, else a repository's.
pub(super) fn level_of(scope: &PieceScope, read: &PiecesRead) -> String {
    match scope {
        PieceScope::Everywhere => DEFAULT.to_string(),
        PieceScope::Within { id } if read.workspaces.iter().any(|one| &one.id == id) => workspace_level(id),
        PieceScope::Within { id } => id.clone(),
    }
}

/// The pieces by id, each by level. **The same file is read once for each repository it applies to**, and is
/// taken once; of two at one level the preferred place wins, and the first read where they tie.
fn collect<T: Clone>(all: Vec<(String, PieceScope, T, PieceKept)>, read: &PiecesRead) -> Vec<(String, Layers<T>)> {
    let mut out: Vec<(String, Layers<T>)> = Vec::new();
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    for (id, scope, piece, kept) in all {
        let within = match &scope {
            PieceScope::Everywhere => String::new(),
            PieceScope::Within { id } => format!("within:{id}"),
        };
        if !seen.insert((kept.path.clone().unwrap_or_else(|| "armada".into()), id.clone(), within)) {
            continue;
        }
        let level = level_of(&scope, read);
        let at = match out.iter().position(|(one, _)| *one == id) {
            Some(at) => at,
            None => {
                out.push((id, Layers(Vec::new())));
                out.len() - 1
            }
        };
        let levels = &mut out[at].1 .0;
        match levels.iter_mut().find(|(one, _)| *one == level) {
            Some((_, held)) if place(&kept) < place(&held.kept) => *held = Layer { piece, kept },
            Some(_) => {}
            None => levels.push((level, Layer { piece, kept })),
        }
    }
    out
}

/// The lifecycles by id, each by level. Armada's are read without a path and are the default's.
pub(super) fn lifecycle_layers(read: &PiecesRead) -> Vec<(String, Layers<LifecycleDefinition>)> {
    let all = read
        .repositories
        .iter()
        .flat_map(|one| one.lifecycles.iter())
        .map(|each| (each.lifecycle.id.clone(), each.lifecycle.scope.clone(), each.lifecycle.clone(), each.kept.clone()))
        .collect();
    collect(all, read)
}

/// The rules by id, each by level.
pub(super) fn rule_layers(read: &PiecesRead) -> Vec<(String, Layers<LifecycleRule>)> {
    let all = read
        .repositories
        .iter()
        .flat_map(|one| one.rules.iter())
        .map(|each| (each.rule.id.clone(), each.rule.scope.clone(), each.rule.clone(), each.kept.clone()))
        .collect();
    collect(all, read)
}

/// The levels of the Workspaces that hold a repository, in the order Fleet lists them.
fn holders_of(read: &PiecesRead, repository: &str) -> Vec<String> {
    read.workspaces.iter().filter(|one| one.repositories.iter().any(|id| id == repository)).map(|one| workspace_level(&one.id)).collect()
}

/// The preference between two layers of one id: the place, then whether it came from the Workspace's home.
fn better<T>(a: &Layer<T>, b: &Layer<T>) -> bool {
    match place(&a.kept).cmp(&place(&b.kept)) {
        std::cmp::Ordering::Equal => a.kept.rank.home && !b.kept.rank.home,
        other => other.is_lt(),
    }
}

/// What a level reads where it has no piece of its own: the level that sets it and its piece, and the
/// Workspace levels that set it alike and so are set aside.
pub(super) struct Inherited<'a, T> {
    pub found: Option<(String, &'a Layer<T>)>,
    pub clash: Vec<String>,
}

/// The piece a level would run without one of its own. A Workspace's reads the default's. **A repository reads
/// the Workspace that sets the piece**; where two set it at one place Fleet keeps neither, so those are set
/// aside, and a repository no other sets it for reads the default's.
pub(super) fn inherited<'a, T>(layers: &'a Layers<T>, level: &str, read: &PiecesRead) -> Inherited<'a, T> {
    if level == DEFAULT {
        return Inherited { found: None, clash: Vec::new() };
    }
    let above = layers.get(DEFAULT).map(|layer| (DEFAULT.to_string(), layer));
    if is_workspace_level(level) {
        return Inherited { found: above, clash: Vec::new() };
    }
    let setting: Vec<(String, &Layer<T>)> =
        holders_of(read, level).into_iter().filter_map(|held| layers.get(&held).map(|layer| (held, layer))).collect();
    let tied = |one: &Layer<T>| {
        setting.iter().filter(|(_, each)| each.kept.rank.source == one.kept.rank.source && each.kept.rank.home == one.kept.rank.home).count()
    };
    let winner = setting
        .iter()
        .filter(|(_, layer)| tied(layer) == 1)
        .fold(None::<&(String, &Layer<T>)>, |best, one| match best {
            Some(best) if !better(one.1, best.1) => Some(best),
            _ => Some(one),
        })
        .map(|(at, layer)| (at.clone(), *layer));
    Inherited { found: winner.or(above), clash: setting.iter().filter(|(_, layer)| tied(layer) > 1).map(|(at, _)| at.clone()).collect() }
}

/// The piece a level runs: its own, else the one it inherits.
pub(super) fn effective<'a, T>(layers: &'a Layers<T>, level: &str, read: &PiecesRead) -> Option<(String, &'a Layer<T>)> {
    match layers.get(level) {
        Some(layer) => Some((level.to_string(), layer)),
        None => inherited(layers, level, read).found,
    }
}

/// The Workspace levels whose pieces of one id tie for a repository, as groups.
pub(super) type Clash = Vec<Vec<String>>;

/// The Workspace levels a repository's tab sets aside for a piece because two set it alike, as groups.
pub(super) fn clash_of<T>(layers: &Layers<T>, read: &PiecesRead) -> Clash {
    let mut groups: Clash = Vec::new();
    for repository in &read.repositories {
        let clash = inherited(layers, &repository.id, read).clash;
        if clash.len() > 1 && !groups.contains(&clash) {
            groups.push(clash);
        }
    }
    groups
}

/// The levels the editor's tabs are, widest first.
pub(super) fn levels_of_read(read: &PiecesRead) -> Vec<String> {
    std::iter::once(DEFAULT.to_string())
        .chain(read.workspaces.iter().map(|one| workspace_level(&one.id)))
        .chain(read.repositories.iter().map(|one| one.id.clone()))
        .collect()
}

// ---- A level's chain: `model.ts` ------------------------------------------------------------------------------------------------

/// The levels of the Workspaces that hold a repository the editor read. One it did not read is in none.
fn holders(read: &PiecesRead, repository: &str) -> Vec<String> {
    if read.repositories.iter().any(|one| one.id == repository) {
        holders_of(read, repository)
    } else {
        Vec::new()
    }
}

/// The levels of a chain that a clash sets aside: those of a group that two or more of the chain are in.
fn set_aside(chain: &[String], clash: &Clash) -> Vec<String> {
    chain
        .iter()
        .filter(|level| clash.iter().any(|group| group.contains(level) && group.iter().filter(|one| chain.contains(one)).count() > 1))
        .cloned()
        .collect()
}

/// The levels a tab reads, nearest first: a repository's own, the Workspaces that hold it, then the default's.
/// Given the clash a piece has, the Workspaces that tie are not read.
pub(super) fn levels_of(scope: &str, read: &PiecesRead, clash: &Clash) -> Vec<String> {
    if scope == DEFAULT {
        return vec![DEFAULT.to_string()];
    }
    let chain: Vec<String> =
        if is_workspace_level(scope) { vec![scope.to_string()] } else { std::iter::once(scope.to_string()).chain(holders(read, scope)).collect() };
    let aside = set_aside(&chain, clash);
    chain.into_iter().filter(|level| !aside.contains(level)).chain(std::iter::once(DEFAULT.to_string())).collect()
}

/// The Workspace levels a clash sets aside on a tab. A tab that keeps a piece of its own (`own`, the levels that
/// set it) reads that, and has none set aside.
pub(super) fn clash_at(clash: &Clash, scope: &str, read: &PiecesRead, own: &[String]) -> Vec<String> {
    if own.iter().any(|one| one == scope) || scope == DEFAULT || is_workspace_level(scope) {
        return Vec::new();
    }
    let chain: Vec<String> = std::iter::once(scope.to_string()).chain(holders(read, scope)).collect();
    set_aside(&chain, clash)
}

/// Whether something added at `own` exists on the tab for `scope`: at that level or below it.
pub(super) fn exists_at(own: &str, scope: &str, read: &PiecesRead, clash: &Clash) -> bool {
    own == DEFAULT || levels_of(scope, read, clash).iter().any(|level| level == own)
}

/// The name a level goes by: a Workspace's, else a repository's, else the level itself.
pub(super) fn level_name(read: &PiecesRead, level: &str) -> String {
    let found = if is_workspace_level(level) {
        read.workspaces.iter().find(|one| workspace_level(&one.id) == level).map(|one| one.name.clone())
    } else {
        read.repositories.iter().find(|one| one.id == level).map(|one| one.name.clone())
    };
    found.unwrap_or_else(|| level.to_string())
}

/// What a mark says of Workspaces whose pieces differ: `Web differ`, `Web and Mobile differ`, `Web, Mobile and Desk differ`.
pub(super) fn differ_said(names: &[String]) -> String {
    let listed = match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    format!("{listed} differ")
}

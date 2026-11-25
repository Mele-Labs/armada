//! What a project with no feature of its own starts its library from: the modules its build files name and the docs at the
//! top of its repositories, and the owner's Draft as Pending, which makes each module a pending feature.
//! `docs/concepts/feature-catalog.md`, *Starting a library*.

use serde::{Deserialize, Serialize};

/// `get_library_start`: the modules and docs of a project's repositories, read from its files with no model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryStart {
    /// A project's id.
    pub project: String,
    /// The modules its build files name, in path order, each once by its slug.
    pub modules: Vec<StartModule>,
    /// The docs at the top of its repositories: a README, an architecture file and the like, and a docs folder with a
    /// trailing `/`. Repository-relative.
    pub docs: Vec<String>,
}

/// A module of the project's repositories, and the pending feature a draft makes of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartModule {
    /// As its manifest names it.
    pub name: String,
    /// The slug its feature takes.
    pub slug: String,
    /// Its folder, repository-relative; empty for a repository's own root.
    pub path: String,
    /// The library holds a feature of that slug, so a draft passes it over.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub held: bool,
}

/// `draft_module_features`: make each module of a project a pending feature, now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftModules {
    /// A project's id.
    pub project: String,
}

/// What `draft_module_features` kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModulesDrafted {
    pub project: String,
    /// The slugs of the pending features it made, in path order.
    pub drafted: Vec<String>,
    /// The slugs it passed over because the library holds them already.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub held: Vec<String>,
}

//! What detection knows by name, read from `known.toml`. **A line in the data is the whole cost of knowing one more thing**,
//! and it keeps vendors' names out of code.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Entry {
    needs: Vec<String>,
    pub(super) what: String,
    #[serde(default)]
    pub(super) screen: bool,
}

#[derive(Deserialize)]
pub(super) struct Needs {
    #[serde(default)]
    needs: Vec<String>,
    #[serde(default)]
    pub(super) files: Vec<String>,
}

#[derive(Deserialize)]
pub(super) struct StoreRule {
    needs: Vec<String>,
    pub(super) name: String,
    #[serde(default)]
    pub(super) ext: Vec<String>,
}

#[derive(Deserialize)]
pub(super) struct ImageRule {
    pub(super) names: Vec<String>,
    pub(super) name: String,
}

#[derive(Deserialize)]
pub(super) struct OutsideRule {
    pub(super) name: String,
    #[serde(default)]
    pub(super) needs: Vec<String>,
    #[serde(default)]
    pub(super) stems: Vec<String>,
    #[serde(default)]
    pub(super) hosts: Vec<String>,
    #[serde(default)]
    pub(super) quoted: Vec<String>,
    #[serde(default)]
    pub(super) summary: String,
}

#[derive(Deserialize)]
pub(super) struct PieceRules {
    pub(super) descend: Vec<String>,
    pub(super) skip: Vec<String>,
}

#[derive(Deserialize)]
pub(super) struct Tooling {
    pub(super) names: Vec<String>,
}

#[derive(Deserialize)]
pub(super) struct Known {
    entry: Vec<Entry>,
    server: Vec<Needs>,
    service: Vec<Needs>,
    command: Vec<Needs>,
    store: Vec<StoreRule>,
    pub(super) image: Vec<ImageRule>,
    pub(super) outside: Vec<OutsideRule>,
    pub(super) pieces: PieceRules,
    tooling: Tooling,
}

/// A dependency name as dependency tools compare it: lower case, `_` and `.` as `-`.
pub(super) fn plain(name: &str) -> String {
    name.to_ascii_lowercase().replace(['_', '.'], "-")
}

/// Whether the dependency `dep` is `need`, or a member of its family: `react` is `react-dom`, `gpui` is `gpui-pre`, and
/// a need ending in `/` is a scope.
fn is(dep: &str, need: &str) -> bool {
    dep == need || dep.strip_prefix(need).is_some_and(|rest| need.ends_with('/') || rest.starts_with(['-', '/']))
}

/// Whether any of `deps` is any of `needs`.
pub(super) fn any_needed(deps: &[String], needs: &[String]) -> bool {
    deps.iter().any(|dep| needs.iter().any(|need| is(dep, need)))
}

impl Known {
    pub(super) fn entry(&self, deps: &[String]) -> Option<&Entry> {
        self.entry.iter().find(|rule| any_needed(deps, &rule.needs))
    }

    pub(super) fn serves(&self, deps: &[String]) -> bool {
        self.server.iter().any(|rule| any_needed(deps, &rule.needs))
    }

    /// A deployed service by its dependencies or by the files beside its manifest.
    pub(super) fn deployed(&self, deps: &[String], has: impl Fn(&str) -> bool) -> bool {
        self.service.iter().any(|rule| any_needed(deps, &rule.needs) || rule.files.iter().any(|file| has(file)))
    }

    pub(super) fn command(&self, deps: &[String]) -> bool {
        self.command.iter().any(|rule| any_needed(deps, &rule.needs))
    }

    pub(super) fn store(&self, deps: &[String]) -> Option<&StoreRule> {
        self.store.iter().find(|rule| any_needed(deps, &rule.needs))
    }

    pub(super) fn outside_of(&self, deps: &[String]) -> Vec<usize> {
        self.outside.iter().enumerate().filter(|(_, rule)| any_needed(deps, &rule.needs)).map(|(at, _)| at).collect()
    }

    pub(super) fn tooling(&self, name: &str) -> bool {
        let name = plain(name);
        self.tooling.names.iter().any(|one| plain(one) == name)
    }

    /// The store an image names, as compose spells it: `postgres:16`, `library/redis`.
    pub(super) fn image_store(&self, image: &str) -> Option<&ImageRule> {
        let base = image.split(':').next().unwrap_or(image).rsplit('/').next().unwrap_or(image).to_ascii_lowercase();
        self.image.iter().find(|rule| rule.names.iter().any(|name| base.starts_with(name.as_str())))
    }
}

pub(super) fn known() -> &'static Known {
    static KNOWN: OnceLock<Known> = OnceLock::new();
    KNOWN.get_or_init(|| toml::from_str(include_str!("known.toml")).expect("known.toml is the shape Known reads"))
}

/// The words of a file name or a path: `github_api.rs` is `github`, `api`, `rs`.
pub(super) fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()).map(str::to_ascii_lowercase).collect()
}

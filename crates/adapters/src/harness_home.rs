//! Claude Code's folder under the person's home, as the shared home writes into it: where each
//! kind is kept, a repository's memory, and its plugins installed through `claude plugin`.
//!
//! **Beside [`crate::ExistingSetup`] and of the same paths.** That reads the folder to show it and
//! cannot write; this is the different type a person's press writes through.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use adapter_traits::{FileRead, HarnessHome, HarnessPiece, HarnessPlace, KeptFolder, PluginRecord};
use serde::Deserialize;

use crate::existing_setup::{AGENT_FILE, COMMANDS, PLUGINS, SKILLS, SUB_AGENTS};
use crate::kit_pieces::{self, Asked, Cli, CommandLine, Installed, Piece, Runner, PROGRAM};
use crate::shared_home::Folder;

/// Where the harness keeps each folder's sessions and memory, by the folder's directory name.
const PROJECTS: &str = crate::session_folders::PROJECTS;
const MEMORY: &str = "memory";
const MEMORY_INDEX: &str = "MEMORY.md";
const MARKETPLACES: &str = ".claude/plugins/known_marketplaces.json";
/// A marketplace's source the harness reads from a GitHub repository.
const FROM_GITHUB: &str = "github";
/// An install fetches a repository and may build, as Kit's Add allows.
const INSTALL_WITHIN: Duration = Duration::from_secs(300);

/// Claude Code under one person's home.
pub struct AgentHome {
    folder: Folder,
    home: PathBuf,
    /// Fleet's `PATH`, which the CLI is found on.
    path: String,
}

impl AgentHome {
    pub fn at(home: impl Into<PathBuf>, path: &str) -> AgentHome {
        let home = home.into();
        AgentHome { folder: Folder::at(&home), home, path: path.to_string() }
    }

    fn decoded<T: for<'de> Deserialize<'de>>(&self, path: &str, what: &'static str) -> Result<Option<T>, String> {
        match self.folder.read(path) {
            FileRead::Absent => Ok(None),
            FileRead::Unreadable(why) => Err(why),
            FileRead::Bytes(bytes) => ipc::decode(what, &bytes).map(Some).map_err(|why| why.to_string()),
        }
    }
}

#[derive(Deserialize)]
struct InstalledPlugins {
    #[serde(default)]
    plugins: BTreeMap<String, Vec<Version>>,
}

#[derive(Deserialize)]
struct Version {
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Deserialize)]
struct Marketplace {
    source: MarketplaceSource,
}

#[derive(Deserialize)]
struct MarketplaceSource {
    source: String,
    #[serde(default)]
    repo: Option<String>,
}

impl HarnessHome for AgentHome {
    fn folder(&self) -> &dyn KeptFolder {
        &self.folder
    }

    fn place(&self, piece: HarnessPiece) -> HarnessPlace {
        match piece {
            HarnessPiece::Skills => HarnessPlace::Folders { dir: SKILLS.to_string() },
            HarnessPiece::SubAgents => HarnessPlace::Files { dir: SUB_AGENTS.to_string(), suffix: ".md".to_string() },
            HarnessPiece::Commands => HarnessPlace::Files { dir: COMMANDS.to_string(), suffix: ".md".to_string() },
            HarnessPiece::GlobalInstructions => HarnessPlace::One { path: AGENT_FILE.to_string() },
        }
    }

    fn memory_dir(&self, root: &str) -> String {
        format!("{PROJECTS}/{}/{MEMORY}", crate::session_folders::directory_name(root))
    }

    fn memory_index(&self) -> String {
        MEMORY_INDEX.to_string()
    }

    /// The plugins installed for the person, not for one project, each with where its marketplace
    /// is added from where that is a GitHub repository.
    fn plugins(&self) -> Result<Vec<PluginRecord>, String> {
        let installed: InstalledPlugins = self.decoded(PLUGINS, "installed plugins")?.unwrap_or(InstalledPlugins { plugins: BTreeMap::new() });
        let known: BTreeMap<String, Marketplace> = self.decoded(MARKETPLACES, "known marketplaces")?.unwrap_or_default();
        Ok(installed
            .plugins
            .into_iter()
            .filter(|(_, versions)| versions.iter().any(|one| one.scope.as_deref().map_or(true, |scope| scope == "user")))
            .filter_map(|(name, _)| {
                let marketplace = name.split_once('@')?.1.to_string();
                let source = known
                    .get(&marketplace)
                    .filter(|held| held.source.source == FROM_GITHUB)
                    .and_then(|held| held.source.repo.clone());
                Some(PluginRecord { name, marketplace, source })
            })
            .collect())
    }

    /// **Only ever on a person's press**: the shared home's Bring is one, which is what `Asked`
    /// stands for here. A marketplace this install added is taken back where the plugin then fails.
    fn install(&self, plugin: &PluginRecord) -> Result<(), String> {
        let cli = Cli { path: self.path.clone(), home: self.home.clone(), within: INSTALL_WITHIN };
        let install = CommandLine { program: PROGRAM.to_string(), args: vec!["plugin".into(), "install".into(), plugin.name.clone()] };
        let Some(source) = &plugin.source else {
            return cli.run(&install).map(|_| ()).map_err(|why| why.said);
        };
        let add = CommandLine {
            program: PROGRAM.to_string(),
            args: vec!["plugin".into(), "marketplace".into(), "add".into(), source.clone()],
        };
        let piece = Piece::Plugin {
            name: plugin.name.split('@').next().unwrap_or(&plugin.name).to_string(),
            marketplace: plugin.marketplace.clone(),
            repository: source.clone(),
            skills: Vec::new(),
            commands: vec![add, install],
        };
        let asked = Asked::when(true).expect("asked is true");
        match kit_pieces::install(asked, &piece, &cli) {
            Installed::Done => Ok(()),
            Installed::Failed { command, error, .. } => Err(format!("`{command}` failed: {error}")),
        }
    }
}

//! What an agent sets a watch up with: `try_watch` runs one reading and keeps nothing,
//! `list_watch_recipes` names the recipes a start may name, and `save_watch_recipe` keeps one on
//! this Mac.
//!
//! **A saved recipe is never written into a repository**: a recipe there travels with the
//! repository, and one an agent writes can name a private repository or a person's own script.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;
use config::{ResolvedWatchRecipe, WatchRecipeSource as Source};
use ipc::{
    SaveWatchRecipe, SessionId, TryWatch, WatchRecipeList, WatchRecipeListed, WatchRecipeSaved, WatchRecipeSource,
    WatchTried, WatchTries,
};
use store::KeptWatchReading;

use super::environment::{self, LoginEnvironment};
use super::running::Folder;
use super::*;
use crate::daemon::Fleet;

fn source(source: Source) -> WatchRecipeSource {
    match source {
        Source::Armada => WatchRecipeSource::Armada,
        Source::Repository => WatchRecipeSource::Repository,
        Source::Machine => WatchRecipeSource::Machine,
    }
}

fn listed(found: &ResolvedWatchRecipe) -> WatchRecipeListed {
    let recipe = &found.recipe;
    WatchRecipeListed {
        name: recipe.name.clone(),
        values: recipe.values().into_iter().map(str::to_string).collect(),
        command: recipe.command.clone(),
        source: source(found.source),
        every_seconds: recipe.every_seconds,
        gives_up_after_seconds: recipe.gives_up_after_seconds,
        environment: recipe.environment.clone(),
    }
}

/// A name a recipe's file can carry.
fn a_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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
    /// The recipes a placed Job may name; with no Job, the shipped ones and this Mac's.
    async fn recipes_placed(&self, job: Option<&str>) -> config::WatchRecipes {
        match job {
            Some(job) => self.recipes_for(job).await,
            None => config::WatchRecipes::of(self.locating().machine_watch_recipes()),
        }
    }

    /// `try_watch`: the command run once, where and as a watch's would run, and nothing kept.
    pub(crate) async fn watch_tried(&self, caller: Option<api::Caller>, tried: TryWatch) -> Result<WatchTried, Refusal> {
        let job = self.placed_job(caller, tried.session_id.as_ref()).await?;
        let (command, environment) = match tried.tries {
            WatchTries::Recipe { recipe, values } => {
                let filled = self.recipe_filled(&job, &recipe, &values).await?;
                (filled.command, filled.environment)
            }
            WatchTries::Command { command } => {
                self.command_checked(&command)?;
                (command, None)
            }
        };
        let scratch = self.scratch_of(&job).to_string_lossy().into_owned();
        let (taken, cwd, login) = match self.watch_folder(&job).await {
            Folder::At(folder) => {
                let login = environment::read(self.watches().shell().as_deref(), &folder).await;
                let taken =
                    self.watch_once(&job, &command, environment.as_deref(), &folder, &login, std::future::pending()).await;
                (taken, folder.to_string_lossy().into_owned(), login)
            }
            Folder::Gone(why) => {
                let login = LoginEnvironment::fallen_back(String::from("Nothing ran, so no shell was read."));
                (store::KeptWatchTaken::Broken { why, output: None }, String::new(), login)
            }
            Folder::Paused => {
                let why = String::from("This Job is paused, so nothing runs for it until the person resumes it.");
                let login = LoginEnvironment::fallen_back(String::from("Nothing ran, so no shell was read."));
                (store::KeptWatchTaken::Broken { why, output: None }, String::new(), login)
            }
            Folder::JobEnded => {
                let why = String::from("This Job has ended, so it has no folder to run in.");
                let login = LoginEnvironment::fallen_back(String::from("Nothing ran, so no shell was read."));
                (store::KeptWatchTaken::Broken { why, output: None }, String::new(), login)
            }
        };
        let reading = wire_reading(&KeptWatchReading { at: self.now().as_str().to_string(), taken });
        Ok(WatchTried { reading, command, cwd, path: login.path(), login_shell: login.wire(), scratch })
    }

    /// `list_watch_recipes`: one per name, the copy a start would run.
    pub(crate) async fn watch_recipes_listed(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<SessionId>,
    ) -> Result<WatchRecipeList, Refusal> {
        let job = self.watching_job(caller, session_id.as_ref()).await?;
        let recipes = self.recipes_placed(job.as_deref()).await;
        Ok(WatchRecipeList {
            recipes: recipes.recipes().iter().map(listed).collect(),
            left_out: recipes.left_out().iter().map(|left| left.to_string()).collect(),
        })
    }

    /// `save_watch_recipe`: checked as a start would check it, then written into this Mac's own
    /// recipes, the script beside it.
    pub(crate) async fn watch_recipe_saved(
        &self,
        caller: Option<api::Caller>,
        save: SaveWatchRecipe,
    ) -> Result<WatchRecipeSaved, Refusal> {
        let job = self.watching_job(caller, save.session_id.as_ref()).await?;
        let name = save.name.trim().to_string();
        if !a_name(&name) {
            return Err(self.watch_unacceptable(REFUSED, "`name` is letters, digits, `-` and `_`, since it names the recipe's file."));
        }
        let every = self.span("every", &save.every)?;
        let gives_up_after = save.gives_up_after.as_deref().map(|written| self.span("gives_up_after", written)).transpose()?;
        let settings = self.machine_settings().now();
        let floor = u32::try_from(settings.get(keys::WATCH_EVERY_SECONDS).as_secs()).unwrap_or(u32::MAX);
        if every < floor {
            return Err(self.watch_unacceptable(
                TOO_OFTEN,
                format!("It asks to run every {every}s, and `timeouts.watchEverySeconds` allows every {floor}s at most."),
            ));
        }
        if gives_up_after.is_some_and(|seconds| seconds > GIVE_UP_CAP_SECONDS) {
            return Err(self.watch_unacceptable(GIVES_UP_TOO_LATE, "A watch gives up after a day at most: `24h`."));
        }
        let script = match save.script.as_deref().map(str::trim).filter(|path| !path.is_empty()) {
            Some(path) => Some(self.script_checked(Path::new(path))?),
            None => None,
        };
        if script.is_none() {
            self.command_checked(&save.command)?;
        } else if save.command.iter().any(|item| item.trim().is_empty() || item.contains('\0')) {
            return Err(self.watch_unacceptable(REFUSED, "`command` is a list of arguments, none of them blank."));
        }
        let Some(folder) = self.locating().machine_watch_folder() else {
            return Err(self.recipe_unwritten("This Fleet keeps no recipes on this Mac."));
        };
        let recipes = self.recipes_placed(job.as_deref()).await;
        let found = recipes.named(&name);
        let shipped = found.is_some_and(|found| {
            found.source == Source::Armada || found.replaced.iter().any(|(source, _)| *source == Source::Armada)
        });
        if shipped {
            return Err(self.watch_illegal(
                RECIPE_SHIPPED,
                format!("{name} is a recipe Armada ships, so a copy here would hide it. Name yours another way."),
            ));
        }
        if let (Some(found), false) = (found, save.replace) {
            return Err(self.watch_illegal(
                RECIPE_EXISTS,
                format!("A recipe named {name} is there already, {}. Pass `replace` to write over it.", found.source),
            ));
        }
        // A machine file already naming it is written over, so this Mac never holds two.
        let file = match found.filter(|found| found.source == Source::Machine) {
            Some(found) => found.path.clone(),
            None => folder.join(format!("{name}.yml")),
        };
        std::fs::create_dir_all(&folder).map_err(|why| self.recipe_unwritten(format!("{} could not be made: {why}", folder.display())))?;
        let copied = match &script {
            Some(from) => Some(self.script_copied(from, &folder, &name)?),
            None => None,
        };
        let mut command: Vec<String> = copied.iter().map(|path| path.to_string_lossy().into_owned()).collect();
        command.extend(save.command.iter().cloned());
        let recipe = config::WatchRecipe {
            name: name.clone(),
            command,
            every_seconds: every,
            gives_up_after_seconds: gives_up_after,
            environment: save.environment.map(|named| named.trim().to_string()).filter(|named| !named.is_empty()),
        };
        let text = config::watch_recipe_written(&recipe);
        if config::parse_watch_recipe(&file, &text).ok().as_ref() != Some(&recipe) {
            return Err(self.watch_unacceptable(REFUSED, "That recipe would not read back as written."));
        }
        std::fs::write(&file, text).map_err(|why| self.recipe_unwritten(format!("{} could not be written: {why}", file.display())))?;
        let saved = ResolvedWatchRecipe { recipe, source: Source::Machine, path: file.clone(), replaced: Vec::new() };
        Ok(WatchRecipeSaved {
            recipe: listed(&saved),
            file: file.to_string_lossy().into_owned(),
            script: copied.map(|path| path.to_string_lossy().into_owned()),
            replaced: found.is_some(),
        })
    }

    fn recipe_unwritten(&self, why: impl Into<String>) -> Refusal {
        Refusal::Fault(ipc::WireError::raised(RECIPE_UNWRITTEN, why.into(), self.run_id()))
    }

    /// `path` as a script a recipe may keep: a file, and not one read as a recipe itself.
    fn script_checked(&self, path: &Path) -> Result<PathBuf, Refusal> {
        if !path.is_absolute() || !path.is_file() {
            return Err(self.watch_unacceptable(REFUSED, format!("`script` is the whole path of a file, and {} is not one.", path.display())));
        }
        if matches!(path.extension().and_then(|ext| ext.to_str()), Some("yml" | "yaml")) {
            return Err(self.watch_unacceptable(REFUSED, "`script` ends `.yml` or `.yaml`, which would be read as a recipe."));
        }
        Ok(path.to_path_buf())
    }

    /// `from` copied into `folder` under the recipe's `name`, its extension kept, and made runnable.
    fn script_copied(&self, from: &Path, folder: &Path, name: &str) -> Result<PathBuf, Refusal> {
        let file = match from.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => format!("{name}.{ext}"),
            None => name.to_string(),
        };
        let to = folder.join(file);
        let unwritten = |why: std::io::Error| self.recipe_unwritten(format!("{} could not be written: {why}", to.display()));
        std::fs::copy(from, &to).map_err(unwritten)?;
        std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o755)).map_err(unwritten)?;
        Ok(to)
    }
}

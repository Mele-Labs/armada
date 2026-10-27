//! What a Job sets a watch up with: the login shell's environment, a try that keeps nothing, the
//! recipes listed, and one saved on this Mac. `crate::tests::watches_terminal` waits on a terminal.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use api::{Refusal, Watches};
use ipc::{
    LoginShell, SaveWatchRecipe, SessionId, StartWatch, TryWatch, WatchReading, WatchRecipeSource, WatchStarts,
    WatchTries,
};

use super::repositories::Planted;
use super::session_host::{eventually, rig_fitted, Rig};
use super::tmp::TempDir;
use crate::watches::environment::{parsed, read};

fn code(refusal: &Refusal) -> (&str, u16) {
    (refusal.error().code.as_str(), refusal.status())
}

/// A stand-in for the person's login shell: prompt escapes either side of the `-c` command, a
/// variable exported in its rc, then the command run.
fn login_shell(dir: &Path) -> PathBuf {
    let shell = dir.join("login-shell");
    std::fs::write(
        &shell,
        "#!/bin/sh\nprintf '\\033[?25h\\033]0;prompt\\007'\nexport WATCH_FIXTURE='from the rc'\n/bin/sh -c \"$2\"\nprintf '\\033[?25l'\n",
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
    shell
}

/// A command whose line says what `WATCH_FIXTURE` held when it ran.
fn says_the_fixture() -> Vec<String> {
    vec![
        String::from("/bin/sh"),
        String::from("-c"),
        String::from(r#"printf '{"state":"passed","finished":true,"says":"%s"}\n' "$WATCH_FIXTURE""#),
    ]
}

#[test]
fn only_what_lies_between_the_markers_is_read() {
    let printed = "\u{1b}[?25hM<STALE=1\0>M\u{1b}]0;title\u{7}M<PATH=/opt/bin:/usr/bin\0MULTI=a\nb\0PWD=/x\0SHLVL=2\0>M\u{1b}[?25l";
    let read = parsed(printed, "M").expect("both markers");
    assert_eq!(read, [("PATH".to_string(), "/opt/bin:/usr/bin".to_string()), ("MULTI".to_string(), "a\nb".to_string())]);
    assert_eq!(parsed("M<A=1\0", "M"), None, "no closing marker");
}

#[tokio::test]
async fn a_login_shell_is_read_and_one_that_prints_nothing_falls_back_saying_so() {
    let dir = TempDir::new();
    let read_from = read(Some(&login_shell(dir.path()).to_string_lossy()), dir.path()).await;
    assert_eq!(read_from.fell_back, None);
    assert!(read_from.variables.contains(&("WATCH_FIXTURE".to_string(), "from the rc".to_string())));

    let fell = read(Some("/usr/bin/true"), dir.path()).await;
    assert!(fell.variables.is_empty());
    assert!(matches!(fell.wire(), LoginShell::FellBack { why } if why.contains("printed no environment")));
    assert!(matches!(read(None, dir.path()).await.wire(), LoginShell::FellBack { .. }));
}

fn rig_offering(recipes: Vec<config::WatchRecipeWritten>, machine: Option<PathBuf>) -> Rig {
    let planted = Arc::new(Planted::nothing());
    planted.will_offer_recipes(recipes);
    if let Some(machine) = machine {
        planted.keeps_machine_recipes_in(machine);
    }
    rig_fitted(move |fittings| fittings.locating = planted)
}

fn tried(job: &SessionId, command: Vec<String>) -> TryWatch {
    TryWatch { session_id: Some(job.clone()), tries: WatchTries::Command { command } }
}

#[tokio::test]
async fn a_try_reads_once_in_the_jobs_folder_given_the_login_shells_environment_and_keeps_nothing() {
    let shells = TempDir::new();
    let rig = rig_offering(Vec::new(), None);
    rig.fleet.watches().read_login_shell(Some(login_shell(shells.path()).to_string_lossy().into_owned()));
    let job = rig.start().await;

    let read = rig.fleet.try_watch(None, tried(&job, says_the_fixture())).await.expect("tried");
    assert!(matches!(&read.reading, WatchReading::Read { says, .. } if says == "from the rc"), "{:?}", read.reading);
    assert_eq!(read.login_shell, LoginShell::Read);
    assert_eq!(std::fs::canonicalize(&read.cwd).unwrap(), std::fs::canonicalize(&rig.root).unwrap());
    assert!(!read.path.is_empty());
    assert!(Path::new(&read.scratch).is_dir(), "the scratch folder is made when named: {}", read.scratch);
    assert!(rig.fleet.list_watches(None, Some(job.clone()), true).await.unwrap().watches.is_empty(), "nothing kept");

    let broken = vec![String::from("/bin/sh"), String::from("-c"), String::from("echo no token here >&2; exit 3")];
    let read = rig.fleet.try_watch(None, tried(&job, broken)).await.expect("tried");
    assert!(
        matches!(&read.reading, WatchReading::Broken { why, output: Some(out), .. } if why == "It exited 3." && out.contains("no token here")),
        "{:?}",
        read.reading
    );
}

#[tokio::test]
async fn a_started_watch_keeps_the_environment_it_started_with() {
    let shells = TempDir::new();
    let rig = rig_offering(Vec::new(), None);
    rig.fleet.watches().read_login_shell(Some(login_shell(shells.path()).to_string_lossy().into_owned()));
    let job = rig.start().await;
    let start = StartWatch {
        session_id: Some(job.clone()),
        name: String::from("Fixture"),
        starts: WatchStarts::Command { command: says_the_fixture(), every: String::from("30s"), gives_up_after: None },
        follows: None,
        set_aside: Vec::new(),
    };
    let started = rig.fleet.start_watch(None, start).await.expect("started");
    assert_eq!(started.login_shell, Some(LoginShell::Read));
    // A shell that would now fail is not read again: the watch keeps what it started with.
    rig.fleet.watches().read_login_shell(Some(String::from("/usr/bin/false")));
    rig.fleet.watches_run_when_due().await;
    eventually(|| async { rig.fleet.watches().running_now() == 0 }).await;
    let read = rig.fleet.list_watches(None, Some(job), true).await.unwrap().watches.remove(0);
    assert!(matches!(&read.last, Some(WatchReading::Read { says, .. }) if says == "from the rc"), "{:?}", read.last);
    assert_eq!(read.login_shell, None, "it passed and ended, so Fleet holds no environment for it");
}

fn shipped() -> config::WatchRecipeWritten {
    config::WatchRecipeWritten::by_armada(
        PathBuf::from("armada/watch-recipes/build.yml"),
        "name: build-state\ncommand: [armada, watch-line, reader, \"{sha}\", \"{context}\"]\nevery: 60s\n".to_string(),
    )
}

fn repository() -> config::WatchRecipeWritten {
    config::WatchRecipeWritten::in_repository(
        PathBuf::from(".armada/watches/qa.yml"),
        "name: qa-deploy\ncommand: [scripts/deploy-state, \"{sha}\"]\nevery: 2m\ngives_up_after: 1h\n".to_string(),
    )
}

#[tokio::test]
async fn the_recipes_are_listed_with_their_values_and_where_each_came_from() {
    let rig = rig_offering(vec![shipped(), repository()], None);
    let job = rig.start().await;
    let listed = rig.fleet.list_watch_recipes(None, Some(job.clone())).await.expect("listed");
    let seen: Vec<(&str, WatchRecipeSource, Vec<String>, Option<u32>)> = listed
        .recipes
        .iter()
        .map(|one| (one.name.as_str(), one.source, one.values.clone(), one.gives_up_after_seconds))
        .collect();
    assert_eq!(
        seen,
        [
            ("build-state", WatchRecipeSource::Armada, vec!["sha".to_string(), "context".to_string()], None),
            ("qa-deploy", WatchRecipeSource::Repository, vec!["sha".to_string()], Some(3600)),
        ]
    );

    // Started and not run: the test binary is no reader.
    let start = StartWatch {
        session_id: Some(job.clone()),
        name: String::from("Build"),
        starts: WatchStarts::Recipe {
            recipe: String::from("build-state"),
            values: [("sha", "abc"), ("context", "deploy")].into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
        },
        follows: None,
        set_aside: Vec::new(),
    };
    let started = rig.fleet.start_watch(None, start).await.expect("started");
    rig.fleet.stop_watch(None, ipc::StopWatch { session_id: None, watch: started.id }).await.expect("stopped");
    let own = std::env::current_exe().unwrap();
    assert_eq!(started.command[0], own.to_string_lossy(), "a shipped recipe runs Fleet's own binary");
    assert_eq!(&started.command[1..], ["watch-line", "reader", "abc", "deploy"]);
}

fn save(job: &SessionId, name: &str, script: Option<&Path>, command: &[&str], replace: bool) -> SaveWatchRecipe {
    SaveWatchRecipe {
        session_id: Some(job.clone()),
        name: name.to_string(),
        script: script.map(|path| path.to_string_lossy().into_owned()),
        command: command.iter().map(|item| item.to_string()).collect(),
        every: String::from("60s"),
        gives_up_after: Some(String::from("2h")),
        environment: None,
        replace,
    }
}

#[tokio::test]
async fn a_saved_recipe_and_its_script_are_kept_on_this_mac_for_every_job() {
    let machine = TempDir::new();
    let folder = machine.path().join("watches");
    let rig = rig_offering(vec![shipped(), repository()], Some(folder.clone()));
    let job = rig.start().await;
    let script = rig.fleet.scratch_of(job.as_str()).join("deploy-state.sh");
    std::fs::write(&script, "#!/bin/sh\necho '{}'\n").unwrap();

    let saved = rig
        .fleet
        .save_watch_recipe(None, save(&job, "staging-deploy", Some(&script), &["{sha}"], false))
        .await
        .expect("saved");
    let copy = folder.join("staging-deploy.sh");
    assert_eq!(saved.script.as_deref(), Some(&*copy.to_string_lossy()));
    assert_eq!(std::fs::metadata(&copy).unwrap().permissions().mode() & 0o111, 0o111, "the copy runs");
    assert_eq!(saved.file, folder.join("staging-deploy.yml").to_string_lossy());
    assert_eq!(saved.recipe.command, [copy.to_string_lossy().into_owned(), "{sha}".to_string()]);
    assert_eq!((saved.recipe.source, saved.replaced), (WatchRecipeSource::Machine, false));
    assert!(!Path::new(&rig.root).join(".armada/watches/staging-deploy.yml").exists(), "never the repository");

    let another = rig.start().await;
    let listed = rig.fleet.list_watch_recipes(None, Some(another.clone())).await.expect("listed");
    assert!(listed.recipes.iter().any(|one| one.name == "staging-deploy" && one.values == ["sha"]));

    let shipped_name = rig.fleet.save_watch_recipe(None, save(&another, "build-state", None, &["x"], true)).await;
    assert_eq!(code(&shipped_name.expect_err("Armada ships it")), ("fleet.watch_recipe_shipped", 409));
    let there = rig.fleet.save_watch_recipe(None, save(&another, "qa-deploy", None, &["x"], false)).await;
    assert_eq!(code(&there.expect_err("the repository has it")), ("fleet.watch_recipe_exists", 409));
    let again = rig.fleet.save_watch_recipe(None, save(&another, "staging-deploy", None, &["/bin/echo"], true)).await;
    let again = again.expect("replace writes over it");
    assert_eq!((again.replaced, again.recipe.command.clone()), (true, vec!["/bin/echo".to_string()]));
    let bad = rig.fleet.save_watch_recipe(None, save(&another, "../escape", None, &["x"], false)).await;
    assert_eq!(code(&bad.expect_err("not a name")), ("fleet.watch_refused", 422));
}

#[tokio::test]
async fn a_jobs_scratch_folder_goes_when_the_job_ends() {
    let rig = rig_offering(Vec::new(), None);
    let job = rig.start().await;
    let scratch = rig.fleet.scratch_of(job.as_str());
    std::fs::write(scratch.join("kept.sh"), "#!/bin/sh\n").unwrap();
    rig.fleet.session_wound_up(job.as_str()).await;
    assert!(!scratch.exists(), "{} outlived its Job", scratch.display());
}

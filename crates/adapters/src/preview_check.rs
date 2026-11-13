//! The check an offer for the owner's preview gets: that the offered tips merge with main and with each
//! other, and that what they touch compiles. `docs/concepts/fleet.md`, *Offering a branch for the preview*.
//!
//! **One kept worktree, never a second.** It is made once under `<checkout>/.armada/preview-check` and
//! moved onto each merged commit with `checkout --detach -f`, so a build directory warmed by one check
//! serves the next. The merges are built on the side with `git merge-tree`, as `scripts/preview` builds
//! the preview, so nothing in the worktree is merged into.
//!
//! **What compiles is a [`Compiler`]**, so a test plants one and no test runs cargo.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::delivery::{read_in, said};

/// The kept worktree, under the checkout that holds `scripts/preview`.
pub const KEPT_TREE: &str = ".armada/preview-check";

/// What the tree is cleaned of between checks. The build directories stay: they are why one tree is kept.
const KEPT_BUILDS: [&str; 3] = ["target", "node_modules", "crates/bridge-native/target"];

/// A compile that has not finished after this is stopped and counts as a failure.
const COMPILING_FOR_AT_MOST: Duration = Duration::from_secs(45 * 60);

const POLL: Duration = Duration::from_millis(100);

/// One thing the offers' changes can break.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Compile {
    /// `cargo check --workspace`.
    Workspace,
    /// `cargo check` of the native Bridge, which is outside the workspace.
    Native,
    /// The repository's TypeScript typecheck.
    Typescript,
}

/// How a compile failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Broke {
    /// The first error line, as the compiler printed it.
    pub why: String,
    /// The files the errors name.
    pub files: Vec<String>,
}

/// What an offer's check came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Ready,
    Failed(Broke),
}

/// What conflict files read as, in the `why` of a failed offer.
pub const CONFLICTS: &str = "it conflicts with main or another offer in";

/// Compiles the kept tree. **Blocking**: a check runs on a thread of its own.
pub trait Compiler: Send + Sync {
    fn compile(&self, tree: &Path, what: Compile) -> Result<(), Broke>;
}

/// What compiling is, as `cargo` and `pnpm` do it, at utility priority.
pub struct Tools;

impl Compiler for Tools {
    fn compile(&self, tree: &Path, what: Compile) -> Result<(), Broke> {
        match what {
            Compile::Workspace => run(tree, "cargo", &["check", "--workspace"]),
            Compile::Native => run(tree, "cargo", &["check", "--manifest-path", "crates/bridge-native/Cargo.toml"]),
            Compile::Typescript => {
                if !tree.join("node_modules").is_dir() {
                    run(tree, "pnpm", &["install", "--frozen-lockfile"])?;
                }
                run(tree, "pnpm", &["typecheck"])
            }
        }
    }
}

/// What the changed `files` need compiled, in the order they are run.
pub fn compiles_for(files: &[String]) -> Vec<Compile> {
    let mut needed = BTreeSet::new();
    for file in files {
        let native = file.starts_with("crates/bridge-native/");
        let cargo_file = Path::new(file).file_name().is_some_and(|name| name.to_string_lossy().starts_with("Cargo."));
        if (file.starts_with("crates/") && !native) || file.starts_with("xtask/") || (cargo_file && !native) {
            needed.insert(Compile::Workspace);
        }
        if native || file.starts_with("crates/ipc/") || file.starts_with("packages/tokens/") {
            needed.insert(Compile::Native);
        }
        if file.starts_with("apps/") || file.starts_with("packages/") {
            needed.insert(Compile::Typescript);
        }
    }
    needed.into_iter().collect()
}

/// The verdict on each of `tips`, in order, as merged onto `base` one after another and compiled.
///
/// **Every tip at once first**: where that compiles, each is ready. Where it does not, they are tried
/// one at a time, each on top of main and the ones that passed, and only the one that breaks fails. A
/// tip that conflicts fails where it was brought in, naming the files, and the rest go on without it.
///
/// `Err` says the check could not run (git would not make the tree), and no verdict is given.
pub fn check(root: &Path, base: &str, tips: &[String], compiler: &dyn Compiler) -> Result<Vec<Verdict>, String> {
    let tree = root.join(KEPT_TREE);
    prepare(root, &tree)?;
    let main = git_out(&tree, &["rev-parse", &format!("{base}^{{commit}}")])?;

    let mut verdicts: Vec<Option<Verdict>> = vec![None; tips.len()];
    let mut clean = Vec::new();
    let mut together = main.clone();
    for (at, tip) in tips.iter().enumerate() {
        match merged(&tree, &together, tip)? {
            Ok(commit) => {
                together = commit;
                clean.push(at);
            }
            Err(files) => verdicts[at] = Some(conflict(files)),
        }
    }
    if clean.is_empty() {
        return Ok(verdicts.into_iter().flatten().collect());
    }

    let broke = match compiled(&tree, &main, &together, compiler)? {
        Ok(()) => {
            for at in clean {
                verdicts[at] = Some(Verdict::Ready);
            }
            return Ok(verdicts.into_iter().flatten().collect());
        }
        Err(broke) => broke,
    };
    // One clean tip that breaks is the one that breaks: trying it alone would compile the same tree.
    if let [only] = clean.as_slice() {
        verdicts[*only] = Some(Verdict::Failed(broke));
        return Ok(verdicts.into_iter().flatten().collect());
    }

    let mut passed = main.clone();
    for at in clean {
        verdicts[at] = Some(match merged(&tree, &passed, &tips[at])? {
            Err(files) => conflict(files),
            Ok(candidate) => match compiled(&tree, &main, &candidate, compiler)? {
                Ok(()) => {
                    passed = candidate;
                    Verdict::Ready
                }
                Err(broke) => Verdict::Failed(broke),
            },
        });
    }
    Ok(verdicts.into_iter().flatten().collect())
}

/// The commit `rev` names in `root`, in full, or `None` where there is none.
pub fn commit_of(root: &Path, rev: &str) -> Option<String> {
    git_out(root, &["rev-parse", "--verify", "--quiet", &format!("{rev}^{{commit}}")]).ok().filter(|commit| !commit.is_empty())
}

fn conflict(files: Vec<String>) -> Verdict {
    let named = if files.is_empty() { String::from("a file git would not name") } else { files.join(", ") };
    Verdict::Failed(Broke { why: format!("{CONFLICTS} {named}"), files })
}

/// The kept worktree, made on first use.
fn prepare(root: &Path, tree: &Path) -> Result<(), String> {
    git_out(root, &["worktree", "prune"])?;
    if tree.join(".git").exists() {
        return Ok(());
    }
    if tree.exists() && tree.read_dir().is_ok_and(|mut entries| entries.next().is_some()) {
        return Err(format!("{} exists and is not a worktree", tree.display()));
    }
    std::fs::create_dir_all(tree.parent().unwrap_or(root)).map_err(|why| why.to_string())?;
    git_out(root, &["worktree", "add", "-q", "--detach", &tree.to_string_lossy(), "HEAD"]).map(|_| ())
}

/// `tip` merged into `onto` on the side: the merge commit, or the files that conflict.
fn merged(tree: &Path, onto: &str, tip: &str) -> Result<Result<String, Vec<String>>, String> {
    let run = read_in(&tree.to_string_lossy(), "git", &["merge-tree", "--write-tree", "--name-only", "--no-messages", onto, tip])
        .map_err(|why| why.to_string())?;
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let mut lines = out.lines().map(str::trim).filter(|line| !line.is_empty());
    match (run.status.code(), lines.next()) {
        (Some(0), Some(made)) => {
            let message = format!("Merge {tip} into the preview check");
            git_out(
                tree,
                &["-c", "user.name=Armada", "-c", "user.email=armada@localhost", "commit-tree", made, "-p", onto, "-p", tip, "-m", &message],
            )
            .map(Ok)
        }
        (Some(1), _) => Ok(Err(lines.map(str::to_string).collect())),
        _ => Err(format!("git merge-tree {onto} {tip} failed: {}", said(&run))),
    }
}

/// The tree moved onto `commit` and what the changes since `main` touch compiled.
fn compiled(tree: &Path, main: &str, commit: &str, compiler: &dyn Compiler) -> Result<Result<(), Broke>, String> {
    git_out(tree, &["checkout", "-q", "--detach", "-f", commit])?;
    let mut clean = vec!["clean", "-fdxq"];
    for kept in KEPT_BUILDS {
        clean.extend(["-e", kept]);
    }
    git_out(tree, &clean)?;
    let changed = git_out(tree, &["diff", "--name-only", main, commit])?;
    let files: Vec<String> = changed.lines().map(str::to_string).collect();
    for what in compiles_for(&files) {
        if let Err(broke) = compiler.compile(tree, what) {
            return Ok(Err(broke));
        }
    }
    Ok(Ok(()))
}

fn git_out(dir: &Path, args: &[&str]) -> Result<String, String> {
    let run = read_in(&dir.to_string_lossy(), "git", args).map_err(|why| why.to_string())?;
    if run.status.success() {
        Ok(String::from_utf8_lossy(&run.stdout).trim().to_string())
    } else {
        Err(format!("git {}: {}", args.join(" "), said(&run)))
    }
}

/// `program` in `tree`, at utility priority, stopped if it runs past [`COMPILING_FOR_AT_MOST`].
fn run(tree: &Path, program: &str, args: &[&str]) -> Result<(), Broke> {
    let mut child = crate::background::command(program)
        .args(args)
        .current_dir(tree)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| Broke { why: format!("{program} would not start: {why}"), files: Vec::new() })?;
    // Each stream on a thread of its own, so a chatty compiler cannot fill a pipe and stall.
    let streams = [child.stdout.take().map(drained), child.stderr.take().map(drained)];
    let deadline = Instant::now() + COMPILING_FOR_AT_MOST;
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Broke { why: format!("{program} took longer than {} minutes", COMPILING_FOR_AT_MOST.as_secs() / 60), files: Vec::new() });
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(why) => return Err(Broke { why: why.to_string(), files: Vec::new() }),
        }
    };
    let said: String = streams.into_iter().flatten().filter_map(|reading| reading.join().ok()).collect::<Vec<_>>().join("\n");
    if success {
        return Ok(());
    }
    Err(Broke {
        why: first_error(&said).unwrap_or_else(|| format!("{program} failed")),
        files: files_named(&said),
    })
}

fn drained(mut stream: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut said = String::new();
        let _ = stream.read_to_string(&mut said);
        said
    })
}

/// The longest a `why` is kept: a compiler line can be a whole type.
const WHY_AT_MOST: usize = 200;

/// The first line a compiler printed as an error, or the last line of output where none reads as one.
pub fn first_error(said: &str) -> Option<String> {
    let reads_as_error = |line: &&str| {
        let line = line.trim_start();
        let rustc = line.starts_with("error") && !line.starts_with("error: could not compile") && !line.starts_with("error: aborting");
        rustc || line.contains(": error TS")
    };
    let line = said.lines().find(reads_as_error).or_else(|| said.lines().rev().find(|line| !line.trim().is_empty()))?;
    Some(line.trim().chars().take(WHY_AT_MOST).collect())
}

/// The files the errors in `said` name: rustc's `--> path:line:col` and tsc's `path(line,col): error`.
pub fn files_named(said: &str) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for line in said.lines() {
        let line = line.trim();
        let named = if let Some(at) = line.strip_prefix("--> ") {
            at.split(':').next()
        } else if line.contains(": error TS") {
            line.split('(').next()
        } else {
            None
        };
        if let Some(file) = named.map(str::trim).filter(|file| !file.is_empty()) {
            if !files.iter().any(|seen| seen == file) {
                files.push(file.to_string());
            }
        }
    }
    files.truncate(8);
    files
}

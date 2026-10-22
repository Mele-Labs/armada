//! The test map's collector for a Cargo workspace: one lap per test, read from
//! LLVM's source-based coverage of a nextest run. `docs/concepts/test-map.md`.
//!
//! **It builds the workspace again, instrumented, so it must never run in a
//! shared machine's foreground.** The build goes to `target/test-map` and
//! leaves the warm one alone, but it is still a cold build of every member; the
//! concept doc's rule puts it on the forge's runner or in a sleeping bay, and
//! keeping it there is whoever schedules it.
//!
//! **A sleeping bay still shares the machine**, so it runs as a Check does:
//! `width` wide, at utility priority, with cargo's jobs capped at `width`
//! unless a build pool is offered. The caller holds Fleet's build pool around
//! [`record`](TestCollector::record); this does not take it.
//!
//! **A profile is tied to its test by a target runner.** nextest runs each
//! test in a process of its own and names it in `NEXTEST_TEST_NAME`, which
//! `LLVM_PROFILE_FILE` cannot expand; the runner script reads it and points
//! the profile into a folder of that test's own. A runner and not a wrapper
//! script, because a wrapper script is declared in the repository's nextest
//! config, and the map never writes to the repository.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use adapter_traits::{CollectorSource, Lap, LapCost, LapGrain, NotRecorded, TestCollector, TestsToRecord};
use checks_runner::{build_cap, CheckWidth};
use core_model::Narrowing;

use crate::background::command;

/// Where each test's process writes its profile, beside a note of which test
/// it is: binary id, test name, binary path and its directory, one a line.
const RUNNER: &str = r#"#!/bin/sh
if [ -n "$NEXTEST_TEST_NAME" ]; then
  lap=$(mktemp -d "$ARMADA_TEST_MAP_LAPS/lap.XXXXXX") || exit 1
  printf '%s\n%s\n%s\n%s\n' "$NEXTEST_BINARY_ID" "$NEXTEST_TEST_NAME" "$1" "$(pwd -P)" > "$lap/test"
  LLVM_PROFILE_FILE="$lap/%p-%m.profraw"
  export LLVM_PROFILE_FILE
fi
exec "$@"
"#;

static NEXT_RUNNER: AtomicU64 = AtomicU64::new(0);

/// Measured laps of a Cargo workspace's tests, one per test.
pub struct NextestCoverage {
    under: Option<String>,
    except: Vec<String>,
    width: CheckWidth,
}

impl NextestCoverage {
    /// For a Check narrowed as `narrowing`, run `width` wide: its laps name
    /// members as `under` spells them, and the members `except` bars are never
    /// run.
    pub fn for_check(narrowing: Option<&Narrowing>, width: CheckWidth) -> NextestCoverage {
        NextestCoverage {
            under: narrowing.and_then(Narrowing::under).map(str::to_string),
            except: narrowing.map(|declared| declared.except().to_vec()).unwrap_or_default(),
            width,
        }
    }
}

impl TestCollector for NextestCoverage {
    fn name(&self) -> &str {
        "nextest coverage"
    }

    fn source(&self) -> CollectorSource {
        CollectorSource::Measured
    }

    fn needs_rebuild(&self) -> bool {
        true
    }

    fn lap_cost(&self) -> LapCost {
        LapCost::Run
    }

    fn record(&self, _check: &str, dir: &str, tests: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded> {
        let root = Path::new(dir)
            .canonicalize()
            .map_err(|why| NotRecorded(format!("{dir} cannot be read: {why}")))?;
        let compiler = Compiler::of(&root)?;
        let tools = compiler.llvm_tools()?;
        let target = root.join("target").join("test-map");
        let laps = target.join("laps");
        let stray = target.join("stray");
        for folder in [&laps, &stray] {
            let _ = std::fs::remove_dir_all(folder);
            std::fs::create_dir_all(folder).map_err(|why| NotRecorded(format!("{}: {why}", folder.display())))?;
        }
        let runner = Runner::written()?;

        let mut run = command("cargo");
        run.current_dir(&root).args(self.arguments(tests));
        // A Check's own cap: nothing where the caller hands it a pool.
        run.envs(build_cap(self.width));
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("NEXTEST_") {
                run.env_remove(&name);
            }
        }
        let rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
        let ran = run
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env("RUSTFLAGS", format!("{rustflags} -C instrument-coverage").trim_start())
            .env("CARGO_TARGET_DIR", &target)
            .env(compiler.runner_variable(), &runner.0)
            .env("ARMADA_TEST_MAP_LAPS", &laps)
            // Build scripts and the listing are instrumented too, and would
            // otherwise write a profile into the directory they ran in.
            .env("LLVM_PROFILE_FILE", stray.join("%p-%m.profraw"))
            .output()
            .map_err(|why| NotRecorded(format!("cargo nextest would not start: {why}")))?;

        let measured = measured_in(&laps)?;
        if measured.is_empty() {
            return Err(NotRecorded(format!(
                "no test wrote a profile: {}",
                last_lines(&String::from_utf8_lossy(&ran.stderr), 12)
            )));
        }
        measured
            .into_iter()
            .map(|((_, test), run)| {
                let lcov = tools.export(&run)?;
                Ok(Lap {
                    member: member_of(&run.cwd, &root, self.under.as_deref()),
                    test,
                    grain: LapGrain::Test,
                    files: files_reached(&lcov)
                        .into_iter()
                        .filter_map(|file| in_repository(&file, &root, &target))
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                })
            })
            .collect()
    }
}

impl NextestCoverage {
    pub(crate) fn arguments(&self, tests: &TestsToRecord) -> Vec<String> {
        let mut arguments: Vec<String> = ["nextest", "run", "--workspace", "--no-fail-fast"].map(String::from).to_vec();
        arguments.extend(["--test-threads".to_string(), self.width.get().to_string()]);
        for member in &self.except {
            arguments.extend(["--exclude".to_string(), member.clone()]);
        }
        if let TestsToRecord::Named(names) = tests {
            // Matched as whole segments of a test's path, as the `test` Check's
            // `one_test` matches a name.
            let filter = names
                .iter()
                .map(|name| format!("test(/(^|::){}(::|$)/)", regex_escaped(name)))
                .collect::<Vec<_>>()
                .join(" | ");
            arguments.extend(["-E".to_string(), filter]);
        }
        arguments
    }
}

fn regex_escaped(text: &str) -> String {
    text.chars()
        .flat_map(|c| ("\\^$.|?*+()[]{}/".contains(c).then_some('\\')).into_iter().chain([c]))
        .collect()
}

/// The runner script, written outside the checkout: cargo splits a runner on
/// spaces, and a checkout's path may hold one.
struct Runner(PathBuf);

impl Runner {
    fn written() -> Result<Runner, NotRecorded> {
        let path = std::env::temp_dir().join(format!(
            "armada-test-map-runner-{}-{}",
            std::process::id(),
            NEXT_RUNNER.fetch_add(1, Ordering::Relaxed)
        ));
        if path.to_string_lossy().contains(char::is_whitespace) {
            return Err(NotRecorded(format!("the runner's path has a space in it: {}", path.display())));
        }
        std::fs::write(&path, RUNNER).map_err(|why| NotRecorded(format!("{}: {why}", path.display())))?;
        set_executable(&path)?;
        Ok(Runner(path))
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), NotRecorded> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|why| NotRecorded(format!("{}: {why}", path.display())))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), NotRecorded> {
    Err(NotRecorded("the runner is a shell script, and this is not a unix".to_string()))
}

/// One test's run: its binary, the directory it ran in, and every profile it
/// wrote, across retries.
#[derive(Default)]
struct MeasuredRun {
    binary: PathBuf,
    cwd: PathBuf,
    profiles: Vec<PathBuf>,
}

/// Every test that wrote a profile, by binary id and test name. A test that
/// wrote none, as one that aborted, is left out rather than said to reach
/// nothing.
fn measured_in(laps: &Path) -> Result<BTreeMap<(String, String), MeasuredRun>, NotRecorded> {
    let unreadable = |why: std::io::Error| NotRecorded(format!("{}: {why}", laps.display()));
    let mut measured: BTreeMap<(String, String), MeasuredRun> = BTreeMap::new();
    for lap in std::fs::read_dir(laps).map_err(unreadable)? {
        let lap = lap.map_err(unreadable)?.path();
        let note = std::fs::read_to_string(lap.join("test")).unwrap_or_default();
        let Ok([binary_id, test, binary, cwd]) =
            <[String; 4]>::try_from(note.lines().map(str::to_string).collect::<Vec<_>>())
        else {
            continue;
        };
        let profiles: Vec<PathBuf> = std::fs::read_dir(&lap)
            .map_err(unreadable)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|extension| extension == "profraw"))
            .collect();
        if profiles.is_empty() {
            continue;
        }
        let run = measured.entry((binary_id, test)).or_default();
        run.binary = PathBuf::from(binary);
        run.cwd = PathBuf::from(cwd);
        run.profiles.extend(profiles);
    }
    Ok(measured)
}

/// The compiler cargo runs in a checkout, as it describes itself.
struct Compiler {
    host: String,
    llvm_major: u32,
    sysroot: PathBuf,
}

impl Compiler {
    fn of(root: &Path) -> Result<Compiler, NotRecorded> {
        let described = said(command("rustc").current_dir(root).arg("-vV"))?;
        let host = described
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .ok_or_else(|| NotRecorded(format!("rustc -vV named no host: {described}")))?
            .to_string();
        let llvm_major = llvm_major(&described)
            .ok_or_else(|| NotRecorded(format!("rustc -vV named no LLVM version: {described}")))?;
        let sysroot = PathBuf::from(said(command("rustc").current_dir(root).args(["--print", "sysroot"]))?.trim());
        Ok(Compiler { host, llvm_major, sysroot })
    }

    fn runner_variable(&self) -> String {
        format!("CARGO_TARGET_{}_RUNNER", self.host.to_uppercase().replace(['-', '.'], "_"))
    }

    /// `llvm-profdata` and `llvm-cov` of the compiler's own LLVM: a profile is
    /// read only by the version that wrote it.
    fn llvm_tools(&self) -> Result<LlvmTools, NotRecorded> {
        for folder in tool_folders(&self.sysroot, &self.host, self.llvm_major) {
            let tools = LlvmTools { profdata: folder.join("llvm-profdata"), cov: folder.join("llvm-cov") };
            if !(tools.profdata.is_file() && tools.cov.is_file()) {
                continue;
            }
            let version = said(command(&tools.profdata.to_string_lossy()).arg("--version")).unwrap_or_default();
            if llvm_major(&version) == Some(self.llvm_major) {
                return Ok(tools);
            }
        }
        Err(NotRecorded(format!(
            "no llvm-profdata and llvm-cov of LLVM {} beside the compiler at {}; \
             `rustup component add llvm-tools-preview` puts them there",
            self.llvm_major,
            self.sysroot.display()
        )))
    }
}

/// Where the LLVM tools of a compiler at `sysroot` are looked for, in order:
/// rustup's `llvm-tools-preview` component, then the LLVM a Homebrew `rust`
/// is built against, which sits under the same prefix.
pub(crate) fn tool_folders(sysroot: &Path, host: &str, llvm_major: u32) -> Vec<PathBuf> {
    let mut folders = vec![sysroot.join("lib").join("rustlib").join(host).join("bin")];
    let cellar = sysroot.parent().and_then(Path::parent);
    if let Some(prefix) = cellar.filter(|cellar| cellar.ends_with("Cellar")).and_then(Path::parent) {
        folders.push(prefix.join("opt").join(format!("llvm@{llvm_major}")).join("bin"));
        folders.push(prefix.join("opt").join("llvm").join("bin"));
    }
    folders
}

/// The major version in `LLVM version: 22.1.8`, as rustc and the LLVM tools
/// each print it.
pub(crate) fn llvm_major(text: &str) -> Option<u32> {
    let (_, after) = text.split_once("LLVM version")?;
    let digits: String = after
        .trim_start_matches([':', ' '])
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

struct LlvmTools {
    profdata: PathBuf,
    cov: PathBuf,
}

impl LlvmTools {
    /// One test's profiles, merged and exported against its binary as lcov.
    fn export(&self, run: &MeasuredRun) -> Result<String, NotRecorded> {
        let lap = run.profiles[0].parent().unwrap_or(Path::new("."));
        let merged = lap.join("merged.profdata");
        said(
            command(&self.profdata.to_string_lossy())
                .args(["merge", "-sparse", "-o"])
                .arg(&merged)
                .args(&run.profiles),
        )?;
        said(
            command(&self.cov.to_string_lossy())
                .args(["export", "-format=lcov"])
                .arg(format!("-instr-profile={}", merged.display()))
                .arg(&run.binary),
        )
    }
}

/// Standard output of a command that has to succeed.
fn said(command: &mut Command) -> Result<String, NotRecorded> {
    // The whole line: at low priority the program is the wrap, not the tool.
    let named = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|word| word.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let out = command
        .output()
        .map_err(|why| NotRecorded(format!("{named} would not start: {why}")))?;
    if !out.status.success() {
        return Err(NotRecorded(format!(
            "{named} failed: {}",
            last_lines(&String::from_utf8_lossy(&out.stderr), 12)
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn last_lines(text: &str, count: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(count)..].join("\n")
}

/// Every source file an lcov export ran a line of, as the export names it.
pub(crate) fn files_reached(lcov: &str) -> Vec<String> {
    let mut reached = Vec::new();
    let mut file: Option<&str> = None;
    for line in lcov.lines() {
        if let Some(named) = line.strip_prefix("SF:") {
            file = Some(named);
        } else if let Some(counted) = line.strip_prefix("DA:") {
            let ran = counted
                .split(',')
                .nth(1)
                .and_then(|count| count.trim().parse::<u64>().ok())
                .is_some_and(|count| count > 0);
            if let Some(named) = file.filter(|_| ran) {
                reached.push(named.to_string());
                file = None;
            }
        } else if line == "end_of_record" {
            file = None;
        }
    }
    reached
}

/// `file` relative to the repository at `root`, `/`-separated. `None` for a
/// file outside it, as the registry's and the toolchain's are, and for one the
/// build generated under `target`.
pub(crate) fn in_repository(file: &str, root: &Path, target: &Path) -> Option<String> {
    let file = Path::new(file);
    if file.starts_with(target) {
        return None;
    }
    let relative = file.strip_prefix(root).ok()?;
    let parts: Vec<String> = relative.iter().map(|part| part.to_string_lossy().into_owned()).collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The member a test binary belongs to, from the package directory it ran in:
/// the child of `under` that holds it. Empty where the Check has no `under`
/// or the package is not below it.
pub(crate) fn member_of(cwd: &Path, root: &Path, under: Option<&str>) -> String {
    under
        .and_then(|under| cwd.strip_prefix(root.join(under.trim_end_matches('/'))).ok())
        .and_then(|below| below.iter().next())
        .map(|member| member.to_string_lossy().into_owned())
        .unwrap_or_default()
}

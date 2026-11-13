//! `armada disk reclaim`: one pass under pressure now, with no Fleet. Fleet drops build output on
//! its own every minute, so this is for a disk that is short before one is running, or while the
//! Fleet running is older than the reclaim: a restart calls it before it builds.
//! `fleet::sweeping::reclaim_root` is the pass, and `adapters::leasing::disk` is the order.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use config::Manifest;
use fleet::headroom::{Bytes, Machine, TheMachine};
use fleet::runtime;

use crate::loopback::Loopback;
use crate::setup::MANIFEST;

/// How long to wait for Fleet to name the repositories it serves.
const FLEET_WITHIN: Duration = Duration::from_secs(5);

/// What `armada disk reclaim --help` prints.
pub const HELP: &str = "armada disk reclaim: drop build output, in the order Fleet's own pass does, until free space is 20 GiB over limits.diskReclaimFloorGib.\n\
    It looks at the repository here and every repository a running Fleet serves, and needs no Fleet.\n\
    It does nothing while free space is at or over the floor, and a floor of 0 is off.";

/// Each repository to look at once, with the branch its slots lease from: the one `cwd` is in, then
/// each of `served`, none twice.
pub fn reach(cwd: &Path, served: &[PathBuf]) -> Vec<(PathBuf, String)> {
    let here = cwd.ancestors().find(|folder| folder.join(".git").exists()).map(Path::to_path_buf);
    let mut roots: Vec<PathBuf> = Vec::new();
    for root in here.into_iter().chain(served.iter().cloned()) {
        let same = |other: &PathBuf| other == &root || other.canonicalize().ok().is_some_and(|other| Some(other) == root.canonicalize().ok());
        if !roots.iter().any(same) {
            roots.push(root);
        }
    }
    roots
        .into_iter()
        .map(|root| {
            let base = Manifest::load(&root.join(MANIFEST)).ok().and_then(|manifest| manifest.base().map(str::to_string));
            (root, base.unwrap_or_else(|| "main".to_string()))
        })
        .collect()
}

/// One pass over each of `roots` under `floor`, a line to `said` for each folder removed whole and
/// one for each pass. `free_at` reads the free bytes on the volume holding a path.
pub fn reclaim_now(roots: &[(PathBuf, String)], floor: u64, free_at: &dyn Fn(&Path) -> Option<u64>, said: &mut dyn FnMut(&str)) {
    for (root, base) in roots {
        fleet::sweeping::reclaim_root(root, base, floor, free_at, said);
    }
}

/// The repositories the Fleet on this machine serves, where one answers.
fn served_by_fleet(home: &str) -> Vec<PathBuf> {
    let at = runtime::support_dir(home).join(runtime::FILE_NAME);
    let Ok(port) = crate::mcp::listening(runtime::read(&at), &at) else {
        return Vec::new();
    };
    let Ok(answer) = Loopback::at(port).get_within("/repositories", FLEET_WITHIN) else {
        return Vec::new();
    };
    if answer.status != 200 {
        return Vec::new();
    }
    ipc::decode::<ipc::RepositoryList>("the repositories", &answer.body)
        .map(|list| list.repositories.into_iter().map(|one| PathBuf::from(one.root)).collect())
        .unwrap_or_default()
}

/// The verb: `--help` says what it does; otherwise one pass, printing its lines. Exits 0 whether or
/// not anything was dropped, and 1 only where the working directory cannot be read.
pub fn run(help: bool) -> ExitCode {
    if help {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let Ok(cwd) = std::env::current_dir() else {
        eprintln!("the working directory could not be read");
        return ExitCode::FAILURE;
    };
    let home = std::env::var("HOME").unwrap_or_default();
    let floor = crate::settings::now_or_shipped().get(config::settings::DISK_RECLAIM_FLOOR_GIB);
    let floor = Bytes::gibibytes(u64::try_from(floor).unwrap_or(0)).count();
    let machine = TheMachine::watching("/");
    let roots = reach(&cwd, &served_by_fleet(&home));
    reclaim_now(&roots, floor, &|path| machine.disk_free_at(path).map(|free| free.count()), &mut |line| println!("{line}"));
    ExitCode::SUCCESS
}

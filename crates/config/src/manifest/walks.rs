//! `walks:`, how a repository's own app is built, listed, opened and pictured as a walk on mock data.
//! `docs/concepts/manifest.md`, *Walks*.
//!
//! **Commands written as a `run` is, with `${name}` where Fleet puts a value.** Fleet splits each
//! into words first and substitutes into each word after, so a path or a URL with a space in it
//! stays one argument. A name a key is not given, and a key that never names what it must, are
//! refused at load: a declaration that cannot run is found when the file is read and not when a
//! Job first shows a walk.
//!
//! **Read from the root only**, as `connectors` is: the tree Fleet builds is the repository's, and
//! a workspace's own value would be read by nothing.

use super::Manifest;
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside `walks`.
const WALK_KEYS: &[&str] = &["build", "binary", "list", "open", "take", "shots", "against", "seed"];

/// What a key may name, which is also what it must: a command that leaves one out is a command
/// that cannot do its one job.
struct Given(&'static [&'static str]);

const BUILD: Given = Given(&[]);
const LIST: Given = Given(&["binary"]);
const OPEN: Given = Given(&["binary", "walk", "job", "url", "fleet_port"]);
const TAKE: Given = Given(&["name", "walk"]);
const SHOTS: Given = Given(&["binary", "walk", "shots"]);
const AGAINST: Given = Given(&["binary", "walk", "shots", "against", "out"]);

/// How a repository's walks are made, kept and compared. Every field but [`Walks::binary`], which
/// is one path, is a command line as the file wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walks {
    pub(super) build: String,
    pub(super) binary: String,
    pub(super) list: String,
    pub(super) open: String,
    pub(super) take: Option<String>,
    pub(super) shots: String,
    pub(super) against: String,
    pub(super) seed: Option<String>,
}

impl Walks {
    /// Builds the app in a tree of the Job's commit, run in the tree's root. Names no `${...}`.
    pub fn build(&self) -> &str {
        &self.build
    }

    /// Where `build` leaves the program, under the tree. **Fleet keeps a copy of this one file** for
    /// each commit and hands every other command its path as `${binary}`.
    pub fn binary(&self) -> &str {
        &self.binary
    }

    /// Prints the walks the program plays, one name a line. Given `${binary}`.
    pub fn list(&self) -> &str {
        &self.list
    }

    /// Opens one walk on the person's screen and stays up until it is closed. Given `${binary}`,
    /// `${walk}`, `${job}`, `${url}` (the address the Job showed) and `${fleet_port}`.
    pub fn open(&self) -> &str {
        &self.open
    }

    /// The arguments one take adds to `open`, once a take, given `${name}` and `${walk}`. **`None`
    /// is a repository whose walks have no takes**, and a Job showing one with takes is refused.
    pub fn take(&self) -> Option<&str> {
        self.take.as_deref()
    }

    /// Plays one walk with no one watching and writes a picture a step into `${shots}`, exiting zero
    /// where it played. Given `${binary}`, `${walk}` and `${shots}`.
    pub fn shots(&self) -> &str {
        &self.shots
    }

    /// `shots`, and then a comparison of what it drew with the pictures in `${against}`, with a
    /// `report.json` written into `${out}`. **Exit 0 is the same and 1 is a drift**, and any other
    /// code is a walk that could not be compared. Given `${binary}`, `${walk}`, `${shots}`,
    /// `${against}` and `${out}`.
    pub fn against(&self) -> &str {
        &self.against
    }

    /// A folder under the tree a first build starts from, **cloned from the same folder of the
    /// repository's preview checkout** where it has one. `None` builds cold the first time.
    pub fn seed(&self) -> Option<&str> {
        self.seed.as_deref()
    }
}

impl Walks {
    /// The native Bridge's own declaration, for a repository that holds it and declares none. Goes once armada.yml declares `walks:` itself.
    pub fn native_bridge() -> Walks {
        let said = |text: &str| text.to_string();
        Walks {
            build: said("env CARGO_PROFILE_RELEASE_INCREMENTAL=true cargo build --release --manifest-path crates/bridge-native/Cargo.toml --target-dir crates/bridge-native/target"),
            binary: said("crates/bridge-native/target/release/bridge-native"),
            list: said("${binary} --list-walks"),
            open: said("${binary} --walk ${walk} --job ${job} --url ${url} --fleet-port ${fleet_port}"),
            take: Some(said("--take ${name}=${walk}")),
            shots: said("${binary} --walk ${walk} --check --shots ${shots} --hold 0"),
            against: said("${binary} --walk ${walk} --check --shots ${shots} --against ${against} --out ${out} --hold 0"),
            seed: Some(said("crates/bridge-native/target")),
        }
    }
}

/// The names written as `${name}` in `text`, in order, or the rest of the text from a `${` that is
/// never closed.
fn names_in(text: &str) -> Result<Vec<&str>, &str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("${") {
        let after = &rest[at + 2..];
        let end = after.find('}').ok_or(&rest[at..])?;
        found.push(&after[..end]);
        rest = &after[end + 1..];
    }
    Ok(found)
}

/// The command at `name`, kept where it names all of what `given` allows and nothing else.
fn command(table: &mut Table<'_>, name: &str, given: &Given, required: bool, out: &mut Vec<Refusal>) -> Option<String> {
    let value = if required { table.required(name, out) } else { table.optional(name) }?;
    let at = table.at(name);
    let text = yaml::text(&at, value, out)?;
    let names = match names_in(&text) {
        Ok(names) => names,
        Err(unclosed) => {
            out.push(Refusal::new(at, Fault::NotAPlaceholder { value: unclosed.to_string(), legal: given.0.to_vec() }));
            return None;
        }
    };
    let before = out.len();
    for found in names.iter().filter(|found| !given.0.contains(*found)) {
        out.push(Refusal::new(&at, Fault::NotAPlaceholder { value: format!("${{{found}}}"), legal: given.0.to_vec() }));
    }
    for must in given.0.iter().filter(|must| !names.contains(*must)) {
        out.push(Refusal::new(&at, Fault::LeavesOutPlaceholder { placeholder: *must }));
    }
    (out.len() == before).then_some(text)
}

/// The path at `name`, kept where it names a file or folder inside the tree.
fn path(table: &mut Table<'_>, name: &str, required: bool, out: &mut Vec<Refusal>) -> Option<String> {
    let value = if required { table.required(name, out) } else { table.optional(name) }?;
    let at = table.at(name);
    let value = yaml::text(&at, value, out)?;
    match super::seed::refused(&value) {
        None => Some(value),
        Some(why) => {
            out.push(Refusal::new(at, Fault::NotAWalkPath { value, why }));
            None
        }
    }
}

/// `walks:`, or `None` where the file declares none. Every key but `take` is required.
pub(super) fn read(top: &mut Table<'_>, in_a_workspace: bool, out: &mut Vec<Refusal>) -> Option<Walks> {
    let value = top.optional("walks")?;
    if in_a_workspace {
        out.push(Refusal::new("walks", Fault::RootOnly));
        return None;
    }
    let mut table = Table::open("walks", value, out)?;
    let build = command(&mut table, "build", &BUILD, true, out);
    let list = command(&mut table, "list", &LIST, true, out);
    let open = command(&mut table, "open", &OPEN, true, out);
    let take = command(&mut table, "take", &TAKE, false, out);
    let shots = command(&mut table, "shots", &SHOTS, true, out);
    let against = command(&mut table, "against", &AGAINST, true, out);
    let binary = path(&mut table, "binary", true, out);
    let seed = path(&mut table, "seed", false, out);
    table.close(WALK_KEYS, out);
    Some(Walks { build: build?, binary: binary?, list: list?, open: open?, take, shots: shots?, against: against?, seed })
}

impl Manifest {
    /// How this repository's app is built, opened and pictured as a walk, **`None` where the file
    /// declares no `walks:`**, which is a repository Fleet cannot show a native walk of. Read from
    /// the Manifest Fleet started with, not live.
    pub fn walks(&self) -> Option<&Walks> {
        self.walks.as_ref()
    }
}

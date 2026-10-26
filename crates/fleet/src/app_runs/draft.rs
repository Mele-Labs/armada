//! A run proposed for a repository: from the dev servers its Jobs started, and from what its own
//! files say. **Where it already declares runs, only what none of them starts is proposed.** **A draft saves nothing**: it is what a person reads, changes and
//! confirms, and [`super::save`] writes the confirmed one.
//!
//! **Run memory first, then the files.** A `dev_server` fact is a command that came up on a port
//! for somebody, so it beats a guess from `package.json`; the files only fill a folder memory says
//! nothing of. One process per folder: the newest fact that succeeded.
//!
//! **A remembered command is proposed only if what it names will be there** ([`setup`]), and the
//! draft proposes the setup that makes it: an install for each lockfile, a `.venv` for each Python
//! folder. **Every process listens on a leased port**, named for it.
//!
//! **A fact is cleaned before it is proposed.** The stream a fact is read from carries the shell
//! around the command (`... --strictPort /tmp/x.log &)`), and Fleet starts a server by exec, with
//! no shell, so a redirect or a `&` would be handed to the program as an argument. A literal port
//! becomes `${port.NAME}`, so each checkout gets a span of its own.

mod files;
mod setup;

use std::path::Path;

use store::KeptRun;

/// What a draft process is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DraftSource {
    /// A dev server a Job started.
    RunMemory,
    /// The `dev` script of a `package.json`.
    PackageJson,
    /// A Python app that serves with uvicorn.
    Python,
    /// A person's own words, as the wire sends a confirmed draft.
    Confirmed,
}

/// The port a process answers on. **Named, never a number**, so two checkouts of one repository
/// do not start two servers on one port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DraftPort {
    /// The `ports:` entry, and the `${port.NAME}` in the process's lines.
    pub(crate) name: String,
    /// The variable Armada sets to it, for a program that reads its port from the environment.
    pub(crate) env: Option<String>,
}

/// One server of a drafted run, written as a `commands.<name>` with `serve`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DraftProcess {
    /// The Command's name, and the process's name in the run.
    pub(crate) name: String,
    pub(crate) serve: String,
    /// The folder under the checkout it starts in. `None` is the root.
    pub(crate) dir: Option<String>,
    /// `None` where nothing in what it was read from names a port.
    pub(crate) port: Option<DraftPort>,
    pub(crate) from: DraftSource,
}

/// A Command a checkout runs before its app can: written as a `commands.<name>` with `run`, and
/// named in `setup.requires`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DraftSetup {
    pub(crate) name: String,
    /// The line, run at the checkout's root.
    pub(crate) run: String,
    /// The path under the root it makes, where it makes one: `frontend/node_modules`, `.venv`.
    pub(crate) creates: Option<String>,
}

/// A run proposed whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunDraft {
    /// The run's key under `runs:`.
    pub(crate) name: String,
    /// What a person is shown for it.
    pub(crate) label: Option<String>,
    pub(crate) processes: Vec<DraftProcess>,
    /// What the checkout runs first, in order. Saved into `setup.requires`.
    pub(crate) setup: Vec<DraftSetup>,
    /// Where the run is Kit's already, what saving it again takes out first. Set by
    /// [`super::replacing::replacing`], never by a draft.
    pub(crate) replacing: Option<super::replacing::Replacing>,
}

/// What a repository's saved runs already hold, so a draft proposes only the rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Beside {
    /// The folders a saved server starts in, `.` for the root.
    pub(crate) dirs: Vec<String>,
    /// Every name a Command, server or port is declared under: a proposal never takes one, since
    /// Kit's copy would win by name and change what it is.
    pub(crate) reserved: Vec<String>,
    /// The line of every Command declared, so a setup already saved is not proposed twice.
    pub(crate) runs: Vec<String>,
    /// What `setup.requires` already names: a proposal of one of these names was saved before,
    /// perhaps with its line edited.
    pub(crate) required: Vec<String>,
}

/// The run `facts` and the files under `root` suggest, or `None` where neither names a server.
/// **Only tests draft with nothing saved beside**: the wire always has a [`Beside`].
#[cfg(test)]
pub(crate) fn drafted(root: &Path, facts: &[KeptRun]) -> Option<RunDraft> {
    drafted_beside(root, facts, &Beside::default())
}

/// [`drafted`] leaving out every process in a folder `beside` says a saved server starts in.
pub(crate) fn drafted_beside(root: &Path, facts: &[KeptRun], beside: &Beside) -> Option<RunDraft> {
    let folders = files::folders(root);
    let mut setup = setup::proposed(&folders);
    setup.retain(|one| !beside.runs.contains(&one.run) && !beside.required.contains(&one.name));
    let mut processes = from_memory(root, facts);
    let named: Vec<Vec<String>> =
        processes.iter().map(|one| setup::named_paths(&one.serve, one.dir.as_deref())).collect();
    let asked: Vec<String> = named.iter().flatten().filter(|path| !setup::made_by(path, &setup)).cloned().collect();
    let answered = setup::tracked(root, &asked);
    let there = |path: &String| {
        setup::made_by(path, &setup) || asked.iter().zip(&answered).any(|(one, tracked)| one == path && *tracked)
    };
    let mut kept = named.iter().map(|paths| paths.iter().all(there));
    processes.retain(|_| kept.next().unwrap_or(false));
    let taken: Vec<String> = processes
        .iter()
        .map(|one| one.dir.clone().unwrap_or_else(|| String::from(".")))
        .collect();
    for found in files::found(root, &folders) {
        let dir = found.dir.clone().unwrap_or_else(|| String::from("."));
        if !taken.contains(&dir) {
            processes.push(found);
        }
    }
    processes.retain(|one| !beside.dirs.contains(&one.dir.clone().unwrap_or_else(|| String::from("."))));
    unique_names(&mut processes, &mut setup, &beside.reserved);
    if processes.is_empty() {
        return None;
    }
    Some(RunDraft {
        name: String::from("dev"),
        label: root.file_name().map(|name| name.to_string_lossy().into_owned()),
        processes,
        setup,
        replacing: None,
    })
}

/// One process per folder, newest first among the facts that succeeded, each on a leased port.
fn from_memory(root: &Path, facts: &[KeptRun]) -> Vec<DraftProcess> {
    let mut newest: Vec<(String, &KeptRun)> = Vec::new();
    for fact in facts.iter().filter(|fact| fact.kind == "dev_server" && fact.succeeded) {
        let Some(folder) = in_the_repository(&fact.workspace) else {
            continue;
        };
        match newest.iter_mut().find(|(held, _)| *held == folder) {
            Some((_, held)) if held.last_ran >= fact.last_ran => {}
            Some((_, held)) => *held = fact,
            None => newest.push((folder, fact)),
        }
    }
    newest
        .into_iter()
        .filter_map(|(folder, fact)| {
            let line = cleaned(&fact.command)?;
            let name = name_of(root, &folder, &line);
            let (serve, env) = match fact.port.and_then(|port| placed(&line, port, &name)) {
                Some(serve) => (serve, None),
                None => leased(&line, &name),
            };
            Some(DraftProcess {
                name: name.clone(),
                serve,
                dir: (folder != ".").then_some(folder),
                port: Some(DraftPort { name, env }),
                from: DraftSource::RunMemory,
            })
        })
        .collect()
}

/// `line` given the port leased as `name`, where it names none: the flag its program reads, after
/// any number it was given. The variable, for a program known by none.
fn leased(line: &str, name: &str) -> (String, Option<String>) {
    let port = format!("${{port.{name}}}");
    let words: Vec<&str> = line.split_whitespace().collect();
    let runs = |program: &str| words.iter().any(|word| *word == program || word.ends_with(&format!("/{program}")));
    if runs("vite") {
        return (format!("{} --port {port} --strictPort", without_port(line)), None);
    }
    if runs("uvicorn") {
        return (format!("{} --port {port}", without_port(line)), None);
    }
    match words.as_slice() {
        ["npm", "run", ..] if words.contains(&"--") => (format!("{line} --port {port}"), Some(String::from("PORT"))),
        ["npm", "run", ..] => (format!("{line} -- --port {port}"), Some(String::from("PORT"))),
        _ => (format!("{line} --port {port}"), Some(String::from("PORT"))),
    }
}

/// `line` without a `--port N`, `--port=N` or `--strictPort` it already holds.
pub(super) fn without_port(line: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut words = line.split_whitespace();
    while let Some(word) = words.next() {
        match word {
            "--port" => {
                words.next();
            }
            "--strictPort" => {}
            port if port.starts_with("--port=") => {}
            other => kept.push(other),
        }
    }
    kept.join(" ")
}

/// The folder a fact's workspace names inside the repository, `.` for its root. **A slot's copy is
/// the same folder**: a Job worked in `.armada/slots/slot-4/apps/web`, and the facts it left
/// belong to `apps/web`. A path outside the repository is none.
pub(super) fn in_the_repository(workspace: &str) -> Option<String> {
    let parts: Vec<&str> = workspace.split('/').filter(|part| !part.is_empty() && *part != ".").collect();
    let parts = match parts.as_slice() {
        [".armada", "slots", _, rest @ ..] => rest,
        other => other,
    };
    if workspace.starts_with('/') || parts.contains(&"..") {
        return None;
    }
    Some(match parts.is_empty() {
        true => String::from("."),
        false => parts.join("/"),
    })
}

/// The command a fact recorded, without the shell around it. `None` where nothing is left, or what
/// is left is not a program Fleet can start by exec: a leading `cd` or an assignment.
fn cleaned(command: &str) -> Option<String> {
    let mut kept: Vec<&str> = Vec::new();
    for token in command.split_whitespace() {
        let bare = token.trim_end_matches(['&', ')']);
        let operator = matches!(token, "&" | "&&" | ";" | "|" | "||" | ">" | ">>" | "2>&1");
        let log = bare.starts_with("/tmp/") || bare.ends_with(".log") || bare.ends_with(".pid");
        if operator || log || bare.is_empty() {
            break;
        }
        kept.push(bare);
        // A token that carried the `&` or `)` ends the command.
        if bare.len() != token.len() {
            break;
        }
    }
    if let Some(first) = kept.first_mut() {
        *first = first.trim_start_matches('(');
    }
    let launching = |token: &&str| matches!(*token, "nohup" | "exec" | "env" | "");
    let program: Vec<&str> = kept.into_iter().skip_while(launching).collect();
    let first = program.first()?;
    if *first == "cd" || first.contains('=') {
        return None;
    }
    Some(program.join(" "))
}

/// `line` with every standalone `port` replaced by `${port.NAME}`, or `None` where it names none.
fn placed(line: &str, port: u16, name: &str) -> Option<String> {
    let digits = port.to_string();
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut found = false;
    let mut at = 0;
    while at < chars.len() {
        if !chars[at].is_ascii_digit() {
            out.push(chars[at]);
            at += 1;
            continue;
        }
        let end = chars[at..].iter().position(|c| !c.is_ascii_digit()).map_or(chars.len(), |n| at + n);
        let run: String = chars[at..end].iter().collect();
        match run == digits {
            true => {
                found = true;
                out.push_str(&format!("${{port.{name}}}"));
            }
            false => out.push_str(&run),
        }
        at = end;
    }
    found.then_some(out)
}

/// A name for a process: its folder's last segment. **At the root, the top-level folder an
/// argument points into**, `backend` for `uvicorn backend.app.main:app`, else `app`: never the
/// program, which says nothing of what is served.
fn name_of(root: &Path, folder: &str, line: &str) -> String {
    let word = match folder {
        "." => line
            .split_whitespace()
            .enumerate()
            .filter(|(at, word)| (*at > 0 || word.contains('/')) && !word.starts_with(['-', '.', '/']))
            .filter_map(|(_, word)| word.split(['/', '.', ':']).next())
            .find(|part| !part.is_empty() && root.join(part).is_dir())
            .unwrap_or("app"),
        folder => folder.rsplit('/').next().unwrap_or(folder),
    };
    safe_name(word)
}

/// A name that is a YAML key and a `${port.NAME}` without quoting.
pub(super) fn safe_name(word: &str) -> String {
    let name: String = word
        .rsplit('/')
        .next()
        .unwrap_or(word)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let name = name.trim_matches('_').to_string();
    match name.is_empty() {
        true => String::from("dev"),
        false => name,
    }
}

/// Two processes or setup Commands never share a name: the second is `name_2`, the third `name_3`.
/// The port follows.
fn unique_names(processes: &mut [DraftProcess], setup: &mut [DraftSetup], reserved: &[String]) {
    let mut seen: Vec<String> = reserved.to_vec();
    let mut fresh = |base: &str| {
        let mut name = base.to_string();
        let mut n = 2;
        while seen.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        seen.push(name.clone());
        name
    };
    for process in processes {
        let base = process.name.clone();
        let name = fresh(&base);
        if name != base {
            process.serve = process.serve.replace(&format!("${{port.{base}}}"), &format!("${{port.{name}}}"));
            if let Some(port) = &mut process.port {
                port.name = name.clone();
            }
            process.name = name;
        }
    }
    for one in setup {
        one.name = fresh(&one.name);
    }
}

//! What was typed, turned into one of four things it could have been.
//!
//! # Parsed by hand, and that is not a stopgap
//!
//! There is no argument-parsing dependency and no workspace dependency table to
//! add one to. A handful of verbs, one optional positional and two flags is less surface
//! than the derive macro that would read it, and a crate added here is added to
//! the binary every other crate links into.
//!
//! # Every fault, and never the first one
//!
//! `config` reports every refusal in a Manifest rather than stopping, and a
//! command line is read the same way: an unknown flag beside a missing name is
//! two lines, so one correction fixes both.

mod test_map;
mod worktree;

use std::fmt;
use std::path::PathBuf;

pub use test_map::TestMapAct;
pub use worktree::WorktreeAct;

/// What the caller asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    /// The daemon. Everything below it needs a port, a store and a process.
    /// `repository` is added at start; none given serves what Fleet remembers.
    Serve { repository: Option<PathBuf> },
    /// One Check the Manifest declares, by name — or tests in it, through the
    /// Check's `one_test` in one run, or what the paths on stdin reach, with
    /// `changed`.
    Check {
        name: String,
        tests: Vec<String>,
        changed: bool,
    },
    /// One Command the Manifest declares, by name.
    Run { name: String },
    /// The Checks a change hits, from the paths read on stdin.
    Covers,
    /// Base checkouts, given back; with `--all`, the machine's store as well.
    Clean { everything: bool },
    /// The agent's door, spoken on stdin and stdout for an agent standing in
    /// this repository. Started by an agent's MCP client, never by a person.
    Mcp,
    /// Hold one hosted session's agent so it outlives a Fleet restart. Started
    /// by Fleet, with `fleet::session_host::keeper::run`'s arguments.
    SessionKeep { args: Vec<String> },
    /// The pool of warm worktrees — `docs/concepts/fleet.md`, *Worktree slots*.
    Worktree(WorktreeAct),
    /// The test map's laps for one Check, measured here and written to a file.
    TestMap(TestMapAct),
    /// One reading of a watch recipe Armada ships, printed as the line Fleet reads. Fleet runs it.
    WatchLine { reader: String, values: Vec<String> },
    /// What the verbs are.
    Help,
}

/// The verbs, in the order the usage prints them, each with what it is for.
///
/// One list, read by both the parser and the usage text — a second copy is how
/// a verb gets added and stays undocumented.
const VERBS: &[(&str, &str)] = &[
    (
        "serve",
        "run Fleet against a repository until it is signalled",
    ),
    (
        "check",
        "run one Check the Manifest declares, by name; names after it run those tests in it \
         in one build, and --changed what the paths on stdin reach",
    ),
    ("run", "run one Command the Manifest declares, by name"),
    (
        COVERS,
        "name the Checks a change hits, from the changed paths on stdin",
    ),
    (
        "clean",
        "give this repository's base checkouts back",
    ),
    (
        MCP,
        "relay this repository's agent door on stdin and stdout — an agent's client runs it",
    ),
    (
        SESSION_KEEP,
        "hold a hosted job's agent across a Fleet restart — Fleet runs it",
    ),
    (
        WORKTREE,
        "lease a warm worktree, give it back, or list who holds each",
    ),
    (
        TEST_MAP,
        "measure which tests reach which files, for CI to hand Fleet",
    ),
    (
        WATCH_LINE,
        "print one reading of a watch recipe Armada ships, as the line Fleet reads; Fleet runs it",
    ),
];

/// The verb an agent's MCP configuration names.
///
/// **One value, read by the parser and written into the file.** The entry
/// `crate::mcp::publish` puts in a repository's `.mcp.json` names this verb, so
/// a rename that missed one of the two would publish a door nothing answers.
pub const MCP: &str = "mcp";

/// The verb that answers which Checks a change hits. CI names it.
pub const COVERS: &str = "covers";

/// `check`'s flag for a run over what the changed paths on stdin reach. CI
/// names it.
pub const CHANGED: &str = "--changed";
/// The verb Fleet starts for each hosted session.
pub const SESSION_KEEP: &str = "session-keep";
/// The worktree pool.
pub const WORKTREE: &str = "worktree";
/// The test map's recorder.
pub const TEST_MAP: &str = "test-map";
/// The readers behind the watch recipes Armada ships, which name this verb.
pub const WATCH_LINE: &str = "watch-line";

/// Read the arguments after the program name.
pub fn read<I: IntoIterator<Item = String>>(args: I) -> Result<Verb, Misread> {
    let args: Vec<String> = args.into_iter().collect();
    let Some(verb) = args.first() else {
        return Err(Misread {
            faults: vec![Fault::NothingAsked],
        });
    };
    if verb == "help" || verb == "--help" || verb == "-h" {
        return Ok(Verb::Help);
    }

    let rest = &args[1..];
    let mut faults = Vec::new();
    let parsed = match verb.as_str() {
        "serve" => {
            let positional = positionals(rest, &[], &mut faults);
            at_most_one("serve", &positional, &mut faults);
            Some(Verb::Serve {
                repository: positional.first().map(PathBuf::from),
            })
        }
        "check" | "run" => {
            let flags: &[&str] = match verb.as_str() {
                "check" => &[CHANGED],
                _ => &[],
            };
            let positional = positionals(rest, flags, &mut faults);
            let changed = verb == "check" && rest.iter().any(|arg| arg == CHANGED);
            if verb == "run" {
                at_most_one(verb, &positional, &mut faults);
            }
            if let (true, Some(test)) = (changed, positional.get(1)) {
                faults.push(Fault::ChangedRunsTheWholeCheck { test: test.clone() });
            }
            match positional.first() {
                Some(name) if verb == "check" => Some(Verb::Check {
                    name: name.clone(),
                    tests: positional.get(1..).unwrap_or(&[]).to_vec(),
                    changed,
                }),
                Some(name) => Some(Verb::Run { name: name.clone() }),
                None => {
                    faults.push(Fault::NoName { verb: verb.clone() });
                    None
                }
            }
        }
        COVERS => {
            let positional = positionals(rest, &[], &mut faults);
            if let Some(given) = positional.first() {
                faults.push(Fault::PathsComeOnStdin {
                    given: given.clone(),
                });
            }
            Some(Verb::Covers)
        }
        MCP => {
            let positional = positionals(rest, &[], &mut faults);
            at_most_one(MCP, &positional, &mut faults);
            if let Some(given) = positional.first() {
                faults.push(Fault::StandingTakesNoPath {
                    verb: verb.clone(),
                    given: given.clone(),
                });
            }
            Some(Verb::Mcp)
        }
        SESSION_KEEP => Some(Verb::SessionKeep {
            args: rest.to_vec(),
        }),
        "clean" => {
            let positional = positionals(rest, &["--all"], &mut faults);
            at_most_one("clean", &positional, &mut faults);
            if let Some(given) = positional.first() {
                faults.push(Fault::StandingTakesNoPath {
                    verb: verb.clone(),
                    given: given.clone(),
                });
            }
            Some(Verb::Clean {
                everything: rest.iter().any(|arg| arg == "--all"),
            })
        }
        WORKTREE => worktree::read(rest, &mut faults),
        TEST_MAP => test_map::read(rest, &mut faults),
        // Every word after the reader is a value, passed whole: a value is never read as a flag.
        WATCH_LINE => match rest.split_first() {
            Some((reader, values)) => Some(Verb::WatchLine {
                reader: reader.clone(),
                values: values.to_vec(),
            }),
            None => {
                faults.push(Fault::NoReader);
                None
            }
        },
        _ => {
            faults.push(Fault::NoSuchVerb {
                given: verb.clone(),
            });
            None
        }
    };

    match parsed {
        Some(verb) if faults.is_empty() => Ok(verb),
        _ => Err(Misread { faults }),
    }
}

/// The arguments that are not flags, with every unrecognised flag recorded.
fn positionals(args: &[String], allowed: &[&str], faults: &mut Vec<Fault>) -> Vec<String> {
    let mut positional = Vec::new();
    for arg in args {
        if arg.starts_with('-') && !allowed.contains(&arg.as_str()) {
            faults.push(Fault::NoSuchFlag {
                given: arg.clone(),
                allowed: allowed.iter().map(|f| (*f).to_string()).collect(),
            });
        } else if !arg.starts_with('-') {
            positional.push(arg.clone());
        }
    }
    positional
}

fn at_most_one(verb: &str, positional: &[String], faults: &mut Vec<Fault>) {
    if positional.len() > 1 {
        faults.push(Fault::TooMany {
            verb: verb.to_string(),
            extra: positional[1..].to_vec(),
        });
    }
}

/// Everything wrong with what was typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Misread {
    pub faults: Vec<Fault>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// `armada`, with nothing after it.
    NothingAsked,
    NoSuchVerb {
        given: String,
    },
    /// `check` and `run` are named at, always. There is no default Check.
    NoName {
        verb: String,
    },
    NoSuchFlag {
        given: String,
        allowed: Vec<String>,
    },
    TooMany {
        verb: String,
        extra: Vec<String>,
    },
    /// The verb acts on the repository the caller is standing in. A path would
    /// let somebody clean a repository they are not looking at — and would let
    /// an agent ask about one it is not working in, which is the same fault one
    /// level over.
    StandingTakesNoPath {
        verb: String,
        given: String,
    },
    /// `covers` reads its paths on stdin, one per line, so a diff of any size
    /// fits and a path is never mistaken for a flag.
    PathsComeOnStdin {
        given: String,
    },
    /// `check --changed` beside a test's name.
    ChangedRunsTheWholeCheck {
        test: String,
    },
    /// `worktree`'s forms are `lease`, `release`, `add`, `remove`, `close`,
    /// `open` and `--status`.
    WorktreeActUnknown {
        given: String,
    },
    /// `test-map` with no form, or one it does not have.
    TestMapFormUnknown { given: Option<String> },
    /// `test-map record` without `--check` or `--out`, or with one as the last word.
    TestMapNeeds { flag: String, valueless: bool },
    /// `worktree lease` with no branch after it.
    NoBranch,
    /// `watch-line` with no reader after it.
    NoReader,
    /// `worktree remove`, `close` or `open` without a slot's number.
    NoSlot {
        form: String,
    },
}

impl fmt::Display for Misread {
    /// One line per fault, then the usage. The person reading this is at a
    /// terminal, and a set of problems folded onto one line sends them back to
    /// the shell to find out which word each one meant.
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        for fault in &self.faults {
            writeln!(out, "{fault}")?;
        }
        write!(out, "{}", Usage)
    }
}

impl std::error::Error for Misread {}

impl fmt::Display for Fault {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::NothingAsked => write!(out, "armada was asked for nothing"),
            Fault::NoSuchVerb { given } => write!(
                out,
                "`{given}` is not one of armada's verbs — they are {}",
                listed(&VERBS.iter().map(|(name, _)| *name).collect::<Vec<_>>())
            ),
            Fault::NoName { verb } => write!(
                out,
                "`armada {verb}` needs the name of one thing the Manifest declares"
            ),
            Fault::NoSuchFlag { given, allowed } if allowed.is_empty() => {
                write!(out, "`{given}` is a flag this verb does not take")
            }
            Fault::NoSuchFlag { given, allowed } => {
                let names: Vec<&str> = allowed.iter().map(String::as_str).collect();
                write!(
                    out,
                    "`{given}` is not a flag this verb takes — it takes {}",
                    listed(&names)
                )
            }
            Fault::TooMany { verb, extra } => write!(
                out,
                "`armada {verb}` takes one argument, and {} came after it",
                listed(&extra.iter().map(String::as_str).collect::<Vec<_>>())
            ),
            Fault::StandingTakesNoPath { verb, given } => write!(
                out,
                "`armada {verb}` acts on the repository you are standing in, so `{given}` \
                 is an argument it has nowhere to put"
            ),
            Fault::PathsComeOnStdin { given } => write!(
                out,
                "`armada {COVERS}` reads the changed paths on stdin, one per line, so `{given}` \
                 has nowhere to go — `git diff --name-only main | armada {COVERS}`"
            ),
            Fault::ChangedRunsTheWholeCheck { test } => write!(
                out,
                "`{CHANGED}` narrows the whole Check to what a change reaches, so `{test}` \
                 has nowhere to go — name tests, or pass `{CHANGED}`"
            ),
            Fault::WorktreeActUnknown { given } => write!(
                out,
                "`armada {WORKTREE} {given}` is not a form this verb takes — they are \
                 `{WORKTREE} lease <branch>`, `{WORKTREE} release [<path>]`, `{WORKTREE} add`, \
                 `{WORKTREE} remove|close|open <n>`, `{WORKTREE} --status`"
            ),
            Fault::TestMapFormUnknown { given } => write!(
                out,
                "`armada {TEST_MAP} {}` is not a form this verb takes — it takes \
                 `{TEST_MAP} record --check <key> --out <file>`",
                given.as_deref().unwrap_or("")
            ),
            Fault::TestMapNeeds { flag, valueless: true } => {
                write!(out, "`{flag}` needs a value after it")
            }
            Fault::TestMapNeeds { flag, valueless: false } => write!(
                out,
                "`armada {TEST_MAP} record` needs `{flag}` — the Check to measure is a key as \
                 `armada check` takes it, and the file is where the laps are written"
            ),
            Fault::NoBranch => write!(
                out,
                "`armada {WORKTREE} lease` needs the branch to cut, and it is cut from the base"
            ),
            Fault::NoReader => write!(
                out,
                "`armada {WATCH_LINE}` needs the reader a watch recipe names, then its values"
            ),
            Fault::NoSlot { form } => write!(
                out,
                "`armada {WORKTREE} {form}` needs a slot's number, as `--status` prints it"
            ),
        }
    }
}

/// The verbs, printed after any refusal and by `armada help`.
pub struct Usage;

impl fmt::Display for Usage {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(out, "armada — Fleet, and what a repository's Manifest says")?;
        writeln!(out)?;
        for (verb, what) in VERBS {
            if *verb == WORKTREE {
                worktree::usage(out)?;
                continue;
            }
            if *verb == TEST_MAP {
                test_map::usage(out)?;
                continue;
            }
            let shape = match *verb {
                "serve" => "serve [<path>]".to_string(),
                "clean" => "clean [--all]".to_string(),
                MCP => MCP.to_string(),
                COVERS => format!("{COVERS} < <paths>"),
                WATCH_LINE => format!("{WATCH_LINE} <reader> <values>"),
                named => format!("{named} <name>"),
            };
            writeln!(out, "  armada {shape:<16}  {what}")?;
        }
        Ok(())
    }
}

fn listed(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

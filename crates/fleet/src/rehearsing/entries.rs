//! What the run sheet lists, and the one entry a run
//! names. **A name resolves here or nowhere**, so nothing a caller sends
//! reaches a process as argv.

use checks_runner::Narrowed;
use config::Manifest;
use core_model::{Narrowing, Prerequisite};

use super::Unrehearsable;

/// One Check or Command, resolved: everything a run of it needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) run: String,
    pub(crate) expect_exit_code: i64,
    pub(crate) requires: Vec<Prerequisite>,
    pub(crate) narrow: Option<Narrowing>,
    pub(crate) destructive: bool,
}

/// The sheet's three groups.
pub(crate) struct Listed {
    setup: Vec<Entry>,
    checks: Vec<Entry>,
    commands: Vec<Entry>,
}

/// What a Manifest declares.
pub(crate) fn declared(theirs: &Manifest) -> Listed {
    let destructive = |name: &str| theirs.command(name).is_some_and(|c| c.is_destructive());
    let setup: Vec<Entry> = theirs
        .prepared_by()
        .iter()
        .map(|prep| command(prep.name(), prep.run(), destructive(prep.name())))
        .collect();
    let checks = theirs
        .checks_as_written()
        .iter()
        .filter_map(|name| {
            let check = theirs.check(name)?;
            Some(Entry {
                name: name.clone(),
                run: check.run().to_string(),
                expect_exit_code: check.expect_exit_code(),
                requires: check.requires().to_vec(),
                narrow: check.narrow().cloned(),
                destructive: false,
            })
        })
        .collect();
    let commands = theirs
        .command_names()
        .into_iter()
        .filter(|name| !setup.iter().any(|entry| &entry.name == name))
        .filter_map(|name| {
            let found = theirs.command(&name)?;
            Some(command(&name, found.run(), found.is_destructive()))
        })
        .collect();
    Listed {
        setup,
        checks,
        commands,
    }
}

fn command(name: &str, run: &str, destructive: bool) -> Entry {
    Entry {
        name: name.to_string(),
        run: run.to_string(),
        expect_exit_code: 0,
        requires: Vec::new(),
        narrow: None,
        destructive,
    }
}

impl Listed {
    /// The entry a run names, or the refusal that lists what there is.
    pub(crate) fn named(&self, name: &str) -> Result<Entry, Unrehearsable> {
        self.every()
            .find(|entry| entry.name == name)
            .cloned()
            .ok_or_else(|| Unrehearsable::NotDeclared {
                name: name.to_string(),
                declared: self.every().map(|entry| entry.name.clone()).collect(),
            })
    }

    /// The three groups as the sheet draws them, narrowed against `changed`.
    #[allow(clippy::type_complexity)]
    pub(crate) fn sheet(
        &self,
        changed: &[String],
    ) -> (Vec<ipc::RunEntry>, Vec<ipc::RunEntry>, Vec<ipc::RunEntry>) {
        let drawn = |entries: &[Entry]| entries.iter().map(|e| wired(e, changed)).collect();
        (
            drawn(&self.setup),
            drawn(&self.checks),
            drawn(&self.commands),
        )
    }

    /// What Verify runs: setup, then every Check, each as it would run alone.
    pub(crate) fn verified(self) -> Vec<(ipc::VerifyGroup, Entry)> {
        let setup = self.setup.into_iter().map(|e| (ipc::VerifyGroup::Setup, e));
        let checks = self
            .checks
            .into_iter()
            .map(|e| (ipc::VerifyGroup::Checks, e));
        setup.chain(checks).collect()
    }

    fn every(&self) -> impl Iterator<Item = &Entry> {
        self.setup
            .iter()
            .chain(self.checks.iter())
            .chain(self.commands.iter())
    }
}

fn wired(entry: &Entry, changed: &[String]) -> ipc::RunEntry {
    let narrow_run = match (&entry.narrow, changed.is_empty()) {
        (Some(narrow), false) => match checks_runner::narrowed(Some(narrow), changed) {
            Narrowed::To(command) => Some(command),
            Narrowed::Nothing | Narrowed::Whole => None,
        },
        _ => None,
    };
    ipc::RunEntry {
        name: entry.name.clone(),
        run: entry.run.clone(),
        narrows: entry.narrow.is_some(),
        narrow_run,
        requires: entry
            .requires
            .iter()
            .map(|needed| needed.name().to_string())
            .collect(),
        expect_exit_code: entry.expect_exit_code,
        destructive: entry.destructive,
        frozen: false,
    }
}

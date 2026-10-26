//! A run saved again under a name Kit's copy already declares, or naming the one it `replaces`:
//! **the run, the servers only it lists, their ports and Kit's setup are taken out and written
//! again** from the confirmed draft, so *Edit the run* is a save that replaces, and may rename. A
//! run only the repository's own `armada.yml` declares is not Kit's to replace, and its save stays
//! refused, by name (`super::save`) or here where `replaces` names it.
//!
//! **Setup is the checkout's whole, not the run's.** The confirm lists every `setup.requires`
//! Command the checkout runs, so what it sends back is the setup wanted: Kit's `setup.requires`
//! becomes the names it sends that the repository does not already require, a Command the
//! repository declares with the same line is left as the repository's, and a Command only Kit's
//! setup named that the confirm dropped goes. A server kept under its name keeps its port's pin,
//! though the confirm names the port after the server.

use config::{CommandEdit, Edit, KitKind, Manifest, PortEdit, RunEdit};

use super::draft::RunDraft;
use super::port_name_in;
use super::unrunnable::Unrunnable;

/// What replacing a run in Kit's copy writes beside the draft's own edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Replacing {
    /// Each removal, before the draft is declared again.
    pub(crate) first: Vec<Edit>,
    /// Kit's `setup.requires`, whole.
    pub(crate) setup_requires: Vec<String>,
    /// A server the draft declares again with a port, and the number its old port was pinned to.
    pub(crate) pins: Vec<(String, u16)>,
    /// The servers the run listed before, which a save stops before starting it again.
    pub(crate) servers: Vec<String>,
    /// The key the run had, whose setup a save stops: `draft.name`'s unless it was renamed.
    pub(crate) run: String,
}

/// `draft` made to replace the run it `replaces`, else the run of its name, where Kit's copy over
/// `merged` declares one; `repository` is the repository's own file, where it has one. Untouched
/// where there is none to replace by name. **A run `replaces` names must be Kit's**, and a rename
/// must not land on another run's key, or nothing is saved.
pub(crate) fn replacing(merged: &Manifest, repository: Option<&Manifest>, draft: &mut RunDraft) -> Result<(), Unrunnable> {
    let named = draft.replaces.clone();
    let key = named.clone().unwrap_or_else(|| draft.name.clone());
    if let Some(key) = &named {
        if merged.app_run(key).is_none() {
            return Err(Unrunnable::NotARun { name: key.clone(), runs: merged.app_run_names().to_vec() });
        }
        if !super::in_kit(merged, key) {
            let why = format!("`{key}` is the repository's own run in its armada.yml, and only a run Kit's copy declares is replaced");
            return Err(Unrunnable::DraftRefused { why });
        }
        if *key != draft.name && merged.app_run(&draft.name).is_some() {
            return Err(Unrunnable::DraftRefused { why: format!("`{}` is a run already", draft.name) });
        }
    }
    let Some(kit) = merged.from_kit() else { return Ok(()) };
    let Some(old) = merged.app_run(&key).filter(|_| super::in_kit(merged, &key)) else { return Ok(()) };
    let kits = |kind: KitKind, name: &str| kit.whole || kit.names.iter().any(|one| one.kind == kind && one.name == name);
    let servers = old.servers().to_vec();
    let mut first = vec![Edit::Run { name: key.clone(), edit: RunEdit::Remove }];
    let mut pins = Vec::new();
    for server in &servers {
        let elsewhere = merged
            .app_run_names()
            .iter()
            .filter(|name| **name != key)
            .any(|name| merged.app_run(name).is_some_and(|run| run.servers().contains(server)));
        let again = draft.processes.iter().any(|one| one.name == *server);
        if (elsewhere && !again) || !kits(KitKind::Command, server) {
            continue;
        }
        first.push(Edit::Command { name: server.clone(), edit: CommandEdit::Remove });
        let port = merged.server(server).and_then(|one| port_name_in(one.serve()));
        if let Some(port) = port.filter(|port| kits(KitKind::Port, port)) {
            if let Some(number) = merged.port(&port).and_then(|one| one.fixed()) {
                pins.push((server.clone(), number));
            }
            first.push(Edit::Port { name: port, edit: PortEdit::Remove });
        }
    }
    pins.retain(|(server, _)| draft.processes.iter().any(|one| one.name == *server && one.port.is_some()));

    let theirs: Vec<String> = repository.map(|own| own.prepared_by().iter().map(|one| one.name().to_string()).collect()).unwrap_or_default();
    let their_line = |name: &str| repository.and_then(|own| own.command(name)).map(|one| one.run().to_string());
    let mut setup_requires = Vec::new();
    for one in std::mem::take(&mut draft.setup) {
        if !theirs.contains(&one.name) {
            setup_requires.push(one.name.clone());
        }
        if their_line(&one.name).as_deref() == Some(one.run.as_str()) {
            continue;
        }
        if kits(KitKind::Command, &one.name) && merged.command(&one.name).is_some() {
            first.push(Edit::Command { name: one.name.clone(), edit: CommandEdit::Remove });
        }
        draft.setup.push(one);
    }
    for gone in merged.prepared_by().iter().map(|one| one.name()) {
        let dropped = !theirs.iter().any(|one| one == gone) && !setup_requires.iter().any(|one| one == gone);
        if dropped && their_line(gone).is_none() && kits(KitKind::Command, gone) {
            first.push(Edit::Command { name: gone.to_string(), edit: CommandEdit::Remove });
        }
    }
    draft.replacing = Some(Replacing { first, setup_requires, pins, servers, run: key });
    Ok(())
}

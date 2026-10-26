//! A process pinned to a number, or given back to leasing: written into the person's copy in
//! Kit as `fixed` on the port the process listens on, **never into the repository's `armada.yml`**,
//! for [`super::save`]'s reason. Nothing is restarted: a pinned process takes its number the
//! next time it starts, which `crate::ports` enforces.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{splice, Edit, Manifest, NewPort, PortEdit};

use super::links::Changes;
use super::unrunnable::Unrunnable;
use super::{port_name_in, RunTarget, RunView};
use crate::daemon::Fleet;

/// The lowest number a person can pin to: the ones below it are not Fleet's to hand out.
const LOWEST: u16 = 1024;

/// `text` with `port` pinned to `to`, and the Manifest it makes as Kit's copy over the file.
fn pinned_text(file: &Path, label: &Path, text: &str, first: &[Edit], port: &str, held: (Option<u32>, Option<String>), to: Option<u16>) -> Result<String, Unrunnable> {
    let refuse = |why: String| Unrunnable::PinRefused { why };
    let fixed = |edit| Edit::Port { name: port.to_string(), edit };
    // Kit's copy replaces a port by name, so one it does not declare yet is declared with what the
    // repository gave it, and one it does declare is edited in place.
    let own = [first, &[fixed(PortEdit::Fixed(to))]].concat();
    let declared_here = [
        first,
        &[
            fixed(PortEdit::Add(NewPort { container: held.0, env: held.1 })),
            fixed(PortEdit::Fixed(to)),
        ],
    ]
    .concat();
    let written = splice(text, &own)
        .or_else(|_| splice(text, &declared_here))
        .map_err(|why| refuse(why.to_string()))?;
    let manifest = match std::fs::read_to_string(file) {
        Ok(theirs) => Manifest::with_kit(file, &theirs, label, &written),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Manifest::from_kit_alone(file, label, &written),
        Err(why) => return Err(Unrunnable::NotSaved { why: format!("{} could not be read: {why}", file.display()) }),
    }
    .map_err(|why| refuse(why.to_string()))?;
    if let Some(left_out) = manifest.from_kit().and_then(|kit| kit.left_out.clone()) {
        return Err(refuse(left_out));
    }
    match manifest.port(port).map(|one| one.fixed()) {
        Some(now) if now == to => Ok(written),
        _ => Err(refuse(format!("`{port}` is not pinned as asked in what was written"))),
    }
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
    /// Pin `process` to `to`, or give it back where `to` is `None`, and answer the run that lists
    /// it as it now stands on `target`. **Where it is already as asked nothing is written.**
    pub(crate) async fn run_port_pinned(
        &self,
        target: &RunTarget,
        process: &str,
        run: Option<&str>,
        to: Option<u16>,
    ) -> Result<RunView, Refusal> {
        let refuse = |why: String| self.run_refusal(Unrunnable::PinRefused { why });
        if to.is_some_and(|port| port < LOWEST) {
            return Err(refuse(format!("{} is below {LOWEST}, which Fleet cannot hand out", to.unwrap_or_default())));
        }
        let checkout = self.run_checkout(target).await?;
        let manifest = self.run_manifest(checkout.served(), checkout.path()).await?;
        let lists = |name: &&String| manifest.app_run(name).is_some_and(|one| one.servers().iter().any(|server| server == process));
        let name = match run {
            Some(name) => name.to_string(),
            None => manifest.app_run_names().iter().find(lists).cloned().ok_or_else(|| refuse(format!("no run lists `{process}`")))?,
        };
        if manifest.app_run(&name).is_none() {
            return Err(self.not_a_run(&manifest, &name));
        }
        let serve = manifest.server(process).map(|server| server.serve().to_string());
        let Some(port) = serve.as_deref().and_then(port_name_in) else {
            return Err(refuse(format!("`{process}` listens on no leased port, so there is nothing to pin")));
        };
        let held = manifest.port(&port).cloned().ok_or_else(|| refuse(format!("`{port}` is not a port this project declares")))?;
        if held.fixed() != to {
            let root = checkout.served().root().to_string();
            let target = self.run_save_target(&root).await?;
            let file = checkout.served().manifest().path().to_path_buf();
            let (text, first) = match target.text {
                Some(text) => (text, Vec::new()),
                None if file.is_file() => (String::new(), Vec::new()),
                None => (String::new(), vec![Edit::Version(1), Edit::Id(self.mint().ulid().as_str().to_string())]),
            };
            let (label, at) = (target.label.clone(), file);
            let held = (held.container(), held.env().map(str::to_string));
            let kept = tokio::task::spawn_blocking(move || pinned_text(&at, &label, &text, &first, &port, held, to))
                .await
                .unwrap_or_else(|why| Err(Unrunnable::NotSaved { why: why.to_string() }))
                .map_err(|why| self.run_refusal(why))?;
            self.manifest_written(&root, kept).await?;
        }
        if to.is_none() {
            self.links_ended_to(checkout.served().root(), process).await;
        }
        let manifest = self.run_manifest(checkout.served(), checkout.path()).await?;
        self.run_viewed(&manifest, &checkout, &name)
    }

    /// Every link remembered to `process` ends: it is leased again, and a sibling's hardcoded
    /// number is a link only once a file or a refused connection shows it.
    async fn links_ended_to(&self, root: &str, process: &str) {
        let end: Vec<_> = self.links_remembered(root).await.into_iter().filter(|link| link.to == process).collect();
        self.servers().links_forgotten_to(process);
        self.links_kept(root, Changes { add: Vec::new(), end }).await;
    }
}

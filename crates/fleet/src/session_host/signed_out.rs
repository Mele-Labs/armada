//! The agents' account signed out. Every turn after fails at once, and **every Job shares the account**,
//! so the Fleet is signed out as one: Bridge is sent [`ipc::AgentSignIn`] whole, and each Job the expiry
//! stopped keeps a mark so it is woken once the account works again.
//!
//! **In memory, never written.** A Fleet that restarts has no mark, and a Job's next turn fails the same
//! way and marks it again. Two things sign it back in: the sign-in Fleet runs on Bridge's press exiting
//! clean, and the model's own words or call in any Job, however he signed in.

use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::Runtime;
use crate::daemon::Fleet;

/// What a Job the expiry stopped is told once the account works again, as the plan's limit wakes one.
const WAKE: &str = "Continue";

/// The longest a sign-in may wait on the browser before it is ended as failed, so a page he closed
/// does not leave every later press starting nothing.
const SIGN_IN_WITHIN: Duration = Duration::from_secs(10 * 60);

/// What signs the account in again: the harness's program with the adapter's arguments, in the person's
/// home, on the `PATH` an agent is given. A test plants another.
#[derive(Clone, Debug)]
pub(crate) struct SignInRun {
    pub program: String,
    pub args: Vec<String>,
    pub directory: String,
    pub path: String,
}

impl SignInRun {
    pub(crate) fn of(host: &crate::daemon::Host) -> SignInRun {
        SignInRun {
            program: host.agent_binary.clone(),
            args: adapters::HeadlessAgent::sign_in_args().iter().map(|one| one.to_string()).collect(),
            directory: host.home.clone(),
            path: host.path.clone(),
        }
    }

    /// Whether it exited 0. **Its input is held open until it ends**: one that reads the end of it gives
    /// up on a pasted code. Its output is read and dropped, so a full pipe never stalls it.
    async fn ran(&self) -> bool {
        let ended = self.ended().await;
        if let Err(why) = &ended {
            eprintln!("the agents' sign-in failed: {why}");
        }
        ended.is_ok()
    }

    /// `Err` of why it did not exit 0.
    async fn ended(&self) -> Result<(), String> {
        let mut child = tokio::process::Command::new(&self.program)
            .args(&self.args)
            .current_dir(&self.directory)
            .env("HOME", &self.directory)
            .env("PATH", &self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|why| format!("`{}` did not start: {why}", self.program))?;
        let held = child.stdin.take();
        let ended = tokio::time::timeout(SIGN_IN_WITHIN, child.wait_with_output()).await;
        drop(held);
        let output = ended
            .map_err(|_| format!("ended after {} minutes on the browser", SIGN_IN_WITHIN.as_secs() / 60))?
            .map_err(|why| format!("could not be waited on: {why}"))?;
        match output.status.success() {
            true => Ok(()),
            false => Err(format!("{}: {}", output.status, String::from_utf8_lossy(&output.stderr).trim())),
        }
    }
}

/// The Fleet-wide state, what a press runs, and the Jobs owed a wake. `std` locks, never held across
/// an `.await`.
pub(crate) struct Held {
    state: Mutex<ipc::AgentSignIn>,
    run: Mutex<SignInRun>,
    /// Jobs to tell `Continue`, for a caller that cannot tell them itself: a terminal Job's report.
    owed: Mutex<Vec<String>>,
}

impl Held {
    pub(crate) fn running(run: SignInRun) -> Held {
        Held { state: Mutex::default(), run: Mutex::new(run), owed: Mutex::default() }
    }

    fn owe(&self, woken: impl IntoIterator<Item = String>) {
        self.owed.lock().expect("the wakes are not held across a panic").extend(woken);
    }

    fn owed(&self) -> Vec<String> {
        std::mem::take(&mut *self.owed.lock().expect("the wakes are not held across a panic"))
    }

    /// Where a test plants a run that answers at once, in place of the browser.
    #[cfg(test)]
    pub(crate) fn plant(&self, run: SignInRun) {
        *self.run.lock().expect("the run is not held across a panic") = run;
    }

    pub(crate) fn now(&self) -> ipc::AgentSignIn {
        self.held().clone()
    }

    fn held(&self) -> std::sync::MutexGuard<'_, ipc::AgentSignIn> {
        self.state.lock().expect("the sign-in is not held across a panic")
    }

    /// Signed out at `at`, where it was signed in. `Some` of the state where it moved: **a repeat keeps
    /// the first instant**.
    fn signed_out(&self, at: &str) -> Option<ipc::AgentSignIn> {
        let mut held = self.held();
        if held.signed_out_since.is_some() {
            return None;
        }
        held.signed_out_since = Some(ipc::Instant::carried(at));
        Some(held.clone())
    }

    fn is_signed_out(&self) -> bool {
        self.held().signed_out_since.is_some()
    }

    /// Signed in. A sign-in still running is left to end. `Some` of the state where it moved.
    fn signed_in(&self) -> Option<ipc::AgentSignIn> {
        let mut held = self.held();
        held.signed_out_since.take()?;
        Some(held.clone())
    }

    /// A sign-in started, with what to run, where none was running.
    fn started(&self) -> Option<(SignInRun, ipc::AgentSignIn)> {
        let mut held = self.held();
        if held.signing_in {
            return None;
        }
        held.signing_in = true;
        let run = self.run.lock().expect("the run is not held across a panic").clone();
        Some((run, held.clone()))
    }

    /// The sign-in ended, and signed in where `clean`. The state after.
    fn finished(&self, clean: bool) -> ipc::AgentSignIn {
        let mut held = self.held();
        held.signing_in = false;
        if clean {
            held.signed_out_since = None;
        }
        held.clone()
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
    /// The harness said its sign-in ran out: the Job is marked, and the Fleet is signed out where it
    /// was not.
    pub(crate) fn signed_out_heard(&self, runtime: &Arc<Runtime>) {
        runtime.state().signed_out = true;
        if let Some(now) = self.agent_sign_in().signed_out(self.now().as_str()) {
            self.publish(ipc::Event::AgentSignInChanged(now));
        }
    }

    /// The model answered in hosted `id`, so the account works: where the Fleet was signed out, every
    /// other Job it stopped is woken. **Not a turn's start**: the harness says `init` before every turn,
    /// the failed ones too.
    pub(crate) async fn signed_in_heard(self: &Arc<Self>, id: &str) {
        if self.signed_in_by(id) {
            self.sign_in_wakes_owed().await;
        }
    }

    /// The account works, as `id`'s answer shows: where the Fleet was signed out, it is signed in and
    /// every other Job it stopped is owed a wake. Whether it was signed out. **A terminal Job's
    /// report** has no `Arc` to tell with, so the notice loop's next tick pays what it owes.
    pub(crate) fn signed_in_by(&self, id: &str) -> bool {
        if !self.agent_sign_in().is_signed_out() {
            return false;
        }
        let marked = self.marks_taken();
        if let Some(now) = self.agent_sign_in().signed_in() {
            self.publish(ipc::Event::AgentSignInChanged(now));
        }
        self.agent_sign_in().owe(marked.into_iter().filter(|one| one != id));
        true
    }

    /// `start_agent_sign_in`: Fleet runs the sign-in, and the state is answered as it left it. **One at
    /// a time**: a press while one runs starts nothing.
    pub(crate) fn agent_sign_in_pressed(self: &Arc<Self>) -> ipc::AgentSignIn {
        let Some((run, now)) = self.agent_sign_in().started() else {
            return self.agent_sign_in().now();
        };
        self.publish(ipc::Event::AgentSignInChanged(now.clone()));
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            match run.ran().await {
                true => fleet.signed_in_everywhere().await,
                false => fleet.publish(ipc::Event::AgentSignInChanged(fleet.agent_sign_in().finished(false))),
            }
        });
        now
    }

    /// The sign-in exited clean. **One sign-in is every Job's**, since they share the account: each mark
    /// is cleared and each marked Job woken.
    pub(crate) async fn signed_in_everywhere(self: &Arc<Self>) {
        let marked = self.marks_taken();
        self.publish(ipc::Event::AgentSignInChanged(self.agent_sign_in().finished(true)));
        self.agent_sign_in().owe(marked);
        self.sign_in_wakes_owed().await;
    }

    /// Every Job the expiry stopped, its mark taken.
    fn marks_taken(&self) -> Vec<String> {
        self.hosts()
            .all()
            .into_iter()
            .filter(|(_, runtime)| std::mem::take(&mut runtime.state().signed_out))
            .map(|(id, _)| id)
            .collect()
    }

    /// Each Job owed a wake told `Continue`. **The notice loop's tick calls it too.** A Job that ended
    /// is not told, which `told_by_fleet` answers.
    pub(crate) async fn sign_in_wakes_owed(self: &Arc<Self>) {
        for id in self.agent_sign_in().owed() {
            let _ = self.told_by_fleet(&id, WAKE, ipc::Notice::SignInWake).await;
        }
    }
}

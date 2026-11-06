//! Which of a Job's terminals a call means, and what Fleet keeps about each: its number, its name
//! and the shell its agent last saw. The shells themselves are `super::job_shell`'s, held by a keeper
//! that outlives Fleet (`crate::terminal_keeping`) and taken back at boot (`super::kept_terminals`).
//!
//! A terminal is `t1`, `t2` and so on, numbered by the store and never given twice in a Job, so an
//! ID names the same terminal after its shell has ended or Fleet has restarted.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{JobTerminal, JobTerminals, ListTerminals, ListedTerminal, RenameTerminal, ShowTerminal};
use store::KeptJobTerminal;

use super::{refusal, SessionRuns};
use crate::daemon::Fleet;

/// `t2` for the Job's second terminal.
pub(crate) fn terminal_id(number: u32) -> String {
    format!("t{number}")
}

/// The number in `id`, where it is spelled as [`terminal_id`] spells it.
pub(crate) fn terminal_number(id: &str) -> Option<u32> {
    let number = id.strip_prefix('t')?.parse().ok()?;
    (terminal_id(number) == id).then_some(number)
}

/// What the person calls a terminal until they rename it.
fn default_name(number: u32) -> String {
    format!("Terminal {number}")
}

/// Which terminal a call opens.
pub(super) enum Opening {
    /// This one: its running shell, or a new shell under its number where it has ended.
    Named(u32),
    /// The next number, with a shell.
    New,
    /// The lowest-numbered open terminal; else the lowest the Job has had; else the first.
    Any,
}

impl SessionRuns {
    /// The lowest-numbered of `session`'s terminals whose shell is in flight.
    fn lowest_open(&self, session: &str) -> Option<u32> {
        self.shells_open(session).first().map(|(number, _)| *number)
    }
}

impl Fleet
{
    /// The person's open: terminal `named`, a `new` one, or neither, as [`Opening`] says. The
    /// terminal's ID and its run id.
    pub(crate) async fn open_job_terminal(
        self: Arc<Self>,
        session_id: &str,
        named: Option<&str>,
        new: bool,
        cols: Option<u16>,
        rows: Option<u16>,
    ) -> Result<(String, String), Refusal> {
        let opening = match (named, new) {
            (Some(_), true) => return Err(self.terminal_and_new()),
            (Some(named), false) => match terminal_number(named) {
                Some(number) => Opening::Named(number),
                None => return Err(self.terminal_never_had(named)),
            },
            (None, true) => Opening::New,
            (None, false) => Opening::Any,
        };
        let (number, run_id, _) = self.job_shell(session_id, opening, cols, rows).await?;
        Ok((terminal_id(number), run_id))
    }

    /// The number `opening` means, minted where it is new. Called with the Job's opens held.
    pub(super) async fn terminal_opened(&self, session: &str, opening: Opening) -> Result<u32, Refusal> {
        match opening {
            Opening::Named(number) => {
                let had = self.terminals_kept(session).await?.iter().any(|one| one.number == number);
                if had { Ok(number) } else { Err(self.terminal_never_had(&terminal_id(number))) }
            }
            Opening::New => self.terminal_minted(session).await,
            Opening::Any => {
                if let Some(open) = self.session_runs().lowest_open(session) {
                    return Ok(open);
                }
                match self.terminals_kept(session).await?.first() {
                    Some(lowest) => Ok(lowest.number),
                    None => self.terminal_minted(session).await,
                }
            }
        }
    }

    async fn terminal_minted(&self, session: &str) -> Result<u32, Refusal> {
        self.store().lock().await.mint_job_terminal(session).map_err(|why| self.ledger_fault(why))
    }

    async fn terminals_kept(&self, session: &str) -> Result<Vec<KeptJobTerminal>, Refusal> {
        self.store().lock().await.job_terminals(session).map_err(|why| self.ledger_fault(why))
    }

    /// The terminal `named` and its run id, where its shell is in flight.
    pub(super) fn terminal_named_open(&self, session: &str, named: &str) -> Result<(u32, String), Refusal> {
        terminal_number(named)
            .and_then(|number| self.session_runs().shell_open(session, number).map(|run_id| (number, run_id)))
            .ok_or_else(|| self.terminal_not_open(named))
    }

    pub(super) fn terminal_not_open(&self, named: &str) -> Refusal {
        refusal(format!("terminal {named} is not open. list_terminals names the open ones"), self.run_id())
    }

    fn terminal_never_had(&self, named: &str) -> Refusal {
        refusal(format!("this job has never had a terminal {named}. list_terminals names the open ones"), self.run_id())
    }

    pub(super) fn terminal_and_new(&self) -> Refusal {
        refusal("`new` opens a terminal and `terminal` names one: give one of them", self.run_id())
    }

    /// The terminal `session`'s agent last typed into or read, whether or not its shell is open.
    pub(super) async fn terminal_last_used(&self, session: &str) -> Option<u32> {
        let kept = self.terminals_kept(session).await.ok()?;
        kept.iter()
            .filter_map(|one| one.seen_at.as_ref().map(|at| (at, one.number)))
            .max()
            .map(|(_, number)| number)
    }

    /// The terminal a read or a watch with none named means: the agent's last used where it is
    /// open, else the lowest-numbered open one.
    pub(crate) async fn terminal_read_by_default(&self, session: &str) -> Option<u32> {
        if let Some(last) = self.terminal_last_used(session).await.filter(|last| self.session_runs().shell_open(session, *last).is_some()) {
            return Some(last);
        }
        self.session_runs().lowest_open(session)
    }

    /// The terminal a terminal watch listens to, fixed when it starts: the one `named`, else
    /// [`Self::terminal_read_by_default`]. `None` where it is not open.
    pub(crate) async fn terminal_watched(&self, session: &str, named: Option<&str>) -> Option<u32> {
        match named {
            Some(named) => self.terminal_named_open(session, named).ok().map(|(number, _)| number),
            None => self.terminal_read_by_default(session).await,
        }
    }

    /// The run id of the shell `session`'s agent last typed into or read in terminal `number`.
    /// **Kept in the store**, since a shell whose keeper is gone is a new one under the same ID.
    pub(super) async fn terminal_seen(&self, session: &str, number: u32) -> Option<String> {
        let kept = self.terminals_kept(session).await.ok()?;
        kept.into_iter().find(|one| one.number == number).and_then(|one| one.seen_run)
    }

    pub(super) async fn terminal_saw(&self, session: &str, number: u32, run_id: &str) {
        let now = self.now();
        let _ = self.store().lock().await.keep_job_terminal_seen(session, number, run_id, &now);
    }

    /// The Job's open terminals, the lowest number first.
    pub(crate) async fn job_terminals_listed(
        &self,
        caller: Option<api::Caller>,
        asked: ListTerminals,
    ) -> Result<JobTerminals, Refusal> {
        let session = self.shell_job(caller, asked.session_id.as_ref())?;
        let kept = self.terminals_kept(&session).await?;
        let terminals = self
            .session_runs()
            .shells_open(&session)
            .into_iter()
            .map(|(number, run_id)| self.listed(&kept, number, run_id))
            .collect();
        Ok(JobTerminals { terminals })
    }

    /// Name the Job's open terminal; a blank name restores `Terminal N`. The terminal as it stands.
    pub(crate) async fn job_terminal_renamed(&self, rename: RenameTerminal) -> Result<ListedTerminal, Refusal> {
        let session = rename.session_id.as_str().trim();
        let (number, run_id) = self.terminal_named_open(session, &rename.terminal)?;
        let mut store = self.store().lock().await;
        store.keep_job_terminal_name(session, number, rename.name.trim()).map_err(|why| self.ledger_fault(why))?;
        let kept = store.job_terminals(session).map_err(|why| self.ledger_fault(why))?;
        Ok(self.listed(&kept, number, run_id))
    }

    /// The agent's ask that Bridge bring a terminal to the front, `terminal.shown`. The terminal as
    /// it stands. Not a use: the agent's last used stays where it was.
    pub(crate) async fn job_terminal_shown(
        &self,
        caller: Option<api::Caller>,
        asked: ShowTerminal,
    ) -> Result<ListedTerminal, Refusal> {
        let session = self.shell_job(caller, asked.session_id.as_ref())?;
        let (number, run_id) = match asked.terminal.as_deref() {
            Some(named) => self.terminal_named_open(&session, named)?,
            None => {
                let number = self
                    .terminal_read_by_default(&session)
                    .await
                    .ok_or_else(|| refusal("this job has no terminal open. type_in_terminal opens one", self.run_id()))?;
                let run_id = self
                    .session_runs()
                    .shell_open(&session, number)
                    .ok_or_else(|| self.terminal_not_open(&terminal_id(number)))?;
                (number, run_id)
            }
        };
        let kept = self.terminals_kept(&session).await?;
        self.publish(ipc::Event::TerminalShown(JobTerminal {
            session_id: ipc::SessionId::carried(&session),
            terminal: terminal_id(number),
            run_id: run_id.clone(),
        }));
        Ok(self.listed(&kept, number, run_id))
    }

    fn listed(&self, kept: &[KeptJobTerminal], number: u32, run_id: String) -> ListedTerminal {
        let name = kept.iter().find(|one| one.number == number).map(|one| one.name.clone()).filter(|name| !name.is_empty());
        ListedTerminal {
            terminal: terminal_id(number),
            name: name.unwrap_or_else(|| default_name(number)),
            running: self.session_runs().shell_running(&run_id),
            run_id,
        }
    }

    /// A Job that ended takes its shells with it.
    pub(crate) fn job_shell_wound_up(&self, session: &str) {
        self.session_runs().stop_shells(session);
    }

    pub(super) fn job_shell_ended(&self, session: &str, number: u32, run_id: &str) {
        self.session_runs().shell_ended(session, number, run_id);
        self.publish(ipc::Event::TerminalExited(JobTerminal {
            session_id: ipc::SessionId::carried(session),
            terminal: terminal_id(number),
            run_id: run_id.to_string(),
        }));
    }
}

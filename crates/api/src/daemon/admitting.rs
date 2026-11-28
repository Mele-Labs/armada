//! Who opened the agent's door, and what that caller may reach through it.
//! `#941`.
//!
//! **Placed, never claimed.** Nothing a caller sends says it is a Job on your words: an argv
//! flag, a header or a name in its MCP configuration is something any agent on
//! the machine can copy (`docs/spikes/011-what-can-one-drone-reach.md`). The
//! daemon places the connection against the sessions it started, as
//! `fleet::peer` places a Drone's, so what the door narrows is decided by who
//! holds the socket.
//!
//! **The rule is the daemon's.** This crate cannot name `fleet::on_your_words::may`, so
//! the answer arrives as a [`WordsReach`] the daemon decided row by row, and the
//! door only enforces it.

use std::future::Future;

use ipc::door::{Reachable, ON_YOUR_WORDS, OPERATOR, REACHABLE};

use crate::mcp::Caller;

/// The fifth surface: which caller a door call came from.
pub trait Admitting: Send + Sync + 'static {
    /// What the Job holding this call's connection may reach.
    ///
    /// **`None` is every other caller**, which the door answers exactly as it
    /// did before a Job on your words could be told apart — a person's own agent
    /// session keeps every row the inventory marks `Yes`.
    ///
    /// **Async because the Operator's reach is read off its project's settings**, which Fleet keeps in the store.
    fn words_reach_at(&self, caller: Caller) -> impl Future<Output = Option<WordsReach>> + Send;

    /// Why the Operator's call is held, where it names a repository, a Job or a pull request outside its project:
    /// the refusal the call is answered with. **`None` lets it through**, and is every call of any other caller.
    /// The daemon decides, since only it knows which projects hold which repositories.
    fn call_held(&self, caller: Caller, call: &ipc::door::Call) -> impl Future<Output = Option<String>> + Send {
        let _ = (caller, call);
        async { None }
    }

    /// The key on a tool's `_meta` the agent's harness reads as loading it at the start, for the
    /// rows the inventory marks `always_loaded`. **`None` by default**, which marks none.
    fn always_loads(&self) -> Option<&'static str> {
        None
    }

    /// Whether this call's connection is held inside a Job: its agent, or any process the agent
    /// started. **For the routes that are the person's alone** (`crate::callers::THE_PERSONS_OWN`),
    /// which refuse it whatever the call says it is. By default, wherever the door would place it.
    fn job_at(&self, caller: Caller) -> impl Future<Output = bool> + Send {
        async move { self.words_reach_at(caller).await.is_some() }
    }
}

/// What one placed Job may call through the door.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordsReach {
    may: Vec<&'static str>,
    otherwise: &'static str,
    session: bool,
    operator: bool,
    /// Acts left out of the reach, each with the sentence that names why: a tool its project switched off.
    withheld: Vec<(&'static str, String)>,
    /// Acts listed and refused at the call, each with why: they are open while the person has said so lately.
    unsaid: Vec<(&'static str, String)>,
}

impl WordsReach {
    /// Decided over [`offerable`] by the daemon's own predicate. `otherwise`
    /// is why anything else is refused, said to the session.
    pub fn deciding(may: impl Fn(&Reachable) -> bool, otherwise: &'static str) -> WordsReach {
        WordsReach {
            may: offerable()
                .filter(|row| may(row))
                .map(|row| row.operation)
                .collect(),
            otherwise,
            session: false,
            operator: false,
            withheld: Vec::new(),
            unsaid: Vec::new(),
        }
    }

    /// What a session Fleet hosts for Bridge may call: **what any agent of the
    /// person's is offered, and the permission tool**, which is the one tool
    /// its own `--permission-prompt-tool` needs. Its acts are the person's own
    /// and are never recorded as a Job's.
    pub fn of_a_hosted_session() -> WordsReach {
        let mut may: Vec<&'static str> = crate::door::offered()
            .iter()
            .map(|shape| shape.operation)
            .collect();
        may.push(ipc::door::ASKS_A_PERSON);
        WordsReach {
            may,
            otherwise: "a job is offered what any agent of the person's is, and no more",
            session: true,
            operator: false,
            withheld: Vec::new(),
            unsaid: Vec::new(),
        }
    }

    /// This session is the Operator's own: what it is offered at the door beyond a hosted session's is
    /// the `Operator` rows and the reads and acts across repositories its rules name. **None of a Job's
    /// own acts** ([`crate::door::JOB_ONLY`]): it is not a Job, so it has no Needs you list and no end.
    pub fn the_operator(mut self) -> WordsReach {
        self.operator = true;
        self.may.retain(|operation| !crate::door::JOB_ONLY.contains(operation));
        self.otherwise = "it is not a Job, so a Job's own acts are not offered to it";
        self
    }

    /// Whether the caller is the Operator's session, which Fleet placed by its connection.
    pub fn is_the_operator(&self) -> bool {
        self.operator
    }

    /// Whether this is a hosted session's reach rather than a Job on your words.
    pub fn is_a_hosted_session(&self) -> bool {
        self.session
    }

    /// Leave `operations` out of the reach, each refused with `why`, which names the switch that is off. **Not listed
    /// and not callable.** Named even for an act the reach never listed, such as one of the Operator's own rows, so the
    /// refusal is the switch's and not that of a tool that does not exist.
    pub fn without(mut self, operations: &[&'static str], why: &str) -> WordsReach {
        for operation in operations {
            self.may.retain(|one| one != operation);
            self.withheld.push((operation, why.to_string()));
        }
        self
    }

    /// Keep `operations` listed and refuse a call to one with `why`. **For what is open for a while**, which a tool list an
    /// agent read at its start could not follow.
    pub fn until_said(mut self, operations: &[&'static str], why: &str) -> WordsReach {
        for operation in operations {
            self.unsaid.push((operation, why.to_string()));
        }
        self
    }

    /// Whether a setting left `operation` out of the reach.
    pub fn is_withheld(&self, operation: &str) -> bool {
        self.withheld.iter().any(|(one, _)| *one == operation)
    }

    /// Whether the tool is in the list the caller is shown.
    pub fn listed(&self, operation: &str) -> bool {
        self.may.contains(&operation)
    }

    /// Whether a call to `operation` goes through.
    pub fn may(&self, operation: &str) -> bool {
        self.listed(operation) && !self.unsaid.iter().any(|(one, _)| *one == operation)
    }

    /// The sentence that refuses `operation` where a setting does, and none where the reach does or nothing does.
    pub fn held_back(&self, operation: &str) -> Option<String> {
        let (_, why) = self.withheld.iter().chain(&self.unsaid).find(|(one, _)| *one == operation)?;
        Some(format!("`{operation}` {why}"))
    }

    /// The refusal a call outside the reach is answered with. **It names the
    /// operation**, so a session reads which act was the person's, and a tool switched off names the switch.
    pub fn refusing(&self, operation: &str) -> String {
        if let Some(why) = self.held_back(operation) {
            return why;
        }
        let whose = if self.operator { "the Operator's" } else { "this Job's" };
        format!("`{operation}` is not {whose} to call: {}", self.otherwise)
    }
}

/// Who is behind a call the transport places rather than the body: originally
/// `Commands::redirect_drone`'s alone, and `#943` gave `Commands::propose_job`
/// and `Commands::propose_from_request` the same word for the same reason —
/// the door lets a Job on your words draft a Job, and what Fleet records for the
/// draft has to say so.
///
/// **Two, because nothing else places a call this way**, so Fleet or a Drone
/// recorded as having redirected or drafted is not a value this can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redirector {
    /// A person, through Bridge or through their own agent's door session.
    Person,
    /// A Job on your words Fleet is hosting, placed by its connection.
    Job,
}

/// Who is steering a Drone: [`Redirector`]'s two. The transport places a Job on your words by
/// its connection, so a body cannot name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Steerer {
    Person,
    Job,
}

impl From<Redirector> for Steerer {
    fn from(by: Redirector) -> Steerer {
        match by {
            Redirector::Person => Steerer::Person,
            Redirector::Job => Steerer::Job,
        }
    }
}

/// Every row a Job on your words could be offered before its daemon decides: what
/// any agent is offered, and the `On your words` rows no other agent is (`#1150`).
pub fn offerable() -> impl Iterator<Item = &'static Reachable> {
    REACHABLE
        .iter()
        .chain(ON_YOUR_WORDS.iter())
}

/// The rows only the Operator's session is offered: its cards and its per-project values.
pub fn operator_rows() -> impl Iterator<Item = &'static Reachable> {
    OPERATOR.iter()
}

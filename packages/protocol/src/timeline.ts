// What happened across the Fleet, one entry each, with the instant it did —
// `crates/ipc/src/timeline.rs`. Since protocol 23.82.
//
// Read as a window off `get_fleet_timeline` (`GET /timeline?since=&until=&limit=`), and appended to by
// each `timeline.appended`. The header rules in `protocol.ts` hold here: hand-written, and the sets
// Fleet closes are written out.

/**
 * What happened. Each names the fact it is read from, in `crates/ipc/src/timeline.rs`.
 *
 * `job.read`, `job.ran`, `session.edited`, `session.read` and `session.ran` are **one entry for a burst**:
 * `says` carries the count (`Read 14 files`, `Edited 3 files`, `Ran 5 commands`) and `detail` the latest
 * path or command (a command cut, with what looks like a secret hidden). Fleet writes the entry when the
 * burst closes but dates it by the first call, so order by `at` and not by `id`. A Job's edits stay
 * `job.files`. `session.waiting` and `session.resolved` are an item the agent put on, and took off, its
 * `waiting_for` list; `session.asked` is what Fleet raised (a question or permission card, a reply to
 * read). `session.subagent` says `Subagent started`, `Subagent resumed` or `Subagent finished` with `Type: description` as its
 * detail; `session.spawned` is the Session whose agent started another (`detail` its title) or forked itself.
 * `session.walked` is a walk the Session showed (`detail` the page's title), `session.committed` a `git commit` that made a
 * commit and `session.pushed` a `git push` that updated a ref (`detail` the branch). Those three and the pull request's own
 * entries are the road a Session's `build` reads its stops from. `session.called` is one call a Session made to a service
 * outside the repository, one entry for each call: `says` is `Called {name}`, `detail` and `call.line` the one line that says what was asked
 * (never an argument or a query), `call.service` the service and `call.name` what it is called. `session.workflow`, `session.model` and
 * `session.effort` are a change a person made in a panel that the agent was told of: `says` is
 * `workflow: old → new`, `model: old → new` or `effort: old → new` (none of a workflow is `custom`, no effort
 * `the default`). `session.step` is the Session moving from one step of its workflow to another as Fleet
 * observed: `says` is `step: old → new`, a step by its label, none before the first as `start` and past the last as
 * `done`. The model and effort entries are dated at the start of the turn that ran on the change. Since 23.91.
 */
export type TimelineKind =
  | "session.started"
  | "session.asked"
  | "session.answered"
  | "session.turn"
  | "session.edited"
  | "session.read"
  | "session.ran"
  | "session.subagent"
  | "session.waiting"
  | "session.resolved"
  | "session.spawned"
  | "session.walked"
  | "session.committed"
  | "session.pushed"
  | "session.called"
  | "session.workflow"
  | "session.step"
  | "session.model"
  | "session.effort"
  | "session.ended"
  | "slot.leased"
  | "slot.released"
  | "pr.opened"
  | "pr.checks"
  | "pr.queued"
  | "pr.merged"
  | "main.red"
  | "main.green";

/**
 * Which service a call went to: `forge`, `tracker` (Linear, Jira, `gh issue`), `web` (a fetch or a search), or any other MCP
 * server by its own name, such as `figma`.
 */
export type CallService = "forge" | "tracker" | "web" | (string & {});

/** One call to a service outside the repository: the service, and the one line that says what was asked of it. */
export type ServiceCall = {
  service: CallService;
  /** What the service is called, as a row says it: the forge's own name, `the issue tracker`, `the web`, or a server's name. */
  name: string;
  /** The host for a web fetch, the tool name for an MCP call, the `gh` subcommand for the forge. */
  line: string;
};

/** What an entry is a state of, as a colour reads it: working, needs the person, failing, passed, waiting. */
export type TimelineTone = "run" | "ask" | "bad" | "ok" | "idle";

/** What it happened to. `main` is the repository's main branch; which one is `manifest_id`. */
export type TimelineSubject =
  | { kind: "session"; id: string }
  | { kind: "pull_request"; number: number; url?: string }
  | { kind: "main" };

/** One thing that happened. */
export type TimelineEvent = {
  /**
   * The number Fleet kept it under. Rises with the order Fleet wrote entries, not with `at`, and is how a
   * client holding a window drops an appended entry it already has.
   */
  id: number;
  /** When it happened. */
  at: string;
  kind: TimelineKind;
  subject: TimelineSubject;
  /** What the subject was called when this happened. Absent for main, or a Job Fleet had no handle for. */
  title?: string;
  tone: TimelineTone;
  /** The fact, bare: `Started`, `Asked: split or wrap?`. */
  says: string;
  /** What it opens to: a Check's name, a branch, a command. */
  detail?: string;
  /**
   * What a burst named: the files or commands behind `Read 14 files` or `Ran 5 commands`, newest first,
   * without repeats, at most 25, each cut and hidden as `detail` is. The first is `detail`; the count in `says`
   * is the whole burst's. Absent for every other kind, and for an entry written before Fleet kept them.
   */
  items?: string[];
  /** The call a `session.called` entry records. Absent for every other kind. */
  call?: ServiceCall;
  /** The repository it belongs to. Absent where Fleet could not place it. */
  manifest_id?: string;
  /** The subject is still working after this. Absent is false. */
  live?: boolean;
  /** The subject is over after this: landed, failed, merged, ended. Absent is false. */
  over?: boolean;
};

/** `get_fleet_timeline`'s query string; each field is optional. */
export type TimelineAsk = {
  /** Entries at or after this. Absent is from the beginning. */
  since?: string;
  /** Entries before this. Absent is up to now. */
  until?: string;
  /** At most this many, the newest. Absent is 500; Fleet cuts a larger one to 2000. */
  limit?: number;
};

/** `get_fleet_timeline`: the window, oldest first. */
export type FleetTimeline = {
  events: TimelineEvent[];
  /** Older entries inside the window were left out to keep to the limit. Absent is false. */
  cut?: boolean;
};

/** `timeline.appended`'s body: the entry Fleet just kept. Under `event` because the stream tags its events with `kind`. */
export type TimelineAppended = { event: TimelineEvent };

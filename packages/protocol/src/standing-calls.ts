// What calls the person, and which of it he has not opened, read by Fleet so a client does not work it out:
// `get_calls`, `calls.changed` and `open_call`. Mirrors `crates/ipc/src/standing_calls.rs`. States and the words a
// call says; what is played, shown or badged stays with the client.

/** What a call is about: something a live Job waits on him for (or a Job Fleet gave up restarting), main read red, a pull request of his that failed. */
export type StandingCallKind = "job" | "main" | "pull";

/** `blocked` where the item holds the Job's turn (a question, a permission, a plan, a slot or delivery choice, a done ask, a browser ask, a start card); `waiting` otherwise. */
export type CallTone = "blocked" | "waiting";

/** One call on the person. */
export type StandingCall = {
  /** Electron's own: `session:<id>:<item>`, `session:<id>:needs:<turn>`, `main:<root>`, `pull:<number>`. */
  key: string;
  /** Whom it is from: `session:<id>` for a Job's calls and for the Job already on a failure, else the key. One badge for each owner. */
  owner: string;
  kind: StandingCallKind;
  /** The Job it is from, or the Job that owns the failure. Absent on a failure nobody owns. */
  session_id?: string;
  title: string;
  text: string;
  /** When it began. A failure Fleet read no instant of reads `1970-01-01T00:00:00.000Z`. */
  since: string;
  tone: CallTone;
  /** He has seen it: a Job call while he is in the Job or has been since it began and the Job does not ask him, a main or pull request call once `open_call` said so. */
  opened: boolean;
};

/** `get_calls` (`GET /calls`), and what `open_call` answers: every call standing, oldest first. */
export type StandingCalls = {
  calls: StandingCall[];
};

/** `needs`: an item began to wait on a live, unpaused Job. `stopped`: a Job Fleet gave up restarting. */
export type CallArrivalKind = "needs" | "stopped";

/** News since the last publish, told once. */
export type CallArrival = {
  session_id: string;
  /** The Job's title, or `Job`. */
  title: string;
  text: string;
  tone: CallTone;
  kind: CallArrivalKind;
};

/** What `calls.changed` carries: the calls whole, and the news. `arrived` is absent where only openedness or the list moved. */
export type CallsChanged = {
  calls: StandingCall[];
  arrived?: CallArrival[];
};

/** `open_call` (`POST /calls/open`): he opened a main or pull request call. A Job's call is opened by `view_session`. */
export type OpenStandingCall = {
  key: string;
};

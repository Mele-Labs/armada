// What calls the person, and which of it he has not opened, read by Fleet so a client does not work it out:
// `get_calls`, `calls.changed` and `open_call`. Mirrors `crates/ipc/src/standing_calls.rs`. States and the words a
// call says; what is played, shown or badged stays with the client.

/**
 * What a call is about: a live Job (an item it waits on him for, or its row where it needs him), main read red, a pull request of his that failed or waits to be merged by hand, a held checkout, another Mac waiting for his Confirm here to pair (`mac_pairing`), or a pull request somebody asked him by name to review (`review_requested`).
 * A `mac_pairing` call's acts are the Phone Gateway's: Confirm is `POST /admin/macs/pair/confirm` and Decline is `POST /admin/macs/forget`, each with `{ mac }` the call's `mac`.
 * A `review_requested` call stands while he is asked, is opened once he has opened the pull request since he was asked, and is on the Dock count until then: the reviews' own count is the `review_requested` calls not opened.
 */
export type StandingCallKind =
  | "job"
  | "main"
  | "pull"
  | "checkout"
  | "mac_pairing"
  | "review_requested";

/** `blocked` where the item holds the Job's turn (a question, a permission, a plan, a slot or delivery choice, a done ask, a browser ask, a start card); `waiting` otherwise. */
export type CallTone = "blocked" | "waiting";

/** One call on the person. */
export type StandingCall = {
  /** Electron's own: `session:<id>:<item>`, `session:<id>:needs:<turn>`, `main:<root>`, `checkout:<root>`, `pull:<number>`; `mac:<id>` for a Mac waiting to pair, and `review:<owner/name>#<number>` for a review asked of him, the repository lowercase. */
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
  /** He has seen it: a Job call while he is in the Job or has been since it began and the Job does not ask him, a main, checkout or pull request call once `open_call` said so. A Mac waiting to pair never is. */
  opened: boolean;
  /** The id of the paired Mac that holds the Job. Absent means this Mac. On `mac_pairing`, the id of the Mac waiting to pair, which Confirm and Decline name. */
  mac?: string;
  /** The pull request a `review_requested` call asks him to review. Absent on every other kind. */
  review?: ReviewAsked;
};

/** A pull request somebody asked him by name to review, as the forge listed it. */
export type ReviewAsked = {
  /** The served repository it is in, which the pull request routes name. */
  manifest_id: string;
  /** As the forge names it: `owner/name`. */
  repository: string;
  number: number;
  title: string;
  /** The pull request's author's login. */
  author: string;
  /** Its page on the forge. */
  address: string;
};

/** `get_calls` (`GET /calls`), and what `open_call` answers: every call standing, oldest first. */
export type StandingCalls = {
  calls: StandingCall[];
};

/** `needs`: an item began to wait on a live, unpaused Job. `stopped`: a Job Fleet gave up restarting. `mac_pairing`: another Mac began to wait for his Confirm to pair. `review_requested`: somebody asked him by name to review a pull request, told once a request. */
export type CallArrivalKind = "needs" | "stopped" | "mac_pairing" | "review_requested";

/** News since the last publish, told once. */
export type CallArrival = {
  /** The Job's id, or on `mac_pairing` and `review_requested` the call's key, which names no Job. */
  session_id: string;
  /** The Job's title, or `Job`; on `mac_pairing` and `review_requested` the call's title. */
  title: string;
  text: string;
  tone: CallTone;
  kind: CallArrivalKind;
  /** The call's key, as `StandingCall.key`. Set on `mac_pairing` and `review_requested`, where `session_id` carries it too for readers that do not read this yet. On `review_requested`, the call of this key carries the pull request. */
  key?: string;
  /** On `needs`, the id of the waiting item that began to wait, as `WaitingItem.id`, so a press on the banner opens that ask. */
  item?: string;
};

/** What `calls.changed` carries: the calls whole, and the news. `arrived` is absent where only openedness or the list moved. */
export type CallsChanged = {
  calls: StandingCall[];
  arrived?: CallArrival[];
};

/** `open_call` (`POST /calls/open`): he opened a main, checkout or pull request call. A Job's call is opened by `view_session`. */
export type OpenStandingCall = {
  key: string;
};

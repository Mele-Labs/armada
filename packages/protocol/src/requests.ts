// Requests: an HTTP request the owner or a Job's agent sends, with the cookie the owner gave for its
// host attached by Fleet. `crates/ipc/src/requests.rs`, `docs/practices/protocol.md`, *Requests*.
// Since 19 Oct 2026.
//
// The header rules in `protocol.ts` hold here. **Only `SetRequestCookie` carries a cookie's value**,
// Bridge to Fleet; every answer names the host whose cookie went and never the value. The file
// `~/.armada/requests.toml` is `RequestEnvironmentsFile`, and a save sends the same type whole.

/** What starts the name of an environment Fleet offers for a server the Job runs. A saved name may not start with it. */
export const SERVER_ENVIRONMENT_PREFIX = "server:";

/** What a request's address names to be replaced by its environment's base. */
export const BASE_PLACEHOLDER = "{{base}}";

/** One header. A list rather than a map: a name may come more than once. */
export type RequestHeader = { name: string; value: string };

/** Who sent a request: the owner from Requests, or the Job's agent through its tool. */
export type RequestBy = "owner" | "agent";

/**
 * `send_request`: `POST /requests/send`. Answered 200 with `RequestSent`, whether or not anything
 * answered. `session_id` is the Job open in Bridge, whose thread gets the row and whose agent is told;
 * absent, nothing is written. `method` is `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD` or `OPTIONS`.
 * A 422 `fleet.request_refused` for a `{{base}}` with no environment, an unknown environment, an address
 * that is not `http` or `https`, another method, or a body over 64 KiB.
 */
export type SendRequest = {
  session_id?: string;
  method: string;
  url: string;
  environment?: string;
  headers: RequestHeader[];
  body?: string;
};

/**
 * `send_request`'s answer. `status` is absent where nothing answered, and `error` says why (refused,
 * timed out, a name that did not resolve). `body` is cut at 256 KiB and `truncated` says so.
 * `cookie_host` is the host whose cookie went. `note` is Fleet's word: a 401 or 403 from a host with
 * no cookie says one can be given here.
 */
export type RequestSent = {
  url: string;
  status?: number;
  ms: number;
  headers: RequestHeader[];
  body: string;
  truncated: boolean;
  cookie_host?: string;
  error?: string;
  note?: string;
};

/**
 * `set_request_cookie`: `POST /requests/cookies`. `host` is letters, digits, `.` and `-`; `cookie` is the
 * whole `Cookie` header's value, one line of printable ASCII. Kept in the Keychain. Answers `RequestHosts`.
 * A 422 `fleet.request_cookie_refused`, a 500 `fleet.request_cookie_unkept`.
 */
export type SetRequestCookie = { host: string; cookie: string };

/** `forget_request_cookie`: `POST /requests/cookies/forget`. Answers `RequestHosts`. */
export type ForgetRequestCookie = { host: string };

/** `list_request_hosts` (`GET /requests/cookies`), and both cookie acts' answer: the hosts holding one, sorted. */
export type RequestHosts = { hosts: string[] };

/** One named base address, with no trailing `/`. */
export type RequestEnvironment = { name: string; base: string };

/**
 * The whole file, and the body of `save_request_environments` (`POST /requests/environments`). `version`
 * is 1. A 422 `fleet.request_environments_refused` names the field; a 500
 * `fleet.request_environments_unwritable`.
 */
export type RequestEnvironmentsFile = { version: number; environment: RequestEnvironment[] };

/** The file as read. A file that will not read is `not_read` with why, and is never drawn as empty. */
export type RequestEnvironmentsRead =
  | { what: "read"; file: RequestEnvironmentsFile }
  | { what: "not_read"; why: string };

/**
 * `list_request_environments`: `GET /requests/environments?session_id=`. `servers` is one for each server
 * running in the Job's slot, named `server:<name>`; empty with no Job named.
 */
export type RequestEnvironments = { saved: RequestEnvironmentsRead; servers: RequestEnvironment[] };

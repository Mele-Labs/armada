// Requests: an HTTP request the owner or a Job's agent sends, collections of saved requests, and environments of variables
// a request's `{{name}}` is filled from. `crates/ipc/src/requests.rs`, `docs/practices/protocol.md`, *Requests*.
// Since 19 Oct 2026; collections and environments since 21 Oct 2026.
//
// The header rules in `protocol.ts` hold here. **No answer carries a secret's value**: `VariableRead` has no field for
// one, and everything Fleet answers or writes carries a secret as `{{name}}`. A secret's value goes once, Bridge to
// Fleet, in `VariableWrite`, and Fleet keeps it in the Keychain. The file both places keep, `RequestsFile` in Rust, is
// Fleet's to read and is not mirrored: Bridge sees only what `get_requests` answers.

import type { RunPlace } from "./run-memory";

/** What starts the name of an environment Fleet offers for a server the Job runs. A saved name may not start with it. */
export const SERVER_ENVIRONMENT_PREFIX = "server:";

/** The one variable a server's environment holds: the address it serves on. */
export const SERVER_BASE_VARIABLE = "base";

/** One header. A list rather than a map: a name may come more than once. */
export type RequestHeader = { name: string; value: string };

/** Who sent a request: the owner from Requests, or the Job's agent through its tool. */
export type RequestBy = "owner" | "agent";

/** Where a collection or environment is kept: the person's own for the repository, the repository's committed one, or a Job's running server. */
export type RequestPlace = "yours" | "repository" | "server";

/** A request saved in a collection. `id` is unique within its collection. */
export type SavedRequest = { id: string; name: string; method: string; url: string; headers: RequestHeader[]; body?: string };

/** A folder of saved requests. Its `headers` go with every request in it; a request's own header of the same name wins. */
export type RequestCollection = { id: string; name: string; headers: RequestHeader[]; requests: SavedRequest[] };

/** A variable as Fleet answers it: a plain value, or a secret of which only whether the Keychain holds a value comes back. */
export type VariableRead = { name: string; secret: false; value: string } | { name: string; secret: true; set: boolean };

/**
 * A variable as Bridge writes it. **A secret's `value` goes once, toward Fleet, which keeps it in the Keychain** and never
 * answers it. A secret written without `value` keeps the one already kept. A plain one with no `value` is refused.
 */
export type VariableWrite = { name: string; secret: false; value: string } | { name: string; secret: true; value?: string };

/** A named set of variables. A server's is `server:<name>` and holds `base` alone. */
export type RequestEnvironmentRead = { name: string; variables: VariableRead[] };
export type RequestEnvironmentWrite = { name: string; variables: VariableWrite[] };

/** A collection or environment as listed: where it is kept, and whether it is the person's own hiding the repository's of the same id or name. */
export type Placed<T> = T & { place: RequestPlace; hides: boolean };

/** A file that would not read, and why. None of its items are listed, and a save into the person's own is refused while it will not read. */
export type RequestsNotRead = { place: RequestPlace; why: string };

/**
 * `get_requests`: `GET /requests?repository=&session_id=`, and each save's answer. `repository` is a root `list_repositories`
 * names; `place` is where the person's own file was read (`RunPlace`). The person's own wins over the repository's by
 * collection `id` and environment `name`, and the repository's it hides is not listed. Server environments follow, with a
 * Job named. `not_read` is empty where both files read. A 422 `fleet.requests_unplaced` where nothing names a repository.
 */
export type Requests = {
  repository: string;
  place: RunPlace;
  collections: Placed<RequestCollection>[];
  environments: Placed<RequestEnvironmentRead>[];
  not_read: RequestsNotRead[];
};

/**
 * `save_request_collection`: `POST /requests/collections`. The whole collection, never a patch, into the person's own.
 * Answers `Requests`. A 422 `fleet.request_collection_refused` names the field; a 409 `fleet.requests_unread` where the
 * person's own will not read; a 500 `fleet.requests_unwritable`.
 */
export type SaveRequestCollection = { repository?: string; session_id?: string; collection: RequestCollection };

/**
 * `save_request_environment`: `POST /requests/environments`. The whole environment into the person's own. **The one body
 * that carries a secret's value.** A variable left out is forgotten, its Keychain item with it. Answers `Requests`. A 422
 * `fleet.request_environment_refused`, a 409 `fleet.requests_unread`, a 500 `fleet.requests_unwritable` or
 * `fleet.request_secret_unkept`.
 */
export type SaveRequestEnvironment = { repository?: string; session_id?: string; environment: RequestEnvironmentWrite };

/** A request the agent saves: a `SavedRequest` before Fleet gives it an id. */
export type RequestToSave = { name: string; method: string; url: string; headers: RequestHeader[]; body?: string };

/**
 * `save_request_to_collection`: `POST /requests/collections/request`, the agent's. `collection` is an id or a name; one of
 * that name is made in the person's own where none is, and the repository's is copied into the person's own. The thread
 * gets a `saved` row.
 */
export type SaveRequestToCollection = { session_id?: string; collection: string; request: RequestToSave };

/** `save_request_to_collection`'s answer: where the request is kept now. */
export type RequestSaved = { collection_id: string; request_id: string };

/** A saved request, by its collection's id or name and its own id or name. */
export type SavedRequestNamed = { collection: string; request: string };

/**
 * `send_request`: `POST /requests/send`. Answered 200 with `RequestSent`, whether or not anything answered. Either `saved`,
 * or the request's own `method` and `url` with an optional `collection` whose headers go under its own; both, or neither,
 * is refused. Fleet fills every `{{name}}` in the address, the headers and the body from `environment`, secrets included,
 * and sends one it cannot fill as written. With a saved request, `headers` go over its own and `body` replaces its own.
 * `session_id` is the Job open in Bridge, whose thread gets the row and whose agent is told; absent, nothing is written.
 * A 422 `fleet.request_refused`, a 500 `fleet.request_secret_unkept` or `fleet.request_unsent`.
 */
export type SendRequest = {
  session_id?: string;
  repository?: string;
  environment?: string;
  headers: RequestHeader[];
  body?: string;
} & ({ saved: SavedRequestNamed; method?: never; url?: never; collection?: never } | { saved?: never; method: string; url: string; collection?: string });

/**
 * `send_request`'s answer. `url` is the address filled in. `status` is absent where nothing answered, and `error` says why
 * (refused, timed out, a name that did not resolve). `body` is cut at 256 KiB and `truncated` says so. `note` names each
 * `{{name}}` sent unfilled. **A secret is `{{name}}` in every field.**
 */
export type RequestSent = {
  url: string;
  status?: number;
  ms: number;
  headers: RequestHeader[];
  body: string;
  truncated: boolean;
  error?: string;
  note?: string;
};

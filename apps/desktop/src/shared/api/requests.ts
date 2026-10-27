// Requests: the collections and environments kept for a repository, a save of one whole, and a send through Fleet. A slice
// imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them. `docs/practices/protocol.md`, *Requests*.

import type { Outcome, RequestCollection, RequestEnvironmentWrite, Requests, RequestSent, SendRequest } from "@armada/protocol";

/** What one of these came to: Fleet's answer, or the refusal to word. */
export type RequestsAnswer<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type RequestsApi = {
  /** `GET /requests`: the collections and environments of the Job's repository, or of `repository`, a root `list_repositories` names. */
  readRequests: (sessionId?: string, repository?: string) => Promise<RequestsAnswer<Requests>>;
  /** `POST /requests/collections`: the whole collection, never a patch, into the person's own. Answers the read again. */
  saveRequestCollection: (collection: RequestCollection, sessionId?: string, repository?: string) => Promise<RequestsAnswer<Requests>>;
  /** `POST /requests/environments`: the whole environment into the person's own. **The one member that carries a secret**, toward Fleet only. */
  saveRequestEnvironment: (environment: RequestEnvironmentWrite, sessionId?: string, repository?: string) => Promise<RequestsAnswer<Requests>>;
  /** `POST /requests/environments/delete`: the environment `name` out of the person's own, its secrets forgotten. Answers the read again. */
  deleteRequestEnvironment: (name: string, sessionId?: string, repository?: string) => Promise<RequestsAnswer<Requests>>;
  /** `POST /requests/send`. With `session_id`, the Job's thread gets the row and its agent is told. Nothing answering is an answer, with `error`. */
  sendRequest: (send: SendRequest) => Promise<RequestsAnswer<RequestSent>>;
};

export const REQUESTS_CHANNELS = {
  readRequests: "bridge:read-requests",
  saveRequestCollection: "bridge:save-request-collection",
  saveRequestEnvironment: "bridge:save-request-environment",
  deleteRequestEnvironment: "bridge:delete-request-environment",
  sendRequest: "bridge:send-request",
} as const;

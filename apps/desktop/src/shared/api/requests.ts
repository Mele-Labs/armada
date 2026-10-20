// Requests: an HTTP request sent through Fleet with the owner's cookie for its host, the hosts holding one and the named
// base addresses. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.
// `docs/practices/protocol.md`, *Requests*.

import type { Outcome, RequestEnvironments, RequestEnvironmentsFile, RequestEnvironmentsRead, RequestHosts, RequestSent, SendRequest } from "@armada/protocol";

/** What one of these came to: Fleet's answer, or the refusal to word. */
export type RequestsAnswer<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type RequestsApi = {
  /** `POST /requests/send`. With `session_id`, the Job's thread gets the row and its agent is told. Nothing answering is an answer, with `error`. */
  sendRequest: (send: SendRequest) => Promise<RequestsAnswer<RequestSent>>;
  /** `GET /requests/cookies`: the hosts holding a cookie. Never a value. */
  listRequestHosts: () => Promise<RequestsAnswer<RequestHosts>>;
  /** `POST /requests/cookies`. **The one member that carries a cookie's value**, and only toward Fleet; the answer names hosts. */
  setRequestCookie: (host: string, cookie: string) => Promise<RequestsAnswer<RequestHosts>>;
  /** `POST /requests/cookies/forget`. */
  forgetRequestCookie: (host: string) => Promise<RequestsAnswer<RequestHosts>>;
  /** `GET /requests/environments`: the saved file, and with a Job named the servers it runs. */
  listRequestEnvironments: (sessionId?: string) => Promise<RequestsAnswer<RequestEnvironments>>;
  /** `POST /requests/environments`: the whole file, never a patch. Answers it read again. */
  saveRequestEnvironments: (file: RequestEnvironmentsFile) => Promise<RequestsAnswer<RequestEnvironmentsRead>>;
};

export const REQUESTS_CHANNELS = {
  sendRequest: "bridge:send-request",
  listRequestHosts: "bridge:list-request-hosts",
  setRequestCookie: "bridge:set-request-cookie",
  forgetRequestCookie: "bridge:forget-request-cookie",
  listRequestEnvironments: "bridge:list-request-environments",
  saveRequestEnvironments: "bridge:save-request-environments",
} as const;

// Requests, as main asks Fleet: a send, the hosts holding a cookie and the named base addresses. One request each and
// nothing kept here. **A cookie's value passes through once, toward Fleet**, and nothing answered carries one back.
// `docs/practices/protocol.md`, *Requests*.

import type { Outcome, RequestEnvironments, RequestEnvironmentsFile, RequestEnvironmentsRead, RequestHosts, RequestSent, SendRequest } from "@armada/protocol";
import type { RequestsAnswer } from "../shared/api/requests";
import { ask, COMMAND_MS } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/**
 * How long a send is waited for. **Fleet stops a request at thirty seconds** (`fleet::requesting::TIMEOUT`) and answers
 * that as an `error`; this is that plus five, so Fleet's answer arrives rather than Bridge's abort. Coupled by this comment.
 */
export const SEND_REQUEST_MS = 35_000;

export class RequestsHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  send(body: SendRequest): Promise<RequestsAnswer<RequestSent>> {
    return this.ask("POST", "/requests/send", body, SEND_REQUEST_MS);
  }

  hosts(): Promise<RequestsAnswer<RequestHosts>> {
    return this.ask("GET", "/requests/cookies");
  }

  setCookie(host: string, cookie: string): Promise<RequestsAnswer<RequestHosts>> {
    return this.ask("POST", "/requests/cookies", { host, cookie });
  }

  forgetCookie(host: string): Promise<RequestsAnswer<RequestHosts>> {
    return this.ask("POST", "/requests/cookies/forget", { host });
  }

  environments(sessionId: string | undefined): Promise<RequestsAnswer<RequestEnvironments>> {
    return this.ask("GET", `/requests/environments${sessionId === undefined ? "" : `?session_id=${encodeURIComponent(sessionId)}`}`);
  }

  saveEnvironments(file: RequestEnvironmentsFile): Promise<RequestsAnswer<RequestEnvironmentsRead>> {
    return this.ask("POST", "/requests/environments", file);
  }

  private async ask<T>(method: "GET" | "POST", path: string, body?: unknown, waitMs: number = COMMAND_MS): Promise<RequestsAnswer<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body, waitMs);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }
}

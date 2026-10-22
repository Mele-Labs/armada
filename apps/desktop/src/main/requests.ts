// Requests, as main asks Fleet: a read of a repository's collections and environments, a save of one whole, and a send. One
// request each and nothing kept here. **A secret's value passes through once, toward Fleet**, and nothing answered carries one
// back. `docs/practices/protocol.md`, *Requests*.

import type { Outcome, Requests, RequestSent, SaveRequestCollection, SaveRequestEnvironment, SendRequest } from "@armada/protocol";
import type { RequestsAnswer } from "../shared/api/requests";
import { ask, COMMAND_MS } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/**
 * How long a send is waited for. **Fleet stops a request at thirty seconds** (`fleet::requesting::TIMEOUT`) and answers
 * that as an `error`; this is that plus five, so Fleet's answer arrives rather than Bridge's abort. Coupled by this comment.
 */
export const SEND_REQUEST_MS = 35_000;

/** Which repository a read or save is for: a Job's, or a root `list_repositories` names. */
export type RequestsWhere = { session_id?: string; repository?: string };

/** `?repository=&session_id=`, with only what is named. */
export function requestsQuery(where: RequestsWhere): string {
  const query = new URLSearchParams();
  if (where.repository !== undefined) query.set("repository", where.repository);
  if (where.session_id !== undefined) query.set("session_id", where.session_id);
  const said = query.toString();
  return said === "" ? "" : `?${said}`;
}

export class RequestsHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  read(where: RequestsWhere): Promise<RequestsAnswer<Requests>> {
    return this.ask("GET", `/requests${requestsQuery(where)}`);
  }

  saveCollection(body: SaveRequestCollection): Promise<RequestsAnswer<Requests>> {
    return this.ask("POST", "/requests/collections", body);
  }

  saveEnvironment(body: SaveRequestEnvironment): Promise<RequestsAnswer<Requests>> {
    return this.ask("POST", "/requests/environments", body);
  }

  send(body: SendRequest): Promise<RequestsAnswer<RequestSent>> {
    return this.ask("POST", "/requests/send", body, SEND_REQUEST_MS);
  }

  private async ask<T>(method: "GET" | "POST", path: string, body?: unknown, waitMs: number = COMMAND_MS): Promise<RequestsAnswer<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body, waitMs);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }
}

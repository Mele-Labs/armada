// What Requests reads and writes, with no React: collections of saved requests, environments of variables, `{{name}}` filled
// in from the environment picked, a request opened from a thread row, an answer's body set out where it is JSON, and a Job's history.

import type { RequestHeader, RequestSent } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { RequestsAnswer } from "../../../shared/api/requests";
import type { RequestStart } from "./store";

export type RequestedRow = Extract<Session["rows"][number], { kind: "requested" }>;

// **PROPOSED, not on the wire**, from here to `METHODS`. Fleet serves none of these yet: only the mock slice does, for the walk
// `requests-collections-and-environments`. The wire is built from them once the owner approves that walk.

/** Where a collection or environment is kept: the person's own for the repository, the repository's committed one, or a Job's running server. */
export type RequestPlace = "yours" | "repository" | "server";

/** A request saved in a collection. `id` is unique within its collection. */
export type SavedRequest = { id: string; name: string; method: string; url: string; headers: RequestHeader[]; body?: string };

/** A folder of saved requests. Its `headers` go with every request in it; a request's own header of the same name wins. */
export type RequestCollection = { id: string; name: string; headers: RequestHeader[]; requests: SavedRequest[] };

/** A variable as Fleet answers it: a plain value, or a secret of which only whether it is set comes back. */
export type VariableRead = { name: string; secret: false; value: string } | { name: string; secret: true; set: boolean };

/**
 * A variable as Bridge writes it. **A secret's `value` goes once, toward Fleet, which keeps it in the Keychain** and never
 * answers it. A secret written without `value` keeps the one already kept.
 */
export type VariableWrite = { name: string; secret: false; value: string } | { name: string; secret: true; value?: string };

/** A named set of variables. A server's is `server:<name>` and offers `base` alone. */
export type RequestEnvironmentRead = { name: string; variables: VariableRead[] };
export type RequestEnvironmentWrite = { name: string; variables: VariableWrite[] };

/** A collection or environment as Requests draws it: where it is kept, and whether the person's own hides the repository's of the same name. */
export type Placed<T> = T & { place: RequestPlace; hides: boolean };

/**
 * What Requests reads for the repository a Job works in. The person's own wins over the repository's of the same name, so a
 * hidden one is not listed: the person's carries `hides`. Server environments are listed with a Job named, and are not written.
 */
export type RequestsRead = { collections: Placed<RequestCollection>[]; environments: Placed<RequestEnvironmentRead>[] };

/**
 * A send. Fleet fills every `{{name}}` in `url`, `headers` and `body` from `environment`, secrets included, and adds the
 * headers of `collection`. With `session_id`, the Job's thread gets a `requested` row holding the request as written, never a secret.
 */
export type ProposedSend = { session_id?: string; method: string; url: string; environment?: string; collection?: string; headers: RequestHeader[]; body?: string };

/** The answer, as `RequestSent` is today without the kept header that model drops. `url` is the address filled in. */
export type ProposedSent = Omit<RequestSent, "cookie_host" | "kept_header">;

/** What the agent hands Fleet to save a request: the collection by name, made where none of that name is. The thread gets a `saved` row. */
export type AgentSavesRequest = { session_id: string; collection: string; request: Omit<SavedRequest, "id"> };

/** The members Requests calls. `sessionId` names the Job whose repository is read; absent, Fleet picks the place. */
export type ProposedRequestsApi = {
  readRequests: (sessionId?: string) => Promise<RequestsAnswer<RequestsRead>>;
  /** The whole collection, never a patch, into the person's own. Answers the read again. */
  saveRequestCollection: (collection: RequestCollection, sessionId?: string) => Promise<RequestsAnswer<RequestsRead>>;
  /** The whole environment into the person's own. **The one member that carries a secret.** Answers the read again. */
  saveRequestEnvironment: (environment: RequestEnvironmentWrite, sessionId?: string) => Promise<RequestsAnswer<RequestsRead>>;
  sendSavedRequest: (send: ProposedSend) => Promise<RequestsAnswer<ProposedSent>>;
};

/** The methods Fleet sends. */
export const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] as const;

/** Headers one to a line, `Name: value`. */
export const headersText = (headers: readonly RequestHeader[]): string => headers.map((one) => `${one.name}: ${one.value}`).join("\n");

/** The box's headers, or the first line that is not `Name: value`. Blank lines are skipped. */
export function headersOf(text: string): { headers: RequestHeader[] } | { bad: string } {
  const headers: RequestHeader[] = [];
  for (const line of text.split("\n")) {
    if (line.trim() === "") continue;
    const at = line.indexOf(":");
    const name = at < 0 ? "" : line.slice(0, at).trim();
    if (name === "" || /\s/.test(name)) return { bad: line.trim() };
    headers.push({ name, value: line.slice(at + 1).trim() });
  }
  return { headers };
}

const NAME = /\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g;

/** The `{{name}}`s a text holds, in order, once each. */
export const namesIn = (text: string): string[] => [...new Set([...text.matchAll(NAME)].map((one) => one[1]!))];

/**
 * The text with each plain variable of `environment` in place. A secret stays `{{name}}`, since Bridge never holds it, and so
 * does a name the environment lacks, which `missing` lists.
 */
export function filledOf(text: string, environment: RequestEnvironmentRead | undefined): { text: string; missing: string[] } {
  const missing: string[] = [];
  const filled = text.replace(NAME, (whole, name: string) => {
    const one = environment?.variables.find((variable) => variable.name === name);
    if (one === undefined) {
      missing.push(name);
      return whole;
    }
    return one.secret ? whole : one.value;
  });
  return { text: filled, missing: [...new Set(missing)] };
}

/** A collection's headers under a request's own: a name the request carries is the request's. */
export function headersWith(collection: readonly RequestHeader[], own: readonly RequestHeader[]): RequestHeader[] {
  const named = new Set(own.map((one) => one.name.toLowerCase()));
  return [...collection.filter((one) => !named.has(one.name.toLowerCase())), ...own];
}

/** A row's request to start from. Its address is filled on the row, so it reads `{{base}}` again where it starts with its environment's base. */
export function startOf(row: Pick<RequestedRow, "method" | "url" | "environment" | "headers" | "body">, environments: readonly RequestEnvironmentRead[] = []): RequestStart {
  const variable = environments.find((one) => one.name === row.environment)?.variables.find((one) => one.name === "base");
  const base = variable === undefined || variable.secret ? undefined : variable.value;
  const url = base !== undefined && base !== "" && row.url.startsWith(base) ? `{{base}}${row.url.slice(base.length)}` : row.url;
  return {
    method: row.method,
    url,
    ...(row.environment === undefined ? {} : { environment: row.environment }),
    headers: row.headers,
    ...(row.body === undefined ? {} : { body: row.body }),
  };
}

/** The answer's body set out two spaces deep where it is JSON, and as it came otherwise. */
export function bodyShown(body: string): { text: string; json: boolean } {
  const trimmed = body.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) return { text: body, json: false };
  try {
    return { text: JSON.stringify(JSON.parse(trimmed), null, 2), json: true };
  } catch {
    return { text: body, json: false };
  }
}

/** The Job's requests, newest first. */
export const historyOf = (session: Session | undefined): RequestedRow[] =>
  (session?.rows ?? []).filter((row): row is RequestedRow => row.kind === "requested").reverse();

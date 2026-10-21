// What Requests reads and writes, with no React: collections of saved requests, environments of variables, `{{name}}` filled
// in from the environment picked, a request opened from a thread row, an answer's body set out where it is JSON, and a Job's history.

import type { RequestEnvironmentRead, RequestHeader, Requests, RequestsNotRead } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { RequestStart } from "./store";

export type RequestedRow = Extract<Session["rows"][number], { kind: "requested" }>;

/** Which file would not read, by where it is kept. The person's own has no path Fleet answers, so it is named by whose it is. */
export const unreadFileOf = (one: RequestsNotRead): string =>
  one.place === "repository" ? "The repository's .armada/requests.toml" : one.place === "yours" ? "Your own requests file" : "A server's environment";

/** The person's own file, where it would not read. Fleet refuses a save over it (409 `fleet.requests_unread`), so none is offered. */
export const yoursUnread = (read: Requests | undefined): RequestsNotRead | undefined => read?.not_read.find((one) => one.place === "yours");

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

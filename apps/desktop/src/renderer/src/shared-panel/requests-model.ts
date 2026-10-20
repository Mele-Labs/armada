// What Requests reads and writes, with no React: the headers as the box spells them, the address with its environment's
// base in place, a request opened from a thread row, an answer's body set out where it is JSON, and a Job's history.

import { BASE_PLACEHOLDER } from "@armada/protocol";
import type { RequestEnvironment, RequestEnvironments, RequestHeader } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { RequestStart } from "./store";

export type RequestedRow = Extract<Session["rows"][number], { kind: "requested" }>;

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

/** Every environment the picker offers: the saved ones, then the Job's servers. A file that did not read offers none of its own. */
export const environmentsOf = (read: RequestEnvironments | undefined): RequestEnvironment[] =>
  read === undefined ? [] : [...(read.saved.what === "read" ? read.saved.file.environment : []), ...read.servers];

/** The address with `{{base}}` replaced, or `undefined` where it holds none or the environment is not known. */
export function resolvedOf(url: string, environment: string, environments: readonly RequestEnvironment[]): string | undefined {
  if (!url.includes(BASE_PLACEHOLDER)) return undefined;
  const base = environments.find((one) => one.name === environment)?.base;
  return base === undefined ? undefined : url.replaceAll(BASE_PLACEHOLDER, base);
}

/** A row's request to start from. Its address is resolved on the row, so it reads `{{base}}` again where it starts with its environment's base. */
export function startOf(row: Pick<RequestedRow, "method" | "url" | "environment" | "headers" | "body">, environments: readonly RequestEnvironment[] = []): RequestStart {
  const base = row.environment === undefined ? undefined : environments.find((one) => one.name === row.environment)?.base;
  const url = base !== undefined && row.url.startsWith(base) ? `${BASE_PLACEHOLDER}${row.url.slice(base.length)}` : row.url;
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

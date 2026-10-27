// The channels Requests travels over: a request to Fleet each, and the renderer names no port. What crosses from the
// renderer is not trusted, so each is checked for its shape here and refused as unsent where it has none. Fleet checks
// what only it can: a method it sends, a header name, an id used twice.

import type { IpcMain } from "electron";
import type { RequestCollection, RequestEnvironmentWrite, RequestHeader, SavedRequest, SendRequest, VariableWrite } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import type { RequestsWhere } from "./requests";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";
const text = (value: unknown): value is string => typeof value === "string";
const record = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);

/** Each of `value` as `one` reads it, or `null` where any one does not read. */
function listOf<T>(value: unknown, one: (item: unknown) => T | null): T[] | null {
  if (!Array.isArray(value)) return null;
  const read = value.map(one);
  return read.every((item): item is T => item !== null) ? read : null;
}

const headerOf = (value: unknown): RequestHeader | null => (record(value) && text(value.name) && text(value.value) ? { name: value.name, value: value.value } : null);
const headersOf = (value: unknown): RequestHeader[] | null => listOf(value, headerOf);

/** A Job, a repository, both or neither; anything else named is refused. */
export function whereOf(sessionId: unknown, repository: unknown): RequestsWhere | null {
  if ((sessionId !== undefined && !named(sessionId)) || (repository !== undefined && !named(repository))) return null;
  return { ...(sessionId === undefined ? {} : { session_id: sessionId }), ...(repository === undefined ? {} : { repository }) };
}

function savedRequestOf(value: unknown): SavedRequest | null {
  if (!record(value)) return null;
  const { id, name, method, url, headers, body } = value;
  const listed = headersOf(headers);
  if (!text(id) || !text(name) || !text(method) || !text(url) || listed === null || (body !== undefined && !text(body))) return null;
  return { id, name, method, url, headers: listed, ...(body === undefined ? {} : { body }) };
}

/** A whole collection, its own fields only: `place` and `hides` are the read's, not the file's. */
export function collectionOf(value: unknown): RequestCollection | null {
  if (!record(value)) return null;
  const { id, name, headers, requests } = value;
  const listed = headersOf(headers);
  const saved = listOf(requests, savedRequestOf);
  if (!text(id) || !text(name) || listed === null || saved === null) return null;
  return { id, name, headers: listed, requests: saved };
}

function variableOf(value: unknown): VariableWrite | null {
  if (!record(value) || !text(value.name)) return null;
  const { name, secret, value: kept } = value;
  if (secret === false) return text(kept) ? { name, secret, value: kept } : null;
  if (secret === true) return kept === undefined ? { name, secret } : text(kept) ? { name, secret, value: kept } : null;
  return null;
}

/** A whole environment. A secret's `value` is passed on once, toward Fleet, and is the only one this channel carries. */
export function environmentOf(value: unknown): RequestEnvironmentWrite | null {
  if (!record(value) || !text(value.name)) return null;
  const variables = listOf(value.variables, variableOf);
  return variables === null ? null : { name: value.name, variables };
}

/** A send's body, or `null` for anything else: a saved request by its names, or one's own method and address, never both. */
export function sendOf(body: unknown): SendRequest | null {
  if (!record(body)) return null;
  const { session_id, repository, environment, headers, body: sent, saved, method, url, collection } = body;
  const listed = headersOf(headers);
  const where = whereOf(session_id, repository);
  if (where === null || listed === null || (environment !== undefined && !named(environment)) || (sent !== undefined && !text(sent))) return null;
  const rest = { ...where, ...(environment === undefined ? {} : { environment }), headers: listed, ...(sent === undefined ? {} : { body: sent }) };
  if (saved !== undefined) {
    if (method !== undefined || url !== undefined || collection !== undefined || !record(saved) || !named(saved.collection) || !named(saved.request)) return null;
    return { ...rest, saved: { collection: saved.collection, request: saved.request } };
  }
  if (!named(method) || !named(url) || (collection !== undefined && !named(collection))) return null;
  return { ...rest, method, url, ...(collection === undefined ? {} : { collection }) };
}

export function handleRequests({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.readRequests, (_event, sessionId: unknown, repository: unknown) => {
    const where = whereOf(sessionId, repository);
    return where === null ? unsent : (connection()?.requests.read(where) ?? unsent);
  });
  ipc.handle(CHANNELS.saveRequestCollection, (_event, body: unknown, sessionId: unknown, repository: unknown) => {
    const where = whereOf(sessionId, repository);
    const collection = collectionOf(body);
    return where === null || collection === null ? unsent : (connection()?.requests.saveCollection({ ...where, collection }) ?? unsent);
  });
  ipc.handle(CHANNELS.saveRequestEnvironment, (_event, body: unknown, sessionId: unknown, repository: unknown) => {
    const where = whereOf(sessionId, repository);
    const environment = environmentOf(body);
    return where === null || environment === null ? unsent : (connection()?.requests.saveEnvironment({ ...where, environment }) ?? unsent);
  });
  ipc.handle(CHANNELS.deleteRequestEnvironment, (_event, name: unknown, sessionId: unknown, repository: unknown) => {
    const where = whereOf(sessionId, repository);
    return where === null || !named(name) ? unsent : (connection()?.requests.deleteEnvironment({ ...where, name }) ?? unsent);
  });
  ipc.handle(CHANNELS.sendRequest, (_event, body: unknown) => {
    const send = sendOf(body);
    return send === null ? unsent : (connection()?.requests.send(send) ?? unsent);
  });
}

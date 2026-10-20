// The channels Requests travels over: a request to Fleet each, and the renderer names no port. What crosses from the
// renderer is not trusted, so each is checked for its shape here and refused as unsent where it has none.

import type { IpcMain } from "electron";
import type { RequestEnvironmentsFile, RequestHeader, SendRequest } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";
const text = (value: unknown): value is string => typeof value === "string";

function headersOf(value: unknown): RequestHeader[] | null {
  if (!Array.isArray(value)) return null;
  const headers = value.filter((one): one is RequestHeader => typeof one === "object" && one !== null && text(one.name) && text(one.value));
  return headers.length === value.length ? headers.map(({ name, value }) => ({ name, value })) : null;
}

/** A send's body, or `null` for anything else. Fleet checks the method, the address and the size itself. */
export function sendOf(body: unknown): SendRequest | null {
  if (typeof body !== "object" || body === null) return null;
  const { session_id, method, url, environment, headers, body: sent } = body as Record<string, unknown>;
  const listed = headersOf(headers);
  if (!named(method) || !named(url) || listed === null) return null;
  if ((session_id !== undefined && !named(session_id)) || (environment !== undefined && !named(environment)) || (sent !== undefined && !text(sent))) return null;
  return {
    ...(session_id === undefined ? {} : { session_id }),
    method,
    url,
    ...(environment === undefined ? {} : { environment }),
    headers: listed,
    ...(sent === undefined ? {} : { body: sent }),
  };
}

/** The whole environments file, or `null`. Fleet names the field it refuses. */
export function environmentsFileOf(body: unknown): RequestEnvironmentsFile | null {
  if (typeof body !== "object" || body === null) return null;
  const { version, environment } = body as Record<string, unknown>;
  if (typeof version !== "number" || !Array.isArray(environment)) return null;
  const listed = environment.filter((one): one is { name: string; base: string } => typeof one === "object" && one !== null && text(one.name) && text(one.base));
  return listed.length === environment.length ? { version, environment: listed.map(({ name, base }) => ({ name, base })) } : null;
}

export function handleRequests({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.sendRequest, (_event, body: unknown) => {
    const send = sendOf(body);
    return send === null ? unsent : (connection()?.requests.send(send) ?? unsent);
  });
  ipc.handle(CHANNELS.listRequestHosts, () => connection()?.requests.hosts() ?? unsent);
  ipc.handle(CHANNELS.setRequestCookie, (_event, host: unknown, header: unknown, value: unknown) =>
    named(host) && named(header) && named(value) ? (connection()?.requests.setCookie(host, header, value) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.forgetRequestCookie, (_event, host: unknown) => (named(host) ? (connection()?.requests.forgetCookie(host) ?? unsent) : unsent));
  ipc.handle(CHANNELS.listRequestEnvironments, (_event, sessionId: unknown) =>
    sessionId === undefined || named(sessionId) ? (connection()?.requests.environments(sessionId) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.saveRequestEnvironments, (_event, body: unknown) => {
    const file = environmentsFileOf(body);
    return file === null ? unsent : (connection()?.requests.saveEnvironments(file) ?? unsent);
  });
}

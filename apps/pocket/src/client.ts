// Every call to a Mac goes through `call`, which signs it with that Mac's key and sends it to that Mac's
// origin. A 401 means that Mac does not know this phone any more: it is that Mac's trouble alone, told
// to whoever listens, and never a reason to leave the screen the owner is on.

import type { Machine } from "./machines";
import { signHeaders } from "./signing";

export class Refused extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}

export const notAnswering = (name?: string) =>
  name === undefined || name === "" ? "Armada is not answering on your Mac. Open it there, then try again." : `Armada is not answering on ${name}. Open it there, then try again.`;

const unpaired = new Set<(origin: string, why: string) => void>();

/** Hears every 401, with the Mac's origin and its sentence. */
export function whenUnpaired(listen: (origin: string, why: string) => void): () => void {
  unpaired.add(listen);
  return () => unpaired.delete(listen);
}

export type CallInit = { method?: string; body?: unknown; signal?: AbortSignal };

export async function call(machine: Machine, path: string, init: CallInit = {}): Promise<Response> {
  const method = init.method ?? "GET";
  const body = init.body === undefined ? "" : JSON.stringify(init.body);
  const headers: Record<string, string> = { ...(await signHeaders(machine.key, machine.deviceId, method, path, body)) };
  if (body !== "") headers["Content-Type"] = "application/json";
  let response: Response;
  try {
    response = await fetch(machine.origin + path, { method, headers, ...(body === "" ? {} : { body }), ...(init.signal === undefined ? {} : { signal: init.signal }) });
  } catch {
    throw new Refused(0, notAnswering(machine.name));
  }
  if (response.status === 401) {
    const why = await response.text();
    unpaired.forEach((listen) => listen(machine.origin, why));
    throw new Refused(401, why);
  }
  if (!response.ok) throw new Refused(response.status, (await response.text()) || notAnswering(machine.name));
  return response;
}

export async function getJson<T>(machine: Machine, path: string): Promise<T> {
  return (await call(machine, path)).json() as Promise<T>;
}

/** The sentence a refusal carries, or the one for a Mac that did not answer. */
export const refusal = (why: unknown, machine?: Machine): string => (why instanceof Refused ? why.message : notAnswering(machine?.name));

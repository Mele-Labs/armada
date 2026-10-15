// What a row may hold. The store keeps opaque hashes and a holder, and refuses the rest, so
// a title, a repository name or code has nowhere to go. `docs/contracts/central-store.md`.

export type Holder = { kind: "session" | "job"; id: string; machine: string; since: number };

/** A hash made on the machine: hex or base64url, long enough that a name is not one. */
export const HASH = /^[A-Za-z0-9_-]{32,128}$/;

const HOLDER_ID = /^[A-Za-z0-9._-]{1,64}$/;
export const MACHINE = /^[^\p{Cc}]{1,64}$/u;

export const MAX_KEYS = 100;
export const MAX_BODY_BYTES = 4096;

// What one account may hold and ask for. `docs/contracts/central-store.md`, Limits.
export const MAX_DEVICES = 16;
export const MAX_CLAIMS = 10_000;
export const MAX_OWNERS = 10_000;
export const REQUESTS_PER_MINUTE = 120;
export const RATE_WINDOW_SECONDS = 60;

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** `value` as an object holding exactly `keys`: a missing key and an extra one both fail. */
export function exactly(value: unknown, keys: readonly string[]): Record<string, unknown> | null {
  if (!isObject(value)) return null;
  const present = Object.keys(value);
  if (present.length !== keys.length || !keys.every((key) => present.includes(key))) return null;
  return value;
}

export function holderOf(value: unknown): Holder | null {
  const row = exactly(value, ["kind", "id", "machine", "since"]);
  if (!row) return null;
  const { kind, id, machine, since } = row;
  if (kind !== "session" && kind !== "job") return null;
  if (typeof id !== "string" || !HOLDER_ID.test(id)) return null;
  if (typeof machine !== "string" || !MACHINE.test(machine)) return null;
  if (typeof since !== "number" || !Number.isSafeInteger(since) || since < 0) return null;
  return { kind, id, machine, since };
}

export function parseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

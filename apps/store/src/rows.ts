// What a row may hold. An owner is opaque hashes and a holder; a claim is opaque hashes and a holder
// with the person's name and the Job's title; a repository's runs are facts about how it runs. The
// store refuses the rest, so a repository name, a file name or code has nowhere to go.
// `docs/contracts/central-store.md`.

export type Holder = {
  kind: "session" | "job";
  id: string;
  machine: string;
  since: number;
};

/** A hash made on the machine: hex or base64url, long enough that a name is not one. */
export const HASH = /^[A-Za-z0-9_-]{32,128}$/;

const HOLDER_ID = /^[A-Za-z0-9._-]{1,64}$/;
export const MACHINE = /^[^\p{Cc}]{1,64}$/u;

export const MAX_KEYS = 100;
export const MAX_BODY_BYTES = 4096;
export const MAX_RUNS_BODY_BYTES = 131_072;

// What one account may hold and ask for. `docs/contracts/central-store.md`, Limits.
export const MAX_DEVICES = 16;
/** What a team store's one object may hold in devices: everyone in the org or team. */
export const MAX_TEAM_DEVICES = 256;
export const MAX_CLAIMS = 10_000;
/** Jobs refused a claim and not yet told to its holder. A full table still answers the refusal and keeps no row. */
export const MAX_CONTENDERS = 10_000;
export const MAX_OWNERS = 10_000;
export const MAX_RUNS = 1000;
export const MAX_FACTS = 200;
export const REQUESTS_PER_MINUTE = 120;
export const RATE_WINDOW_SECONDS = 60;
/** A holder's claims lapse this long after its last renewal, or after its last claim where it never renewed. */
export const LAPSE_SECONDS = 30 * 60;

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
  const row = exactly(value, [
    "kind",
    "id",
    "machine",
    "since",
  ]);
  if (!row) return null;
  const { kind, id, machine, since } = row;
  if (kind !== "session" && kind !== "job") return null;
  if (typeof id !== "string" || !HOLDER_ID.test(id)) return null;
  if (typeof machine !== "string" || !MACHINE.test(machine)) return null;
  if (typeof since !== "number" || !Number.isSafeInteger(since) || since < 0) return null;
  return {
    kind,
    id,
    machine,
    since,
  };
}

/** What a claim is on. Its key is a hash, and the same hash under two kinds is two claims. */
export const CLAIM_KINDS = [
  "item",
  "file",
  "test",
  "red",
  "pull_request",
] as const;
export type ClaimKind = (typeof CLAIM_KINDS)[number];

export const claimKindOf = (value: unknown): ClaimKind | null =>
  CLAIM_KINDS.includes(value as ClaimKind) ? (value as ClaimKind) : null;

/** A claim's holder, with the person's name and the Job's title in the clear so another Mac can say who. */
export type ClaimHolder = Holder & {
  name: string;
  title: string;
};

// Empty is allowed: a Mac that knows no name or title sends none rather than a placeholder.
const NAME = /^[^\p{Cc}]{0,64}$/u;
const TITLE = /^[^\p{Cc}]{0,200}$/u;

export function claimHolderOf(value: unknown): ClaimHolder | null {
  const row = exactly(value, [
    "kind",
    "id",
    "machine",
    "since",
    "name",
    "title",
  ]);
  if (!row) return null;
  const { name, title, ...rest } = row;
  const holder = holderOf(rest);
  if (!holder) return null;
  if (typeof name !== "string" || !NAME.test(name)) return null;
  if (typeof title !== "string" || !TITLE.test(title)) return null;
  return {
    ...holder,
    name,
    title,
  };
}

/** Who a renewal is for: a holder without the parts that belong to one claim. */
export type Renewing = Pick<Holder, "kind" | "id" | "machine">;

export function renewingOf(value: unknown): Renewing | null {
  const row = exactly(value, [
    "kind",
    "id",
    "machine",
  ]);
  if (!row) return null;
  const holder = holderOf({
    ...row,
    since: 0,
  });
  return holder
    ? {
        kind: holder.kind,
        id: holder.id,
        machine: holder.machine,
      }
    : null;
}

/** One thing a repository ran: the facts Fleet keeps for the Kit `runs` place `cloud`. */
export type Fact = {
  workspace: string;
  kind: (typeof FACT_KINDS)[number];
  command: string;
  port?: number;
  succeeded: boolean;
  last_ran: number;
  source: string;
};

const FACT_KINDS = [
  "tests",
  "lint",
  "build",
  "dev_server",
  "typecheck",
  "format",
] as const;
const FACT_SOURCE = /^[A-Za-z0-9_-]{1,32}$/;
const CONTROL = /\p{Cc}/u;

const textOf = (value: unknown, most: number): value is string =>
  typeof value === "string" && value.length >= 1 && value.length <= most && !CONTROL.test(value);

const FACT_KEYS = [
  "workspace",
  "kind",
  "command",
  "succeeded",
  "last_ran",
  "source",
] as const;

export function factOf(value: unknown): Fact | null {
  // `port` may be absent, so the exact key check runs on the set the fact has.
  const withPort = isObject(value) && "port" in value;
  const row = exactly(
    value,
    withPort
      ? [
          ...FACT_KEYS,
          "port",
        ]
      : FACT_KEYS,
  );
  if (!row) return null;
  const { workspace, kind, command, port, succeeded, last_ran, source } = row;
  if (!textOf(workspace, 200) || workspace.startsWith("/") || workspace.split("/").includes(".."))
    return null;
  if (!FACT_KINDS.includes(kind as Fact["kind"])) return null;
  if (!textOf(command, 400)) return null;
  if (withPort && (typeof port !== "number" || !Number.isInteger(port) || port < 1 || port > 65535))
    return null;
  if (typeof succeeded !== "boolean") return null;
  if (typeof last_ran !== "number" || !Number.isSafeInteger(last_ran) || last_ran < 0) return null;
  if (typeof source !== "string" || !FACT_SOURCE.test(source)) return null;
  const fact = {
    workspace,
    kind: kind as Fact["kind"],
    command,
    succeeded,
    last_ran,
    source,
  };
  return withPort
    ? {
        ...fact,
        port: port as number,
      }
    : fact;
}

/** `value` as at most `MAX_FACTS` facts with no (workspace, kind, command) twice: an empty list is allowed. */
export function factsOf(value: unknown): Fact[] | null {
  if (!Array.isArray(value) || value.length > MAX_FACTS) return null;
  const facts: Fact[] = [];
  const seen = new Set<string>();
  for (const item of value) {
    const fact = factOf(item);
    if (!fact) return null;
    const identity = JSON.stringify([
      fact.workspace,
      fact.kind,
      fact.command,
    ]);
    if (seen.has(identity)) return null;
    seen.add(identity);
    facts.push(fact);
  }
  return facts;
}

export function parseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

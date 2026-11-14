import { env } from "cloudflare:workers";
import { runInDurableObject } from "cloudflare:test";
import { vi } from "vitest";
import { signRequest, toBase64, type SignedHeaders } from "../src/signing";
import worker from "../src/index";
import type { ClaimHolder, ClaimKind, Fact, Holder } from "../src/rows";

export const STORE = "https://store.test";

/** The Worker vars a test sets on top of the bindings: `TEAM` makes the store a team store. */
export type Vars = Partial<Pick<Env, "TEAM" | "ENROL_LIMIT">>;

/** A request to the Worker, as the platform would hand it over. The vars go on a copy of the bindings, because writing them to `env` would stay set for every later test. */
export const toStore = (url: string, init?: RequestInit, vars: Vars = {}): Promise<Response> =>
  worker.fetch(new Request(url, init), {
    ...env,
    ...vars,
  } as Env);

/** 64 characters standing for a hash a machine made. */
export const hash = (letter: string): string => letter.repeat(64);

export const holder = (id: string, machine = "studio"): Holder => ({
  kind: "job",
  id,
  machine,
  since: 1760449990,
});

/** A claim's holder: a holder with the person's name and the Job's title. */
export const claimant = (
  id: string,
  machine = "studio",
  name = "Nick",
  title = "Fix the flaky queue test",
): ClaimHolder => ({
  ...holder(id, machine),
  name,
  title,
});

export const fact = (change: Partial<Fact> = {}): Fact => ({
  workspace: ".",
  kind: "tests",
  command: "cargo test",
  succeeded: true,
  last_ran: 1760449990000,
  source: "jobs",
  ...change,
});

export type Device = {
  id: string;
  key: CryptoKey;
  account: number;
  token: string;
  /** The vars of the store it was enrolled at, which its requests go back to. */
  vars?: Vars;
};

/** Answers the forge's user endpoint for `token` with `account`, until restored. */
export function forgeAnswers(account: number, status = 200) {
  return vi.spyOn(globalThis, "fetch").mockImplementation(async () =>
    status === 200
      ? Response.json({
          id: account,
          login: "someone",
          name: "A Name",
        })
      : Response.json(
          {
            message: "Bad credentials",
          },
          {
            status,
          },
        ),
  );
}

/** What a stand-in forge answers to the membership call of a team store. */
export type Membership = {
  status: number;
  state?: string;
};

/**
 * Answers the forge's user endpoint with `account` and `login`, and its membership endpoints with `membership`,
 * until restored. Every call lands in the mock, so a test can read the URL a membership was asked at.
 */
export function teamForgeAnswers(account: number, login: string, membership: Membership) {
  return vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
    const url = new URL(String(input instanceof Request ? input.url : input));
    if (url.pathname === "/user")
      return Response.json({
        id: account,
        login,
        name: "A Name",
      });
    return membership.status === 200
      ? Response.json({
          state: membership.state,
          role: "member",
        })
      : Response.json(
          {
            message: "Nope",
          },
          {
            status: membership.status,
          },
        );
  });
}

export async function newKeyPair() {
  const pair = (await crypto.subtle.generateKey(
    {
      name: "ECDSA",
      namedCurve: "P-256",
    },
    true,
    [
      "sign",
      "verify",
    ],
  )) as CryptoKeyPair;
  const spki = (await crypto.subtle.exportKey("spki", pair.publicKey)) as ArrayBuffer;
  return {
    key: pair.privateKey,
    spki: toBase64(spki),
  };
}

/** Enrols a fresh machine under `account`, through the Worker, against a stand-in forge. */
export async function enrolDevice(account: number, machine = "studio"): Promise<Device> {
  const forge = forgeAnswers(account);
  try {
    return await enrolAt(account, machine);
  } finally {
    forge.mockRestore();
  }
}

/** Enrols a fresh machine at a team store, as `login`, against a forge that says it is a member. */
export async function enrolMember(
  vars: Vars,
  account: number,
  login: string,
  machine = "studio",
): Promise<Device> {
  const forge = teamForgeAnswers(account, login, {
    status: 200,
    state: "active",
  });
  try {
    return await enrolAt(account, machine, vars);
  } finally {
    forge.mockRestore();
  }
}

/** An address no other test enrols from, so the per-address limit never meets a test that is not about it. */
export const randomAddress = (): string =>
  `10.${Math.floor(Math.random() * 256)}.${Math.floor(Math.random() * 256)}.${Math.floor(Math.random() * 256)}`;

/** The enrolment itself, against whatever the forge is already answering. */
async function enrolAt(account: number, machine: string, vars: Vars = {}): Promise<Device> {
  const { key, spki } = await newKeyPair();
  const token = `device-flow-token-${crypto.randomUUID()}`;
  const answer = await toStore(
    `${STORE}/v1/enrol`,
    {
      method: "POST",
      headers: {
        "CF-Connecting-IP": randomAddress(),
      },
      body: JSON.stringify({
        token,
        public_key: spki,
        machine,
      }),
    },
    vars,
  );
  const { device } = (await answer.json()) as {
    device: string;
  };
  return {
    id: device,
    key,
    account,
    token,
    vars,
  };
}

/** A signed request from `device`. `sign` overrides what the signature is made over. */
export async function call(
  device: Device,
  method: string,
  target: string,
  body?: unknown,
  sign: {
    key?: CryptoKey;
    time?: string;
    nonce?: string;
    signedBody?: string;
    headers?: Partial<SignedHeaders>;
  } = {},
): Promise<Response> {
  const text = body === undefined ? "" : JSON.stringify(body);
  const headers = await signRequest(
    sign.key ?? device.key,
    device.id,
    method,
    target,
    sign.signedBody ?? text,
    sign.time,
    sign.nonce,
  );
  return toStore(
    `${STORE}${target}`,
    {
      method,
      headers: {
        ...headers,
        ...sign.headers,
      },
      body: text === "" ? undefined : text,
    },
    device.vars,
  );
}

export async function claim(
  device: Device,
  repo: string,
  kind: ClaimKind,
  key: string,
  who: ClaimHolder,
) {
  return call(device, "POST", "/v1/claims", {
    repo,
    kind,
    key,
    holder: who,
  });
}

/** A red main claim: the kind Fleet's client has always made. */
export const claimRed = (device: Device, repo: string, red: string, who: ClaimHolder) =>
  claim(device, repo, "red", red, who);

export const claimPath = (repo: string, kind: ClaimKind, key: string) =>
  `/v1/claims/${repo}/${kind}/${key}`;

export async function renew(device: Device, who: Pick<Holder, "kind" | "id" | "machine">) {
  return call(device, "POST", "/v1/renewals", {
    holder: {
      kind: who.kind,
      id: who.id,
      machine: who.machine,
    },
  });
}

/** The Durable Object for `account`, as the Worker would reach it. */
export const accountObject = (account: number) =>
  env.ACCOUNT.get(env.ACCOUNT.idFromName(String(account)));

/** The Durable Object of the team store named `name`, which is `TEAM` in lowercase behind `team:`. */
export const teamObject = (name: string) => env.ACCOUNT.get(env.ACCOUNT.idFromName(`team:${name}`));

/** Everything a Durable Object holds, as text: `[]` when it holds no table at all. */
export const everythingStoredFor = (account: number): Promise<string> =>
  everythingStoredIn(accountObject(account));

export async function everythingStoredIn(
  object: ReturnType<typeof accountObject>,
): Promise<string> {
  return runInDurableObject(object, (_instance, state) => {
    const sql = state.storage.sql;
    const tables = sql
      .exec<{
        name: string;
      }>(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE '\\_cf\\_%' ESCAPE '\\'",
      )
      .toArray();
    return JSON.stringify(
      tables.map(({ name }) => ({
        name,
        rows: sql.exec(`SELECT * FROM ${name}`).toArray(),
      })),
    );
  });
}

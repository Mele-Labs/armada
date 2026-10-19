import { runInDurableObject } from "cloudflare:test";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MAX_CLAIMS, MAX_DEVICES, MAX_OWNERS, MAX_RUNS, MAX_RUNS_BODY_BYTES, REQUESTS_PER_MINUTE } from "../src/rows";
import { newNonce, signRequest } from "../src/signing";
import { STORE, accountObject, call, claim, enrolDevice, everythingStoredFor, fact, forgeAnswers, hash, holder, newKeyPair, toStore } from "./support";

afterEach(() => vi.restoreAllMocks());

const enrolFrom = (ip: string, spki: string) =>
  toStore(`${STORE}/v1/enrol`, {
    method: "POST",
    headers: { "CF-Connecting-IP": ip },
    body: JSON.stringify({ token: "any-token", public_key: spki, machine: "studio" }),
  });

/**
 * Enrols from `ip` until the first refusal. The limiter's minute is the wall clock's, so a run that
 * crosses one starts the count again: enough tries for two whole minutes' worth always reach a 429.
 */
async function untilRefused(ip: string, spki: string): Promise<Response[]> {
  const answers: Response[] = [];
  for (let n = 0; n < 25; n++) {
    const answer = await enrolFrom(ip, spki);
    answers.push(answer);
    if (answer.status === 429) break;
  }
  return answers;
}

describe("enrolling too often", () => {
  it("is refused with 429 once an address has used its minute, and the forge is not asked", async () => {
    const { spki } = await newKeyPair();
    const forge = forgeAnswers(8001);

    const statuses = (await untilRefused("203.0.113.7", spki)).map((answer) => answer.status);

    expect(statuses.at(-1)).toBe(429);
    expect(statuses.slice(0, -1).every((status) => status === 200)).toBe(true);
    expect(statuses.length).toBeGreaterThan(1);
    expect(forge).toHaveBeenCalledTimes(statuses.length - 1);
  });

  it("does not hold one address's minute against another", async () => {
    const { spki } = await newKeyPair();
    forgeAnswers(8002);
    for (let n = 0; n < 15; n++) await enrolFrom("203.0.113.8", spki);
    expect((await enrolFrom("203.0.113.9", spki)).status).toBe(200);
  });

  it("names the refusal", async () => {
    const { spki } = await newKeyPair();
    forgeAnswers(8003);
    const last = (await untilRefused("203.0.113.10", spki)).at(-1)!;
    expect(last.status).toBe(429);
    expect(await last.json()).toEqual({ error: "enrol_rate_limited" });
  });
});

describe("a request for an account or device that never enrolled", () => {
  it("is refused 401 and leaves the account with no table at all", async () => {
    const { key } = await newKeyPair();
    const stranger = { id: `9001.${"a".repeat(32)}`, key, account: 9001, token: "" };

    const answer = await call(stranger, "POST", "/v1/claims", { repo: hash("a"), red: hash("b"), holder: holder("j-1") });

    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "unknown_device" });
    expect(await everythingStoredFor(9001)).toBe("[]");
  });

  it("stores nothing for a request that is only half signed", async () => {
    const answer = await toStore(`${STORE}/v1/claims/${hash("a")}/${hash("b")}`, {
      headers: { "X-Armada-Device": `9002.${"b".repeat(32)}` },
    });
    expect(answer.status).toBe(401);
    expect(await everythingStoredFor(9002)).toBe("[]");
  });

  it("stores nothing for an oversize body", async () => {
    const { key } = await newKeyPair();
    const stranger = { id: `9003.${"c".repeat(32)}`, key, account: 9003, token: "" };
    const answer = await call(stranger, "POST", "/v1/claims", { padding: "x".repeat(5000) });
    expect(answer.status).toBe(413);
    expect(await everythingStoredFor(9003)).toBe("[]");
  });

  it("takes no nonce for an unknown device of an account that has others", async () => {
    const real = await enrolDevice(9004);
    const { key } = await newKeyPair();
    const stranger = { ...real, id: `9004.${"d".repeat(32)}`, key };
    const before = await everythingStoredFor(9004);

    const answer = await call(stranger, "GET", `/v1/claims/${hash("a")}/${hash("b")}`);

    expect(answer.status).toBe(401);
    expect(await everythingStoredFor(9004)).toBe(before);
    expect(before).not.toContain(stranger.id);
  });

  it("is still the same Durable Object once the account does enrol", async () => {
    await everythingStoredFor(9005);
    const device = await enrolDevice(9005);
    expect((await claim(device, hash("a"), hash("b"), holder("j-1"))).status).toBe(200);
  });
});

describe("devices", () => {
  it(`stop at ${MAX_DEVICES} per account with 413 devices_full, and another account is unaffected`, async () => {
    const forge = forgeAnswers(9101);
    const { spki } = await newKeyPair();
    for (let n = 0; n < MAX_DEVICES; n++) {
      const answer = await enrolFrom(`198.51.100.${n + 1}`, spki);
      expect(answer.status).toBe(200);
    }

    const over = await enrolFrom("198.51.100.200", spki);
    expect(over.status).toBe(413);
    expect(await over.json()).toEqual({ error: "devices_full" });
    const stored = JSON.parse(await everythingStoredFor(9101)) as { name: string; rows: unknown[] }[];
    expect(stored.find((table) => table.name === "devices")!.rows).toHaveLength(MAX_DEVICES);

    forge.mockRestore();
    expect((await enrolDevice(9102)).id.startsWith("9102.")).toBe(true);
  });

  it("make room again once one signs out", async () => {
    const devices = [];
    for (let n = 0; n < MAX_DEVICES; n++) devices.push(await enrolDevice(9103));
    forgeAnswers(9103);
    const { spki } = await newKeyPair();
    expect((await enrolFrom("198.51.100.201", spki)).status).toBe(413);

    expect((await call(devices[1]!, "DELETE", `/v1/devices/${devices[0]!.id}`)).status).toBe(204);

    expect((await enrolFrom("198.51.100.202", spki)).status).toBe(200);
  });
});

const rowKey = (n: number) => `k${String(n).padStart(8, "0")}`.padEnd(40, "0");

/** Puts `count` rows into a table directly, as a long-lived account would have. */
async function fill(account: number, table: "claims" | "owners", count: number) {
  await runInDurableObject(accountObject(account), (_instance, state) => {
    const sql = state.storage.sql;
    for (let n = 0; n < count; n++) {
      const key = rowKey(n);
      if (table === "claims") sql.exec("INSERT INTO claims (repo, red, kind, id, machine, since) VALUES (?, ?, 'job', 'j-0', 'studio', 1)", key, key);
      else sql.exec("INSERT INTO owners (key, kind, id, machine, since) VALUES (?, 'job', 'j-0', 'studio', 1)", key);
    }
  });
}

describe("claims", () => {
  it(`stop at ${MAX_CLAIMS} per account with 413 claims_full`, async () => {
    const device = await enrolDevice(9201);
    await fill(9201, "claims", MAX_CLAIMS);

    const over = await claim(device, hash("a"), hash("b"), holder("j-1"));

    expect(over.status).toBe(413);
    expect(await over.json()).toEqual({ error: "claims_full" });
    expect((await call(device, "GET", `/v1/claims/${hash("a")}/${hash("b")}`)).status).toBe(404);
  });

  it("still answer 409 with the holder when the claim already exists and the table is full", async () => {
    const device = await enrolDevice(9202);
    expect((await claim(device, hash("a"), hash("b"), holder("j-1"))).status).toBe(200);
    await fill(9202, "claims", MAX_CLAIMS - 1);

    const again = await claim(device, hash("a"), hash("b"), holder("j-2"));
    expect(again.status).toBe(409);
    expect(await again.json()).toEqual({ holder: holder("j-1") });
  });

  it("take a new claim again once one is given back", async () => {
    const device = await enrolDevice(9203);
    await fill(9203, "claims", MAX_CLAIMS);
    expect((await claim(device, hash("a"), hash("b"), holder("j-1"))).status).toBe(413);
    const full = rowKey(0);
    await call(device, "DELETE", `/v1/claims/${full}/${full}`);
    expect((await claim(device, hash("a"), hash("b"), holder("j-1"))).status).toBe(200);
  });
});

describe("owners", () => {
  it(`stop at ${MAX_OWNERS} per account with 413 owners_full, and an existing owner still updates`, async () => {
    const device = await enrolDevice(9301);
    await fill(9301, "owners", MAX_OWNERS);

    const over = await call(device, "PUT", `/v1/owners/${hash("a")}`, { holder: holder("j-1") });
    expect(over.status).toBe(413);
    expect(await over.json()).toEqual({ error: "owners_full" });

    const present = rowKey(0);
    expect((await call(device, "PUT", `/v1/owners/${present}`, { holder: holder("j-2") })).status).toBe(204);
  });
});

describe("runs", () => {
  const path = `/v1/runs/${hash("a")}`;

  it("stop at the body limit with 413 too_large, and every other route keeps 4096", async () => {
    const device = await enrolDevice(9501);
    const roomy = Array.from({ length: 150 }, (_, n) => fact({ command: `${n} `.padEnd(400, "c"), workspace: "w".repeat(200) }));
    expect(JSON.stringify({ facts: roomy }).length).toBeLessThan(MAX_RUNS_BODY_BYTES);
    expect((await call(device, "PUT", path, { facts: roomy })).status).toBe(204);

    const over = await call(device, "PUT", path, { facts: [fact()], padding: "x".repeat(MAX_RUNS_BODY_BYTES) });
    expect(over.status).toBe(413);
    expect(await over.json()).toEqual({ error: "too_large" });
    const other = await call(device, "POST", "/v1/claims", { padding: "x".repeat(5000) });
    expect(other.status).toBe(413);
  });

  it(`stop at ${MAX_RUNS} repositories per account with 413 runs_full, and an existing one still updates`, async () => {
    const device = await enrolDevice(9502);
    await runInDurableObject(accountObject(9502), (_instance, state) => {
      for (let n = 0; n < MAX_RUNS; n++) state.storage.sql.exec("INSERT INTO runs (repo, facts) VALUES (?, ?)", rowKey(n), JSON.stringify([fact()]));
    });

    const over = await call(device, "PUT", path, { facts: [fact()] });
    expect(over.status).toBe(413);
    expect(await over.json()).toEqual({ error: "runs_full" });
    expect((await call(device, "GET", path)).status).toBe(404);

    const present = `/v1/runs/${rowKey(0)}`;
    expect((await call(device, "PUT", present, { facts: [fact({ command: "make" })] })).status).toBe(204);
  });

  it("make room again once one repository is emptied", async () => {
    const device = await enrolDevice(9503);
    await runInDurableObject(accountObject(9503), (_instance, state) => {
      for (let n = 0; n < MAX_RUNS; n++) state.storage.sql.exec("INSERT INTO runs (repo, facts) VALUES (?, ?)", rowKey(n), JSON.stringify([fact()]));
    });
    expect((await call(device, "PUT", path, { facts: [fact()] })).status).toBe(413);
    expect((await call(device, "PUT", `/v1/runs/${rowKey(0)}`, { facts: [] })).status).toBe(204);
    expect((await call(device, "PUT", path, { facts: [fact()] })).status).toBe(204);
  });

  it("are served by an account enrolled before the table existed, from its first verified request", async () => {
    const device = await enrolDevice(9504);
    await runInDurableObject(accountObject(9504), (instance, state) => {
      state.storage.sql.exec("DROP TABLE runs");
      (instance as unknown as { runsMade: boolean }).runsMade = false;
    });
    expect((await call(device, "GET", path)).status).toBe(404);
    expect((await call(device, "PUT", path, { facts: [fact()] })).status).toBe(204);
    expect(await (await call(device, "GET", path)).json()).toEqual({ facts: [fact()] });
  });

  it("make no table for a stranger", async () => {
    const { key } = await newKeyPair();
    const stranger = { id: `9505.${"f".repeat(32)}`, key, account: 9505, token: "" };
    expect((await call(stranger, "PUT", path, { facts: [fact()] })).status).toBe(401);
    expect(await everythingStoredFor(9505)).toBe("[]");
  });
});

describe("a device's requests", () => {
  it(`are refused with 429 once ${REQUESTS_PER_MINUTE} verified ones fall in a minute`, async () => {
    const device = await enrolDevice(9401);
    const path = `/v1/claims/${hash("a")}/${hash("b")}`;

    for (let n = 0; n < REQUESTS_PER_MINUTE; n++) {
      expect((await call(device, "GET", path)).status).toBe(404);
    }
    const over = await call(device, "GET", path);

    expect(over.status).toBe(429);
    expect(await over.json()).toEqual({ error: "rate_limited" });
    expect(over.headers.get("Retry-After")).toBe("60");
  });

  it("do not spend another device's minute", async () => {
    const [busy, quiet] = [await enrolDevice(9402), await enrolDevice(9402)];
    const path = `/v1/claims/${hash("a")}/${hash("b")}`;
    for (let n = 0; n < REQUESTS_PER_MINUTE; n++) await call(busy, "GET", path);
    expect((await call(busy, "GET", path)).status).toBe(429);
    expect((await call(quiet, "GET", path)).status).toBe(404);
  });

  it("are not counted when they fail to verify, so a stranger cannot use up a device's minute", async () => {
    const device = await enrolDevice(9403);
    const { key } = await newKeyPair();
    const path = `/v1/claims/${hash("a")}/${hash("b")}`;
    for (let n = 0; n < REQUESTS_PER_MINUTE + 5; n++) {
      const headers = await signRequest(key, device.id, "GET", path, "", undefined, newNonce());
      const answer = await toStore(`${STORE}${path}`, { headers });
      expect(answer.status).toBe(401);
    }
    expect((await call(device, "GET", path)).status).toBe(404);
  });

  it("start again once the minute has passed", async () => {
    const device = await enrolDevice(9404);
    const path = `/v1/claims/${hash("a")}/${hash("b")}`;
    vi.useFakeTimers({ toFake: ["Date"] });
    try {
      for (let n = 0; n < REQUESTS_PER_MINUTE; n++) await call(device, "GET", path);
      expect((await call(device, "GET", path)).status).toBe(429);
      vi.setSystemTime(Date.now() + 61_000);
      expect((await call(device, "GET", path)).status).toBe(404);
    } finally {
      vi.useRealTimers();
    }
  });
});

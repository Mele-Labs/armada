import { afterEach, describe, expect, it, vi } from "vitest";
import { MAX_DEVICES, MAX_TEAM_DEVICES } from "../src/rows";
import {
  STORE,
  call,
  toStore,
  claimRed,
  claimant,
  enrolDevice,
  enrolMember,
  everythingStoredFor,
  everythingStoredIn,
  forgeAnswers,
  hash,
  newKeyPair,
  randomAddress,
  teamForgeAnswers,
  teamObject,
  type Vars,
} from "./support";

afterEach(() => vi.restoreAllMocks());

const enrol = (body: unknown) =>
  toStore(`${STORE}/v1/enrol`, {
    method: "POST",
    body: JSON.stringify(body),
  });

describe("enrolling a device", () => {
  it("files it under the forge's numeric id and keeps no token", async () => {
    const token = "device-flow-token-for-this-test";
    const { spki } = await newKeyPair();
    const logs = (
      [
        "log",
        "info",
        "warn",
        "error",
        "debug",
      ] as const
    ).map((level) => vi.spyOn(console, level));
    const forge = forgeAnswers(424242);

    const answer = await enrol({
      token,
      public_key: spki,
      machine: "studio",
    });
    const text = await answer.text();

    expect(answer.status).toBe(200);
    const { device } = JSON.parse(text) as {
      device: string;
    };
    expect(device).toMatch(/^424242\.[0-9a-f]{32}$/);
    expect(text).not.toContain(token);

    expect(forge).toHaveBeenCalledTimes(1);
    const [url, init] = forge.mock.calls[0]!;
    expect(String(url)).toBe("https://forge.test/user");
    expect((init!.headers as Record<string, string>).Authorization).toBe(`Bearer ${token}`);

    const stored = await everythingStoredFor(424242);
    expect(stored).toContain(device);
    expect(stored).toContain(spki);
    expect(stored).not.toContain(token);
    expect(stored).not.toContain("someone");
    expect(stored).not.toContain("A Name");

    for (const log of logs) expect(JSON.stringify(log.mock.calls)).not.toContain(token);
  });

  it("puts two machines of one account in one store, and another account's apart", async () => {
    const [a, b, other] = [
      await enrolDevice(7001),
      await enrolDevice(7001),
      await enrolDevice(7002),
    ];
    expect(a.id.startsWith("7001.")).toBe(true);
    expect(other.id.startsWith("7002.")).toBe(true);

    await claimRed(a, hash("a"), hash("b"), claimant("j-1"));
    expect((await call(b, "GET", `/v1/claims/${hash("a")}/red/${hash("b")}`)).status).toBe(200);
    expect((await call(other, "GET", `/v1/claims/${hash("a")}/red/${hash("b")}`)).status).toBe(404);
  });

  it("registers nothing when the forge refuses the token", async () => {
    const { spki } = await newKeyPair();
    forgeAnswers(7100, 401);

    const answer = await enrol({
      token: "expired-token",
      public_key: spki,
      machine: "studio",
    });

    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "forge_refused",
    });
    expect(await everythingStoredFor(7100)).not.toContain(spki);
  });

  it("says so when the forge cannot be reached, and registers nothing", async () => {
    const { spki } = await newKeyPair();
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("offline"));

    const answer = await enrol({
      token: "any-token",
      public_key: spki,
      machine: "studio",
    });

    expect(answer.status).toBe(502);
    expect(await answer.json()).toEqual({
      error: "forge_unreachable",
    });
  });

  it("refuses a key that is not a P-256 public key before it asks the forge", async () => {
    const forge = forgeAnswers(7200);
    const answer = await enrol({
      token: "any-token",
      public_key: "bm90IGEga2V5",
      machine: "studio",
    });
    expect(answer.status).toBe(400);
    expect(forge).not.toHaveBeenCalled();
  });

  it("refuses a body with a field it does not know", async () => {
    const { spki } = await newKeyPair();
    const forge = forgeAnswers(7300);
    const answer = await enrol({
      token: "any-token",
      public_key: spki,
      machine: "studio",
      login: "someone",
    });
    expect(answer.status).toBe(400);
    expect(forge).not.toHaveBeenCalled();
  });

  it("refuses an id that is not a positive number", async () => {
    const { spki } = await newKeyPair();
    for (const id of [
      "12",
      0,
      -4,
      1.5,
    ]) {
      vi.spyOn(globalThis, "fetch").mockImplementation(async () =>
        Response.json({
          id,
        }),
      );
      const answer = await enrol({
        token: "any-token",
        public_key: spki,
        machine: "studio",
      });
      expect(answer.status).toBe(502);
      vi.restoreAllMocks();
    }
  });
});

const enrolAs = async (vars: Vars, token = "any-token") => {
  const { spki } = await newKeyPair();
  return toStore(
    `${STORE}/v1/enrol`,
    {
      method: "POST",
      headers: {
        "CF-Connecting-IP": randomAddress(),
      },
      body: JSON.stringify({
        token,
        public_key: spki,
        machine: "studio",
      }),
    },
    vars,
  );
};

// Each test has a team of its own, because the object of one outlives the test that filled it.
const team = (TEAM: string): Vars => ({
  TEAM,
});

describe("a team store", () => {
  it("enrols a member of the org, asking the forge's membership endpoint, and keeps no token", async () => {
    const token = "team-device-flow-token";
    const forge = teamForgeAnswers(8001, "someone", {
      status: 200,
      state: "active",
    });
    const logs = (
      [
        "log",
        "info",
        "warn",
        "error",
        "debug",
      ] as const
    ).map((level) => vi.spyOn(console, level));

    const answer = await enrolAs(team("Acme"), token);
    const text = await answer.text();

    expect(answer.status).toBe(200);
    expect(JSON.parse(text).device).toMatch(/^8001\.[0-9a-f]{32}$/);
    expect(forge.mock.calls.map(([url]) => String(url))).toEqual([
      "https://forge.test/user",
      "https://forge.test/user/memberships/orgs/Acme",
    ]);
    for (const [, init] of forge.mock.calls)
      expect((init!.headers as Record<string, string>).Authorization).toBe(`Bearer ${token}`);

    const stored = await everythingStoredIn(teamObject("acme"));
    expect(stored).toContain("8001.");
    expect(stored).not.toContain(token);
    expect(await everythingStoredFor(8001)).toBe("[]");
    for (const log of logs) expect(JSON.stringify(log.mock.calls)).not.toContain(token);
  });

  it("refuses a user who is not in the org with 403 not_a_member, and registers nothing", async () => {
    const { spki } = await newKeyPair();
    for (const membership of [
      {
        status: 404,
      },
      {
        status: 200,
        state: "pending",
      },
    ]) {
      teamForgeAnswers(8101, "outsider", membership);
      const answer = await toStore(
        `${STORE}/v1/enrol`,
        {
          method: "POST",
          headers: {
            "CF-Connecting-IP": randomAddress(),
          },
          body: JSON.stringify({
            token: "any-token",
            public_key: spki,
            machine: "studio",
          }),
        },
        team("Outsiders"),
      );
      expect(answer.status).toBe(403);
      expect(await answer.json()).toEqual({
        error: "not_a_member",
      });
      vi.restoreAllMocks();
    }
    expect(await everythingStoredIn(teamObject("outsiders"))).not.toContain(spki);
    expect(await everythingStoredFor(8101)).toBe("[]");
  });

  it("asks one team's membership by the login, and refuses a member who is still pending", async () => {
    const forge = teamForgeAnswers(8201, "Some-One", {
      status: 200,
      state: "active",
    });
    const answer = await enrolAs(team("Acme/Platform_Crew-2"));
    expect(answer.status).toBe(200);
    expect(forge.mock.calls.map(([url]) => String(url))).toEqual([
      "https://forge.test/user",
      "https://forge.test/orgs/Acme/teams/Platform_Crew-2/memberships/Some-One",
    ]);
    vi.restoreAllMocks();

    teamForgeAnswers(8202, "pending-one", {
      status: 200,
      state: "pending",
    });
    const refused = await enrolAs(team("Acme/Platform_Crew-2"));
    expect(refused.status).toBe(403);
    expect(await refused.json()).toEqual({
      error: "not_a_member",
    });
    expect(await everythingStoredIn(teamObject("acme/platform_crew-2"))).not.toContain("8202.");
  });

  it("answers 403 org_restricted when the org's app restrictions block the membership read", async () => {
    teamForgeAnswers(8301, "someone", {
      status: 403,
    });
    const answer = await enrolAs(team("Restricted"));
    expect(answer.status).toBe(403);
    expect(await answer.json()).toEqual({
      error: "org_restricted",
    });
    expect(await everythingStoredIn(teamObject("restricted"))).toBe("[]");
  });

  it("maps the membership call's 401 to forge_refused, and an unreachable or failing forge to forge_unreachable", async () => {
    teamForgeAnswers(8401, "someone", {
      status: 401,
    });
    const refused = await enrolAs(team("Mapping"));
    expect(refused.status).toBe(401);
    expect(await refused.json()).toEqual({
      error: "forge_refused",
    });
    vi.restoreAllMocks();

    teamForgeAnswers(8402, "someone", {
      status: 503,
    });
    const failing = await enrolAs(team("Mapping"));
    expect(failing.status).toBe(502);
    expect(await failing.json()).toEqual({
      error: "forge_unreachable",
    });
    vi.restoreAllMocks();

    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      if (String(input).endsWith("/user"))
        return Response.json({
          id: 8403,
          login: "someone",
        });
      throw new Error("offline");
    });
    const offline = await enrolAs(team("Mapping"));
    expect(offline.status).toBe(502);
    expect(await offline.json()).toEqual({
      error: "forge_unreachable",
    });
  });

  it("puts two members with different account ids in one store, so a claim of one is held against the other", async () => {
    const a = await enrolMember(team("Pair"), 8501, "first");
    const b = await enrolMember(team("Pair"), 8502, "second");
    expect(a.id.startsWith("8501.")).toBe(true);
    expect(b.id.startsWith("8502.")).toBe(true);

    expect((await claimRed(a, hash("a"), hash("b"), claimant("j-1"))).status).toBe(200);
    expect((await claimRed(b, hash("a"), hash("b"), claimant("j-2", "laptop", "Sam"))).status).toBe(
      409,
    );
    expect((await call(b, "GET", `/v1/claims/${hash("a")}/red/${hash("b")}`)).status).toBe(200);
    expect(await everythingStoredFor(8501)).toBe("[]");
    expect(await everythingStoredFor(8502)).toBe("[]");
  });

  it("keeps a team's object apart from another team's and from a personal store's", async () => {
    const acme = await enrolMember(team("Apart"), 8601, "first");
    const platform = await enrolMember(team("Apart/Platform_Crew-2"), 8601, "first");
    await claimRed(acme, hash("a"), hash("b"), claimant("j-1"));
    expect((await call(platform, "GET", `/v1/claims/${hash("a")}/red/${hash("b")}`)).status).toBe(
      404,
    );
    expect(await everythingStoredIn(teamObject("apart/platform_crew-2"))).not.toContain("j-1");
  });

  it(`stops at ${MAX_TEAM_DEVICES} devices with 413 devices_full, where a personal store stops at ${MAX_DEVICES}`, async () => {
    expect(MAX_DEVICES).toBe(16);
    expect(MAX_TEAM_DEVICES).toBe(256);
    const { spki } = await newKeyPair();
    const enrolFrom = (n: number, vars: Vars) =>
      toStore(
        `${STORE}/v1/enrol`,
        {
          method: "POST",
          headers: {
            "CF-Connecting-IP": `10.1.${n >> 8}.${n & 255}`,
          },
          body: JSON.stringify({
            token: "any-token",
            public_key: spki,
            machine: "studio",
          }),
        },
        vars,
      );

    forgeAnswers(8701);
    for (let n = 0; n < MAX_DEVICES; n++)
      expect(
        (
          await enrolFrom(n, {
            TEAM: "",
          })
        ).status,
      ).toBe(200);
    const personal = await enrolFrom(MAX_DEVICES, {
      TEAM: "",
    });
    expect(personal.status).toBe(413);
    expect(await personal.json()).toEqual({
      error: "devices_full",
    });
    vi.restoreAllMocks();

    teamForgeAnswers(8702, "someone", {
      status: 200,
      state: "active",
    });
    for (let n = 0; n < MAX_TEAM_DEVICES; n++)
      expect((await enrolFrom(1000 + n, team("Limit"))).status).toBe(200);
    const over = await enrolFrom(2000, team("Limit"));
    expect(over.status).toBe(413);
    expect(await over.json()).toEqual({
      error: "devices_full",
    });
  });

  it("refuses every request with 500 bad_team_config when TEAM is not a GitHub org or org/team", async () => {
    const forge = forgeAnswers(8801);
    for (const config of [
      " ",
      "acme/",
      "/platform",
      "acme/platform/extra",
      "ac me",
      "acme.io",
      "a".repeat(40),
      "acme/plat.form",
      "acme/plat form",
    ]) {
      for (const path of [
        "/v1/enrol",
        "/v1/claims",
        "/v1/elsewhere",
      ]) {
        const answer = await toStore(
          `${STORE}${path}`,
          {
            method: "POST",
            body: "{}",
          },
          {
            TEAM: config,
          },
        );
        expect(answer.status, `${config} at ${path}`).toBe(500);
        expect(await answer.json()).toEqual({
          error: "bad_team_config",
        });
      }
    }
    expect(forge).not.toHaveBeenCalled();
  });
});

describe("listing the devices", () => {
  it("answers a personal store's two devices, oldest first, with no key", async () => {
    const first = await enrolDevice(8901, "studio");
    const second = await enrolDevice(8901, "laptop");
    await enrolDevice(8902, "someone else's");

    const answer = await call(second, "GET", "/v1/devices");

    expect(answer.status).toBe(200);
    expect(await answer.json()).toEqual({
      devices: [
        {
          device: first.id,
          machine: "studio",
        },
        {
          device: second.id,
          machine: "laptop",
        },
      ],
    });
    expect(JSON.stringify(await (await call(first, "GET", "/v1/devices")).json())).not.toContain(
      "public_key",
    );
  });

  it("answers both members' Macs of a team store", async () => {
    const vars = team("Listing");
    const a = await enrolMember(vars, 8911, "first", "studio");
    const b = await enrolMember(vars, 8912, "second", "laptop");

    const answer = await call(b, "GET", "/v1/devices");

    expect(answer.status).toBe(200);
    expect(await answer.json()).toEqual({
      devices: [
        {
          device: a.id,
          machine: "studio",
        },
        {
          device: b.id,
          machine: "laptop",
        },
      ],
    });
  });

  it("drops a device that signed out, and answers 405 bad_method to any other method", async () => {
    const a = await enrolDevice(8921, "studio");
    const b = await enrolDevice(8921, "laptop");
    expect((await call(b, "DELETE", `/v1/devices/${a.id}`)).status).toBe(204);
    expect(await (await call(b, "GET", "/v1/devices")).json()).toEqual({
      devices: [
        {
          device: b.id,
          machine: "laptop",
        },
      ],
    });

    for (const method of [
      "POST",
      "PUT",
      "DELETE",
    ]) {
      const answer = await call(b, method, "/v1/devices");
      expect(answer.status, method).toBe(405);
      expect(await answer.json()).toEqual({
        error: "bad_method",
      });
    }
  });

  it("answers 401 to an unsigned call", async () => {
    const answer = await toStore(`${STORE}/v1/devices`);
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "unsigned",
    });
  });
});

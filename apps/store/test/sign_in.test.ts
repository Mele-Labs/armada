import { afterEach, describe, expect, it, vi } from "vitest";
import {
  STORE,
  call,
  enrolDevice,
  everythingStoredFor,
  everythingStoredIn,
  newKeyPair,
  randomAddress,
  teamObject,
  toStore,
  type Vars,
} from "./support";

afterEach(() => vi.restoreAllMocks());

const CLIENT = "Iv1.abcdef0123456789";
const CODE = {
  device_code: "3584d83530557fdd1f46af8289938c8ef79f9dc5",
  user_code: "WDJB-MJHT",
  verification_uri: "https://forge.test/login/device",
  expires_in: 900,
  interval: 5,
};

/** What the stand-in forge answers: the login endpoints with `login`, the user and membership endpoints as a team store asks them. */
type Forge = {
  code?: () => Response;
  token?: () => Response;
  user?: {
    id: number;
    login: string;
  };
  membership?: {
    status: number;
    state?: string;
  };
  unreachable?: "code" | "token";
};

function forge(answers: Forge) {
  return vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
    const url = new URL(String(input instanceof Request ? input.url : input));
    if (url.pathname === "/login/device/code") {
      if (answers.unreachable === "code") throw new TypeError("down");
      return answers.code?.() ?? Response.json(CODE);
    }
    if (url.pathname === "/login/oauth/access_token") {
      if (answers.unreachable === "token") throw new TypeError("down");
      return answers.token!();
    }
    if (url.pathname === "/user")
      return Response.json({
        ...answers.user,
        name: "A Name",
      });
    const membership = answers.membership ?? {
      status: 200,
      state: "active",
    };
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

const post = (path: string, body: unknown, vars: Vars = {}) =>
  toStore(
    `${STORE}${path}`,
    {
      method: "POST",
      headers: {
        "CF-Connecting-IP": randomAddress(),
      },
      body: JSON.stringify(body),
    },
    vars,
  );

const poll = async (extra: Record<string, unknown> = {}, vars: Vars = {}) => {
  const { spki } = await newKeyPair();
  return post(
    "/v1/sign_in/poll",
    {
      client_id: CLIENT,
      device_code: CODE.device_code,
      public_key: spki,
      machine: "iPhone",
      ...extra,
    },
    vars,
  );
};

const forgeSays =
  (body: unknown, status = 200) =>
  () =>
    Response.json(body, {
      status,
    });

describe("starting a phone's sign-in", () => {
  it("posts the client id to the forge's device-code endpoint and answers its code", async () => {
    const spy = forge({});

    const answer = await post("/v1/sign_in", {
      client_id: CLIENT,
    });

    expect(answer.status).toBe(200);
    expect(await answer.json()).toEqual(CODE);
    expect(spy).toHaveBeenCalledTimes(1);
    const [url, init] = spy.mock.calls[0]!;
    expect(String(url)).toBe("https://forge.test/login/device/code");
    expect(init?.method).toBe("POST");
    expect((init!.headers as Record<string, string>).Accept).toBe("application/json");
    expect(String(init!.body)).toBe(`client_id=${encodeURIComponent(CLIENT)}`);
  });

  it("answers the forge's refusal as 400 forge_refused with its reason", async () => {
    forge({
      code: forgeSays(
        {
          error: "device_flow_disabled",
          error_description: "off",
        },
        200,
      ),
    });
    const answer = await post("/v1/sign_in", {
      client_id: CLIENT,
    });
    expect(answer.status).toBe(400);
    expect(await answer.json()).toEqual({
      error: "forge_refused",
      reason: "device_flow_disabled",
    });

    forge({
      code: forgeSays(
        {
          message: "Not Found",
        },
        404,
      ),
    });
    const bare = await post("/v1/sign_in", {
      client_id: CLIENT,
    });
    expect(bare.status).toBe(400);
    expect(await bare.json()).toEqual({
      error: "forge_refused",
    });
  });

  it("passes on no text the forge sent beyond a lowercase reason", async () => {
    forge({
      code: forgeSays(
        {
          error: "<script>alert(1)</script>",
        },
        400,
      ),
    });
    const answer = await post("/v1/sign_in", {
      client_id: CLIENT,
    });
    expect(await answer.json()).toEqual({
      error: "forge_refused",
    });
  });

  it("says forge_unreachable when the forge is down or answers 5xx, and forge_unreadable for an answer with no code", async () => {
    forge({
      unreachable: "code",
    });
    const down = await post("/v1/sign_in", {
      client_id: CLIENT,
    });
    expect(down.status).toBe(502);
    expect(await down.json()).toEqual({
      error: "forge_unreachable",
    });

    forge({
      code: forgeSays({}, 503),
    });
    expect(
      (
        await post("/v1/sign_in", {
          client_id: CLIENT,
        })
      ).status,
    ).toBe(502);

    forge({
      code: forgeSays({
        user_code: "WDJB-MJHT",
      }),
    });
    const unreadable = await post("/v1/sign_in", {
      client_id: CLIENT,
    });
    expect(unreadable.status).toBe(502);
    expect(await unreadable.json()).toEqual({
      error: "forge_unreadable",
    });
  });

  it("refuses a body that is not exactly a client id, before it asks the forge", async () => {
    const spy = forge({});
    for (const body of [
      {},
      {
        client_id: "",
      },
      {
        client_id: "a".repeat(65),
      },
      {
        client_id: "has space",
      },
      {
        client_id: "a/b",
      },
      {
        client_id: 1,
      },
      {
        client_id: CLIENT,
        scope: "repo",
      },
    ]) {
      const answer = await post("/v1/sign_in", body);
      expect(answer.status, JSON.stringify(body)).toBe(400);
      expect(await answer.json()).toEqual({
        error: "invalid_body",
      });
    }
    expect(spy).not.toHaveBeenCalled();
    const get = await toStore(`${STORE}/v1/sign_in`);
    expect(get.status).toBe(405);
  });

  it("is counted by the enrol limit before it calls the forge", async () => {
    const spy = forge({});
    const limit = vi.fn(async () => ({
      success: false,
    }));
    const answer = await post(
      "/v1/sign_in",
      {
        client_id: CLIENT,
      },
      {
        ENROL_LIMIT: {
          limit,
        },
      } as Vars,
    );
    expect(answer.status).toBe(429);
    expect(await answer.json()).toEqual({
      error: "enrol_rate_limited",
    });
    expect(limit).toHaveBeenCalledTimes(1);
    expect(spy).not.toHaveBeenCalled();
  });
});

describe("polling a phone's sign-in", () => {
  it("answers 202 pending while the person has not entered the code, and registers nothing", async () => {
    const spy = forge({
      token: forgeSays({
        error: "authorization_pending",
      }),
    });
    const answer = await poll();
    expect(answer.status).toBe(202);
    expect(await answer.json()).toEqual({
      waiting: "pending",
    });
    const [url, init] = spy.mock.calls[0]!;
    expect(String(url)).toBe("https://forge.test/login/oauth/access_token");
    expect(String(init!.body)).toBe(
      new URLSearchParams({
        client_id: CLIENT,
        device_code: CODE.device_code,
        grant_type: "urn:ietf:params:oauth:grant-type:device_code",
      }).toString(),
    );
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("answers 202 slow_down with the forge's longer interval, or 10 where it names none", async () => {
    forge({
      token: forgeSays({
        error: "slow_down",
        interval: 15,
      }),
    });
    expect(await (await poll()).json()).toEqual({
      waiting: "slow_down",
      interval: 15,
    });
    forge({
      token: forgeSays({
        error: "slow_down",
      }),
    });
    const answer = await poll();
    expect(answer.status).toBe(202);
    expect(await answer.json()).toEqual({
      waiting: "slow_down",
      interval: 10,
    });
  });

  it("answers 410 expired and 403 denied, and registers nothing", async () => {
    forge({
      token: forgeSays({
        error: "expired_token",
      }),
    });
    const expired = await poll();
    expect(expired.status).toBe(410);
    expect(await expired.json()).toEqual({
      error: "expired",
    });

    forge({
      token: forgeSays({
        error: "access_denied",
      }),
    });
    const denied = await poll();
    expect(denied.status).toBe(403);
    expect(await denied.json()).toEqual({
      error: "denied",
    });
  });

  it("answers any other forge error as 400 forge_refused with its reason", async () => {
    forge({
      token: forgeSays({
        error: "incorrect_device_code",
      }),
    });
    const answer = await poll();
    expect(answer.status).toBe(400);
    expect(await answer.json()).toEqual({
      error: "forge_refused",
      reason: "incorrect_device_code",
    });
  });

  it("says forge_unreachable when the forge is down or answers 5xx, and forge_unreadable for an answer with no token", async () => {
    forge({
      unreachable: "token",
    });
    expect((await poll()).status).toBe(502);
    forge({
      token: forgeSays({}, 500),
    });
    const failing = await poll();
    expect(failing.status).toBe(502);
    expect(await failing.json()).toEqual({
      error: "forge_unreachable",
    });
    forge({
      token: forgeSays({
        token_type: "bearer",
      }),
    });
    const unreadable = await poll();
    expect(unreadable.status).toBe(502);
    expect(await unreadable.json()).toEqual({
      error: "forge_unreadable",
    });
  });

  it("registers a phone for the account the token names, and the Mac of that account lists it as a phone", async () => {
    const mac = await enrolDevice(9401, "studio");
    const other = await enrolDevice(9402, "elsewhere");
    forge({
      token: forgeSays({
        access_token: "phone-token-9401",
        token_type: "bearer",
      }),
      user: {
        id: 9401,
        login: "someone",
      },
    });

    const answer = await poll();

    expect(answer.status).toBe(200);
    const { device } = (await answer.json()) as {
      device: string;
    };
    expect(device).toMatch(/^9401\.[0-9a-f]{32}$/);
    const peers = (await (await call(mac, "GET", "/v1/peers")).json()) as {
      devices: {
        device: string;
        machine: string;
        kind: string;
        address: string;
      }[];
    };
    expect(peers.devices).toEqual([
      expect.objectContaining({
        device,
        machine: "iPhone",
        kind: "phone",
        address: "",
      }),
    ]);
    expect(await (await call(other, "GET", "/v1/peers")).json()).toEqual({
      devices: [],
    });
  });

  it("keeps no token and sends none to the Durable Object, the answer or the log", async () => {
    const token = "phone-flow-token-that-must-not-leak";
    const logs = (
      [
        "log",
        "info",
        "warn",
        "error",
        "debug",
      ] as const
    ).map((level) => vi.spyOn(console, level));
    const spy = forge({
      token: forgeSays({
        access_token: token,
        token_type: "bearer",
      }),
      user: {
        id: 9411,
        login: "someone",
      },
    });

    const answer = await poll();
    const text = await answer.text();

    expect(answer.status).toBe(200);
    expect(text).not.toContain(token);
    const [url, init] = spy.mock.calls[1]!;
    expect(String(url)).toBe("https://forge.test/user");
    expect((init!.headers as Record<string, string>).Authorization).toBe(`Bearer ${token}`);
    const stored = await everythingStoredFor(9411);
    expect(stored).not.toContain(token);
    expect(stored).not.toContain("someone");
    for (const log of logs) expect(JSON.stringify(log.mock.calls)).not.toContain(token);
  });

  it("is not counted by the enrol limit", async () => {
    forge({
      token: forgeSays({
        error: "authorization_pending",
      }),
    });
    const limit = vi.fn(async () => ({
      success: false,
    }));
    const answer = await poll({}, {
      ENROL_LIMIT: {
        limit,
      },
    } as Vars);
    expect(answer.status).toBe(202);
    expect(limit).not.toHaveBeenCalled();
  });

  it("refuses a body that is not exactly its four fields, before it asks the forge", async () => {
    const spy = forge({
      token: forgeSays({
        error: "authorization_pending",
      }),
    });
    const { spki } = await newKeyPair();
    const whole = {
      client_id: CLIENT,
      device_code: CODE.device_code,
      public_key: spki,
      machine: "iPhone",
    };
    const bodies = [
      {},
      {
        ...whole,
        extra: 1,
      },
      {
        ...whole,
        client_id: "no good",
      },
      {
        ...whole,
        device_code: "",
      },
      {
        ...whole,
        device_code: "a".repeat(129),
      },
      {
        ...whole,
        device_code: "line\nbreak",
      },
      {
        ...whole,
        device_code: 5,
      },
      {
        ...whole,
        machine: "",
      },
      {
        ...whole,
        machine: "x".repeat(65),
      },
      {
        ...whole,
        machine: "tab\there",
      },
    ];
    for (const body of bodies) {
      const answer = await post("/v1/sign_in/poll", body);
      expect(answer.status, JSON.stringify(body)).toBe(400);
      expect(await answer.json()).toEqual({
        error: "invalid_body",
      });
    }
    const { public_key: _gone, ...missing } = whole;
    expect((await post("/v1/sign_in/poll", missing)).status).toBe(400);
    const badKey = await post("/v1/sign_in/poll", {
      ...whole,
      public_key: "bm90IGEga2V5",
    });
    expect(badKey.status).toBe(400);
    expect(await badKey.json()).toEqual({
      error: "invalid_public_key",
    });
    expect(spy).not.toHaveBeenCalled();
    expect((await toStore(`${STORE}/v1/sign_in/poll`)).status).toBe(405);
  });

  it("registers nothing when the forge refuses the token", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const { pathname } = new URL(String(input instanceof Request ? input.url : input));
      return pathname === "/login/oauth/access_token"
        ? Response.json({
            access_token: "revoked",
          })
        : Response.json(
            {
              message: "Bad credentials",
            },
            {
              status: 401,
            },
          );
    });
    const answer = await poll();
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "forge_refused",
    });
  });
});

describe("polling a phone's sign-in at a team store", () => {
  const team = (TEAM: string): Vars => ({
    TEAM,
  });

  it("runs the membership check and registers a member's phone in the team's object", async () => {
    const vars = team("Phone-team");
    const spy = forge({
      token: forgeSays({
        access_token: "team-phone-token",
      }),
      user: {
        id: 9421,
        login: "member",
      },
      membership: {
        status: 200,
        state: "active",
      },
    });

    const answer = await poll({}, vars);

    expect(answer.status).toBe(200);
    const { device } = (await answer.json()) as {
      device: string;
    };
    expect(device).toMatch(/^9421\./);
    const urls = spy.mock.calls.map(([input]) => String(input));
    expect(urls).toContain("https://forge.test/user/memberships/orgs/Phone-team");
    const stored = await everythingStoredIn(teamObject("phone-team"));
    expect(stored).toContain(device);
    expect(stored).toContain('"kind":"phone"');
    expect(stored).not.toContain("team-phone-token");
  });

  it("refuses a user who is not in the team with 403 not_a_member, and registers nothing", async () => {
    const vars = team("Phone-outsiders");
    forge({
      token: forgeSays({
        access_token: "outsider-token",
      }),
      user: {
        id: 9431,
        login: "outsider",
      },
      membership: {
        status: 404,
      },
    });

    const answer = await poll({}, vars);

    expect(answer.status).toBe(403);
    expect(await answer.json()).toEqual({
      error: "not_a_member",
    });
    expect(await everythingStoredIn(teamObject("phone-outsiders"))).toBe("[]");
  });

  it("refuses a member who is still pending, and answers org_restricted when the org blocks the read", async () => {
    forge({
      token: forgeSays({
        access_token: "pending-token",
      }),
      user: {
        id: 9441,
        login: "pending",
      },
      membership: {
        status: 200,
        state: "pending",
      },
    });
    const pending = await poll({}, team("Phone-pending"));
    expect(pending.status).toBe(403);
    expect(await pending.json()).toEqual({
      error: "not_a_member",
    });

    forge({
      token: forgeSays({
        access_token: "restricted-token",
      }),
      user: {
        id: 9442,
        login: "restricted",
      },
      membership: {
        status: 403,
      },
    });
    const restricted = await poll({}, team("Phone-restricted"));
    expect(restricted.status).toBe(403);
    expect(await restricted.json()).toEqual({
      error: "org_restricted",
    });
  });

  it("refuses every poll with 500 bad_team_config when TEAM is not an org or org/team", async () => {
    const spy = forge({});
    const answer = await poll({}, team("not/a/team"));
    expect(answer.status).toBe(500);
    expect(await answer.json()).toEqual({
      error: "bad_team_config",
    });
    expect(spy).not.toHaveBeenCalled();
  });
});

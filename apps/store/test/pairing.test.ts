import { runInDurableObject } from "cloudflare:test";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  STORE,
  accountObject,
  call,
  claimRed,
  claimant,
  enrolDevice,
  enrolMember,
  hash,
  newKeyPair,
  toStore,
  type Device,
  type Vars,
} from "./support";

afterEach(() => vi.restoreAllMocks());

type Peer = {
  device: string;
  machine: string;
  kind: string;
  address: string;
  public_key: string;
};
const peersOf = async (device: Device): Promise<Peer[]> => {
  const answer = await call(device, "GET", "/v1/peers");
  expect(answer.status).toBe(200);
  return (
    (await answer.json()) as {
      devices: Peer[];
    }
  ).devices;
};

const present = (device: Device, kind: string, address: string) =>
  call(device, "PUT", "/v1/presence", {
    kind,
    address,
  });

// Each test has a team of its own, because the object of one outlives the test that filled it.
const team = (TEAM: string): Vars => ({
  TEAM,
});

describe("a device's presence", () => {
  it("sets its own kind and address and no other device's", async () => {
    const a = await enrolDevice(9001, "studio");
    const b = await enrolDevice(9001, "laptop");

    const answer = await present(a, "mac", "https://studio.example.com");

    expect(answer.status).toBe(204);
    expect(await peersOf(b)).toEqual([
      expect.objectContaining({
        device: a.id,
        kind: "mac",
        address: "https://studio.example.com",
      }),
    ]);
    // `a` wrote its row alone: b's is as enrolled, a Mac with no address.
    expect(await peersOf(a)).toEqual([
      expect.objectContaining({
        device: b.id,
        kind: "mac",
        address: "",
      }),
    ]);
  });

  it("is replaced by the next put, and cleared by an empty address", async () => {
    const a = await enrolDevice(9002);
    const b = await enrolDevice(9002);
    await present(a, "mac", "https://one.example.com");
    await present(a, "mac", "https://two.example.com:8443");
    expect((await peersOf(b))[0]!.address).toBe("https://two.example.com:8443");
    await present(a, "mac", "");
    expect((await peersOf(b))[0]!.address).toBe("");
  });

  it("refuses a body that is not exactly a kind and an address, with 400 invalid_body", async () => {
    const a = await enrolDevice(9003);
    const bodies: unknown[] = [
      {},
      {
        kind: "mac",
      },
      {
        address: "",
      },
      {
        kind: "mac",
        address: "",
        extra: 1,
      },
      {
        kind: "tablet",
        address: "",
      },
      {
        kind: 1,
        address: "",
      },
      {
        kind: "mac",
        address: null,
      },
      {
        kind: "mac",
        address: "http://studio.example.com",
      },
      {
        kind: "mac",
        address: "studio.example.com",
      },
      {
        kind: "mac",
        address: "https://studio.example.com/path",
      },
      {
        kind: "mac",
        address: "https://studio.example.com/",
      },
      {
        kind: "mac",
        address: "https://studio.example.com?a=1",
      },
      {
        kind: "mac",
        address: "https://studio.example.com#frag",
      },
      {
        kind: "mac",
        address: "https://user@studio.example.com",
      },
      {
        kind: "mac",
        address: "https://studio.example.com:0",
      },
      {
        kind: "mac",
        address: "https://studio.example.com:65536",
      },
      {
        kind: "mac",
        address: "https://studio.example.com:",
      },
      {
        kind: "mac",
        address: `https://${"a".repeat(189)}.com`,
      },
      [],
    ];
    for (const body of bodies) {
      const answer = await call(a, "PUT", "/v1/presence", body);
      expect(answer.status, JSON.stringify(body)).toBe(400);
      expect(await answer.json()).toEqual({
        error: "invalid_body",
      });
    }
  });

  it("accepts the longest address, 200 characters, and a phone's kind", async () => {
    const a = await enrolDevice(9004);
    const b = await enrolDevice(9004);
    const address = `https://${"a".repeat(188)}.com`;
    expect(address).toHaveLength(200);
    expect((await present(a, "phone", address)).status).toBe(204);
    expect(await peersOf(b)).toEqual([
      expect.objectContaining({
        kind: "phone",
        address,
      }),
    ]);
  });

  it("answers 405 bad_method to a method the path does not take", async () => {
    const a = await enrolDevice(9005);
    for (const method of [
      "GET",
      "POST",
      "DELETE",
    ]) {
      const answer = await call(a, method, "/v1/presence");
      expect(answer.status, method).toBe(405);
    }
    for (const method of [
      "PUT",
      "POST",
      "DELETE",
    ]) {
      const answer = await call(a, method, "/v1/peers");
      expect(answer.status, method).toBe(405);
    }
  });

  it("is refused 401 to an unsigned call", async () => {
    expect(
      (
        await toStore(`${STORE}/v1/presence`, {
          method: "PUT",
          body: "{}",
        })
      ).status,
    ).toBe(401);
    expect((await toStore(`${STORE}/v1/peers`)).status).toBe(401);
  });
});

describe("a device's peers", () => {
  it("lists the account's other devices with key and address, oldest first, and not the caller", async () => {
    const a = await enrolDevice(9101, "studio");
    const b = await enrolDevice(9101, "laptop");
    const c = await enrolDevice(9101, "phone");
    await present(b, "mac", "https://laptop.example.com");
    await present(c, "phone", "");

    const forA = await peersOf(a);

    expect(forA.map((peer) => peer.device)).toEqual([
      b.id,
      c.id,
    ]);
    expect(forA).toEqual([
      {
        device: b.id,
        machine: "laptop",
        kind: "mac",
        address: "https://laptop.example.com",
        public_key: expect.stringMatching(/^[A-Za-z0-9+/=]+$/),
      },
      {
        device: c.id,
        machine: "phone",
        kind: "phone",
        address: "",
        public_key: expect.any(String),
      },
    ]);
    expect((await peersOf(c)).map((peer) => peer.device)).toEqual([
      a.id,
      b.id,
    ]);
  });

  it("is empty for a device that is alone", async () => {
    expect(await peersOf(await enrolDevice(9102))).toEqual([]);
  });

  it("never lists another account's device on a personal store, where each account has an object of its own", async () => {
    const mine = await enrolDevice(9111, "studio");
    const mineToo = await enrolDevice(9111, "laptop");
    const theirs = await enrolDevice(9112, "their studio");
    await present(theirs, "mac", "https://theirs.example.com");
    // An account whose number is the start of the other's is another account.
    const short = await enrolDevice(911, "short");

    const listed = await peersOf(mine);

    expect(listed.map((peer) => peer.device)).toEqual([
      mineToo.id,
    ]);
    expect(JSON.stringify(listed)).not.toContain(theirs.id);
    expect(JSON.stringify(listed)).not.toContain("theirs.example.com");
    expect(await peersOf(theirs)).toEqual([]);
    expect(await peersOf(short)).toEqual([]);
  });

  it("never lists another member's device on a team store, where every member's devices are in one object", async () => {
    const vars = team("Peers");
    const first = await enrolMember(vars, 9121, "first", "studio");
    const firstToo = await enrolMember(vars, 9121, "first", "laptop");
    const second = await enrolMember(vars, 9122, "second", "studio");
    const secondToo = await enrolMember(vars, 9122, "second", "laptop");
    // The account whose number begins with another's.
    const long = await enrolMember(vars, 91210, "long", "studio");
    await present(second, "mac", "https://second.example.com");
    await present(long, "mac", "https://long.example.com");

    // One object holds all five, which is why the filter has to be the account.
    const everyone = await call(first, "GET", "/v1/devices");
    expect(
      (
        (await everyone.json()) as {
          devices: unknown[];
        }
      ).devices,
    ).toHaveLength(5);

    const listed = await peersOf(first);

    expect(listed.map((peer) => peer.device)).toEqual([
      firstToo.id,
    ]);
    expect(JSON.stringify(listed)).not.toContain(second.id);
    expect(JSON.stringify(listed)).not.toContain(secondToo.id);
    expect(JSON.stringify(listed)).not.toContain(long.id);
    expect((await peersOf(second)).map((peer) => peer.device)).toEqual([
      secondToo.id,
    ]);
    expect(await peersOf(long)).toEqual([]);
  });

  it("lists a team member's own phone to the member's Mac and not to a colleague's", async () => {
    const vars = team("Peers-and-phones");
    const mac = await enrolMember(vars, 9131, "first");
    const colleague = await enrolMember(vars, 9132, "second");
    const phone = await enrolMember(vars, 9131, "first", "iPhone");
    await present(phone, "phone", "");

    expect(
      (await peersOf(mac)).map((peer) => [
        peer.device,
        peer.kind,
      ]),
    ).toEqual([
      [
        phone.id,
        "phone",
      ],
    ]);
    expect(await peersOf(colleague)).toEqual([]);
  });
});

describe("a store's answers to a page on another origin", () => {
  const exposed = {
    "access-control-allow-origin": "*",
    "access-control-expose-headers": "Retry-After",
  };

  it("carry the CORS headers on a refusal and on an answer", async () => {
    const a = await enrolDevice(9201);
    const refusals = [
      await toStore(`${STORE}/v1/peers`),
      await toStore(`${STORE}/v1/enrol`, {
        method: "POST",
        body: "{}",
      }),
      await toStore(`${STORE}/v1/enrol`),
      await toStore(`${STORE}/v1/sign_in`, {
        method: "POST",
        body: "{}",
      }),
      await toStore(`${STORE}/v1/nothing`),
      await call(a, "GET", "/v1/presence"),
      await call(a, "GET", "/v1/claims/nothing"),
    ];
    const answers = [
      await call(a, "GET", "/v1/peers"),
      await present(a, "mac", ""),
      await call(a, "GET", "/v1/claims"),
    ];
    for (const answer of [
      ...refusals,
      ...answers,
    ]) {
      expect(answer.status).toBeGreaterThanOrEqual(200);
      for (const [name, value] of Object.entries(exposed))
        expect(answer.headers.get(name), `${answer.status} ${name}`).toBe(value);
    }
    expect(refusals.map((answer) => answer.status)).toEqual([
      401,
      400,
      405,
      400,
      401,
      405,
      400,
    ]);
  });

  it("carry them on a team store's bad_team_config refusal", async () => {
    const answer = await toStore(`${STORE}/v1/peers`, {}, team("not/a/team"));
    expect(answer.status).toBe(500);
    expect(answer.headers.get("access-control-allow-origin")).toBe("*");
  });

  it("carry them, and Retry-After, on a rate limit", async () => {
    const a = await enrolDevice(9202);
    let limited: Response | undefined;
    for (let n = 0; n < 130 && !limited; n++) {
      const answer = await call(a, "GET", "/v1/peers");
      if (answer.status === 429) limited = answer;
    }
    expect(limited?.headers.get("retry-after")).toBe("60");
    expect(limited?.headers.get("access-control-allow-origin")).toBe("*");
    expect(limited?.headers.get("access-control-expose-headers")).toBe("Retry-After");
  });

  it("answers an OPTIONS preflight of any /v1 path with 204 and the methods and headers a signed call needs", async () => {
    for (const path of [
      "/v1/peers",
      "/v1/presence",
      "/v1/sign_in",
      "/v1/sign_in/poll",
      "/v1/enrol",
      "/v1/claims/a/b/c",
    ]) {
      const answer = await toStore(`${STORE}${path}`, {
        method: "OPTIONS",
        headers: {
          Origin: "https://pocket.example.com",
          "Access-Control-Request-Method": "PUT",
        },
      });
      expect(answer.status, path).toBe(204);
      expect(await answer.text()).toBe("");
      expect(answer.headers.get("access-control-allow-origin")).toBe("*");
      expect(answer.headers.get("access-control-expose-headers")).toBe("Retry-After");
      expect(answer.headers.get("access-control-allow-methods")).toBe("GET, POST, PUT, DELETE");
      expect(answer.headers.get("access-control-allow-headers")).toBe(
        "Content-Type, X-Armada-Device, X-Armada-Time, X-Armada-Nonce, X-Armada-Signature",
      );
      expect(answer.headers.get("access-control-max-age")).toBe("600");
      expect(answer.headers.get("access-control-allow-credentials")).toBeNull();
    }
  });

  it("does not answer a preflight of a path outside /v1", async () => {
    const answer = await toStore(`${STORE}/elsewhere`, {
      method: "OPTIONS",
    });
    expect(answer.status).toBe(404);
    expect(answer.headers.get("access-control-allow-origin")).toBeNull();
  });
});

describe("a store whose devices table has no kind or address", () => {
  it("gets both at the first verified request and keeps its rows, as Macs with no address", async () => {
    const account = 9301;
    const { key, spki } = await newKeyPair();
    const device: Device = {
      id: `${account}.${"d".repeat(32)}`,
      key,
      account,
      token: "",
    };
    const other = `${account}.${"c".repeat(32)}`;
    await runInDurableObject(accountObject(account), (_instance, state) => {
      const sql = state.storage.sql;
      sql.exec(`
        CREATE TABLE devices (id TEXT PRIMARY KEY, public_key TEXT NOT NULL, machine TEXT NOT NULL);
        CREATE TABLE nonces (device TEXT NOT NULL, nonce TEXT NOT NULL, seen INTEGER NOT NULL, PRIMARY KEY (device, nonce));
        CREATE TABLE owners (key TEXT PRIMARY KEY, kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL);
        CREATE TABLE claims (repo TEXT NOT NULL, kind TEXT NOT NULL, key TEXT NOT NULL, holder_kind TEXT NOT NULL, holder_id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL, PRIMARY KEY (repo, kind, key));
        CREATE TABLE holders (kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, name TEXT NOT NULL, title TEXT NOT NULL, renewed INTEGER NOT NULL, PRIMARY KEY (kind, id, machine));
        CREATE TABLE contenders (repo TEXT NOT NULL, kind TEXT NOT NULL, key TEXT NOT NULL, contender_kind TEXT NOT NULL, contender_id TEXT NOT NULL, machine TEXT NOT NULL, name TEXT NOT NULL, title TEXT NOT NULL, since INTEGER NOT NULL, at INTEGER NOT NULL, told INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (repo, kind, key, contender_kind, contender_id, machine));
      `);
      sql.exec(
        "INSERT INTO devices (id, public_key, machine) VALUES (?, ?, 'studio')",
        device.id,
        spki,
      );
      sql.exec(
        "INSERT INTO devices (id, public_key, machine) VALUES (?, ?, 'laptop')",
        other,
        spki,
      );
    });

    // The first verified request is a claim, which touches no column of devices.
    expect((await claimRed(device, hash("a"), hash("b"), claimant("s-1"))).status).toBe(200);

    expect(await peersOf(device)).toEqual([
      {
        device: other,
        machine: "laptop",
        kind: "mac",
        address: "",
        public_key: spki,
      },
    ]);
    expect((await present(device, "phone", "https://old.example.com")).status).toBe(204);
    const columns = await runInDurableObject(accountObject(account), (_instance, state) =>
      state.storage.sql
        .exec<{
          name: string;
        }>("SELECT name FROM pragma_table_info('devices') ORDER BY cid")
        .toArray()
        .map((column) => column.name),
    );
    expect(columns).toEqual([
      "id",
      "public_key",
      "machine",
      "kind",
      "address",
    ]);
    const rows = await runInDurableObject(accountObject(account), (_instance, state) =>
      state.storage.sql.exec("SELECT id FROM devices ORDER BY rowid").toArray(),
    );
    expect(rows).toHaveLength(2);
  });

  it("is left alone by a second upgrade, and the first new device is a Mac", async () => {
    const a = await enrolDevice(9302);
    const b = await enrolDevice(9302);
    await present(a, "phone", "");
    // A table that has the columns takes no second ALTER and keeps what presence set.
    await runInDurableObject(accountObject(9302), (instance) => {
      (
        instance as unknown as {
          upgraded: boolean;
        }
      ).upgraded = false;
    });
    expect((await peersOf(b))[0]).toEqual(
      expect.objectContaining({
        device: a.id,
        kind: "phone",
      }),
    );
  });
});

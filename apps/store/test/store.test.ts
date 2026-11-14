import { runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { CLAIM_KINDS, LAPSE_SECONDS } from "../src/rows";
import { signRequest } from "../src/signing";
import vector from "../test-vectors/red-claim.json";
import {
  STORE,
  accountObject,
  call,
  toStore,
  claim,
  claimPath,
  claimRed,
  claimant,
  renew,
  enrolDevice,
  everythingStoredFor,
  fact,
  hash,
  holder,
  newKeyPair,
} from "./support";

const now = () => Math.floor(Date.now() / 1000);

type Answered = {
  holder: Record<string, unknown>;
  renewed: number;
};
const answered = async (answer: Response) => (await answer.json()) as Answered;

/** Sets when each holder last renewed, as though that many seconds had passed since. */
async function renewedAgo(account: number, ago: Record<string, number>) {
  await runInDurableObject(accountObject(account), (_instance, state) => {
    for (const [id, seconds] of Object.entries(ago))
      state.storage.sql.exec("UPDATE holders SET renewed = ? WHERE id = ?", now() - seconds, id);
  });
}

/** The ids of the holders that hold a claim, straight from the table. */
async function holdersOfClaims(account: number): Promise<string[]> {
  return runInDurableObject(accountObject(account), (_instance, state) =>
    state.storage.sql
      .exec<{
        holder_id: string;
      }>("SELECT DISTINCT holder_id FROM claims ORDER BY holder_id")
      .toArray()
      .map((row) => row.holder_id),
  );
}

const alarmOf = (account: number) =>
  runInDurableObject(accountObject(account), (_instance, state) => state.storage.getAlarm());

describe("a claim", () => {
  it("is won by exactly one of eight concurrent claims", async () => {
    const [a, b] = [
      await enrolDevice(5001),
      await enrolDevice(5001),
    ];
    const attempts = Array.from(
      {
        length: 8,
      },
      (_, n) => claimRed(n % 2 ? a : b, hash("a"), hash("b"), claimant(`j-${n}`)),
    );
    const answers = await Promise.all(attempts);

    const won = answers.filter((answer) => answer.status === 200);
    const lost = answers.filter((answer) => answer.status === 409);
    expect(won).toHaveLength(1);
    expect(lost).toHaveLength(7);

    const winner = (await answered(won[0]!)).holder;
    for (const answer of lost) expect((await answered(answer)).holder).toEqual(winner);
    const read = await call(a, "GET", claimPath(hash("a"), "red", hash("b")));
    expect((await answered(read)).holder).toEqual(winner);
  });

  for (const [n, kind] of CLAIM_KINDS.entries())
    it(`of kind ${kind} is taken once and refused to a second holder with the first's name, Mac and Job title`, async () => {
      const [home, work] = [
        await enrolDevice(5011 + n),
        await enrolDevice(5011 + n),
      ];
      const first = claimant("s-1", "Home Mac", "Nick", "Fix the flaky queue test");
      const taken = await claim(home, hash("a"), kind, hash("b"), first);
      expect(taken.status).toBe(200);
      expect((await answered(taken)).holder).toEqual(first);

      const second = await claim(
        work,
        hash("a"),
        kind,
        hash("b"),
        claimant("s-2", "Work Mac", "Sam", "Speed up the queue"),
      );
      expect(second.status).toBe(409);
      const refused = await answered(second);
      expect(refused.holder).toEqual(first);
      expect(refused.renewed).toBeGreaterThanOrEqual(now() - 5);

      const read = await call(work, "GET", claimPath(hash("a"), kind, hash("b")));
      expect(read.status).toBe(200);
      expect((await answered(read)).holder).toEqual(first);
    });

  it("under one key of two kinds is two claims", async () => {
    const a = await enrolDevice(5009);
    expect((await claim(a, hash("a"), "test", hash("b"), claimant("s-1"))).status).toBe(200);
    expect((await claim(a, hash("a"), "item", hash("b"), claimant("s-2"))).status).toBe(200);
    const test = await answered(await call(a, "GET", claimPath(hash("a"), "test", hash("b"))));
    const item = await answered(await call(a, "GET", claimPath(hash("a"), "item", hash("b"))));
    expect(test.holder.id).toBe("s-1");
    expect(item.holder.id).toBe("s-2");
  });

  it("made by Fleet's client for a red main is taken under the new key", async () => {
    const redClaim = {
      body: JSON.parse(vector.body) as {
        repo: string;
        key: string;
        holder: Record<string, unknown>;
      },
    };
    const [a, b] = [
      await enrolDevice(5010),
      await enrolDevice(5010),
    ];
    const taken = await call(a, "POST", "/v1/claims", redClaim.body);
    expect(taken.status).toBe(200);
    expect((await answered(taken)).holder).toEqual(redClaim.body.holder);
    const read = await call(b, "GET", claimPath(redClaim.body.repo, "red", redClaim.body.key));
    expect((await answered(read)).holder).toEqual(redClaim.body.holder);
    const second = await call(b, "POST", "/v1/claims", {
      ...redClaim.body,
      holder: claimant("s-2", "Work Mac"),
    });
    expect(second.status).toBe(409);
  });

  it("leaves another red of the same repository free", async () => {
    const a = await enrolDevice(5003);
    expect((await claimRed(a, hash("a"), hash("b"), claimant("j-1"))).status).toBe(200);
    expect((await claimRed(a, hash("a"), hash("c"), claimant("j-2"))).status).toBe(200);
  });

  it("reads as 404 until taken, and 404 again once given back", async () => {
    const a = await enrolDevice(5004);
    const path = claimPath(hash("a"), "file", hash("b"));
    expect((await call(a, "GET", path)).status).toBe(404);
    await claim(a, hash("a"), "file", hash("b"), claimant("j-1"));
    expect((await call(a, "GET", path)).status).toBe(200);
    expect((await call(a, "DELETE", path)).status).toBe(204);
    expect((await call(a, "GET", path)).status).toBe(404);
  });

  it("can be taken over by deleting it and claiming again", async () => {
    const [a, b] = [
      await enrolDevice(5005),
      await enrolDevice(5005),
    ];
    await claimRed(a, hash("a"), hash("b"), claimant("j-1"));
    await call(b, "DELETE", claimPath(hash("a"), "red", hash("b")));
    expect((await claimRed(b, hash("a"), hash("b"), claimant("j-2"))).status).toBe(200);
  });

  it("is cleared for one kind of a repository, and no other kind or repository", async () => {
    const a = await enrolDevice(5006);
    await claimRed(a, hash("a"), hash("b"), claimant("j-1"));
    await claimRed(a, hash("a"), hash("c"), claimant("j-2"));
    await claim(a, hash("a"), "item", hash("b"), claimant("j-2"));
    await claimRed(a, hash("d"), hash("b"), claimant("j-3"));
    expect((await call(a, "DELETE", `/v1/claims/${hash("a")}/red`)).status).toBe(204);
    expect((await call(a, "GET", claimPath(hash("a"), "red", hash("b")))).status).toBe(404);
    expect((await call(a, "GET", claimPath(hash("a"), "red", hash("c")))).status).toBe(404);
    expect((await call(a, "GET", claimPath(hash("a"), "item", hash("b")))).status).toBe(200);
    expect((await call(a, "GET", claimPath(hash("d"), "red", hash("b")))).status).toBe(200);
  });

  it("is not seen by another account", async () => {
    const [mine, theirs] = [
      await enrolDevice(5007),
      await enrolDevice(5008),
    ];
    await claimRed(mine, hash("a"), hash("b"), claimant("j-1"));
    expect((await call(theirs, "GET", claimPath(hash("a"), "red", hash("b")))).status).toBe(404);
    expect((await claimRed(theirs, hash("a"), hash("b"), claimant("j-2"))).status).toBe(200);
  });
});

describe("a renewal", () => {
  it("renews every claim a Job holds in one call, and no other Job's", async () => {
    const a = await enrolDevice(5501);
    for (const kind of CLAIM_KINDS) await claim(a, hash("a"), kind, hash("b"), claimant("s-1"));
    await claim(a, hash("c"), "file", hash("d"), claimant("s-2"));
    await renewedAgo(5501, {
      "s-1": 20 * 60,
      "s-2": 20 * 60,
    });

    const renewal = await renew(a, claimant("s-1"));

    expect(renewal.status).toBe(200);
    expect(await renewal.json()).toEqual({
      claims: CLAIM_KINDS.length,
    });
    for (const kind of CLAIM_KINDS) {
      const read = await answered(await call(a, "GET", claimPath(hash("a"), kind, hash("b"))));
      expect(read.renewed).toBeGreaterThanOrEqual(now() - 5);
    }
    const other = await answered(await call(a, "GET", claimPath(hash("c"), "file", hash("d"))));
    expect(other.renewed).toBeLessThanOrEqual(now() - 20 * 60 + 5);
  });

  it("is for one Mac's holder: the same id on another Mac renews nothing", async () => {
    const a = await enrolDevice(5502);
    await claimRed(a, hash("a"), hash("b"), claimant("s-1", "Home Mac"));
    expect(await (await renew(a, claimant("s-1", "Work Mac"))).json()).toEqual({
      claims: 0,
    });
  });

  it("answers 0 for a holder whose claims lapsed, so it knows to claim again", async () => {
    const a = await enrolDevice(5503);
    await claimRed(a, hash("a"), hash("b"), claimant("s-1"));
    await renewedAgo(5503, {
      "s-1": LAPSE_SECONDS + 1,
    });
    expect(await (await renew(a, claimant("s-1"))).json()).toEqual({
      claims: 0,
    });
    expect((await call(a, "GET", claimPath(hash("a"), "red", hash("b")))).status).toBe(404);
  });

  it("refuses a body that is not a holder, and a method it does not take", async () => {
    const a = await enrolDevice(5504);
    const withSince = await call(a, "POST", "/v1/renewals", {
      holder: claimant("s-1"),
    });
    expect(withSince.status).toBe(400);
    const person = await call(a, "POST", "/v1/renewals", {
      holder: {
        kind: "person",
        id: "s-1",
        machine: "studio",
      },
    });
    expect(person.status).toBe(400);
    expect((await call(a, "GET", "/v1/renewals")).status).toBe(405);
  });
});

describe("the lapse", () => {
  it("is set as an alarm when a claim is taken, for 30 minutes after it", async () => {
    const a = await enrolDevice(5601);
    await claimRed(a, hash("a"), hash("b"), claimant("s-1"));
    const alarm = await alarmOf(5601);
    expect(alarm).not.toBeNull();
    expect(Math.abs(alarm! / 1000 - (now() + LAPSE_SECONDS))).toBeLessThanOrEqual(5);
  });

  it("drops by alarm a claim not renewed for 30 minutes, and keeps one renewed inside them", async () => {
    const a = await enrolDevice(5602);
    await claim(a, hash("a"), "item", hash("b"), claimant("s-lapsed"));
    await claim(a, hash("a"), "file", hash("c"), claimant("s-lapsed"));
    await claim(a, hash("a"), "test", hash("d"), claimant("s-renewing"));
    await renewedAgo(5602, {
      "s-lapsed": LAPSE_SECONDS + 1,
      "s-renewing": LAPSE_SECONDS - 60,
    });

    expect(await runDurableObjectAlarm(accountObject(5602))).toBe(true);

    expect(await holdersOfClaims(5602)).toEqual([
      "s-renewing",
    ]);
    const next = await alarmOf(5602);
    expect(Math.abs(next! / 1000 - (now() + 60))).toBeLessThanOrEqual(5);
  });

  it("reads as gone at once, before the alarm has run, and the key is free to claim", async () => {
    const [home, work] = [
      await enrolDevice(5603),
      await enrolDevice(5603),
    ];
    await claim(home, hash("a"), "item", hash("b"), claimant("s-1", "Home Mac"));
    await renewedAgo(5603, {
      "s-1": LAPSE_SECONDS,
    });
    expect((await call(work, "GET", claimPath(hash("a"), "item", hash("b")))).status).toBe(404);
    expect(
      (await claim(work, hash("a"), "item", hash("b"), claimant("s-2", "Work Mac"))).status,
    ).toBe(200);
  });
});

describe("a store built on the October schema", () => {
  it("drops its red claims at the first verified request, and the red is claimed again under the new key", async () => {
    const account = 5701;
    const { key, spki } = await newKeyPair();
    const device = {
      id: `${account}.${"e".repeat(32)}`,
      key,
      account,
      token: "",
    };
    await runInDurableObject(accountObject(account), (_instance, state) => {
      const sql = state.storage.sql;
      sql.exec(`
        CREATE TABLE devices (id TEXT PRIMARY KEY, public_key TEXT NOT NULL, machine TEXT NOT NULL);
        CREATE TABLE nonces (device TEXT NOT NULL, nonce TEXT NOT NULL, seen INTEGER NOT NULL, PRIMARY KEY (device, nonce));
        CREATE TABLE claims (repo TEXT NOT NULL, red TEXT NOT NULL, kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL, PRIMARY KEY (repo, red));
        CREATE TABLE owners (key TEXT PRIMARY KEY, kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL);
      `);
      sql.exec(
        "INSERT INTO devices (id, public_key, machine) VALUES (?, ?, 'studio')",
        device.id,
        spki,
      );
      sql.exec(
        "INSERT INTO claims (repo, red, kind, id, machine, since) VALUES (?, ?, 'session', 's-old', 'studio', 1)",
        hash("a"),
        hash("b"),
      );
    });

    expect((await call(device, "GET", claimPath(hash("a"), "red", hash("b")))).status).toBe(404);
    expect((await claimRed(device, hash("a"), hash("b"), claimant("s-new"))).status).toBe(200);
    expect(await holdersOfClaims(account)).toEqual([
      "s-new",
    ]);
  });
});

describe("a row", () => {
  it("refuses a field a claim's holder does not have, and a title on an owner", async () => {
    const a = await enrolDevice(5101);
    const withRepository = {
      ...claimant("j-1"),
      repository: "mele-labs/armada",
    };
    expect((await claimRed(a, hash("a"), hash("b"), withRepository as never)).status).toBe(400);
    const withPath = await call(a, "POST", "/v1/claims", {
      repo: hash("a"),
      kind: "red",
      key: hash("b"),
      holder: claimant("j-1"),
      path: "src/main.rs",
    });
    expect(withPath.status).toBe(400);
    const owner = await call(a, "PUT", `/v1/owners/${hash("c")}`, {
      holder: claimant("j-1"),
    });
    expect(owner.status).toBe(400);
  });

  it("refuses a claim's holder with no name or title, or one too long or with a control character", async () => {
    const a = await enrolDevice(5104);
    const { name: _name, ...nameless } = claimant("j-1");
    const { title: _title, ...untitled } = claimant("j-1");
    for (const bad of [
      nameless,
      untitled,
      claimant("j-1", "studio", "n".repeat(65)),
      claimant("j-1", "studio", "Nick", "t".repeat(201)),
      claimant("j-1", "studio", "Nick", "two\nlines"),
    ])
      expect((await claimRed(a, hash("a"), hash("b"), bad as never)).status).toBe(400);
    expect(
      (await claimRed(a, hash("a"), hash("b"), claimant("j-1", "studio", "", ""))).status,
    ).toBe(200);
  });

  it("refuses a kind it does not know, in a body and in a path", async () => {
    const a = await enrolDevice(5105);
    expect((await claim(a, hash("a"), "branch" as never, hash("b"), claimant("j-1"))).status).toBe(
      400,
    );
    const read = await call(a, "GET", `/v1/claims/${hash("a")}/branch/${hash("b")}`);
    expect(read.status).toBe(400);
    expect(await read.json()).toEqual({
      error: "invalid_key",
    });
  });

  it("refuses a key that is a name and not a hash", async () => {
    const a = await enrolDevice(5102);
    expect((await claim(a, "armada", "file", "src/main.rs", claimant("j-1"))).status).toBe(400);
    const owner = await call(a, "PUT", "/v1/owners/armada-pull-1", {
      holder: holder("j-1"),
    });
    expect(owner.status).toBe(400);
  });

  it("refuses a holder that is not a session or a job", async () => {
    const a = await enrolDevice(5103);
    const person = {
      ...claimant("j-1"),
      kind: "person",
    };
    expect((await claimRed(a, hash("a"), hash("b"), person as never)).status).toBe(400);
    const yesterday = {
      ...claimant("j-1"),
      since: "yesterday",
    };
    expect((await claimRed(a, hash("a"), hash("b"), yesterday as never)).status).toBe(400);
  });
});

describe("an owner", () => {
  it("is put, read by keys, replaced and deleted", async () => {
    const a = await enrolDevice(5201);
    const [one, two, three] = [
      hash("1"),
      hash("2"),
      hash("3"),
    ];
    await call(a, "PUT", `/v1/owners/${one}`, {
      holder: holder("s-1", "laptop"),
    });
    await call(a, "PUT", `/v1/owners/${two}`, {
      holder: holder("j-2", "desktop"),
    });

    const read = await call(a, "GET", `/v1/owners?keys=${one},${two},${three}`);
    expect(await read.json()).toEqual({
      [one]: holder("s-1", "laptop"),
      [two]: holder("j-2", "desktop"),
    });

    await call(a, "PUT", `/v1/owners/${one}`, {
      holder: holder("s-7", "desktop"),
    });
    expect(await (await call(a, "GET", `/v1/owners?keys=${one}`)).json()).toEqual({
      [one]: holder("s-7", "desktop"),
    });

    expect((await call(a, "DELETE", `/v1/owners/${one}`)).status).toBe(204);
    expect(await (await call(a, "GET", `/v1/owners?keys=${one},${two}`)).json()).toEqual({
      [two]: holder("j-2", "desktop"),
    });
  });

  it("is shared by the account's machines", async () => {
    const [a, b] = [
      await enrolDevice(5202),
      await enrolDevice(5202),
    ];
    await call(a, "PUT", `/v1/owners/${hash("1")}`, {
      holder: holder("s-1"),
    });
    expect(await (await call(b, "GET", `/v1/owners?keys=${hash("1")}`)).json()).toEqual({
      [hash("1")]: holder("s-1"),
    });
  });
});

describe("a repository's runs", () => {
  const path = `/v1/runs/${hash("r")}`;

  it("are put and read back as they were sent", async () => {
    const a = await enrolDevice(5501);
    const facts = [
      fact(),
      {
        ...fact({
          workspace: "apps/web",
          kind: "dev_server",
          command: "pnpm dev",
        }),
        port: 5173,
      },
    ];
    expect(
      (
        await call(a, "PUT", path, {
          facts,
        })
      ).status,
    ).toBe(204);
    const read = await call(a, "GET", path);
    expect(read.status).toBe(200);
    expect(await read.json()).toEqual({
      facts,
    });
  });

  it("are replaced whole by the next put, not merged", async () => {
    const a = await enrolDevice(5502);
    await call(a, "PUT", path, {
      facts: [
        fact(),
        fact({
          command: "cargo clippy",
          kind: "lint",
        }),
      ],
    });
    const later = [
      fact({
        command: "cargo build",
        kind: "build",
      }),
    ];
    expect(
      (
        await call(a, "PUT", path, {
          facts: later,
        })
      ).status,
    ).toBe(204);
    expect(await (await call(a, "GET", path)).json()).toEqual({
      facts: later,
    });
  });

  it("are shared by the account's machines and by no other account", async () => {
    const [a, b, other] = [
      await enrolDevice(5503),
      await enrolDevice(5503),
      await enrolDevice(5504),
    ];
    await call(a, "PUT", path, {
      facts: [
        fact(),
      ],
    });
    expect(await (await call(b, "GET", path)).json()).toEqual({
      facts: [
        fact(),
      ],
    });
    const theirs = await call(other, "GET", path);
    expect(theirs.status).toBe(404);
    expect(await theirs.json()).toEqual({
      error: "no_runs",
    });
  });

  it("are removed by a put of an empty list, which holds no row", async () => {
    const a = await enrolDevice(5505);
    await call(a, "PUT", path, {
      facts: [
        fact(),
      ],
    });
    expect(
      (
        await call(a, "PUT", path, {
          facts: [],
        })
      ).status,
    ).toBe(204);
    const read = await call(a, "GET", path);
    expect(read.status).toBe(404);
    expect(await read.json()).toEqual({
      error: "no_runs",
    });
    expect(
      JSON.parse(await everythingStoredFor(5505)).find(
        (table: { name: string }) => table.name === "runs",
      ).rows,
    ).toEqual([]);
  });

  it("read as 404 no_runs until put, and an empty put for a new repository writes nothing", async () => {
    const a = await enrolDevice(5506);
    expect(await (await call(a, "GET", path)).json()).toEqual({
      error: "no_runs",
    });
    expect(
      (
        await call(a, "PUT", path, {
          facts: [],
        })
      ).status,
    ).toBe(204);
    expect((await call(a, "GET", path)).status).toBe(404);
  });

  it("keep a repository's facts apart from another's", async () => {
    const a = await enrolDevice(5507);
    await call(a, "PUT", `/v1/runs/${hash("r")}`, {
      facts: [
        fact(),
      ],
    });
    await call(a, "PUT", `/v1/runs/${hash("s")}`, {
      facts: [
        fact({
          command: "make",
        }),
      ],
    });
    expect(await (await call(a, "GET", `/v1/runs/${hash("r")}`)).json()).toEqual({
      facts: [
        fact(),
      ],
    });
  });

  describe("refuse with 400 invalid_body", () => {
    const refused = async (id: number, body: unknown) => {
      const a = await enrolDevice(id);
      const answer = await call(a, "PUT", path, body);
      expect(answer.status).toBe(400);
      expect(await answer.json()).toEqual({
        error: "invalid_body",
      });
      expect((await call(a, "GET", path)).status).toBe(404);
    };

    it("a fact with a field it does not have", () =>
      refused(5601, {
        facts: [
          {
            ...fact(),
            title: "x",
          },
        ],
      }));
    it("a body with a field beside facts", () =>
      refused(5602, {
        facts: [
          fact(),
        ],
        name: "armada",
      }));
    it("a body whose facts are missing", () => refused(5603, {}));
    it.each([
      "workspace",
      "kind",
      "command",
      "succeeded",
      "last_ran",
      "source",
    ])("a fact with no %s", (field) => {
      const { [field]: _gone, ...rest } = fact() as Record<string, unknown>;
      return refused(5610, {
        facts: [
          rest,
        ],
      });
    });
    it.each([
      "jobs",
      "",
      7,
    ])("a kind of %j", (kind) =>
      refused(5604, {
        facts: [
          fact({
            kind: kind as never,
          }),
        ],
      }),
    );
    it.each([
      0,
      65536,
      1.5,
      "80",
      -1,
      null,
    ])("a port of %j", (port) =>
      refused(5605, {
        facts: [
          {
            ...fact(),
            port,
          },
        ],
      }),
    );
    it("two facts of one workspace, kind and command", () =>
      refused(5606, {
        facts: [
          fact(),
          fact({
            succeeded: false,
            last_ran: 9,
          }),
        ],
      }));
    it("201 facts", () =>
      refused(5607, {
        facts: Array.from(
          {
            length: 201,
          },
          (_, n) =>
            fact({
              command: `cmd ${n}`,
            }),
        ),
      }));
    it.each([
      [
        "a workspace that is empty",
        {
          workspace: "",
        },
      ],
      [
        "a workspace that is absolute",
        {
          workspace: "/etc",
        },
      ],
      [
        "a workspace that leaves the repository",
        {
          workspace: "../x",
        },
      ],
      [
        "a workspace of 201 characters",
        {
          workspace: "w".repeat(201),
        },
      ],
      [
        "a command with a newline",
        {
          command: "pnpm test\nrm -rf x",
        },
      ],
      [
        "a command of 401 characters",
        {
          command: "c".repeat(401),
        },
      ],
      [
        "a command that is empty",
        {
          command: "",
        },
      ],
      [
        "a source with a space",
        {
          source: "my jobs",
        },
      ],
      [
        "a source of 33 characters",
        {
          source: "s".repeat(33),
        },
      ],
      [
        "a last_ran that is a fraction",
        {
          last_ran: 1.5,
        },
      ],
      [
        "a last_ran that is negative",
        {
          last_ran: -1,
        },
      ],
      [
        "a succeeded that is text",
        {
          succeeded: "yes",
        },
      ],
    ])("%s", (_name, change) =>
      refused(5608, {
        facts: [
          fact(change as never),
        ],
      }),
    );
    it("facts that are not a list", () =>
      refused(5609, {
        facts: fact(),
      }));
    it("a body that is not an object", () =>
      refused(5611, [
        fact(),
      ]));
  });

  it("accept the limits themselves: 200 facts, a 200-character workspace, a 400-character command, port 65535", async () => {
    const a = await enrolDevice(5612);
    const long = fact({
      workspace: "w".repeat(200),
      command: "c".repeat(400),
      port: 65535,
    });
    const many = Array.from(
      {
        length: 199,
      },
      (_, n) =>
        fact({
          command: `cmd ${n}`,
        }),
    );
    const facts = [
      long,
      ...many,
    ];
    expect(
      (
        await call(a, "PUT", path, {
          facts,
        })
      ).status,
    ).toBe(204);
    expect(await (await call(a, "GET", path)).json()).toEqual({
      facts,
    });
  });

  it("refuse a key that is a name and not a hash", async () => {
    const a = await enrolDevice(5613);
    expect(
      (
        await call(a, "PUT", "/v1/runs/armada", {
          facts: [
            fact(),
          ],
        })
      ).status,
    ).toBe(400);
    expect((await call(a, "GET", "/v1/runs/armada")).status).toBe(400);
  });

  it("refuse a method the path does not take", async () => {
    const a = await enrolDevice(5614);
    expect((await call(a, "DELETE", path)).status).toBe(405);
  });

  it("are refused unsigned, and from a device that never enrolled", async () => {
    const a = await enrolDevice(5615);
    await call(a, "PUT", path, {
      facts: [
        fact(),
      ],
    });
    const bare = await toStore(`${STORE}${path}`);
    expect(bare.status).toBe(401);
    expect(await bare.json()).toEqual({
      error: "unsigned",
    });
    const stranger = {
      ...a,
      id: `5615.${"e".repeat(32)}`,
      key: (await newKeyPair()).key,
    };
    const answer = await call(stranger, "PUT", path, {
      facts: [
        fact(),
      ],
    });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "unknown_device",
    });
  });
});

describe("a device", () => {
  it("stops being accepted once signed out", async () => {
    const [a, b] = [
      await enrolDevice(5301),
      await enrolDevice(5301),
    ];
    expect((await call(a, "DELETE", `/v1/devices/${b.id}`)).status).toBe(204);
    const after = await call(b, "GET", `/v1/claims/${hash("a")}/red/${hash("b")}`);
    expect(after.status).toBe(401);
    expect(await after.json()).toEqual({
      error: "unknown_device",
    });
  });
});

describe("a request", () => {
  const target = `/v1/claims/${hash("a")}/red/${hash("b")}`;

  it("signed by a key that is not the device's is refused", async () => {
    const a = await enrolDevice(5401);
    const stranger = await newKeyPair();
    const answer = await call(a, "GET", target, undefined, {
      key: stranger.key,
    });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "bad_signature",
    });
  });

  it("whose body changed after signing is refused", async () => {
    const a = await enrolDevice(5402);
    const body = {
      repo: hash("a"),
      kind: "red",
      key: hash("b"),
      holder: claimant("j-1"),
    };
    const answer = await call(a, "POST", "/v1/claims", body, {
      signedBody: JSON.stringify({
        ...body,
        holder: claimant("j-2"),
      }),
    });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "bad_signature",
    });
    expect((await call(a, "GET", target)).status).toBe(404);
  });

  it("whose path changed after signing is refused", async () => {
    const a = await enrolDevice(5403);
    const headers = await signRequest(a.key, a.id, "DELETE", `/v1/claims/${hash("a")}/red`);
    const answer = await toStore(`${STORE}/v1/claims/${hash("d")}/red`, {
      method: "DELETE",
      headers,
    });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "bad_signature",
    });
  });

  it("older than the skew is refused, and so is one from the future", async () => {
    const a = await enrolDevice(5404);
    for (const time of [
      now() - 120,
      now() + 120,
    ]) {
      const answer = await call(a, "GET", target, undefined, {
        time: String(time),
      });
      expect(answer.status).toBe(401);
      expect(await answer.json()).toEqual({
        error: "stale_time",
      });
    }
  });

  it("inside the skew is accepted", async () => {
    const a = await enrolDevice(5405);
    for (const time of [
      now() - 30,
      now() + 30,
    ]) {
      expect(
        (
          await call(a, "GET", target, undefined, {
            time: String(time),
          })
        ).status,
      ).toBe(404);
    }
  });

  it("replayed with the same nonce is refused", async () => {
    const a = await enrolDevice(5406);
    const headers = await signRequest(a.key, a.id, "GET", target);
    const send = () =>
      toStore(`${STORE}${target}`, {
        headers,
      });

    expect((await send()).status).toBe(404);
    const replay = await send();
    expect(replay.status).toBe(401);
    expect(await replay.json()).toEqual({
      error: "replayed_nonce",
    });
  });

  it("replayed twice at once lets one through", async () => {
    const a = await enrolDevice(5407);
    const headers = await signRequest(a.key, a.id, "GET", target);
    const answers = await Promise.all(
      [
        1,
        2,
        3,
        4,
      ].map(() =>
        toStore(`${STORE}${target}`, {
          headers,
        }),
      ),
    );
    expect(answers.map((answer) => answer.status).sort()).toEqual([
      401,
      401,
      401,
      404,
    ]);
  });

  it("a nonce is the device's own, so another device may use the same text", async () => {
    const [a, b] = [
      await enrolDevice(5408),
      await enrolDevice(5408),
    ];
    const nonce = "0123456789abcdef0123456789abcdef";
    expect(
      (
        await call(a, "GET", target, undefined, {
          nonce,
        })
      ).status,
    ).toBe(404);
    expect(
      (
        await call(b, "GET", target, undefined, {
          nonce,
        })
      ).status,
    ).toBe(404);
  });

  it("with no signature is refused", async () => {
    const a = await enrolDevice(5409);
    const answer = await call(a, "GET", target, undefined, {
      headers: {
        "X-Armada-Signature": "",
      },
    });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "unsigned",
    });
    expect((await toStore(`${STORE}${target}`)).status).toBe(401);
  });

  it("from a device that never enrolled is refused", async () => {
    const stranger = {
      id: `5410.${"0".repeat(32)}`,
      key: (await newKeyPair()).key,
      account: 5410,
      token: "",
    };
    const answer = await call(stranger, "GET", target);
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({
      error: "unknown_device",
    });
  });
});

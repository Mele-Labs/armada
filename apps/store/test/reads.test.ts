import { runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { accountObject, call, claim, claimant, enrolDevice, hash, holder, renew } from "./support";

// The free plan allows 5,000,000 rows read a day, so what one call reads must not grow with what the account holds.
// `docs/contracts/central-store.md`, *Cost*.

const keyOf = (n: number) => n.toString(16).padStart(64, "0");

type Counted = {
  sql: SqlStorage;
  cursors?: SqlStorageCursor<Record<string, SqlStorageValue>>[];
  forget?: () => void;
};

/** Puts `count` file claims of `j-0` into the account, each with `j-1` waiting on it, as a busy account holds. */
async function busy(account: number, count: number) {
  await runInDurableObject(accountObject(account), (instance, state) => {
    const sql = state.storage.sql;
    const now = Math.floor(Date.now() / 1000);
    sql.exec(
      "INSERT OR IGNORE INTO holders (kind, id, machine, name, title, renewed) VALUES ('job', 'j-0', 'studio', '', '', ?)",
      now,
    );
    for (let n = 0; n < count; n++) {
      sql.exec(
        "INSERT INTO claims (repo, kind, key, holder_kind, holder_id, machine, since) VALUES (?, 'file', ?, 'job', 'j-0', 'studio', 1)",
        keyOf(n),
        keyOf(n),
      );
      sql.exec(
        "INSERT INTO contenders (repo, kind, key, contender_kind, contender_id, machine, name, title, since, at) VALUES (?, 'file', ?, 'job', 'j-1', 'studio', '', '', 1, ?)",
        keyOf(n),
        keyOf(n),
        now,
      );
    }
    (instance as unknown as Counted).forget?.();
  });
}

/** From now on, keeps every cursor the account's SQL opens, so the rows they read can be summed. */
async function startCounting(account: number) {
  await runInDurableObject(accountObject(account), (instance) => {
    const counted = instance as unknown as Counted;
    const inner = counted.sql;
    const cursors: NonNullable<Counted["cursors"]> = [];
    counted.cursors = cursors;
    counted.sql = new Proxy(inner, {
      get(target, property) {
        if (property !== "exec") {
          const value = Reflect.get(target, property, target);
          return typeof value === "function" ? value.bind(target) : value;
        }
        return (query: string, ...bindings: SqlStorageValue[]) => {
          const cursor = target.exec(query, ...bindings);
          cursors.push(cursor);
          return cursor;
        };
      },
    });
  });
}

/** The rows written since `startCounting`. */
const rowsWritten = (account: number): Promise<number> =>
  runInDurableObject(accountObject(account), (instance) =>
    ((instance as unknown as Counted).cursors ?? []).reduce(
      (sum, cursor) => sum + cursor.rowsWritten,
      0,
    ),
  );

/** The rows read since `startCounting`. */
const rowsRead = (account: number): Promise<number> =>
  runInDurableObject(accountObject(account), (instance) =>
    ((instance as unknown as Counted).cursors ?? []).reduce(
      (sum, cursor) => sum + cursor.rowsRead,
      0,
    ),
  );

describe("a busy account", () => {
  it("reads a handful of rows for each claim and renewal, however many claims it holds", async () => {
    const account = 9701;
    const device = await enrolDevice(account);
    await busy(account, 2000);
    // The fill wrote behind the object's back, so the first call lapses and reads the holders and contenders once.
    expect((await claim(device, hash("a"), "file", hash("b"), claimant("j-2"))).status).toBe(200);

    await startCounting(account);
    const calls = 20;
    for (let n = 0; n < calls; n++) {
      const answer = await claim(device, keyOf(n), "file", keyOf(n), claimant("j-3"));
      expect(answer.status).toBe(409);
    }
    for (let n = 0; n < 5; n++) expect((await renew(device, holder("j-2"))).status).toBe(200);

    const read = await rowsRead(account);
    expect(read / (calls + 5)).toBeLessThan(10);
  });

  it("reads a handful of rows on the first call after the object wakes", async () => {
    const account = 9702;
    const device = await enrolDevice(account);
    await busy(account, 2000);
    expect((await claim(device, hash("a"), "file", hash("b"), claimant("j-2"))).status).toBe(200);
    await runInDurableObject(accountObject(account), (instance) => {
      // What a woken object has: its tables, and nothing it kept in memory.
      const woken = instance as unknown as {
        upgraded: boolean;
        lapseFloor: number | undefined;
        recent: Map<string, number[]>;
      };
      woken.upgraded = false;
      woken.lapseFloor = undefined;
      woken.recent.clear();
    });

    await startCounting(account);
    expect((await claim(device, keyOf(1), "file", keyOf(1), claimant("j-3"))).status).toBe(409);
    expect((await claim(device, hash("c"), "file", hash("d"), claimant("j-3"))).status).toBe(200);

    expect(await rowsRead(account)).toBeLessThan(200);
  });
});

// The free plan allows 100,000 rows written a day, and every index a row is in counts again.
describe("a signed request", () => {
  it("writes one nonce row, and its renewal one row more", async () => {
    const account = 9704;
    const device = await enrolDevice(account);
    expect((await claim(device, hash("a"), "file", hash("b"), claimant("j-1"))).status).toBe(200);

    await startCounting(account);
    const calls = 10;
    for (let n = 0; n < calls; n++) expect((await renew(device, holder("j-1"))).status).toBe(200);

    expect(await rowsWritten(account)).toBe(calls * 2);
  });

  it("is refused when sent again, after the object wakes too", async () => {
    const account = 9705;
    const device = await enrolDevice(account);
    const body = {
      holder: {
        kind: "job",
        id: "j-1",
        machine: "studio",
      },
    };
    const sent = {
      nonce: "replayed-nonce-0001",
      time: String(Math.floor(Date.now() / 1000)),
    };
    expect((await call(device, "POST", "/v1/renewals", body, sent)).status).toBe(200);
    await runInDurableObject(accountObject(account), (instance) => {
      (
        instance as unknown as {
          recent: Map<string, number[]>;
        }
      ).recent.clear();
    });
    const again = await call(device, "POST", "/v1/renewals", body, sent);
    expect(again.status).toBe(401);
    expect(await again.json()).toEqual({
      error: "replayed_nonce",
    });
  });
});

describe("an account deployed before tallies", () => {
  it("counts each table once on its first verified request, then keeps the count, and keeps nonces without an index", async () => {
    const account = 9703;
    const device = await enrolDevice(account);
    await runInDurableObject(accountObject(account), (instance, state) => {
      const sql = state.storage.sql;
      for (const table of [
        "devices",
        "claims",
        "owners",
        "runs",
        "contenders",
      ])
        sql.exec(`DROP TRIGGER ${table}_added; DROP TRIGGER ${table}_removed;`);
      sql.exec(`
        DROP TABLE tallies;
        DROP TABLE nonces;
        CREATE TABLE nonces (device TEXT NOT NULL, nonce TEXT NOT NULL, seen INTEGER NOT NULL, PRIMARY KEY (device, nonce));
        CREATE INDEX nonces_seen ON nonces (seen);
      `);
      sql.exec(
        "INSERT INTO nonces (device, nonce, seen) VALUES (?, 'taken-before-the-copy', ?)",
        device.id,
        Math.floor(Date.now() / 1000),
      );
      for (let n = 0; n < 3; n++)
        sql.exec(
          "INSERT INTO claims (repo, kind, key, holder_kind, holder_id, machine, since) VALUES (?, 'file', ?, 'job', 'j-0', 'studio', 1)",
          keyOf(n),
          keyOf(n),
        );
      (
        instance as unknown as {
          upgraded: boolean;
        }
      ).upgraded = false;
    });

    expect((await claim(device, hash("a"), "file", hash("b"), claimant("j-1"))).status).toBe(200);
    expect((await call(device, "DELETE", `/v1/claims/${keyOf(0)}/file/${keyOf(0)}`)).status).toBe(
      204,
    );

    await runInDurableObject(accountObject(account), (_instance, state) => {
      const sql = state.storage.sql;
      const tallies = Object.fromEntries(
        sql
          .exec<{
            name: string;
            n: number;
          }>("SELECT name, n FROM tallies")
          .toArray()
          .map((row) => [
            row.name,
            row.n,
          ]),
      );
      expect(tallies).toEqual({
        devices: 1,
        claims: 3,
        owners: 0,
        runs: 0,
        contenders: 0,
      });
      const indexes = sql
        .exec<{
          name: string;
        }>("SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'nonces'")
        .toArray()
        .map((row) => row.name);
      expect(indexes).toEqual([]);
      const nonces = sql
        .exec<{
          sql: string;
        }>("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'nonces'")
        .one().sql;
      expect(nonces).toContain("WITHOUT ROWID");
      // The nonces taken before the copy were carried into it.
      expect(sql.exec("SELECT COUNT(*) AS n FROM nonces").one().n).toBe(3);
    });
  });
});

import { DurableObject } from "cloudflare:workers";
import {
  DEVICE_HEADER,
  NONCE_HEADER,
  NONCE_KEEP_SECONDS,
  SIGNATURE_HEADER,
  SKEW_SECONDS,
  TIME_HEADER,
  bodyHash,
  fromBase64,
  fromHex,
  importPublicKey,
  signedText,
  verifySignature,
} from "./signing";
import {
  HASH,
  LAPSE_SECONDS,
  MAX_BODY_BYTES,
  MAX_RUNS,
  MAX_RUNS_BODY_BYTES,
  MAX_CLAIMS,
  MAX_CONTENDERS,
  MAX_KEYS,
  MAX_OWNERS,
  RATE_WINDOW_SECONDS,
  REQUESTS_PER_MINUTE,
  claimHolderOf,
  claimKindOf,
  exactly,
  factsOf,
  holderOf,
  parseJson,
  renewingOf,
  type ClaimHolder,
  type ClaimKind,
  type Holder,
  type Renewing,
} from "./rows";

const NONCE = /^[A-Za-z0-9_-]{16,64}$/;
const TIME = /^[0-9]{1,12}$/;

const reply = (status: number, body?: unknown): Response =>
  body === undefined
    ? new Response(null, {
        status,
      })
    : Response.json(body, {
        status,
      });
const refuse = (status: number, error: string): Response =>
  reply(status, {
    error,
  });

type HolderRow = {
  kind: Holder["kind"];
  id: string;
  machine: string;
  since: number;
};
const holderFrom = (row: HolderRow): Holder => ({
  kind: row.kind,
  id: row.id,
  machine: row.machine,
  since: row.since,
});

type ClaimRow = HolderRow & {
  name: string;
  title: string;
  renewed: number;
};
/** A claim as it is answered: who holds it, and when its holder last renewed. */
const claimFrom = (row: ClaimRow) => ({
  holder: {
    ...holderFrom(row),
    name: row.name,
    title: row.title,
  },
  renewed: row.renewed,
});

const nowSeconds = (): number => Math.floor(Date.now() / 1000);

/** One account's devices, claims, owners and the runs of its repositories. A Durable Object runs one request at a time. */
export class Account extends DurableObject<Env> {
  private readonly sql = this.ctx.storage.sql;

  /** Whether the tables exist. They are made by the first enrolment and by nothing a stranger can send. */
  private made = false;

  /** Whether the tables that came after enrolment, `runs` and the claims of every kind, are made. An account enrolled
   * before them has them from its first verified request. */
  private upgraded = false;

  /** Called by the Worker's enrol, which has already checked the key parses. `full` when the account has `limit` devices. */
  register(device: string, publicKey: string, machine: string, limit: number): "ok" | "full" {
    this.makeTables();
    if (this.count("devices") >= limit) return "full";
    this.sql.exec(
      "INSERT INTO devices (id, public_key, machine) VALUES (?, ?, ?)",
      device,
      publicKey,
      machine,
    );
    return "ok";
  }

  private makeTables(): void {
    this.sql.exec(`
      CREATE TABLE IF NOT EXISTS devices (id TEXT PRIMARY KEY, public_key TEXT NOT NULL, machine TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS nonces (device TEXT NOT NULL, nonce TEXT NOT NULL, seen INTEGER NOT NULL, PRIMARY KEY (device, nonce));
      CREATE INDEX IF NOT EXISTS nonces_seen ON nonces (seen);
      CREATE TABLE IF NOT EXISTS owners (key TEXT PRIMARY KEY, kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL);
    `);
    this.upgrade();
    this.made = true;
  }

  /**
   * Makes the tables that came after the first. **The claims table was keyed `(repo, red)`** and held red main only,
   * with no name, title or renewal. Its rows are dropped with it: no Job renews them, so the lapse would drop them
   * anyway, and the Mac that held one claims it again under the new key.
   *
   * A renewal writes one `holders` row, however many claims the Job holds, because the free plan counts rows written.
   * `contenders` holds the Jobs a held claim refused, until the claim's holder is told or the claim ends.
   */
  private upgrade(): void {
    const claims = this.sql
      .exec<{
        sql: string;
      }>("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'claims'")
      .toArray()[0];
    if (claims && !claims.sql.includes("holder_kind")) this.sql.exec("DROP TABLE claims");
    this.sql.exec(`
      CREATE TABLE IF NOT EXISTS claims (repo TEXT NOT NULL, kind TEXT NOT NULL, key TEXT NOT NULL, holder_kind TEXT NOT NULL, holder_id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL, PRIMARY KEY (repo, kind, key));
      CREATE INDEX IF NOT EXISTS claims_holder ON claims (holder_kind, holder_id, machine);
      CREATE TABLE IF NOT EXISTS holders (kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, name TEXT NOT NULL, title TEXT NOT NULL, renewed INTEGER NOT NULL, PRIMARY KEY (kind, id, machine));
      CREATE TABLE IF NOT EXISTS runs (repo TEXT PRIMARY KEY, facts TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS contenders (repo TEXT NOT NULL, kind TEXT NOT NULL, key TEXT NOT NULL, contender_kind TEXT NOT NULL, contender_id TEXT NOT NULL, machine TEXT NOT NULL, name TEXT NOT NULL, title TEXT NOT NULL, since INTEGER NOT NULL, at INTEGER NOT NULL, PRIMARY KEY (repo, kind, key, contender_kind, contender_id, machine));
    `);
    this.upgraded = true;
  }

  private hasTables(): boolean {
    this.made ||=
      this.sql
        .exec("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'devices'")
        .toArray().length > 0;
    return this.made;
  }

  private count(table: "devices" | "claims" | "owners" | "runs" | "contenders"): number {
    return this.sql
      .exec<{
        n: number;
      }>(`SELECT COUNT(*) AS n FROM ${table}`)
      .one().n;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const body = new Uint8Array(await request.arrayBuffer());
    const most = url.pathname.startsWith("/v1/runs/") ? MAX_RUNS_BODY_BYTES : MAX_BODY_BYTES;
    if (body.length > most) return refuse(413, "too_large");
    const refused = await this.authenticate(request, url.pathname + url.search, body);
    if (refused) return refused;
    // Only a verified request gets here, so a stranger still cannot make a table.
    if (!this.upgraded) this.upgrade();
    // Nothing in `route` awaits, so a read and the write after it cannot be split by another request.
    const answer = this.route(request.method, url, new TextDecoder().decode(body));
    if (url.pathname.startsWith("/v1/claims") || url.pathname === "/v1/renewals")
      await this.keepAlarm();
    return answer;
  }

  /** Drops every claim of each holder that has not renewed for `LAPSE_SECONDS`, its contenders, and the holder. */
  private lapse(now: number): void {
    const cutoff = now - LAPSE_SECONDS;
    this.sql.exec(
      `DELETE FROM contenders WHERE (repo, kind, key) IN (
         SELECT repo, kind, key FROM claims WHERE (holder_kind, holder_id, machine) IN (SELECT kind, id, machine FROM holders WHERE renewed <= ?))`,
      cutoff,
    );
    this.sql.exec(
      "DELETE FROM claims WHERE (holder_kind, holder_id, machine) IN (SELECT kind, id, machine FROM holders WHERE renewed <= ?)",
      cutoff,
    );
    this.sql.exec("DELETE FROM holders WHERE renewed <= ?", cutoff);
  }

  /** When the holder that renewed longest ago lapses, or `null` with no holder. */
  private nextLapse(): number | null {
    const at = this.sql
      .exec<{
        at: number | null;
      }>("SELECT MIN(renewed) AS at FROM holders")
      .one().at;
    return at === null ? null : at + LAPSE_SECONDS;
  }

  /** One alarm at a time, set for the next lapse. A renewal does not move it; it fires early and sets the next. */
  private async keepAlarm(): Promise<void> {
    if ((await this.ctx.storage.getAlarm()) !== null) return;
    const next = this.nextLapse();
    if (next !== null) await this.ctx.storage.setAlarm(next * 1000);
  }

  /** The lapse, for a store nobody calls. A holder left holding nothing goes too. */
  async alarm(): Promise<void> {
    if (!this.hasTables()) return;
    if (!this.upgraded) this.upgrade();
    this.lapse(nowSeconds());
    this.sql.exec(
      "DELETE FROM holders WHERE NOT EXISTS (SELECT 1 FROM claims WHERE holder_kind = holders.kind AND holder_id = holders.id AND claims.machine = holders.machine)",
    );
    const next = this.nextLapse();
    if (next !== null) await this.ctx.storage.setAlarm(next * 1000);
  }

  private async authenticate(
    request: Request,
    target: string,
    body: Uint8Array,
  ): Promise<Response | null> {
    const device = request.headers.get(DEVICE_HEADER);
    const time = request.headers.get(TIME_HEADER);
    const nonce = request.headers.get(NONCE_HEADER);
    const sent = request.headers.get(SIGNATURE_HEADER);
    const signature = sent ? fromHex(sent) : null;
    if (!device || !time || !nonce || !signature || !TIME.test(time) || !NONCE.test(nonce)) {
      return refuse(401, "unsigned");
    }
    // An account nobody enrolled has no tables, and a stranger's request must not make them.
    if (!this.hasTables()) return refuse(401, "unknown_device");
    const row = this.sql
      .exec<{
        public_key: string;
      }>("SELECT public_key FROM devices WHERE id = ?", device)
      .toArray()[0];
    if (!row) return refuse(401, "unknown_device");

    const now = Math.floor(Date.now() / 1000);
    if (Math.abs(now - Number(time)) > SKEW_SECONDS) return refuse(401, "stale_time");

    const key = await importPublicKey(fromBase64(row.public_key) ?? new Uint8Array());
    const text = signedText(request.method, target, time, nonce, await bodyHash(body));
    if (!(await verifySignature(key, text, signature))) return refuse(401, "bad_signature");

    // Only a request that verified takes a nonce or a place in the minute, so garbage cannot fill the table
    // or use up a device's requests. The nonces of the last minute are the count.
    this.sql.exec("DELETE FROM nonces WHERE seen < ?", now - NONCE_KEEP_SECONDS);
    const recent = this.sql
      .exec<{
        n: number;
      }>(
        "SELECT COUNT(*) AS n FROM nonces WHERE device = ? AND seen > ?",
        device,
        now - RATE_WINDOW_SECONDS,
      )
      .one().n;
    if (recent >= REQUESTS_PER_MINUTE) {
      return Response.json(
        {
          error: "rate_limited",
        },
        {
          status: 429,
          headers: {
            "Retry-After": String(RATE_WINDOW_SECONDS),
          },
        },
      );
    }
    const taken = this.sql.exec(
      "INSERT OR IGNORE INTO nonces (device, nonce, seen) VALUES (?, ?, ?)",
      device,
      nonce,
      now,
    );
    return taken.rowsWritten === 0 ? refuse(401, "replayed_nonce") : null;
  }

  private route(method: string, url: URL, text: string): Response {
    const parts = url.pathname.split("/").slice(1);
    const [v1, area, first, second, third] = parts;
    if (v1 !== "v1" || parts.length > 5) return refuse(404, "no_route");

    if (area === "devices" && parts.length === 2) {
      return method === "GET" ? this.listDevices() : refuse(405, "bad_method");
    }
    if (area === "devices" && parts.length === 3) {
      return method === "DELETE" ? this.signOut(first!) : refuse(405, "bad_method");
    }
    if (area === "renewals" && parts.length === 2) {
      return method === "POST" ? this.renew(text) : refuse(405, "bad_method");
    }
    if (area === "claims") {
      // A claim that lapsed reads as gone before its alarm has run.
      this.lapse(nowSeconds());
      if (parts.length === 2)
        return method === "POST" ? this.claim(text) : refuse(405, "bad_method");
      const kind = claimKindOf(second);
      if (!HASH.test(first ?? "") || parts.length === 3 || !kind) return refuse(400, "invalid_key");
      if (third === undefined)
        return method === "DELETE" ? this.clearKind(first!, kind) : refuse(405, "bad_method");
      if (!HASH.test(third)) return refuse(400, "invalid_key");
      if (method === "GET") return this.readClaim(first!, kind, third);
      return method === "DELETE" ? this.giveBack(first!, kind, third) : refuse(405, "bad_method");
    }
    if (area === "owners") {
      if (parts.length === 2)
        return method === "GET" ? this.readOwners(url) : refuse(405, "bad_method");
      if (!HASH.test(first ?? "") || parts.length !== 3) return refuse(400, "invalid_key");
      if (method === "PUT") return this.putOwner(first!, text);
      return method === "DELETE" ? this.deleteOwner(first!) : refuse(405, "bad_method");
    }
    if (area === "runs") {
      if (!HASH.test(first ?? "") || parts.length !== 3) return refuse(400, "invalid_key");
      if (method === "PUT") return this.putRuns(first!, text);
      return method === "GET" ? this.readRuns(first!) : refuse(405, "bad_method");
    }
    return refuse(404, "no_route");
  }

  /** Every device of this object and the machine it named, never its key. The table has no enrolment time, so its rowid, which a new row always takes past the last, is the order they enrolled in. */
  private listDevices(): Response {
    const devices = this.sql
      .exec<{
        device: string;
        machine: string;
      }>("SELECT id AS device, machine FROM devices ORDER BY rowid")
      .toArray();
    return reply(200, {
      devices,
    });
  }

  private signOut(device: string): Response {
    this.sql.exec("DELETE FROM nonces WHERE device = ?", device);
    const gone = this.sql.exec("DELETE FROM devices WHERE id = ?", device);
    return gone.rowsWritten === 0 ? refuse(404, "no_device") : reply(204);
  }

  /** Claim if free: one INSERT that does nothing when the key is held. A win renews its holder. */
  private claim(text: string): Response {
    const body = exactly(parseJson(text), [
      "repo",
      "kind",
      "key",
      "holder",
    ]);
    const holder = claimHolderOf(body?.holder);
    const kind = claimKindOf(body?.kind);
    if (!body || !holder || !kind || !HASH.test(String(body.repo)) || !HASH.test(String(body.key)))
      return refuse(400, "invalid_body");
    const repo = String(body.repo);
    const key = String(body.key);
    if (this.count("claims") >= MAX_CLAIMS && !this.claimed(repo, kind, key))
      return refuse(413, "claims_full");
    const won = this.sql.exec(
      "INSERT OR IGNORE INTO claims (repo, kind, key, holder_kind, holder_id, machine, since) VALUES (?, ?, ?, ?, ?, ?, ?)",
      repo,
      kind,
      key,
      holder.kind,
      holder.id,
      holder.machine,
      holder.since,
    );
    if (won.rowsWritten > 0) this.putHolder(holder);
    const held = this.claimed(repo, kind, key);
    if (won.rowsWritten === 0 && held) this.putContender(repo, kind, key, held.holder, holder);
    return reply(won.rowsWritten > 0 ? 200 : 409, held);
  }

  /** A claim refused is kept for its holder's next renewal, unless the asker is the holder or the table is full. */
  private putContender(
    repo: string,
    kind: ClaimKind,
    key: string,
    held: Holder,
    asker: ClaimHolder,
  ): void {
    if (held.kind === asker.kind && held.id === asker.id && held.machine === asker.machine) return;
    const known =
      this.sql
        .exec(
          "SELECT 1 FROM contenders WHERE repo = ? AND kind = ? AND key = ? AND contender_kind = ? AND contender_id = ? AND machine = ?",
          repo,
          kind,
          key,
          asker.kind,
          asker.id,
          asker.machine,
        )
        .toArray().length > 0;
    if (!known && this.count("contenders") >= MAX_CONTENDERS) return;
    this.sql.exec(
      `INSERT INTO contenders (repo, kind, key, contender_kind, contender_id, machine, name, title, since, at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
       ON CONFLICT (repo, kind, key, contender_kind, contender_id, machine) DO UPDATE SET name = excluded.name, title = excluded.title, at = excluded.at`,
      repo,
      kind,
      key,
      asker.kind,
      asker.id,
      asker.machine,
      asker.name,
      asker.title,
      asker.since,
      nowSeconds(),
    );
  }

  /** The holder's row: the name and title it last sent, renewed now. */
  private putHolder(holder: ClaimHolder): void {
    this.sql.exec(
      `INSERT INTO holders (kind, id, machine, name, title, renewed) VALUES (?, ?, ?, ?, ?, ?)
       ON CONFLICT (kind, id, machine) DO UPDATE SET name = excluded.name, title = excluded.title, renewed = excluded.renewed`,
      holder.kind,
      holder.id,
      holder.machine,
      holder.name,
      holder.title,
      nowSeconds(),
    );
  }

  private claimed(repo: string, kind: ClaimKind, key: string): ReturnType<typeof claimFrom> | null {
    const row = this.sql
      .exec<ClaimRow>(
        `SELECT claims.holder_kind AS kind, claims.holder_id AS id, claims.machine, claims.since, holders.name, holders.title, holders.renewed
         FROM claims JOIN holders ON holders.kind = claims.holder_kind AND holders.id = claims.holder_id AND holders.machine = claims.machine
         WHERE claims.repo = ? AND claims.kind = ? AND claims.key = ?`,
        repo,
        kind,
        key,
      )
      .toArray()[0];
    return row ? claimFrom(row) : null;
  }

  private readClaim(repo: string, kind: ClaimKind, key: string): Response {
    const held = this.claimed(repo, kind, key);
    return held ? reply(200, held) : refuse(404, "no_claim");
  }

  /** Give back and take over are the same delete; a take over then claims. */
  private giveBack(repo: string, kind: ClaimKind, key: string): Response {
    this.sql.exec(
      "DELETE FROM contenders WHERE repo = ? AND kind = ? AND key = ?",
      repo,
      kind,
      key,
    );
    this.sql.exec("DELETE FROM claims WHERE repo = ? AND kind = ? AND key = ?", repo, kind, key);
    return reply(204);
  }

  /** Every claim of one kind under a repository: main went green, and its reds end. */
  private clearKind(repo: string, kind: ClaimKind): Response {
    this.sql.exec("DELETE FROM contenders WHERE repo = ? AND kind = ?", repo, kind);
    this.sql.exec("DELETE FROM claims WHERE repo = ? AND kind = ?", repo, kind);
    return reply(204);
  }

  /** One call renews every claim the holder has, by writing its one row. Answers how many claims it holds, so a
   * holder whose claims lapsed reads `0` and claims again, and who asked for them since the last renewal, once. */
  private renew(text: string): Response {
    const body = exactly(parseJson(text), [
      "holder",
    ]);
    const who = renewingOf(body?.holder);
    if (!who) return refuse(400, "invalid_body");
    this.lapse(nowSeconds());
    this.sql.exec(
      "UPDATE holders SET renewed = ? WHERE kind = ? AND id = ? AND machine = ?",
      nowSeconds(),
      who.kind,
      who.id,
      who.machine,
    );
    const claims = this.sql
      .exec<{
        n: number;
      }>(
        "SELECT COUNT(*) AS n FROM claims WHERE holder_kind = ? AND holder_id = ? AND machine = ?",
        who.kind,
        who.id,
        who.machine,
      )
      .one().n;
    return reply(200, {
      claims,
      contested: claims === 0 ? [] : this.takeContested(who),
    });
  }

  /** The contenders on the claims `who` holds, deleted as they are answered so each is told once. */
  private takeContested(who: Renewing) {
    const mine = `SELECT repo, kind, key FROM claims WHERE holder_kind = ? AND holder_id = ? AND machine = ?`;
    const rows = this.sql
      .exec<{
        repo: string;
        kind: ClaimKind;
        key: string;
        contender_kind: Holder["kind"];
        contender_id: string;
        machine: string;
        name: string;
        title: string;
        since: number;
      }>(
        `SELECT repo, kind, key, contender_kind, contender_id, machine, name, title, since FROM contenders
         WHERE (repo, kind, key) IN (${mine}) ORDER BY at, rowid`,
        who.kind,
        who.id,
        who.machine,
      )
      .toArray();
    if (rows.length > 0)
      this.sql.exec(
        `DELETE FROM contenders WHERE (repo, kind, key) IN (${mine})`,
        who.kind,
        who.id,
        who.machine,
      );
    return rows.map((row) => ({
      repo: row.repo,
      kind: row.kind,
      key: row.key,
      holder: {
        kind: row.contender_kind,
        id: row.contender_id,
        machine: row.machine,
        since: row.since,
        name: row.name,
        title: row.title,
      },
    }));
  }

  private putOwner(key: string, text: string): Response {
    const body = exactly(parseJson(text), [
      "holder",
    ]);
    const holder = holderOf(body?.holder);
    if (!holder) return refuse(400, "invalid_body");
    const known = this.sql.exec("SELECT 1 FROM owners WHERE key = ?", key).toArray().length > 0;
    if (!known && this.count("owners") >= MAX_OWNERS) return refuse(413, "owners_full");
    this.sql.exec(
      `INSERT INTO owners (key, kind, id, machine, since) VALUES (?, ?, ?, ?, ?)
       ON CONFLICT (key) DO UPDATE SET kind = excluded.kind, id = excluded.id, machine = excluded.machine, since = excluded.since`,
      key,
      holder.kind,
      holder.id,
      holder.machine,
      holder.since,
    );
    return reply(204);
  }

  private deleteOwner(key: string): Response {
    this.sql.exec("DELETE FROM owners WHERE key = ?", key);
    return reply(204);
  }

  private readOwners(url: URL): Response {
    const keys = (url.searchParams.get("keys") ?? "").split(",").filter((key) => key !== "");
    if (keys.length > MAX_KEYS || !keys.every((key) => HASH.test(key)))
      return refuse(400, "invalid_key");
    const found: Record<string, Holder> = {};
    for (const key of keys) {
      const row = this.sql
        .exec<HolderRow>("SELECT kind, id, machine, since FROM owners WHERE key = ?", key)
        .toArray()[0];
      if (row) found[key] = holderFrom(row);
    }
    return reply(200, found);
  }

  /** Replaces every fact held for the repository. A repository with none holds no row. */
  private putRuns(repo: string, text: string): Response {
    const body = exactly(parseJson(text), [
      "facts",
    ]);
    const facts = factsOf(body?.facts);
    if (!facts) return refuse(400, "invalid_body");
    if (facts.length === 0) {
      this.sql.exec("DELETE FROM runs WHERE repo = ?", repo);
      return reply(204);
    }
    const known = this.sql.exec("SELECT 1 FROM runs WHERE repo = ?", repo).toArray().length > 0;
    if (!known && this.count("runs") >= MAX_RUNS) return refuse(413, "runs_full");
    this.sql.exec(
      "INSERT INTO runs (repo, facts) VALUES (?, ?) ON CONFLICT (repo) DO UPDATE SET facts = excluded.facts",
      repo,
      JSON.stringify(facts),
    );
    return reply(204);
  }

  private readRuns(repo: string): Response {
    const row = this.sql
      .exec<{
        facts: string;
      }>("SELECT facts FROM runs WHERE repo = ?", repo)
      .toArray()[0];
    return row
      ? reply(200, {
          facts: JSON.parse(row.facts),
        })
      : refuse(404, "no_runs");
  }
}

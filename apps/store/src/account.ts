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
  MAX_BODY_BYTES,
  MAX_RUNS,
  MAX_RUNS_BODY_BYTES,
  MAX_CLAIMS,
  MAX_DEVICES,
  MAX_KEYS,
  MAX_OWNERS,
  RATE_WINDOW_SECONDS,
  REQUESTS_PER_MINUTE,
  exactly,
  factsOf,
  holderOf,
  parseJson,
  type Holder,
} from "./rows";

const NONCE = /^[A-Za-z0-9_-]{16,64}$/;
const TIME = /^[0-9]{1,12}$/;

const reply = (status: number, body?: unknown): Response =>
  body === undefined ? new Response(null, { status }) : Response.json(body, { status });
const refuse = (status: number, error: string): Response => reply(status, { error });

type HolderRow = { kind: Holder["kind"]; id: string; machine: string; since: number };
const holderFrom = (row: HolderRow): Holder => ({ kind: row.kind, id: row.id, machine: row.machine, since: row.since });

/** One account's devices, claims, owners and the runs of its repositories. A Durable Object runs one request at a time. */
export class Account extends DurableObject<Env> {
  private readonly sql = this.ctx.storage.sql;

  /** Whether the tables exist. They are made by the first enrolment and by nothing a stranger can send. */
  private made = false;

  /** Whether `runs` exists. It came after the others, so an account enrolled before it has it from its first verified request. */
  private runsMade = false;

  /** Called by the Worker's enrol, which has already checked the key parses. `full` when the account has its devices. */
  register(device: string, publicKey: string, machine: string): "ok" | "full" {
    this.makeTables();
    if (this.count("devices") >= MAX_DEVICES) return "full";
    this.sql.exec("INSERT INTO devices (id, public_key, machine) VALUES (?, ?, ?)", device, publicKey, machine);
    return "ok";
  }

  private makeTables(): void {
    this.sql.exec(`
      CREATE TABLE IF NOT EXISTS devices (id TEXT PRIMARY KEY, public_key TEXT NOT NULL, machine TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS nonces (device TEXT NOT NULL, nonce TEXT NOT NULL, seen INTEGER NOT NULL, PRIMARY KEY (device, nonce));
      CREATE INDEX IF NOT EXISTS nonces_seen ON nonces (seen);
      CREATE TABLE IF NOT EXISTS claims (repo TEXT NOT NULL, red TEXT NOT NULL, kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL, PRIMARY KEY (repo, red));
      CREATE TABLE IF NOT EXISTS owners (key TEXT PRIMARY KEY, kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL);
    `);
    this.makeRunsTable();
    this.made = true;
  }

  private makeRunsTable(): void {
    this.sql.exec("CREATE TABLE IF NOT EXISTS runs (repo TEXT PRIMARY KEY, facts TEXT NOT NULL)");
    this.runsMade = true;
  }

  private hasTables(): boolean {
    this.made ||= this.sql.exec("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'devices'").toArray().length > 0;
    return this.made;
  }

  private count(table: "devices" | "claims" | "owners" | "runs"): number {
    return this.sql.exec<{ n: number }>(`SELECT COUNT(*) AS n FROM ${table}`).one().n;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const body = new Uint8Array(await request.arrayBuffer());
    const most = url.pathname.startsWith("/v1/runs/") ? MAX_RUNS_BODY_BYTES : MAX_BODY_BYTES;
    if (body.length > most) return refuse(413, "too_large");
    const refused = await this.authenticate(request, url.pathname + url.search, body);
    if (refused) return refused;
    // Nothing below awaits, so a read and the write after it cannot be split by another request.
    return this.route(request.method, url, new TextDecoder().decode(body));
  }

  private async authenticate(request: Request, target: string, body: Uint8Array): Promise<Response | null> {
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
    const row = this.sql.exec<{ public_key: string }>("SELECT public_key FROM devices WHERE id = ?", device).toArray()[0];
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
      .exec<{ n: number }>("SELECT COUNT(*) AS n FROM nonces WHERE device = ? AND seen > ?", device, now - RATE_WINDOW_SECONDS)
      .one().n;
    if (recent >= REQUESTS_PER_MINUTE) {
      return Response.json({ error: "rate_limited" }, { status: 429, headers: { "Retry-After": String(RATE_WINDOW_SECONDS) } });
    }
    const taken = this.sql.exec("INSERT OR IGNORE INTO nonces (device, nonce, seen) VALUES (?, ?, ?)", device, nonce, now);
    return taken.rowsWritten === 0 ? refuse(401, "replayed_nonce") : null;
  }

  private route(method: string, url: URL, text: string): Response {
    const parts = url.pathname.split("/").slice(1);
    const [v1, area, first, second] = parts;
    if (v1 !== "v1" || parts.length > 4) return refuse(404, "no_route");

    if (area === "devices" && parts.length === 3) {
      return method === "DELETE" ? this.signOut(first!) : refuse(405, "bad_method");
    }
    if (area === "claims") {
      if (parts.length === 2) return method === "POST" ? this.claim(text) : refuse(405, "bad_method");
      if (!HASH.test(first ?? "") || (second !== undefined && !HASH.test(second))) return refuse(400, "invalid_key");
      if (second === undefined) return method === "DELETE" ? this.clearRepository(first!) : refuse(405, "bad_method");
      if (method === "GET") return this.readClaim(first!, second);
      return method === "DELETE" ? this.giveBack(first!, second) : refuse(405, "bad_method");
    }
    if (area === "owners") {
      if (parts.length === 2) return method === "GET" ? this.readOwners(url) : refuse(405, "bad_method");
      if (!HASH.test(first ?? "") || parts.length !== 3) return refuse(400, "invalid_key");
      if (method === "PUT") return this.putOwner(first!, text);
      return method === "DELETE" ? this.deleteOwner(first!) : refuse(405, "bad_method");
    }
    if (area === "runs") {
      if (!HASH.test(first ?? "") || parts.length !== 3) return refuse(400, "invalid_key");
      // Only a verified request gets here, so a stranger still cannot make a table.
      if (!this.runsMade) this.makeRunsTable();
      if (method === "PUT") return this.putRuns(first!, text);
      return method === "GET" ? this.readRuns(first!) : refuse(405, "bad_method");
    }
    return refuse(404, "no_route");
  }

  private signOut(device: string): Response {
    this.sql.exec("DELETE FROM nonces WHERE device = ?", device);
    const gone = this.sql.exec("DELETE FROM devices WHERE id = ?", device);
    return gone.rowsWritten === 0 ? refuse(404, "no_device") : reply(204);
  }

  /** Claim if free: one INSERT that does nothing when the red is held. */
  private claim(text: string): Response {
    const body = exactly(parseJson(text), ["repo", "red", "holder"]);
    const holder = holderOf(body?.holder);
    if (!body || !holder || !HASH.test(String(body.repo)) || !HASH.test(String(body.red))) return refuse(400, "invalid_body");
    const repo = String(body.repo);
    const red = String(body.red);
    if (this.count("claims") >= MAX_CLAIMS && !this.claimHolder(repo, red)) return refuse(413, "claims_full");
    const won = this.sql.exec(
      "INSERT OR IGNORE INTO claims (repo, red, kind, id, machine, since) VALUES (?, ?, ?, ?, ?, ?)",
      repo, red, holder.kind, holder.id, holder.machine, holder.since,
    );
    if (won.rowsWritten > 0) return reply(200, { holder });
    return reply(409, { holder: this.claimHolder(repo, red) });
  }

  private claimHolder(repo: string, red: string): Holder | null {
    const row = this.sql
      .exec<HolderRow>("SELECT kind, id, machine, since FROM claims WHERE repo = ? AND red = ?", repo, red)
      .toArray()[0];
    return row ? holderFrom(row) : null;
  }

  private readClaim(repo: string, red: string): Response {
    const holder = this.claimHolder(repo, red);
    return holder ? reply(200, { holder }) : refuse(404, "no_claim");
  }

  /** Give back and take over are the same delete; a take over then claims. */
  private giveBack(repo: string, red: string): Response {
    this.sql.exec("DELETE FROM claims WHERE repo = ? AND red = ?", repo, red);
    return reply(204);
  }

  private clearRepository(repo: string): Response {
    this.sql.exec("DELETE FROM claims WHERE repo = ?", repo);
    return reply(204);
  }

  private putOwner(key: string, text: string): Response {
    const body = exactly(parseJson(text), ["holder"]);
    const holder = holderOf(body?.holder);
    if (!holder) return refuse(400, "invalid_body");
    const known = this.sql.exec("SELECT 1 FROM owners WHERE key = ?", key).toArray().length > 0;
    if (!known && this.count("owners") >= MAX_OWNERS) return refuse(413, "owners_full");
    this.sql.exec(
      `INSERT INTO owners (key, kind, id, machine, since) VALUES (?, ?, ?, ?, ?)
       ON CONFLICT (key) DO UPDATE SET kind = excluded.kind, id = excluded.id, machine = excluded.machine, since = excluded.since`,
      key, holder.kind, holder.id, holder.machine, holder.since,
    );
    return reply(204);
  }

  private deleteOwner(key: string): Response {
    this.sql.exec("DELETE FROM owners WHERE key = ?", key);
    return reply(204);
  }

  private readOwners(url: URL): Response {
    const keys = (url.searchParams.get("keys") ?? "").split(",").filter((key) => key !== "");
    if (keys.length > MAX_KEYS || !keys.every((key) => HASH.test(key))) return refuse(400, "invalid_key");
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
    const body = exactly(parseJson(text), ["facts"]);
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
      repo, JSON.stringify(facts),
    );
    return reply(204);
  }

  private readRuns(repo: string): Response {
    const row = this.sql.exec<{ facts: string }>("SELECT facts FROM runs WHERE repo = ?", repo).toArray()[0];
    return row ? reply(200, { facts: JSON.parse(row.facts) }) : refuse(404, "no_runs");
  }
}

var __defProp = Object.defineProperty;
var __name = (target, value) => __defProp(target, "name", { value, configurable: true });

// src/account.ts
import { DurableObject } from "cloudflare:workers";

// src/signing.ts
var SKEW_SECONDS = 60;
var NONCE_KEEP_SECONDS = 120;
var DEVICE_HEADER = "X-Armada-Device";
var TIME_HEADER = "X-Armada-Time";
var NONCE_HEADER = "X-Armada-Nonce";
var SIGNATURE_HEADER = "X-Armada-Signature";
var ECDSA = {
  name: "ECDSA",
  hash: "SHA-256"
};
var CURVE = {
  name: "ECDSA",
  namedCurve: "P-256"
};
var hex = /* @__PURE__ */ __name((bytes) => Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join(""), "hex");
function fromHex(text) {
  if (text.length % 2 !== 0 || !/^[0-9a-f]*$/.test(text)) return null;
  const out = new Uint8Array(text.length / 2);
  for (let at = 0; at < out.length; at++) out[at] = parseInt(text.slice(at * 2, at * 2 + 2), 16);
  return out;
}
__name(fromHex, "fromHex");
function fromBase64(text) {
  try {
    return Uint8Array.from(atob(text), (char) => char.charCodeAt(0));
  } catch {
    return null;
  }
}
__name(fromBase64, "fromBase64");
async function bodyHash(body) {
  const bytes = typeof body === "string" ? new TextEncoder().encode(body) : body;
  return hex(await crypto.subtle.digest("SHA-256", bytes));
}
__name(bodyHash, "bodyHash");
function signedText(method, target, time, nonce, hash) {
  return `${method}${target}${time}${nonce}${hash}`;
}
__name(signedText, "signedText");
async function importPublicKey(spki) {
  return crypto.subtle.importKey("spki", spki, CURVE, false, [
    "verify"
  ]);
}
__name(importPublicKey, "importPublicKey");
async function verifySignature(key, text, signature) {
  if (signature.length !== 64) return false;
  return crypto.subtle.verify(ECDSA, key, signature, new TextEncoder().encode(text));
}
__name(verifySignature, "verifySignature");

// src/rows.ts
var HASH = /^[A-Za-z0-9_-]{32,128}$/;
var HOLDER_ID = /^[A-Za-z0-9._-]{1,64}$/;
var MACHINE = /^[^\p{Cc}]{1,64}$/u;
var MAX_KEYS = 100;
var MAX_BODY_BYTES = 4096;
var MAX_RUNS_BODY_BYTES = 131072;
var MAX_DEVICES = 16;
var MAX_TEAM_DEVICES = 256;
var MAX_CLAIMS = 1e4;
var MAX_CONTENDERS = 1e4;
var MAX_OWNERS = 1e4;
var MAX_RUNS = 1e3;
var MAX_FACTS = 200;
var REQUESTS_PER_MINUTE = 120;
var RATE_WINDOW_SECONDS = 60;
var LAPSE_SECONDS = 30 * 60;
var isObject = /* @__PURE__ */ __name((value) => typeof value === "object" && value !== null && !Array.isArray(value), "isObject");
function exactly(value, keys) {
  if (!isObject(value)) return null;
  const present = Object.keys(value);
  if (present.length !== keys.length || !keys.every((key) => present.includes(key))) return null;
  return value;
}
__name(exactly, "exactly");
function holderOf(value) {
  const row = exactly(value, [
    "kind",
    "id",
    "machine",
    "since"
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
    since
  };
}
__name(holderOf, "holderOf");
var DEVICE_KINDS = [
  "mac",
  "phone"
];
var deviceKindOf = /* @__PURE__ */ __name((value) => DEVICE_KINDS.includes(value) ? value : null, "deviceKindOf");
var ADDRESS = /^https:\/\/[A-Za-z0-9](?:[A-Za-z0-9.-]{0,198}[A-Za-z0-9])?(?::([0-9]{1,5}))?$/;
function addressOf(value) {
  if (value === "") return "";
  if (typeof value !== "string" || value.length > 200) return null;
  const port = ADDRESS.exec(value)?.[1];
  if (!ADDRESS.test(value) || port !== void 0 && (Number(port) < 1 || Number(port) > 65535))
    return null;
  return value;
}
__name(addressOf, "addressOf");
var CLAIM_KINDS = [
  "item",
  "file",
  "test",
  "red",
  "pull_request"
];
var claimKindOf = /* @__PURE__ */ __name((value) => CLAIM_KINDS.includes(value) ? value : null, "claimKindOf");
var NAME = /^[^\p{Cc}]{0,64}$/u;
var TITLE = /^[^\p{Cc}]{0,200}$/u;
function claimHolderOf(value) {
  const row = exactly(value, [
    "kind",
    "id",
    "machine",
    "since",
    "name",
    "title"
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
    title
  };
}
__name(claimHolderOf, "claimHolderOf");
function renewingOf(value) {
  const row = exactly(value, [
    "kind",
    "id",
    "machine"
  ]);
  if (!row) return null;
  const holder = holderOf({
    ...row,
    since: 0
  });
  return holder ? {
    kind: holder.kind,
    id: holder.id,
    machine: holder.machine
  } : null;
}
__name(renewingOf, "renewingOf");
var FACT_KINDS = [
  "tests",
  "lint",
  "build",
  "dev_server",
  "typecheck",
  "format"
];
var FACT_SOURCE = /^[A-Za-z0-9_-]{1,32}$/;
var CONTROL = /\p{Cc}/u;
var textOf = /* @__PURE__ */ __name((value, most) => typeof value === "string" && value.length >= 1 && value.length <= most && !CONTROL.test(value), "textOf");
var FACT_KEYS = [
  "workspace",
  "kind",
  "command",
  "succeeded",
  "last_ran",
  "source"
];
function factOf(value) {
  const withPort = isObject(value) && "port" in value;
  const row = exactly(
    value,
    withPort ? [
      ...FACT_KEYS,
      "port"
    ] : FACT_KEYS
  );
  if (!row) return null;
  const { workspace, kind, command, port, succeeded, last_ran, source } = row;
  if (!textOf(workspace, 200) || workspace.startsWith("/") || workspace.split("/").includes(".."))
    return null;
  if (!FACT_KINDS.includes(kind)) return null;
  if (!textOf(command, 400)) return null;
  if (withPort && (typeof port !== "number" || !Number.isInteger(port) || port < 1 || port > 65535))
    return null;
  if (typeof succeeded !== "boolean") return null;
  if (typeof last_ran !== "number" || !Number.isSafeInteger(last_ran) || last_ran < 0) return null;
  if (typeof source !== "string" || !FACT_SOURCE.test(source)) return null;
  const fact = {
    workspace,
    kind,
    command,
    succeeded,
    last_ran,
    source
  };
  return withPort ? {
    ...fact,
    port
  } : fact;
}
__name(factOf, "factOf");
function factsOf(value) {
  if (!Array.isArray(value) || value.length > MAX_FACTS) return null;
  const facts = [];
  const seen = /* @__PURE__ */ new Set();
  for (const item of value) {
    const fact = factOf(item);
    if (!fact) return null;
    const identity = JSON.stringify([
      fact.workspace,
      fact.kind,
      fact.command
    ]);
    if (seen.has(identity)) return null;
    seen.add(identity);
    facts.push(fact);
  }
  return facts;
}
__name(factsOf, "factsOf");
function parseJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return void 0;
  }
}
__name(parseJson, "parseJson");

// src/account.ts
var NONCE = /^[A-Za-z0-9_-]{16,64}$/;
var TIME = /^[0-9]{1,12}$/;
var reply = /* @__PURE__ */ __name((status, body) => body === void 0 ? new Response(null, {
  status
}) : Response.json(body, {
  status
}), "reply");
var refuse = /* @__PURE__ */ __name((status, error) => reply(status, {
  error
}), "refuse");
var holderFrom = /* @__PURE__ */ __name((row) => ({
  kind: row.kind,
  id: row.id,
  machine: row.machine,
  since: row.since
}), "holderFrom");
var claimFrom = /* @__PURE__ */ __name((row) => ({
  holder: {
    ...holderFrom(row),
    name: row.name,
    title: row.title
  },
  renewed: row.renewed
}), "claimFrom");
var nowSeconds = /* @__PURE__ */ __name(() => Math.floor(Date.now() / 1e3), "nowSeconds");
var NONCES_COLUMNS = "device TEXT NOT NULL, seen INTEGER NOT NULL, nonce TEXT NOT NULL, PRIMARY KEY (device, seen, nonce)";
var COUNTED = [
  "devices",
  "claims",
  "owners",
  "runs",
  "contenders"
];
var Account = class extends DurableObject {
  static {
    __name(this, "Account");
  }
  sql = this.ctx.storage.sql;
  /** Whether the tables exist. They are made by the first enrolment and by nothing a stranger can send. */
  made = false;
  /** Whether the tables that came after enrolment, `runs` and the claims of every kind, are made. An account enrolled
   * before them has them from its first verified request. */
  upgraded = false;
  /**
   * No holder or contender lapses before this time, in epoch seconds, so the lapse is skipped until then. A write here
   * only moves a renewal or an ask later, so it never passes the real time. `undefined` until the object reads its
   * alarm, which is set no later than the next lapse.
   */
  lapseFloor;
  /** When each device's verified requests of the last minute were seen, read from its nonces once. */
  recent = /* @__PURE__ */ new Map();
  /** Drops what is kept in memory about renewals and nonces, for a write made outside this class, as a test's. The next
   * claims call or renewal lapses. */
  forget() {
    this.lapseFloor = Number.NEGATIVE_INFINITY;
    this.recent.clear();
  }
  /** Called by the Worker's enrol, which has already checked the key parses. `full` when the account has `limit` devices. */
  register(device, publicKey, machine, limit, kind) {
    this.makeTables();
    if (this.count("devices") >= limit) return "full";
    this.sql.exec(
      "INSERT INTO devices (id, public_key, machine, kind) VALUES (?, ?, ?, ?)",
      device,
      publicKey,
      machine,
      kind
    );
    return "ok";
  }
  makeTables() {
    this.sql.exec(`
      CREATE TABLE IF NOT EXISTS devices (id TEXT PRIMARY KEY, public_key TEXT NOT NULL, machine TEXT NOT NULL, kind TEXT NOT NULL DEFAULT 'mac', address TEXT NOT NULL DEFAULT '');
      CREATE TABLE IF NOT EXISTS nonces (${NONCES_COLUMNS}) WITHOUT ROWID;
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
   * `contenders` holds the Jobs a held claim refused, for as long as each keeps asking and the claim stands. A table
   * made before it had `told` gets the column here, its rows told none. A `devices` table made before it had `kind` and
   * `address` gets them here, its rows Macs with no address.
   *
   * **Nonces are one row each, with no index beside them**: keyed by device, signed time and nonce, without a rowid, so
   * a request's cleanup and rate count read only its own device's rows of the last two minutes, and a nonce costs one
   * row written to keep and one to drop. A replay repeats the signed time, so it still meets its own row. A table made
   * before it, with a rowid and an index on the time, is copied into the new one here.
   *
   * `tallies` holds how many rows each capped table has, kept by a trigger on each insert and delete, so a cap check
   * reads one row. The free plan counts every row a statement reads, and a `COUNT(*)` reads the whole table. A table
   * with no tally is counted once, here.
   */
  upgrade() {
    const nonces = this.sql.exec("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'nonces'").toArray()[0];
    if (nonces && !nonces.sql.includes("WITHOUT ROWID"))
      this.sql.exec(`
        CREATE TABLE nonces_kept (${NONCES_COLUMNS}) WITHOUT ROWID;
        INSERT OR IGNORE INTO nonces_kept (device, seen, nonce) SELECT device, seen, nonce FROM nonces;
        DROP TABLE nonces;
        ALTER TABLE nonces_kept RENAME TO nonces;
      `);
    const devices = this.sql.exec("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'devices'").toArray()[0];
    if (devices && !devices.sql.includes("kind"))
      this.sql.exec("ALTER TABLE devices ADD COLUMN kind TEXT NOT NULL DEFAULT 'mac'");
    if (devices && !devices.sql.includes("address"))
      this.sql.exec("ALTER TABLE devices ADD COLUMN address TEXT NOT NULL DEFAULT ''");
    const claims = this.sql.exec("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'claims'").toArray()[0];
    if (claims && !claims.sql.includes("holder_kind")) {
      this.sql.exec("DROP TABLE claims");
      if (this.hasTable("tallies")) this.sql.exec("DELETE FROM tallies WHERE name = 'claims'");
    }
    const contenders = this.sql.exec("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'contenders'").toArray()[0];
    if (contenders && !contenders.sql.includes("told"))
      this.sql.exec("ALTER TABLE contenders ADD COLUMN told INTEGER NOT NULL DEFAULT 0");
    this.sql.exec(`
      CREATE TABLE IF NOT EXISTS claims (repo TEXT NOT NULL, kind TEXT NOT NULL, key TEXT NOT NULL, holder_kind TEXT NOT NULL, holder_id TEXT NOT NULL, machine TEXT NOT NULL, since INTEGER NOT NULL, PRIMARY KEY (repo, kind, key));
      CREATE INDEX IF NOT EXISTS claims_holder ON claims (holder_kind, holder_id, machine);
      CREATE TABLE IF NOT EXISTS holders (kind TEXT NOT NULL, id TEXT NOT NULL, machine TEXT NOT NULL, name TEXT NOT NULL, title TEXT NOT NULL, renewed INTEGER NOT NULL, PRIMARY KEY (kind, id, machine));
      CREATE TABLE IF NOT EXISTS runs (repo TEXT PRIMARY KEY, facts TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS contenders (repo TEXT NOT NULL, kind TEXT NOT NULL, key TEXT NOT NULL, contender_kind TEXT NOT NULL, contender_id TEXT NOT NULL, machine TEXT NOT NULL, name TEXT NOT NULL, title TEXT NOT NULL, since INTEGER NOT NULL, at INTEGER NOT NULL, told INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (repo, kind, key, contender_kind, contender_id, machine));
      CREATE TABLE IF NOT EXISTS tallies (name TEXT PRIMARY KEY, n INTEGER NOT NULL);
    `);
    const tallied = new Set(
      this.sql.exec("SELECT name FROM tallies").toArray().map((row) => row.name)
    );
    for (const table of COUNTED) {
      this.sql.exec(`
        CREATE TRIGGER IF NOT EXISTS ${table}_added AFTER INSERT ON ${table} BEGIN UPDATE tallies SET n = n + 1 WHERE name = '${table}'; END;
        CREATE TRIGGER IF NOT EXISTS ${table}_removed AFTER DELETE ON ${table} BEGIN UPDATE tallies SET n = n - 1 WHERE name = '${table}'; END;
      `);
      if (!tallied.has(table))
        this.sql.exec(`INSERT INTO tallies (name, n) SELECT ?, COUNT(*) FROM ${table}`, table);
    }
    this.upgraded = true;
  }
  hasTable(name) {
    return this.sql.exec("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?", name).toArray().length > 0;
  }
  hasTables() {
    this.made ||= this.sql.exec("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'devices'").toArray().length > 0;
    return this.made;
  }
  /** The rows `table` holds, from its tally. */
  count(table) {
    return this.sql.exec("SELECT n FROM tallies WHERE name = ?", table).one().n;
  }
  async fetch(request) {
    const url = new URL(request.url);
    const body = new Uint8Array(await request.arrayBuffer());
    const most = url.pathname.startsWith("/v1/runs/") ? MAX_RUNS_BODY_BYTES : MAX_BODY_BYTES;
    if (body.length > most) return refuse(413, "too_large");
    const refused = await this.authenticate(request, url.pathname + url.search, body);
    if (refused) return refused;
    if (!this.upgraded) this.upgrade();
    const lapsing = url.pathname.startsWith("/v1/claims") || url.pathname === "/v1/renewals";
    if (lapsing && this.lapseFloor === void 0) {
      const alarm = await this.ctx.storage.getAlarm();
      if (alarm !== null) this.lapseFloor = Math.floor(alarm / 1e3);
    }
    const answer = this.route(
      request.method,
      url,
      new TextDecoder().decode(body),
      request.headers.get(DEVICE_HEADER) ?? ""
    );
    if (lapsing) await this.keepAlarm();
    return answer;
  }
  /** Drops every claim of each holder that has not renewed for `LAPSE_SECONDS`, its contenders, and the holder, and
   * every contender that has not asked for as long. */
  lapse(now) {
    const cutoff = now - LAPSE_SECONDS;
    this.sql.exec("DELETE FROM contenders WHERE at <= ?", cutoff);
    this.sql.exec(
      `DELETE FROM contenders WHERE (repo, kind, key) IN (
         SELECT repo, kind, key FROM claims WHERE (holder_kind, holder_id, machine) IN (SELECT kind, id, machine FROM holders WHERE renewed <= ?))`,
      cutoff
    );
    this.sql.exec(
      "DELETE FROM claims WHERE (holder_kind, holder_id, machine) IN (SELECT kind, id, machine FROM holders WHERE renewed <= ?)",
      cutoff
    );
    this.sql.exec("DELETE FROM holders WHERE renewed <= ?", cutoff);
  }
  /**
   * The lapse, only once something can have lapsed. A lapse scans the holders and contenders, so it runs about once per
   * lapse period rather than on every call.
   */
  lapseIfDue(now) {
    if (this.lapseFloor !== void 0 && now < this.lapseFloor) return;
    this.lapse(now);
    this.lapseFloor = this.nextLapse() ?? Number.POSITIVE_INFINITY;
  }
  /** A holder renewed, or a contender asked, at `at`: it lapses no earlier than `LAPSE_SECONDS` on. */
  lapsesFrom(at) {
    if (this.lapseFloor !== void 0)
      this.lapseFloor = Math.min(this.lapseFloor, at + LAPSE_SECONDS);
  }
  /** When the holder that renewed, or the contender that asked, longest ago lapses, or `null` with neither. */
  nextLapse() {
    const at = this.sql.exec(
      "SELECT MIN(at) AS at FROM (SELECT renewed AS at FROM holders UNION ALL SELECT at FROM contenders)"
    ).one().at;
    return at === null ? null : at + LAPSE_SECONDS;
  }
  /** One alarm at a time, set for the next lapse. A renewal does not move it; it fires early and sets the next. */
  async keepAlarm() {
    if (await this.ctx.storage.getAlarm() !== null) return;
    const next = this.nextLapse();
    if (next !== null) await this.ctx.storage.setAlarm(next * 1e3);
  }
  /** The lapse, for a store nobody calls. A holder left holding nothing goes too, and so do the nonces of a device that
   * stopped calling, which its own requests would have cleared. */
  async alarm() {
    if (!this.hasTables()) return;
    if (!this.upgraded) this.upgrade();
    const now = nowSeconds();
    this.lapse(now);
    this.sql.exec(
      "DELETE FROM holders WHERE NOT EXISTS (SELECT 1 FROM claims WHERE holder_kind = holders.kind AND holder_id = holders.id AND claims.machine = holders.machine)"
    );
    this.sql.exec("DELETE FROM nonces WHERE seen < ?", now - NONCE_KEEP_SECONDS);
    const next = this.nextLapse();
    this.lapseFloor = next ?? Number.POSITIVE_INFINITY;
    if (next !== null) await this.ctx.storage.setAlarm(next * 1e3);
  }
  async authenticate(request, target, body) {
    const device = request.headers.get(DEVICE_HEADER);
    const time = request.headers.get(TIME_HEADER);
    const nonce = request.headers.get(NONCE_HEADER);
    const sent = request.headers.get(SIGNATURE_HEADER);
    const signature = sent ? fromHex(sent) : null;
    if (!device || !time || !nonce || !signature || !TIME.test(time) || !NONCE.test(nonce)) {
      return refuse(401, "unsigned");
    }
    if (!this.hasTables()) return refuse(401, "unknown_device");
    const row = this.sql.exec("SELECT public_key FROM devices WHERE id = ?", device).toArray()[0];
    if (!row) return refuse(401, "unknown_device");
    const now = Math.floor(Date.now() / 1e3);
    if (Math.abs(now - Number(time)) > SKEW_SECONDS) return refuse(401, "stale_time");
    const key = await importPublicKey(fromBase64(row.public_key) ?? new Uint8Array());
    const text = signedText(request.method, target, time, nonce, await bodyHash(body));
    if (!await verifySignature(key, text, signature)) return refuse(401, "bad_signature");
    this.sql.exec(
      "DELETE FROM nonces WHERE device = ? AND seen < ?",
      device,
      now - NONCE_KEEP_SECONDS
    );
    const recent = this.recentOf(device, now);
    if (recent.length >= REQUESTS_PER_MINUTE) {
      return Response.json(
        {
          error: "rate_limited"
        },
        {
          status: 429,
          headers: {
            "Retry-After": String(RATE_WINDOW_SECONDS)
          }
        }
      );
    }
    const signed = Number(time);
    const taken = this.sql.exec(
      "INSERT OR IGNORE INTO nonces (device, seen, nonce) VALUES (?, ?, ?)",
      device,
      signed,
      nonce
    );
    if (taken.rowsWritten === 0) return refuse(401, "replayed_nonce");
    recent.push(signed);
    return null;
  }
  /** When `device`'s verified requests of the last minute were seen: its nonces, read from the table only the first time. */
  recentOf(device, now) {
    const since = now - RATE_WINDOW_SECONDS;
    const kept = this.recent.get(device) ?? this.sql.exec("SELECT seen FROM nonces WHERE device = ? AND seen > ?", device, since).toArray().map((row) => row.seen);
    const within = kept.filter((seen) => seen > since);
    this.recent.set(device, within);
    return within;
  }
  route(method, url, text, caller) {
    const parts = url.pathname.split("/").slice(1);
    const [v1, area, first, second, third] = parts;
    if (v1 !== "v1" || parts.length > 5) return refuse(404, "no_route");
    if (area === "devices" && parts.length === 2) {
      return method === "GET" ? this.listDevices() : refuse(405, "bad_method");
    }
    if (area === "devices" && parts.length === 3) {
      return method === "DELETE" ? this.signOut(first) : refuse(405, "bad_method");
    }
    if (area === "presence" && parts.length === 2) {
      return method === "PUT" ? this.putPresence(caller, text) : refuse(405, "bad_method");
    }
    if (area === "peers" && parts.length === 2) {
      return method === "GET" ? this.listPeers(caller) : refuse(405, "bad_method");
    }
    if (area === "renewals" && parts.length === 2) {
      return method === "POST" ? this.renew(text) : refuse(405, "bad_method");
    }
    if (area === "claims") {
      this.lapseIfDue(nowSeconds());
      if (parts.length === 2)
        return method === "POST" ? this.claim(text) : method === "GET" ? this.listClaims() : refuse(405, "bad_method");
      const kind = claimKindOf(second);
      if (!HASH.test(first ?? "") || parts.length === 3 || !kind) return refuse(400, "invalid_key");
      if (third === void 0)
        return method === "DELETE" ? this.clearKind(first, kind) : refuse(405, "bad_method");
      if (!HASH.test(third)) return refuse(400, "invalid_key");
      if (method === "GET") return this.readClaim(first, kind, third);
      return method === "DELETE" ? this.giveBack(first, kind, third) : refuse(405, "bad_method");
    }
    if (area === "owners") {
      if (parts.length === 2)
        return method === "GET" ? this.readOwners(url) : refuse(405, "bad_method");
      if (!HASH.test(first ?? "") || parts.length !== 3) return refuse(400, "invalid_key");
      if (method === "PUT") return this.putOwner(first, text);
      return method === "DELETE" ? this.deleteOwner(first) : refuse(405, "bad_method");
    }
    if (area === "runs") {
      if (!HASH.test(first ?? "") || parts.length !== 3) return refuse(400, "invalid_key");
      if (method === "PUT") return this.putRuns(first, text);
      return method === "GET" ? this.readRuns(first) : refuse(405, "bad_method");
    }
    return refuse(404, "no_route");
  }
  /** Every device of this object and the machine it named, never its key. The table has no enrolment time, so its rowid, which a new row always takes past the last, is the order they enrolled in. */
  listDevices() {
    const devices = this.sql.exec("SELECT id AS device, machine FROM devices ORDER BY rowid").toArray();
    return reply(200, {
      devices
    });
  }
  /** Sets the caller's own row: what it is and where it can be reached. No other device's row is written. */
  putPresence(caller, text) {
    const body = exactly(parseJson(text), [
      "kind",
      "address"
    ]);
    const kind = deviceKindOf(body?.kind);
    const address = addressOf(body?.address);
    if (!kind || address === null) return refuse(400, "invalid_body");
    this.sql.exec("UPDATE devices SET kind = ?, address = ? WHERE id = ?", kind, address, caller);
    return reply(204);
  }
  /**
   * The caller's other devices: those whose id starts with the caller's account id and a dot. A team store holds every
   * member's devices in one object, and a device is listed only to the forge account that enrolled it, so the account
   * id is the filter and nothing else is.
   */
  listPeers(caller) {
    const prefix = `${caller.split(".")[0]}.`;
    const devices = this.sql.exec(
      "SELECT id AS device, machine, kind, address, public_key FROM devices WHERE substr(id, 1, ?) = ? AND id != ? ORDER BY rowid",
      prefix.length,
      prefix,
      caller
    ).toArray();
    return reply(200, {
      devices
    });
  }
  signOut(device) {
    this.sql.exec("DELETE FROM nonces WHERE device = ?", device);
    this.recent.delete(device);
    const gone = this.sql.exec("DELETE FROM devices WHERE id = ?", device);
    return gone.rowsWritten === 0 ? refuse(404, "no_device") : reply(204);
  }
  /** Claim if free: one INSERT that does nothing when the key is held. A win renews its holder. */
  claim(text) {
    const body = exactly(parseJson(text), [
      "repo",
      "kind",
      "key",
      "holder"
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
      holder.since
    );
    if (won.rowsWritten > 0) this.putHolder(holder);
    const held = this.claimed(repo, kind, key);
    if (won.rowsWritten === 0 && held) this.putContender(repo, kind, key, held.holder, holder);
    return reply(won.rowsWritten > 0 ? 200 : 409, held);
  }
  /** Every live claim, grouped by its holder, with what each holder waits on. A holder that only contends has no `holders` row, so its name, title and renewal are its newest contender's. */
  listClaims() {
    const holders = new Map(
      this.sql.exec("SELECT kind, id, machine, name, title, renewed FROM holders").toArray().map((row) => [
        `${row.kind}\0${row.id}\0${row.machine}`,
        row
      ])
    );
    const groups = /* @__PURE__ */ new Map();
    const groupOf = /* @__PURE__ */ __name((kind, id, machine, name, title, renewed) => {
      const at = `${kind}\0${id}\0${machine}`;
      const known = groups.get(at);
      const row = holders.get(at);
      if (known) {
        if (!row) {
          known.renewed = renewed;
          known.holder.name = name;
          known.holder.title = title;
        }
        return known;
      }
      const made = {
        holder: {
          kind,
          id,
          machine,
          name: row?.name ?? name,
          title: row?.title ?? title
        },
        renewed: row?.renewed ?? renewed,
        claims: [],
        waits: []
      };
      groups.set(at, made);
      return made;
    }, "groupOf");
    for (const row of this.sql.exec(
      `SELECT claims.repo, claims.kind, claims.key, claims.since, claims.holder_kind, claims.holder_id, claims.machine, holders.name, holders.title, holders.renewed
         FROM claims JOIN holders ON holders.kind = claims.holder_kind AND holders.id = claims.holder_id AND holders.machine = claims.machine
         ORDER BY claims.kind, claims.since, claims.key, claims.repo`
    ).toArray()) {
      groupOf(
        row.holder_kind,
        row.holder_id,
        row.machine,
        row.name,
        row.title,
        row.renewed
      ).claims.push({
        repo: row.repo,
        kind: row.kind,
        key: row.key,
        since: row.since
      });
    }
    for (const row of this.sql.exec(
      `SELECT claims.repo, claims.kind, claims.key, claims.since, claims.holder_kind, claims.holder_id, claims.machine, holders.name, holders.title, holders.renewed,
                contenders.contender_kind, contenders.contender_id, contenders.machine AS contender_machine, contenders.name AS contender_name, contenders.title AS contender_title, contenders.at
         FROM contenders
         JOIN claims ON claims.repo = contenders.repo AND claims.kind = contenders.kind AND claims.key = contenders.key
         JOIN holders ON holders.kind = claims.holder_kind AND holders.id = claims.holder_id AND holders.machine = claims.machine
         ORDER BY contenders.at, contenders.rowid`
    ).toArray()) {
      groupOf(
        row.contender_kind,
        row.contender_id,
        row.contender_machine,
        row.contender_name,
        row.contender_title,
        row.at
      ).waits.push({
        repo: row.repo,
        kind: row.kind,
        key: row.key,
        holder: claimFrom({
          kind: row.holder_kind,
          id: row.holder_id,
          machine: row.machine,
          since: row.since,
          name: row.name,
          title: row.title,
          renewed: row.renewed
        }).holder
      });
    }
    const order = /* @__PURE__ */ __name((a, b) => a < b ? -1 : a > b ? 1 : 0, "order");
    const holdersOut = [
      ...groups.values()
    ].sort(
      (a, b) => order(a.holder.name, b.holder.name) || order(a.holder.machine, b.holder.machine) || order(a.holder.kind, b.holder.kind) || order(a.holder.id, b.holder.id)
    );
    for (const group of holdersOut)
      group.waits.sort(
        (a, b) => order(a.kind, b.kind) || order(a.key, b.key) || order(a.repo, b.repo)
      );
    return reply(200, {
      holders: holdersOut
    });
  }
  /** A claim refused is kept while its asker keeps asking, unless the asker is the holder or the table is full. An asker already told stays told. */
  putContender(repo, kind, key, held, asker) {
    if (held.kind === asker.kind && held.id === asker.id && held.machine === asker.machine) return;
    const known = this.sql.exec(
      "SELECT 1 FROM contenders WHERE repo = ? AND kind = ? AND key = ? AND contender_kind = ? AND contender_id = ? AND machine = ?",
      repo,
      kind,
      key,
      asker.kind,
      asker.id,
      asker.machine
    ).toArray().length > 0;
    if (!known && this.count("contenders") >= MAX_CONTENDERS) return;
    const now = nowSeconds();
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
      now
    );
    this.lapsesFrom(now);
  }
  /** The holder's row: the name and title it last sent, renewed now. */
  putHolder(holder) {
    this.lapsesFrom(nowSeconds());
    this.sql.exec(
      `INSERT INTO holders (kind, id, machine, name, title, renewed) VALUES (?, ?, ?, ?, ?, ?)
       ON CONFLICT (kind, id, machine) DO UPDATE SET name = excluded.name, title = excluded.title, renewed = excluded.renewed`,
      holder.kind,
      holder.id,
      holder.machine,
      holder.name,
      holder.title,
      nowSeconds()
    );
  }
  claimed(repo, kind, key) {
    const row = this.sql.exec(
      `SELECT claims.holder_kind AS kind, claims.holder_id AS id, claims.machine, claims.since, holders.name, holders.title, holders.renewed
         FROM claims JOIN holders ON holders.kind = claims.holder_kind AND holders.id = claims.holder_id AND holders.machine = claims.machine
         WHERE claims.repo = ? AND claims.kind = ? AND claims.key = ?`,
      repo,
      kind,
      key
    ).toArray()[0];
    return row ? claimFrom(row) : null;
  }
  readClaim(repo, kind, key) {
    const held = this.claimed(repo, kind, key);
    return held ? reply(200, held) : refuse(404, "no_claim");
  }
  /** Give back and take over are the same delete; a take over then claims. */
  giveBack(repo, kind, key) {
    this.sql.exec(
      "DELETE FROM contenders WHERE repo = ? AND kind = ? AND key = ?",
      repo,
      kind,
      key
    );
    this.sql.exec("DELETE FROM claims WHERE repo = ? AND kind = ? AND key = ?", repo, kind, key);
    return reply(204);
  }
  /** Every claim of one kind under a repository: main went green, and its reds end. */
  clearKind(repo, kind) {
    this.sql.exec("DELETE FROM contenders WHERE repo = ? AND kind = ?", repo, kind);
    this.sql.exec("DELETE FROM claims WHERE repo = ? AND kind = ?", repo, kind);
    return reply(204);
  }
  /** One call renews every claim the holder has, by writing its one row. Answers how many claims it holds, so a
   * holder whose claims lapsed reads `0` and claims again, and who asked for them since the last renewal, once. */
  renew(text) {
    const body = exactly(parseJson(text), [
      "holder"
    ]);
    const who = renewingOf(body?.holder);
    if (!who) return refuse(400, "invalid_body");
    this.lapseIfDue(nowSeconds());
    this.sql.exec(
      "UPDATE holders SET renewed = ? WHERE kind = ? AND id = ? AND machine = ?",
      nowSeconds(),
      who.kind,
      who.id,
      who.machine
    );
    const claims = this.sql.exec(
      "SELECT COUNT(*) AS n FROM claims WHERE holder_kind = ? AND holder_id = ? AND machine = ?",
      who.kind,
      who.id,
      who.machine
    ).one().n;
    return reply(200, {
      claims,
      contested: claims === 0 ? [] : this.takeContested(who)
    });
  }
  /** The contenders on the claims `who` holds that were not told yet, marked told as they are answered so each is told once. */
  takeContested(who) {
    const mine = `SELECT repo, kind, key FROM claims WHERE holder_kind = ? AND holder_id = ? AND machine = ?`;
    const rows = this.sql.exec(
      `SELECT repo, kind, key, contender_kind, contender_id, machine, name, title, since FROM contenders
         WHERE told = 0 AND (repo, kind, key) IN (${mine}) ORDER BY at, rowid`,
      who.kind,
      who.id,
      who.machine
    ).toArray();
    if (rows.length > 0)
      this.sql.exec(
        `UPDATE contenders SET told = 1 WHERE told = 0 AND (repo, kind, key) IN (${mine})`,
        who.kind,
        who.id,
        who.machine
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
        title: row.title
      }
    }));
  }
  putOwner(key, text) {
    const body = exactly(parseJson(text), [
      "holder"
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
      holder.since
    );
    return reply(204);
  }
  deleteOwner(key) {
    this.sql.exec("DELETE FROM owners WHERE key = ?", key);
    return reply(204);
  }
  readOwners(url) {
    const keys = (url.searchParams.get("keys") ?? "").split(",").filter((key) => key !== "");
    if (keys.length > MAX_KEYS || !keys.every((key) => HASH.test(key)))
      return refuse(400, "invalid_key");
    const found = {};
    for (const key of keys) {
      const row = this.sql.exec("SELECT kind, id, machine, since FROM owners WHERE key = ?", key).toArray()[0];
      if (row) found[key] = holderFrom(row);
    }
    return reply(200, found);
  }
  /** Replaces every fact held for the repository. A repository with none holds no row. */
  putRuns(repo, text) {
    const body = exactly(parseJson(text), [
      "facts"
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
      JSON.stringify(facts)
    );
    return reply(204);
  }
  readRuns(repo) {
    const row = this.sql.exec("SELECT facts FROM runs WHERE repo = ?", repo).toArray()[0];
    return row ? reply(200, {
      facts: JSON.parse(row.facts)
    }) : refuse(404, "no_runs");
  }
};

// src/team.ts
var ORG = /^[A-Za-z0-9-]{1,39}$/;
var SLUG = /^[A-Za-z0-9_-]{1,100}$/;
function teamOf(env) {
  const text = env.TEAM;
  if (text === void 0 || text === "") return null;
  const [org, slug, ...rest] = text.split("/");
  if (!org || !ORG.test(org) || rest.length > 0) return "bad";
  if (slug === void 0)
    return {
      org
    };
  return SLUG.test(slug) ? {
    org,
    slug
  } : "bad";
}
__name(teamOf, "teamOf");
var teamObjectName = /* @__PURE__ */ __name(({ org, slug }) => `team:${org}${slug === void 0 ? "" : `/${slug}`}`.toLowerCase(), "teamObjectName");

// src/enrol.ts
var FORGE_USER_URL = "https://api.github.com/user";
var FORGE_LOGIN_URL = "https://github.com";
var refuse2 = /* @__PURE__ */ __name((status, error) => Response.json(
  {
    error
  },
  {
    status
  }
), "refuse");
async function askForge(url, token) {
  try {
    return await fetch(url, {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/json",
        "User-Agent": "armada-store"
      }
    });
  } catch {
    return refuse2(502, "forge_unreachable");
  }
}
__name(askForge, "askForge");
async function accountOf(token, env) {
  const answer = await askForge(env.FORGE_USER_URL ?? FORGE_USER_URL, token);
  if (answer.status === 401 || answer.status === 403) return refuse2(401, "forge_refused");
  if (!answer.ok) return refuse2(502, "forge_unreachable");
  const user = await answer.json().catch(() => null);
  const id = user?.id;
  if (typeof id !== "number" || !Number.isSafeInteger(id) || id <= 0)
    return refuse2(502, "forge_unreadable");
  return {
    id,
    login: user?.login
  };
}
__name(accountOf, "accountOf");
async function refuseNonMember(token, login, team, env) {
  const api = new URL(env.FORGE_USER_URL ?? FORGE_USER_URL).origin;
  if (team.slug !== void 0 && (typeof login !== "string" || login === ""))
    return refuse2(502, "forge_unreadable");
  const url = team.slug === void 0 ? `${api}/user/memberships/orgs/${team.org}` : `${api}/orgs/${team.org}/teams/${team.slug}/memberships/${encodeURIComponent(String(login))}`;
  const answer = await askForge(url, token);
  if (answer.status === 404) return refuse2(403, "not_a_member");
  if (answer.status === 403) return refuse2(403, "org_restricted");
  if (answer.status === 401) return refuse2(401, "forge_refused");
  if (!answer.ok) return refuse2(502, "forge_unreachable");
  const membership = await answer.json().catch(() => null);
  return membership?.state === "active" ? null : refuse2(403, "not_a_member");
}
__name(refuseNonMember, "refuseNonMember");
async function refuseIfCounted(request, env) {
  const address = request.headers.get("CF-Connecting-IP") ?? "unknown";
  if (env.ENROL_LIMIT && !(await env.ENROL_LIMIT.limit({
    key: address
  })).success)
    return refuse2(429, "enrol_rate_limited");
  return null;
}
__name(refuseIfCounted, "refuseIfCounted");
async function registerToken(token, publicKey, machine, kind, team, env) {
  const account = await accountOf(token, env);
  if (account instanceof Response) return account;
  if (team) {
    const refused = await refuseNonMember(token, account.login, team, env);
    if (refused) return refused;
  }
  const device = `${account.id}.${hex(crypto.getRandomValues(new Uint8Array(16)))}`;
  const placed = await env.ACCOUNT.get(
    env.ACCOUNT.idFromName(team ? teamObjectName(team) : String(account.id))
  ).register(device, publicKey, machine, team ? MAX_TEAM_DEVICES : MAX_DEVICES, kind);
  if (placed === "full") return refuse2(413, "devices_full");
  return Response.json({
    device
  });
}
__name(registerToken, "registerToken");
async function enrol(request, env) {
  if (request.method !== "POST") return refuse2(405, "bad_method");
  const team = teamOf(env);
  if (team === "bad") return refuse2(500, "bad_team_config");
  const text = await request.text();
  if (text.length > MAX_BODY_BYTES) return refuse2(413, "too_large");
  const body = exactly(parseJson(text), [
    "token",
    "public_key",
    "machine"
  ]);
  const { token, public_key: publicKey, machine } = body ?? {};
  if (typeof token !== "string" || token === "" || token.length > 512 || typeof publicKey !== "string" || typeof machine !== "string" || !MACHINE.test(machine)) {
    return refuse2(400, "invalid_body");
  }
  if (!await isP256Key(publicKey)) return refuse2(400, "invalid_public_key");
  const counted = await refuseIfCounted(request, env);
  if (counted) return counted;
  return registerToken(token, publicKey, machine, "mac", team, env);
}
__name(enrol, "enrol");
async function isP256Key(publicKey) {
  try {
    await importPublicKey(fromBase64(publicKey) ?? new Uint8Array());
    return true;
  } catch {
    return false;
  }
}
__name(isP256Key, "isP256Key");

// src/sign_in.ts
var CLIENT_ID = /^[A-Za-z0-9._-]{1,64}$/;
var DEVICE_CODE = /^[\x20-\x7e]{1,128}$/;
var REASON = /^[a-z_]{1,64}$/;
var SLOWER_INTERVAL = 10;
var reasonOf = /* @__PURE__ */ __name((error) => typeof error === "string" && REASON.test(error) ? {
  reason: error
} : {}, "reasonOf");
async function askLogin(env, path, form) {
  let answer;
  try {
    answer = await fetch(`${env.FORGE_LOGIN_URL ?? FORGE_LOGIN_URL}${path}`, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/x-www-form-urlencoded",
        "User-Agent": "armada-store"
      },
      body: new URLSearchParams(form)
    });
  } catch {
    return refuse2(502, "forge_unreachable");
  }
  const body = await answer.json().catch(() => null);
  return {
    status: answer.status,
    body: typeof body === "object" && body !== null && !Array.isArray(body) ? body : {}
  };
}
__name(askLogin, "askLogin");
var refusedBy = /* @__PURE__ */ __name((status, error) => status >= 500 ? refuse2(502, "forge_unreachable") : error !== void 0 || status >= 400 ? Response.json(
  {
    error: "forge_refused",
    ...reasonOf(error)
  },
  {
    status: 400
  }
) : null, "refusedBy");
async function bodyOf(request, keys) {
  if (request.method !== "POST") return refuse2(405, "bad_method");
  const text = await request.text();
  if (text.length > MAX_BODY_BYTES) return refuse2(413, "too_large");
  const body = exactly(parseJson(text), keys);
  const clientId = body?.client_id;
  if (!body || typeof clientId !== "string" || !CLIENT_ID.test(clientId))
    return refuse2(400, "invalid_body");
  return body;
}
__name(bodyOf, "bodyOf");
async function signIn(request, env) {
  const body = await bodyOf(request, [
    "client_id"
  ]);
  if (body instanceof Response) return body;
  const counted = await refuseIfCounted(request, env);
  if (counted) return counted;
  const asked = await askLogin(env, "/login/device/code", {
    client_id: String(body.client_id)
  });
  if (asked instanceof Response) return asked;
  const { device_code, user_code, verification_uri, interval, expires_in, error } = asked.body;
  const refused = refusedBy(asked.status, error);
  if (refused) return refused;
  if (typeof device_code !== "string" || typeof user_code !== "string" || typeof verification_uri !== "string" || typeof interval !== "number" || typeof expires_in !== "number")
    return refuse2(502, "forge_unreadable");
  return Response.json({
    device_code,
    user_code,
    verification_uri,
    interval,
    expires_in
  });
}
__name(signIn, "signIn");
async function signInPoll(request, env) {
  const team = teamOf(env);
  if (team === "bad") return refuse2(500, "bad_team_config");
  const body = await bodyOf(request, [
    "client_id",
    "device_code",
    "public_key",
    "machine"
  ]);
  if (body instanceof Response) return body;
  const { client_id, device_code, public_key, machine } = body;
  if (typeof device_code !== "string" || !DEVICE_CODE.test(device_code) || typeof public_key !== "string" || typeof machine !== "string" || !MACHINE.test(machine))
    return refuse2(400, "invalid_body");
  if (!await isP256Key(public_key)) return refuse2(400, "invalid_public_key");
  const asked = await askLogin(env, "/login/oauth/access_token", {
    client_id: String(client_id),
    device_code,
    grant_type: "urn:ietf:params:oauth:grant-type:device_code"
  });
  if (asked instanceof Response) return asked;
  const { access_token: token, error, interval } = asked.body;
  if (asked.status >= 500) return refuse2(502, "forge_unreachable");
  if (error === "authorization_pending")
    return Response.json(
      {
        waiting: "pending"
      },
      {
        status: 202
      }
    );
  if (error === "slow_down")
    return Response.json(
      {
        waiting: "slow_down",
        interval: typeof interval === "number" ? interval : SLOWER_INTERVAL
      },
      {
        status: 202
      }
    );
  if (error === "expired_token") return refuse2(410, "expired");
  if (error === "access_denied") return refuse2(403, "denied");
  const refused = refusedBy(asked.status, error);
  if (refused) return refused;
  if (typeof token !== "string" || token === "") return refuse2(502, "forge_unreadable");
  return registerToken(token, public_key, machine, "phone", team, env);
}
__name(signInPoll, "signInPoll");

// src/index.ts
var DEVICE = /^([0-9]{1,16})\.[0-9a-f]{32}$/;
var CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Expose-Headers": "Retry-After"
};
var PREFLIGHT = {
  ...CORS,
  "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE",
  "Access-Control-Allow-Headers": "Content-Type, X-Armada-Device, X-Armada-Time, X-Armada-Nonce, X-Armada-Signature",
  "Access-Control-Max-Age": "600"
};
function withCors(answer) {
  const copy = new Response(answer.body, answer);
  for (const [name, value] of Object.entries(CORS)) copy.headers.set(name, value);
  return copy;
}
__name(withCors, "withCors");
var index_default = {
  async fetch(request, env) {
    const { pathname } = new URL(request.url);
    if (!pathname.startsWith("/v1/")) return route(request, env, pathname);
    if (request.method === "OPTIONS")
      return new Response(null, {
        status: 204,
        headers: PREFLIGHT
      });
    return withCors(await route(request, env, pathname));
  }
};
async function route(request, env, pathname) {
  const team = teamOf(env);
  if (team === "bad")
    return Response.json(
      {
        error: "bad_team_config"
      },
      {
        status: 500
      }
    );
  if (pathname === "/v1/enrol") return enrol(request, env);
  if (pathname === "/v1/sign_in") return signIn(request, env);
  if (pathname === "/v1/sign_in/poll") return signInPoll(request, env);
  if (!pathname.startsWith("/v1/"))
    return Response.json(
      {
        error: "no_route"
      },
      {
        status: 404
      }
    );
  const account = DEVICE.exec(request.headers.get(DEVICE_HEADER) ?? "")?.[1];
  if (!account)
    return Response.json(
      {
        error: "unsigned"
      },
      {
        status: 401
      }
    );
  return env.ACCOUNT.get(env.ACCOUNT.idFromName(team ? teamObjectName(team) : account)).fetch(
    request
  );
}
__name(route, "route");
export {
  Account,
  index_default as default
};

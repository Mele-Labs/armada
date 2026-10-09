// What main holds of Sessions, read off a real listener: the list, a thread opened and followed, and
// the routes each act is sent to. **What crosses the wire is under test**, so a route named wrongly or
// a repository taken from the wrong place fails here rather than on a person's press.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { SessionRecord, SessionRow } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { SessionsHost, withRow, withSession } from "./sessions";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

const AT = "2026-10-07T13:48:02Z";

const record = (id: string, change: Partial<SessionRecord> = {}): SessionRecord => ({
  id,
  harness: "a_harness",
  origin: "bridge",
  manifest_id: "armada",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  hosted: { turn: { state: "idle" }, mode: "auto", running: false },
  ...change,
});

const said = (id: string, text: string): SessionRow => ({ kind: "message", id, at: AT, from: { kind: "agent" }, text });

type Sent = { method: string; path: string; body: unknown };

/** A Fleet that lists these sessions, serves their threads and answers every act with the session. */
async function fleet(sent: Sent[], sessions: SessionRecord[], threads: Record<string, SessionRow[]> = {}, refuse = false): Promise<number> {
  const server = createServer((request, response) => {
    let raw = "";
    request.on("data", (chunk) => (raw += String(chunk)));
    request.on("end", () => {
      const url = new URL(request.url ?? "/", "http://localhost");
      sent.push({ method: request.method ?? "", path: `${url.pathname}${url.search}`, body: raw === "" ? undefined : JSON.parse(raw) });
      const answer = (status: number, body: unknown) => {
        response.writeHead(status, { "content-type": "application/json" });
        response.end(JSON.stringify(body));
      };
      if (refuse && url.pathname.endsWith("/merge")) return answer(409, { code: "fleet.merge_checks_not_passed", message: "Checks have not passed: lint." });
      if (url.pathname === "/sessions") return answer(200, { sessions });
      if (url.pathname === "/sessions/one") {
        const id = url.searchParams.get("session_id") ?? "";
        return answer(200, { session: sessions.find((one) => one.id === id), rows: threads[id] ?? [] });
      }
      if (url.pathname.startsWith("/sessions/")) return answer(200, sessions[0]);
      return answer(200, { manifest_id: "armada", number: 1, state: "merged", auto_merge: false, checks: { state: "passed" }, title: "t", branch: "b", address: "a" });
    });
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  return (server.address() as AddressInfo).port;
}

function host(port: number | null) {
  const published: Partial<BridgeState>[] = [];
  const sessions = new SessionsHost((change) => published.push(change), () => port);
  return { sessions, published, last: <K extends keyof BridgeState>(key: K) => [...published].reverse().find((one) => key in one)?.[key] };
}

it("holds every session Fleet lists, and a session that changed replaces its row", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a"), record("b")]);
  const { sessions, last } = host(port);
  await sessions.again(port);
  expect(last("sessions")).toMatchObject({ state: "read", sessions: [{ id: "a" }, { id: "b" }] });
  sessions.changed(record("b", { title: "Named" }));
  sessions.changed(record("c"));
  expect(last("sessions")).toMatchObject({ sessions: [{ id: "c" }, { id: "a" }, { id: "b", title: "Named" }] });
});

it("says a Fleet that does not serve sessions answered nothing, so the surface stays off", async () => {
  const server = createServer((_request, response) => {
    response.writeHead(404, { "content-type": "application/json" });
    response.end(JSON.stringify({ code: "fleet.not_found", message: "no such route" }));
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  const port = (server.address() as AddressInfo).port;
  const { sessions, last } = host(port);
  await sessions.again(port);
  expect(last("sessions")).toMatchObject({ state: "failed" });
});

it("drops a row for a thread nobody opened, and follows one that was", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")], { a: [said("r1", "first")] });
  const { sessions, last } = host(port);
  await sessions.again(port);
  sessions.row({ session_id: "a", row: said("r0", "before it was opened") });
  expect(last("sessionThreads")).toBeUndefined();
  await sessions.watch("a");
  expect(last("sessionThreads")).toEqual({ a: [said("r1", "first")] });
  sessions.row({ session_id: "a", row: said("r2", "second") });
  sessions.row({ session_id: "a", row: said("r1", "first, edited") });
  expect(last("sessionThreads")).toEqual({ a: [said("r1", "first, edited"), said("r2", "second")] });
  // Opened once: asking again reads nothing more.
  const reads = sent.filter((one) => one.path.startsWith("/sessions/one")).length;
  await sessions.watch("a");
  expect(sent.filter((one) => one.path.startsWith("/sessions/one")).length).toBe(reads);
});

it("keeps a row that arrives while the thread is being read", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")], { a: [said("r1", "first")] });
  const { sessions, last } = host(port);
  await sessions.again(port);
  const reading = sessions.watch("a");
  sessions.row({ session_id: "a", row: said("r2", "arrived mid-read") });
  await reading;
  expect(last("sessionThreads")).toEqual({ a: [said("r1", "first"), said("r2", "arrived mid-read")] });
});

it("opens a window when a Session shows a page, once, and never for a row a thread read brought", async () => {
  const sent: Sent[] = [];
  const shown = { kind: "window", id: "w1", at: AT, title: "Findings", url: "http://localhost:5173/" } as const;
  const port = await fleet(sent, [record("a")], { a: [shown] });
  const { sessions } = host(port);
  const opened: string[][] = [];
  sessions.onWindow((id, title, url) => opened.push([id, title, url]));
  await sessions.again(port);
  await sessions.watch("a");
  sessions.row({ session_id: "a", row: shown });
  expect(opened).toEqual([]);
  sessions.row({ session_id: "a", row: { ...shown, id: "w2" } });
  sessions.row({ session_id: "a", row: { ...shown, id: "w2" } });
  expect(opened).toEqual([["a", "Findings", "http://localhost:5173/"]]);
});

it("reopens only an address the ledger shows as a window", async () => {
  const sent: Sent[] = [];
  const url = "http://localhost:5173/";
  const port = await fleet(sent, [
    record("a", {
      attachments: [
        { kind: "artifact", target: url, state: "standing", detail: { form: "window", title: "Findings" }, since: AT, changed_at: AT },
        { kind: "artifact", target: "/repo/notes.md", state: "standing", detail: { form: "file" }, since: AT, changed_at: AT },
      ],
    }),
  ]);
  const { sessions } = host(port);
  const opened: string[][] = [];
  sessions.onWindow((id, title, address) => opened.push([id, title, address]));
  await sessions.again(port);
  expect(sessions.openWindow("a", url)).toEqual({ ok: true });
  expect(sessions.openWindow("a", "http://evil.example/")).toMatchObject({ ok: false });
  expect(sessions.openWindow("a", "/repo/notes.md")).toMatchObject({ ok: false });
  expect(sessions.openWindow("nobody", url)).toMatchObject({ ok: false });
  expect(opened).toEqual([["a", "Findings", url]]);
});

it("sends each act to its own route with the session named, and folds what Fleet answers", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a", { title: "Fleet's answer" })]);
  const { sessions, last } = host(port);
  await sessions.start({ manifest_id: "armada", title: "T" });
  await sessions.send({ session_id: "a", text: "hi", attachments: [{ name: "n.png", media_type: "image/png", data: "AAAA" }] });
  await sessions.answer({ session_id: "a", call: "c1", answer: "allow_and_remember" });
  await sessions.answerWaiting({ session_id: "a", item_id: "ask:c1", choice: 1 });
  await sessions.tune({ session_id: "a", mode: "plan", effort: "high" });
  await sessions.end("a");
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual([
    "POST /sessions/start",
    "POST /sessions/message",
    "POST /sessions/ask/answer",
    "POST /sessions/waiting/answer",
    "POST /sessions/tune",
    "POST /sessions/close",
  ]);
  expect(sent[1]!.body).toEqual({ session_id: "a", text: "hi", attachments: [{ name: "n.png", media_type: "image/png", data: "AAAA" }] });
  expect(sent[2]!.body).toEqual({ session_id: "a", call: "c1", answer: "allow_and_remember" });
  expect(sent[3]!.body).toEqual({ session_id: "a", item_id: "ask:c1", choice: 1 });
  expect(sent[5]!.body).toEqual({ session_id: "a" });
  expect(last("sessions")).toMatchObject({ sessions: [{ id: "a", title: "Fleet's answer" }] });
});

it("presses a pull request against the session's own repository", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a", { manifest_id: "storefront" })]);
  const { sessions } = host(port);
  await sessions.again(port);
  sent.length = 0;
  await sessions.press("a", 1847, "read");
  await sessions.press("a", 1847, "ready");
  await sessions.press("a", 1847, "merge");
  await sessions.press("a", 1847, "auto_merge");
  await sessions.press("a", 1847, "review");
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual([
    "GET /pull_requests/storefront/1847",
    "POST /pull_requests/storefront/1847/ready",
    "POST /pull_requests/storefront/1847/merge",
    "POST /pull_requests/storefront/1847/auto_merge",
    "POST /pull_request_reviews/storefront",
  ]);
  expect(sent[4]!.body).toEqual({ pull_request: "1847", session_id: "a" });
});

it("hands Fleet's refusal of a merge back with its words", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")], {}, true);
  const { sessions } = host(port);
  await sessions.again(port);
  const refused = await sessions.press("a", 1843, "merge");
  expect(refused).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { message: "Checks have not passed: lint." } } });
});

it("refuses a press with no connection or on a session it does not hold, before anything is sent", async () => {
  const { sessions } = host(null);
  expect(await sessions.press("a", 1, "merge")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  const sent: Sent[] = [];
  const port = await fleet(sent, []);
  const connected = host(port).sessions;
  expect(await connected.press("nobody", 1, "merge")).toEqual({ ok: false, outcome: { ok: false, why: "no_manifest" } });
  expect(sent).toEqual([]);
});

it("replaces a row by its id and appends one it has not seen, and puts a new session first", () => {
  expect(withRow([said("1", "a")], said("2", "b")).map((one) => one.id)).toEqual(["1", "2"]);
  expect(withRow([said("1", "a"), said("2", "b")], said("1", "c"))[0]).toEqual(said("1", "c"));
  expect(withSession([record("a")], record("b")).map((one) => one.id)).toEqual(["b", "a"]);
});

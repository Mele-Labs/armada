// What main holds of Sessions, read off a real listener: the list, a thread opened and followed, and
// the routes each act is sent to. **What crosses the wire is under test**, so a route named wrongly or
// a repository taken from the wrong place fails here rather than on a person's press.

import { mkdir, mkdtemp, readdir, readFile, rm, utimes, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it, vi } from "vitest";
import { WebSocketServer } from "ws";

import type { RetroRow, SessionRecord, SessionRow, SessionRunMessage, WatchSummary } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { REOPEN_WITHIN_MS, SessionsHost, THREADS_HELD, withRow, withSession } from "./sessions";

let listening: Server | null = null;
/** What the Fleet answers a take of a window's sign-in with. */
let held: string[] = [];
/** While set, Fleet holds the answer to a thread read until it settles. */
let gate: Promise<void> | null = null;
/** Directories a test kept threads in. */
const kept: string[] = [];

afterEach(async () => {
  held = [];
  gate = null;
  vi.useRealTimers();
  await Promise.all(kept.splice(0).map((dir) => rm(dir, { recursive: true, force: true })));
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

/** What the fake Fleet answers `GET /watches?session_id=` with, by Job. */
let watchesOf: Record<string, WatchSummary[]> = {};
const watch = (id: string, change: Partial<WatchSummary> = {}): WatchSummary => ({ id, session_id: "a", name: "Staging deploy", command: ["deploy-status"], every_seconds: 60, gives_up_after_seconds: 3600, state: "running", live: true, started_at: AT, readings: [], verdicts: [], ...change });

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
      if (url.pathname === "/watches") return answer(200, { session_id: url.searchParams.get("session_id"), watches: watchesOf[url.searchParams.get("session_id") ?? ""] ?? [] });
      if (url.pathname === "/sessions/window/failed") return answer(200, { told: true });
      if (url.pathname === "/sessions/one") {
        const id = url.searchParams.get("session_id") ?? "";
        const reply = () => answer(200, { session: sessions.find((one) => one.id === id), rows: threads[id] ?? [] });
        return gate === null ? reply() : void gate.then(reply);
      }
      if (url.pathname === "/sessions/window/sign_in") return answer(200, { sign_in: held });
      if (url.pathname === "/sessions/notes") return answer(202, { session: { ...sessions[0], title: "Noted twice" }, candidates: [[{ file: "src/Rows.tsx", line: 171, why: "the label is on this line", text: "<li>" }], []] });
      if (url.pathname === "/sessions/note") return answer(202, { session: { ...sessions[0], title: "Noted" }, candidates: [{ file: "src/Rows.tsx", line: 171, why: "the label is on this line", text: "<li>" }] });
      if (url.pathname === "/mods/session") return answer(200, { session_id: "mod-1", title: "Usage meter" });
      if (url.pathname === "/sessions/mcp") return answer(200, { servers: [{ name: "linear", state: "needs_sign_in", tools: 0 }] });
      if (url.pathname === "/sessions/mcp/act") return answer(200, { servers: [{ name: "linear", state: "connecting", tools: 0 }], sign_in: { url: "https://linear.example/authorize", callback_expected: true } });
      if (url.pathname.startsWith("/sessions/")) return answer(200, sessions[0]);
      return answer(200, { manifest_id: "armada", number: 1, state: "merged", auto_merge: false, checks: { state: "passed" }, title: "t", branch: "b", address: "a" });
    });
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  return (server.address() as AddressInfo).port;
}

function host(port: number | null, dir?: string) {
  const published: Partial<BridgeState>[] = [];
  const sessions = new SessionsHost((change) => published.push(change), () => port);
  if (dir !== undefined) sessions.keepIn(dir);
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

it("reads a Job's watches once as its thread opens, and takes each watches.changed whole over them", async () => {
  const sent: Sent[] = [];
  watchesOf = { a: [watch("w1")] };
  const port = await fleet(sent, [record("a")], { a: [said("r1", "first")] });
  const { sessions, last } = host(port);
  await sessions.again(port);
  await sessions.watch("a");
  expect(sent.filter((one) => one.path === "/watches?session_id=a")).toHaveLength(1);
  await vi.waitFor(() => expect(last("sessionWatches")).toMatchObject({ a: [{ id: "w1", state: "running" }] }));
  sessions.watchesChanged({ session_id: "a", watches: [watch("w1", { state: "failed", live: false }), watch("w2")] });
  expect(last("sessionWatches")).toMatchObject({ a: [{ id: "w1", state: "failed" }, { id: "w2" }] });
  watchesOf = {};
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

it("keeps the sessions it holds when a later read fails, and publishes nothing for it", async () => {
  let failing = false;
  const server = createServer((request, response) => {
    request.resume();
    request.on("end", () => {
      response.writeHead(failing ? 500 : 200, { "content-type": "application/json" });
      response.end(JSON.stringify(failing ? { code: "fleet.busy", message: "busy" } : { sessions: [record("a"), record("b")] }));
    });
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  const port = (server.address() as AddressInfo).port;
  const { sessions, published, last } = host(port);
  await sessions.again(port);
  const before = published.length;
  failing = true;
  await sessions.again(port);
  expect(published.length).toBe(before);
  expect(last("sessions")).toMatchObject({ state: "read", sessions: [{ id: "a" }, { id: "b" }] });
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

const retroFrame = (change: Partial<RetroRow> = {}): RetroRow => ({
  kind: "retro",
  id: "retro-1",
  at: AT,
  stage: "asking_agent",
  stage_since: "2026-10-12T14:00:00Z",
  characters: 0,
  items: [],
  ...change,
});

it("holds one retro row for the press and draws only its latest frame, however they cross", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")], { a: [said("r1", "first")] });
  const { sessions, last } = host(port);
  await sessions.again(port);
  await sessions.watch("a");
  const reading = retroFrame({ stage: "reading_record", stage_since: "2026-10-12T14:00:03Z" });
  const writing = retroFrame({ stage: "writing", stage_since: "2026-10-12T14:00:09Z", characters: 300, items: [{ title: "A" }] });
  sessions.row({ session_id: "a", row: retroFrame() });
  sessions.row({ session_id: "a", row: writing });
  // Frames cross: an earlier one reaches main after a later one.
  sessions.row({ session_id: "a", row: reading });
  expect(last("sessionThreads")).toEqual({ a: [said("r1", "first"), writing] });
  const written = retroFrame({ stage: "written", stage_since: "2026-10-12T14:00:30Z", characters: 900, retro: 3 });
  sessions.row({ session_id: "a", row: written });
  sessions.row({ session_id: "a", row: writing });
  expect(last("sessionThreads")).toEqual({ a: [said("r1", "first"), written] });
});

it("draws a thread opened later from the row Fleet kept, and an older frame that arrives mid-read does not replace it", async () => {
  const sent: Sent[] = [];
  const kept = retroFrame({ stage: "written", stage_since: "2026-10-12T14:00:30Z", retro: 2 });
  const port = await fleet(sent, [record("a")], { a: [kept] });
  const { sessions, last } = host(port);
  await sessions.again(port);
  const reading = sessions.watch("a");
  sessions.row({ session_id: "a", row: retroFrame({ stage: "writing", stage_since: "2026-10-12T14:00:09Z", characters: 40 }) });
  await reading;
  expect(last("sessionThreads")).toEqual({ a: [kept] });
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
  await vi.waitFor(() => expect(opened).toEqual([["a", "Findings", "http://localhost:5173/"]]));
  await new Promise((done) => setTimeout(done, 20));
  expect(opened.length).toBe(1);
});

it("takes the sign-in addresses Fleet holds for a window once, and hands them to the window that opens, never to a row or the ledger", async () => {
  const sent: Sent[] = [];
  const url = "https://staging.example.com/app";
  const link = "https://staging.example.com/dev/login?token=abc";
  held = [link];
  const port = await fleet(sent, [record("a")]);
  const { sessions, published } = host(port);
  const opened: unknown[] = [];
  sessions.onWindow((id, _title, address, _takes, origins, signIn) => opened.push([id, address, origins, signIn]));
  await sessions.again(port);
  sessions.row({
    session_id: "a",
    row: { kind: "window", id: "w1", at: AT, title: "Staging", url, origins: ["https://login.example.org"] },
  });
  await vi.waitFor(() => expect(opened).toEqual([["a", url, ["https://login.example.org"], [link]]]));
  expect(sent.filter((one) => one.path === "/sessions/window/sign_in")).toEqual([
    { method: "POST", path: "/sessions/window/sign_in", body: { session_id: "a", url } },
  ]);
  expect(JSON.stringify(published)).not.toContain("token=abc");
});

it("opens a window with no sign-in where Fleet holds none or does not answer", async () => {
  const sent: Sent[] = [];
  const url = "https://staging.example.com/app";
  const port = await fleet(sent, [record("a")]);
  const { sessions } = host(port);
  const opened: unknown[] = [];
  sessions.onWindow((id, _title, _address, _takes, _origins, signIn) => opened.push([id, signIn]));
  await sessions.again(port);
  sessions.row({ session_id: "a", row: { kind: "window", id: "w1", at: AT, title: "Staging", url } });
  await vi.waitFor(() => expect(opened).toEqual([["a", []]]));
});

it("opens the window with its takes, from the row that shows it and from the ledger that reopens it, and reads a ledger that is not takes as none", async () => {
  const sent: Sent[] = [];
  const url = "http://localhost:5173/";
  const takes = [
    { name: "Every state", url },
    { name: "Needs you first", url: `${url}?scenario=needs-you` },
  ];
  const attached = (variations: string) => ({
    kind: "artifact",
    target: url,
    state: "standing" as const,
    detail: { form: "window", title: "Findings", variations },
    since: AT,
    changed_at: AT,
  });
  const port = await fleet(sent, [record("a", { attachments: [attached(JSON.stringify(takes))] }), record("b", { attachments: [attached(`[{"name":"x","url":"javascript:1"}]`)] })]);
  const { sessions } = host(port);
  const opened: unknown[] = [];
  sessions.onWindow((id, _title, address, withTakes) => opened.push([id, address, withTakes]));
  await sessions.again(port);
  sessions.row({ session_id: "a", row: { kind: "window", id: "w1", at: AT, title: "Findings", url, variations: takes } });
  await vi.waitFor(() => expect(opened.length).toBe(1));
  expect(sessions.openWindow("a", url)).toEqual({ ok: true });
  expect(sessions.openWindow("b", url)).toEqual({ ok: true });
  expect(opened).toEqual([["a", url, takes], ["a", url, takes], ["b", url, []]]);
});

it("reopens a window from the ledger with the origins it may pass through and no sign-in, and reads origins that are not a list as none", async () => {
  const sent: Sent[] = [];
  const url = "https://staging.example.com/app";
  const attached = (origins: string) => ({
    kind: "artifact",
    target: url,
    state: "standing" as const,
    detail: { form: "window", title: "Staging", origins },
    since: AT,
    changed_at: AT,
  });
  const port = await fleet(sent, [
    record("a", { attachments: [attached(JSON.stringify(["https://login.example.org"]))] }),
    record("b", { attachments: [attached(`{"login":"https://login.example.org"}`)] }),
  ]);
  const { sessions } = host(port);
  const opened: unknown[] = [];
  sessions.onWindow((id, _title, _address, _takes, origins, signIn) => opened.push([id, origins, signIn]));
  await sessions.again(port);
  expect(sessions.openWindow("a", url)).toEqual({ ok: true });
  expect(sessions.openWindow("b", url)).toEqual({ ok: true });
  expect(opened).toEqual([["a", ["https://login.example.org"], []], ["b", [], []]]);
  expect(sent.some((one) => one.path === "/sessions/window/sign_in")).toBe(false);
});

it("brings the walk forward when a Session asks in it, once, and never for an answered question or one a read brought", async () => {
  const sent: Sent[] = [];
  const walk = "http://localhost:5173/";
  const asked = {
    kind: "window_ask",
    id: "q1",
    at: AT,
    question: "Beside or under?",
    place: { url: `${walk}findings`, selector: "main > h1" },
    window: walk,
  } as const;
  const port = await fleet(
    sent,
    [record("a", { attachments: [{ kind: "artifact", target: walk, state: "standing", detail: { form: "window", title: "Findings" }, since: AT, changed_at: AT }] })],
    { a: [asked] },
  );
  const { sessions } = host(port);
  const forward: string[][] = [];
  sessions.onAsked((id, title, window, place) => forward.push([id, title, window, place]));
  await sessions.again(port);
  await sessions.watch("a");
  sessions.row({ session_id: "a", row: asked });
  sessions.row({ session_id: "a", row: { ...asked, id: "q2", answer: "Under" } });
  expect(forward).toEqual([]);
  sessions.row({ session_id: "a", row: { ...asked, id: "q3" } });
  sessions.row({ session_id: "a", row: { ...asked, id: "q3" } });
  await vi.waitFor(() => expect(forward).toEqual([["a", "Findings", walk, `${walk}findings`]]));
  await new Promise((done) => setTimeout(done, 20));
  expect(forward.length).toBe(1);
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
  await sessions.begin({ session_id: "a", delivery: "keep_local" });
  await sessions.end("a");
  await sessions.windowClosed({ session_id: "a", url: "http://localhost:5173/" });
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual([
    "POST /sessions/start",
    "POST /sessions/message",
    "POST /sessions/ask/answer",
    "POST /sessions/waiting/answer",
    "POST /sessions/tune",
    "POST /sessions/begin",
    "POST /sessions/close",
    "POST /sessions/window/closed",
  ]);
  expect(sent[1]!.body).toEqual({ session_id: "a", text: "hi", attachments: [{ name: "n.png", media_type: "image/png", data: "AAAA" }] });
  expect(sent[2]!.body).toEqual({ session_id: "a", call: "c1", answer: "allow_and_remember" });
  expect(sent[3]!.body).toEqual({ session_id: "a", item_id: "ask:c1", choice: 1 });
  expect(sent[5]!.body).toEqual({ session_id: "a", delivery: "keep_local" });
  expect(sent[6]!.body).toEqual({ session_id: "a" });
  expect(sent[7]!.body).toEqual({ session_id: "a", url: "http://localhost:5173/" });
  expect(last("sessions")).toMatchObject({ sessions: [{ id: "a", title: "Fleet's answer" }] });
});

it("pauses and resumes a session on their own routes with the session named", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions } = host(port);
  await sessions.pause("a");
  await sessions.resume("a");
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/pause", "POST /sessions/resume"]);
  expect(sent.map((one) => one.body)).toEqual([{ session_id: "a" }, { session_id: "a" }]);
});

it("turns Auto land on and off on its own route, naming the session as id", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions } = host(port);
  await sessions.setAutoLand("a", true);
  await sessions.setAutoLand("a", false);
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/auto_land", "POST /sessions/auto_land"]);
  expect(sent.map((one) => one.body)).toEqual([{ id: "a", on: true }, { id: "a", on: false }]);
});

it("reads a Job's MCP servers on the query string and acts on one on its own route, folding nothing", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a b")]);
  const { sessions, published } = host(port);
  const read = await sessions.mcp("a b");
  const acted = await sessions.actOnMcp("a b", "linear", "sign_in");
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["GET /sessions/mcp?session_id=a%20b", "POST /sessions/mcp/act"]);
  expect(sent[1]!.body).toEqual({ session_id: "a b", server: "linear", act: "sign_in" });
  expect(read).toEqual({ ok: true, value: { servers: [{ name: "linear", state: "needs_sign_in", tools: 0 }] } });
  expect(acted).toMatchObject({ ok: true, value: { sign_in: { url: "https://linear.example/authorize", callback_expected: true } } });
  expect(published).toEqual([]);
  expect(await host(null).sessions.mcp("a")).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
  expect(await host(null).sessions.actOnMcp("a", "linear", "reconnect")).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
});

it("retries a failed hand off on its own route with the old Job named", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions } = host(port);
  await sessions.retryHandoff("a");
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/handoff/retry"]);
  expect(sent[0]!.body).toEqual({ session_id: "a" });
});

it("sends a pinned note to its own route, folds the record Fleet answers and hands the candidates back", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions, last } = host(port);
  const note = { session_id: "a", said: "too wide", address: "http://localhost:5173/", place: { selector: "li", element: { tag: "li", text: "x" }, location: "/usage" }, state: {} };
  const done = await sessions.sendNote(note);
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/note"]);
  expect(sent[0]!.body).toEqual(note);
  expect(done).toMatchObject({ ok: true, value: { candidates: [{ file: "src/Rows.tsx", line: 171 }] } });
  expect(last("sessions")).toMatchObject({ sessions: [{ id: "a", title: "Noted" }] });
  expect(await host(null).sessions.sendNote(note)).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
});

it("starts a Mod Job on its own route with the note as given, and folds no record", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions, published } = host(port);
  const start = { title: "Usage meter", note: { said: "", address: "file:///index.html", place: { selector: "li", element: { tag: "li", text: "x" }, location: "/usage" }, state: { selected: [] } } };
  const done = await sessions.startMod(start);
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /mods/session"]);
  expect(sent[0]!.body).toEqual(start);
  expect(done).toEqual({ ok: true, value: { session_id: "mod-1", title: "Usage meter" } });
  expect(published).toEqual([]);
  expect(await host(null).sessions.startMod(start)).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
});

it("sends a walk's notes together to their own route, folds the record and hands back one candidate list a note", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions, last } = host(port);
  const one = { said: "too wide", address: "http://localhost:5173/", place: { selector: "li", element: { tag: "li", text: "x" }, location: "/usage" }, state: {} };
  const notes = { session_id: "a", notes: [one, { ...one, said: "and this", address: "http://localhost:5174/" }] };
  const done = await sessions.sendNotes(notes);
  expect(sent.map((each) => `${each.method} ${each.path}`)).toEqual(["POST /sessions/notes"]);
  expect(sent[0]!.body).toEqual(notes);
  expect(done).toMatchObject({ ok: true, value: { candidates: [[{ file: "src/Rows.tsx", line: 171 }], []] } });
  expect(last("sessions")).toMatchObject({ sessions: [{ id: "a", title: "Noted twice" }] });
  expect(await host(null).sessions.sendNotes(notes)).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
});

it("claims a pull request on its own route with the session named, and holds nothing of the answer", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions, published } = host(port);
  const claimed = await sessions.claimPullRequest({ number: 1823, session_id: "a" });
  expect(claimed.ok).toBe(true);
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/claim_pull_request"]);
  expect(sent[0]!.body).toEqual({ number: 1823, session_id: "a" });
  expect(published).toEqual([]);
  expect(await host(null).sessions.claimPullRequest({ number: 1823, session_id: "a" })).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
});

it("claims an issue on its own route with the session and its pull requests named", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions, published } = host(port);
  const claimed = await sessions.claimIssue({ issue: "ABC-12", session_id: "a", pull_requests: [9, 4] });
  expect(claimed.ok).toBe(true);
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/claim_issue"]);
  expect(sent[0]!.body).toEqual({ issue: "ABC-12", session_id: "a", pull_requests: [9, 4] });
  expect(published).toEqual([]);
  expect(await host(null).sessions.claimIssue({ issue: "7", session_id: "a" })).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
});

it("settles a follow-up on its own route, naming the Session, and folds nothing of the answer", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions, published } = host(port);
  const settled = await sessions.settleLater({ id: "later-2", how: "dropped", session_id: "a" });
  expect(settled.ok).toBe(true);
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/settle_later"]);
  expect(sent[0]!.body).toEqual({ id: "later-2", how: "dropped", session_id: "a" });
  expect(published).toEqual([]);
  expect(await host(null).sessions.settleLater({ id: "later-2", how: "done", session_id: "a" })).toMatchObject({ ok: false, outcome: { why: "not_connected" } });
});

// Fleet answers `{ told }` here, not the record: folded, it sat in the list with no `attachments` and broke every redraw.
it("tells Fleet a window could not show its page, and folds nothing of the answer", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions, published } = host(port);
  const told = await sessions.windowFailed({ session_id: "a", url: "http://localhost:5173/", why: "refused" });
  expect(told).toEqual({ ok: true, value: { told: true } });
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/window/failed"]);
  expect(published).toEqual([]);
});

it("writes a Session's retro on its own route and answers when it is written", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, [record("a")]);
  const { sessions } = host(port);
  expect(await sessions.retro("a")).toEqual({ ok: true });
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/a/retro"]);
  expect(await host(null).sessions.retro("a")).toMatchObject({ ok: false, why: "not_connected" });
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
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual([
    "GET /pull_requests/storefront/1847",
    "POST /pull_requests/storefront/1847/ready",
    "POST /pull_requests/storefront/1847/merge",
    "POST /pull_requests/storefront/1847/auto_merge",
  ]);
});

it("presses a pull request no Session holds by its repository's Manifest", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, []);
  const { sessions } = host(port);
  sent.length = 0;
  await sessions.pressInRepository("storefront", 2405, "merge");
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /pull_requests/storefront/2405/merge"]);
  expect(await host(null).sessions.pressInRepository("storefront", 2405, "merge")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
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

it("starts a run, reads its socket to the exit, and hands a refusal back", async () => {
  const sent: Sent[] = [];
  const sockets = new WebSocketServer({ noServer: true });
  const server = createServer((request, response) => {
    let raw = "";
    request.on("data", (chunk) => (raw += String(chunk)));
    request.on("end", () => {
      sent.push({ method: request.method ?? "", path: request.url ?? "", body: raw === "" ? undefined : JSON.parse(raw) });
      const refused = (JSON.parse(raw) as { session_id?: string }).session_id === "ended";
      response.writeHead(refused ? 409 : 202, { "content-type": "application/json" });
      response.end(JSON.stringify(refused ? { code: "fleet.session_ended", message: "That Session has ended." } : { run_id: "r1" }));
    });
  });
  server.on("upgrade", (request, socket, head) => {
    sent.push({ method: "UPGRADE", path: request.url ?? "", body: undefined });
    sockets.handleUpgrade(request, socket, head, (ws) => {
      ws.send(JSON.stringify({ kind: "opened", protocol_id: "p", run_id: "r1" }));
      ws.send(JSON.stringify({ kind: "output", text: "hi\r\n" }));
      ws.send(JSON.stringify({ kind: "exited", code: 0 }));
      ws.close();
    });
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  const port = (server.address() as AddressInfo).port;
  const { sessions } = host(port);
  const heard: SessionRunMessage[] = [];
  const closed = new Promise<void>((done) =>
    sessions.onRun((runId, message) => {
      expect(runId).toBe("r1");
      heard.push(message);
      if (message.kind === "exited") done();
    }),
  );
  expect(await sessions.startRun({ session_id: "a", command: "echo hi", cols: 100, rows: 30 })).toEqual({ ok: true, value: { run_id: "r1" } });
  await closed;
  expect(sent[0]?.body).toEqual({ session_id: "a", command: "echo hi", cols: 100, rows: 30 });
  expect(heard.map((one) => one.kind)).toEqual(["opened", "output", "exited"]);
  expect(heard[2]).toEqual({ kind: "exited", code: 0 });
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual(["POST /sessions/run/start", "UPGRADE /sessions/run/r1/observe"]);
  expect(await sessions.startRun({ session_id: "ended", command: "ls" })).toMatchObject({ ok: false, outcome: { ok: false, why: "refused" } });
  expect(await host(null).sessions.startRun({ session_id: "a", command: "ls" })).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  sockets.close();
});

it("types into a run and resizes its terminal, and says so where nothing is connected", async () => {
  const sent: Sent[] = [];
  const server = createServer((request, response) => {
    let raw = "";
    request.on("data", (chunk) => (raw += String(chunk)));
    request.on("end", () => {
      sent.push({ method: request.method ?? "", path: request.url ?? "", body: JSON.parse(raw) });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify({ run_id: "r1" }));
    });
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  const { sessions } = host((server.address() as AddressInfo).port);
  expect(await sessions.writeRun({ run_id: "r1", data: "ls\r" })).toEqual({ ok: true, value: { run_id: "r1" } });
  expect(await sessions.resizeRun({ run_id: "r1", cols: 100, rows: 30 })).toEqual({ ok: true, value: { run_id: "r1" } });
  expect(sent).toEqual([
    { method: "POST", path: "/sessions/run/write", body: { run_id: "r1", data: "ls\r" } },
    { method: "POST", path: "/sessions/run/resize", body: { run_id: "r1", cols: 100, rows: 30 } },
  ]);
  expect(await host(null).sessions.writeRun({ run_id: "r1", data: "x" })).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
});

/** A Fleet that opens shells and blocks by the run ids given, each socket sending its run's messages and closing. */
async function shellFleet(opens: string[], runs: Record<string, SessionRunMessage[]>): Promise<number> {
  const sockets = new WebSocketServer({ noServer: true });
  const server = createServer((request, response) => {
    request.resume();
    request.on("end", () => {
      response.writeHead(request.url === "/sessions/run/start" ? 202 : 200, { "content-type": "application/json" });
      response.end(JSON.stringify({ run_id: request.url === "/sessions/run/start" ? "b1" : opens.shift() }));
    });
  });
  server.on("upgrade", (request, socket, head) => {
    const runId = decodeURIComponent(/\/sessions\/run\/([^/]+)\/observe/.exec(request.url ?? "")?.[1] ?? "");
    sockets.handleUpgrade(request, socket, head, (ws) => {
      for (const message of runs[runId] ?? []) ws.send(JSON.stringify(message));
      ws.close();
    });
  });
  server.on("close", () => sockets.close());
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  return (server.address() as AddressInfo).port;
}

const endOf = (sessions: SessionsHost, runId: string) =>
  new Promise<void>((done) => sessions.onRun((id, message) => id === runId && message.kind === "exited" && done()));

it("keeps what a shell printed, its last 256 KiB cut at a line, for a window that comes to it late", async () => {
  const lines = Array.from({ length: 30_000 }, (_, at) => `line ${String(at).padStart(5, "0")}\r\n`);
  const chunks = Array.from({ length: 30 }, (_, at) => lines.slice(at * 1000, (at + 1) * 1000).join(""));
  const port = await shellFleet(["r1"], { r1: [{ kind: "opened", protocol_id: "p", run_id: "r1" }, ...chunks.map((text) => ({ kind: "output" as const, text })), { kind: "exited", code: 0 }] });
  const { sessions } = host(port);
  const ended = endOf(sessions, "r1");
  await sessions.openTerminal({ session_id: "a" });
  await ended;
  const kept = sessions.soFar("r1") ?? "";
  expect(kept.length).toBeLessThanOrEqual(256 * 1024);
  expect(kept.length).toBeGreaterThan(256 * 1024 - 20);
  expect(lines.join("").endsWith(kept)).toBe(true);
  expect(kept.startsWith("line ")).toBe(true);
  expect(sessions.soFar("nobody")).toBeNull();
});

it("lets an ended shell's output go when its Job's next shell opens, and a block's as soon as it ends", async () => {
  const port = await shellFleet(["r1"], {
    r1: [{ kind: "output", text: "one\r\n" }, { kind: "exited", code: 0 }],
    r2: [{ kind: "output", text: "two\r\n" }, { kind: "exited", code: 0 }],
    b1: [{ kind: "output", text: "hi\r\n" }, { kind: "exited", code: 0 }],
  });
  const { sessions } = host(port);
  let ended = endOf(sessions, "r1");
  await sessions.openTerminal({ session_id: "a" });
  await ended;
  expect(sessions.soFar("r1")).toBe("one\r\n");
  expect(sessions.shellNow("a")).toEqual({ runId: "r1", ended: true });
  expect(sessions.shellNow("b")).toBeNull();
  ended = endOf(sessions, "b1");
  await sessions.startRun({ session_id: "a", command: "echo hi" });
  await ended;
  expect(sessions.soFar("b1")).toBeNull();
  expect(sessions.soFar("r1")).toBe("one\r\n");
  ended = endOf(sessions, "r2");
  sessions.terminal("opened", { session_id: "a", run_id: "r2" });
  await ended;
  expect(sessions.soFar("r1")).toBeNull();
  expect(sessions.soFar("r2")).toBe("two\r\n");
  expect(sessions.shellNow("a")).toEqual({ runId: "r2", ended: true });
});

it("holds the twenty threads opened last, and reads one let go again when it is reopened", async () => {
  const sent: Sent[] = [];
  const ids = Array.from({ length: THREADS_HELD + 1 }, (_, at) => `s${at}`);
  const port = await fleet(sent, ids.map((id) => record(id)), Object.fromEntries(ids.map((id) => [id, [said(`${id}r`, id)]])));
  const { sessions, last } = host(port);
  await sessions.again(port);
  for (const id of ids) await sessions.watch(id);
  const held = last("sessionThreads") as Record<string, SessionRow[]>;
  expect(Object.keys(held)).toEqual(ids.slice(1));
  // A thread still held is drawn from here, and a row for the one let go is dropped.
  const reads = sent.filter((one) => one.path.startsWith("/sessions/one")).length;
  await sessions.watch("s5");
  expect(sent.filter((one) => one.path.startsWith("/sessions/one")).length).toBe(reads);
  sessions.row({ session_id: "s0", row: said("late", "after it went") });
  expect(Object.keys(last("sessionThreads") as object)).toEqual(ids.slice(1));
  await sessions.watch("s0");
  expect(sent.filter((one) => one.path.startsWith("/sessions/one")).length).toBe(reads + 1);
  expect(Object.keys(last("sessionThreads") as object)).toHaveLength(THREADS_HELD);
});

describe("the threads kept on disk", () => {
  const folder = async (): Promise<string> => {
    const dir = await mkdtemp(join(tmpdir(), "session-threads-"));
    kept.push(dir);
    return dir;
  };
  const gated = (): (() => void) => {
    let open = () => {};
    gate = new Promise<void>((done) => (open = done));
    return open;
  };

  it("says which threads are being read while Fleet has not answered, and none after", async () => {
    const sent: Sent[] = [];
    const port = await fleet(sent, [record("a")], { a: [said("r1", "first")] });
    const { sessions, last } = host(port);
    await sessions.again(port);
    const open = gated();
    const reading = sessions.watch("a");
    expect(last("sessionThreadsReading")).toEqual(["a"]);
    open();
    await reading;
    expect(last("sessionThreadsReading")).toEqual([]);
    // A read that again() starts is said the same way.
    const again = gated();
    const reread = sessions.again(port);
    await vi.waitFor(() => expect(last("sessionThreadsReading")).toEqual(["a"]));
    again();
    await reread;
    expect(last("sessionThreadsReading")).toEqual([]);
  });

  it("draws the copy on disk before Fleet answers, follows the stream on it, and is replaced by the answer", async () => {
    const dir = await folder();
    await writeFile(join(dir, "a.json"), JSON.stringify([said("r1", "old"), { kind: "window", id: "w1", at: AT }]));
    const sent: Sent[] = [];
    const port = await fleet(sent, [record("a")], { a: [said("r1", "new")] });
    const { sessions, last } = host(port, dir);
    await sessions.again(port);
    const open = gated();
    const reading = sessions.watch("a");
    await vi.waitFor(() => expect(last("sessionThreads")).toMatchObject({ a: [said("r1", "old"), { id: "w1" }] }));
    expect(last("sessionThreadsReading")).toEqual(["a"]);
    sessions.row({ session_id: "a", row: said("r2", "live") });
    expect(last("sessionThreads")).toMatchObject({ a: [said("r1", "old"), { id: "w1" }, said("r2", "live")] });
    open();
    await reading;
    expect(last("sessionThreads")).toEqual({ a: [said("r1", "new"), said("r2", "live")] });
    expect(last("sessionThreadsReading")).toEqual([]);
  });

  it("reads a thread opened before the port was known once the connection comes, drawing the disk copy first", async () => {
    const dir = await folder();
    await writeFile(join(dir, "a.json"), JSON.stringify([said("r1", "old")]));
    const sent: Sent[] = [];
    const port = await fleet(sent, [record("a")], { a: [said("r1", "new")] });
    let at: number | null = null;
    const published: Partial<BridgeState>[] = [];
    const sessions = new SessionsHost((change) => published.push(change), () => at);
    sessions.keepIn(dir);
    await sessions.watch("a");
    expect(sent).toEqual([]);
    at = port;
    const open = gated();
    const connecting = sessions.again(port);
    const last = <K extends keyof BridgeState>(key: K) => [...published].reverse().find((one) => key in one)?.[key];
    await vi.waitFor(() => expect(last("sessionThreads")).toEqual({ a: [said("r1", "old")] }));
    open();
    await connecting;
    expect(last("sessionThreads")).toEqual({ a: [said("r1", "new")] });
    expect(last("sessionThreadsReading")).toEqual([]);
  });

  it("leaves the copy on disk drawn when Fleet's read fails", async () => {
    const dir = await folder();
    await writeFile(join(dir, "a.json"), JSON.stringify([said("r1", "old")]));
    const server = createServer((request, response) => {
      request.resume();
      request.on("end", () => {
        response.writeHead(500, { "content-type": "application/json" });
        response.end(JSON.stringify({ code: "fleet.busy", message: "busy" }));
      });
    });
    listening = server;
    await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
    const { sessions, last } = host((server.address() as AddressInfo).port, dir);
    await sessions.watch("a");
    expect(last("sessionThreads")).toEqual({ a: [said("r1", "old")] });
    expect(last("sessionThreadsReading")).toEqual([]);
  });

  it("writes a changed thread once the debounce has passed, and keeps only the newest THREADS_HELD files", async () => {
    const dir = await folder();
    await mkdir(dir, { recursive: true });
    for (let at = 0; at < THREADS_HELD + 5; at++) {
      const file = join(dir, `old${at}.json`);
      await writeFile(file, "[]");
      await utimes(file, new Date(2020, 0, 1 + at), new Date(2020, 0, 1 + at));
    }
    const sent: Sent[] = [];
    const port = await fleet(sent, [record("a")], { a: [said("r1", "first")] });
    const { sessions } = host(port, dir);
    await sessions.again(port);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    await sessions.watch("a");
    await vi.advanceTimersByTimeAsync(1999);
    expect(await readdir(dir)).not.toContain("a.json");
    await vi.advanceTimersByTimeAsync(1);
    await vi.waitFor(async () => expect(await readdir(dir)).toHaveLength(THREADS_HELD));
    expect(JSON.parse(await readFile(join(dir, "a.json"), "utf8"))).toEqual([said("r1", "first")]);
    // The oldest went, the newest stayed.
    expect(await readdir(dir)).not.toContain("old0.json");
    expect(await readdir(dir)).toContain(`old${THREADS_HELD + 4}.json`);
  });

  it("writes nothing once the window is gone", async () => {
    const dir = await folder();
    const sent: Sent[] = [];
    const port = await fleet(sent, [record("a")], { a: [said("r1", "first")] });
    const { sessions } = host(port, dir);
    await sessions.again(port);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    await sessions.watch("a");
    sessions.close();
    await vi.advanceTimersByTimeAsync(5000);
    expect(await readdir(dir)).toEqual([]);
  });
});

describe("a window shown while Bridge was away", () => {
  const url = "https://staging.example.com/app";
  const shownAt = Date.parse(AT);
  const showing = (extra: Partial<SessionRecord> = {}) =>
    record("a", {
      attachments: [{ kind: "artifact", target: url, state: "standing", detail: { form: "window", title: "Staging" }, since: AT, changed_at: AT }],
      ...extra,
    });
  const row = (change: Record<string, unknown> = {}) => ({ kind: "window", id: "w1", at: AT, title: "Staging", url, ...change }) as SessionRow;

  async function reopened(age: number, rows: SessionRow[], session = showing()) {
    // Several of these run in one test: the Fleet before is let go of, as `afterEach` lets go of the last.
    const before = listening;
    if (before !== null) await new Promise<void>((done) => before.close(() => done()));
    const port = await fleet([], [session], { a: rows });
    const opened: string[][] = [];
    const sessions = new SessionsHost(() => {}, () => port, () => shownAt + age);
    sessions.onWindow((id, title, address) => opened.push([id, title, address]));
    await sessions.again(port);
    return { sessions, port, opened };
  }

  it("opens when it was shown within fifteen minutes", async () => {
    const { opened } = await reopened(REOPEN_WITHIN_MS - 1000, [row()]);
    await vi.waitFor(() => expect(opened).toEqual([["a", "Staging", url]]));
  });

  it("opens at the edge of the quarter hour and not a moment after", async () => {
    const edge = await reopened(REOPEN_WITHIN_MS, [row()]);
    await vi.waitFor(() => expect(edge.opened).toHaveLength(1));
    const past = await reopened(REOPEN_WITHIN_MS + 1, [row()]);
    await new Promise((done) => setTimeout(done, 40));
    expect(past.opened).toEqual([]);
  });

  it("stays closed when it was shown longer ago, as a walk from before a restart", async () => {
    const { opened } = await reopened(REOPEN_WITHIN_MS * 4, [row()]);
    await new Promise((done) => setTimeout(done, 40));
    expect(opened).toEqual([]);
  });

  it("stays closed when the row has an end, or the session is not live", async () => {
    const ended = await reopened(1000, [row({ ended_at: AT })]);
    const gone = await reopened(1000, [row()], showing({ state: "ended" }));
    await new Promise((done) => setTimeout(done, 40));
    expect(ended.opened).toEqual([]);
    expect(gone.opened).toEqual([]);
  });

  it("opens once in a process, however many times Bridge reconnects", async () => {
    const { sessions, port, opened } = await reopened(1000, [row()]);
    await vi.waitFor(() => expect(opened).toHaveLength(1));
    await sessions.again(port);
    await sessions.again(port);
    await new Promise((done) => setTimeout(done, 40));
    expect(opened).toHaveLength(1);
  });

  it("opens the newest shown page and not the ones before it", async () => {
    const { opened } = await reopened(1000, [row({ id: "w1", url: "https://old.example.com/" }), row({ id: "w2" })]);
    await vi.waitFor(() => expect(opened).toEqual([["a", "Staging", url]]));
  });
});

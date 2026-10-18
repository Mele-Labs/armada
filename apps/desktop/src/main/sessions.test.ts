// What main holds of Sessions, read off a real listener: the list, a thread opened and followed, and
// the routes each act is sent to. **What crosses the wire is under test**, so a route named wrongly or
// a repository taken from the wrong place fails here rather than on a person's press.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it, vi } from "vitest";
import { WebSocketServer } from "ws";

import type { RetroRow, SessionRecord, SessionRow, SessionRunMessage } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { REOPEN_WITHIN_MS, SessionsHost, THREADS_HELD, withRow, withSession } from "./sessions";

let listening: Server | null = null;
/** What the Fleet answers a take of a window's sign-in with. */
let held: string[] = [];

afterEach(async () => {
  held = [];
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
      if (url.pathname === "/sessions/window/failed") return answer(200, { told: true });
      if (url.pathname === "/sessions/one") {
        const id = url.searchParams.get("session_id") ?? "";
        return answer(200, { session: sessions.find((one) => one.id === id), rows: threads[id] ?? [] });
      }
      if (url.pathname === "/sessions/window/sign_in") return answer(200, { sign_in: held });
      if (url.pathname === "/sessions/note") return answer(202, { session: { ...sessions[0], title: "Noted" }, candidates: [{ file: "src/Rows.tsx", line: 171, why: "the label is on this line", text: "<li>" }] });
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
  await sessions.end("a");
  await sessions.windowClosed({ session_id: "a", url: "http://localhost:5173/" });
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual([
    "POST /sessions/start",
    "POST /sessions/message",
    "POST /sessions/ask/answer",
    "POST /sessions/waiting/answer",
    "POST /sessions/tune",
    "POST /sessions/close",
    "POST /sessions/window/closed",
  ]);
  expect(sent[1]!.body).toEqual({ session_id: "a", text: "hi", attachments: [{ name: "n.png", media_type: "image/png", data: "AAAA" }] });
  expect(sent[2]!.body).toEqual({ session_id: "a", call: "c1", answer: "allow_and_remember" });
  expect(sent[3]!.body).toEqual({ session_id: "a", item_id: "ask:c1", choice: 1 });
  expect(sent[5]!.body).toEqual({ session_id: "a" });
  expect(sent[6]!.body).toEqual({ session_id: "a", url: "http://localhost:5173/" });
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

// A fixture Phone Gateway: answers the phone's routes for one Mac from `wire.ts`. Shared by the fixture
// server (`vite.config.ts`, mode `fixture`) and the acceptance test, so both read the same Macs.
// It checks that a call is signed, not that the signature is good: there is no pairing behind it.

import type { AnswerWaiting, DismissWaiting, SessionRecord, SessionRow, SessionThread, TuneSession } from "@armada/protocol";

import type { PhoneSession } from "../gateway";
import type { FixtureMac } from "./wire";

export type Answered = { status: number; type?: string; body: string };

/** What the fixture was sent, oldest first, for a test to read. */
export type Posted = { path: string; body: unknown };

const json = (value: unknown, status = 200): Answered => ({ status, type: "application/json", body: JSON.stringify(value) });
const said = (status: number, sentence: string): Answered => ({ status, type: "text/plain", body: sentence });

export function answer(mac: FixtureMac, posted: Posted[], method: string, target: string, body: string, signed: boolean): Answered {
  const url = new URL(target, "http://fixture");
  const path = url.pathname;
  if (method === "POST" && path === "/pair") return json({ device_id: `fixture-${mac.name}` }, 202);
  if (!path.startsWith("/api/")) return said(404, "Not found.");
  if (!signed) return said(400, "This request is not signed.");
  const id = url.searchParams.get("session_id") ?? "";
  if (method === "GET") {
    switch (path) {
      case "/api/machine":
        return json({ name: mac.name });
      case "/api/needs":
        return json({ sessions: mac.needs });
      case "/api/sessions":
        return json({ sessions: mac.live });
      case "/api/repositories":
        return json(mac.repositories);
      case "/api/plan_usage":
        return json(mac.usage);
      case "/api/models":
        return json(mac.models);
      case "/api/sessions/one": {
        const thread = mac.threads[id];
        return thread === undefined ? said(404, "No Job has that id on this Mac.") : json(thread);
      }
      case "/api/sessions/shown": {
        const text = mac.shown[id]?.[url.searchParams.get("path") ?? ""];
        return text === undefined ? said(404, "That Job did not show that file.") : json({ path: url.searchParams.get("path"), text });
      }
      case "/api/push/key":
        return said(404, "Push is not served by the fixture.");
    }
    return said(404, "Not found.");
  }
  if (method === "POST") {
    const sent = body === "" ? undefined : (JSON.parse(body) as Record<string, unknown>);
    posted.push({ path, body: sent });
    if (path === "/api/sessions") return json({ session: mac.live[0] }, 201);
    return act(mac, path, sent ?? {}) ?? { status: 204, body: "" };
  }
  return said(405, "Not allowed.");
}

const now = () => new Date().toISOString();
let made = 0;
const rowId = (id: string) => `${id}-${Date.now()}-${made++}`;

/** The Job's list row, brought in line with its thread: what it waits on, whether a turn runs, and the Mac's Needs you. */
function settle(mac: FixtureMac, thread: SessionThread): void {
  const record = thread.session;
  const at = mac.live.findIndex((one) => one.id === record.id);
  if (record.state === "ended") {
    if (at >= 0) mac.live.splice(at, 1);
  } else if (at >= 0) {
    const was = mac.live[at]!;
    const waiting = (record.waiting_for ?? []).length > 0 || record.hosted?.asked !== undefined;
    const next: PhoneSession = { ...was, held: record.hosted?.start !== undefined, working: record.hosted?.turn.state === "working", waiting };
    if (waiting) next.waiting_since = was.waiting_since ?? now();
    else delete next.waiting_since;
    mac.live[at] = next;
  }
  mac.needs = mac.live.filter((one) => one.waiting);
}

const hosted = (record: SessionRecord) => (record.hosted ??= { turn: { state: "idle" }, mode: "auto", running: true });
const drop = (record: SessionRecord, item: string) => (record.waiting_for = (record.waiting_for ?? []).filter((one) => one.id !== item));
const push = (thread: SessionThread, row: SessionRow) => thread.rows.push(row);

/** What each Job act does to the fixture, as Fleet would. Answers undefined for 204 with no body. */
function act(mac: FixtureMac, path: string, sent: Record<string, unknown>): Answered | undefined {
  const id = String(sent["session_id"] ?? "");
  const thread = mac.threads[id];
  if (!path.startsWith("/api/sessions/") || path === "/api/sessions/answer") return undefined;
  if (thread === undefined) return said(404, "No Job has that id on this Mac.");
  const record = thread.session;
  const job = hosted(record);
  const answered = (): Answered => {
    settle(mac, thread);
    return json(record);
  };
  switch (path) {
    case "/api/sessions/begin": {
      const message = typeof sent["message"] === "string" ? sent["message"] : (job.start?.message ?? "");
      delete job.start;
      drop(record, "first_message");
      job.turn = { state: "working" };
      job.running = true;
      push(thread, { kind: "message", id: rowId(id), at: now(), from: { kind: "you" }, text: message });
      return answered();
    }
    case "/api/sessions/message":
      push(thread, { kind: "message", id: rowId(id), at: now(), from: { kind: "you" }, text: String(sent["text"] ?? "") });
      job.turn = { state: "working" };
      return answered();
    case "/api/sessions/stop":
      job.turn = { state: "idle" };
      return answered();
    case "/api/sessions/pause":
      job.turn = { state: "idle" };
      job.paused = { at: now(), resuming: false };
      return answered();
    case "/api/sessions/resume":
      delete job.paused;
      return answered();
    case "/api/sessions/close":
      record.state = "ended";
      record.ended_at = now();
      job.turn = { state: "idle" };
      return answered();
    case "/api/sessions/tune": {
      const tune = sent as TuneSession;
      if (tune.model !== undefined) job.model = tune.model;
      if (tune.effort !== undefined) job.effort = tune.effort;
      job.mode = tune.mode;
      return answered();
    }
    case "/api/sessions/run/start": {
      const command = String(sent["command"] ?? "");
      const item = (record.waiting_for ?? []).find((one) => one.act?.kind === "run" && one.act.target === command);
      if (item !== undefined) drop(record, item.id);
      push(thread, { kind: "ran", id: rowId(id), at: now(), command, code: 0, tail: "Merged 3 branches into preview.\nFleet restarted on preview-4e735d6.\nBridge reconnected." });
      settle(mac, thread);
      return json({ run_id: rowId("run") }, 202);
    }
    case "/api/sessions/waiting/answer":
    case "/api/sessions/waiting/dismiss": {
      const given = sent as AnswerWaiting & DismissWaiting;
      drop(record, given.item_id);
      const draft = given.item_id.startsWith("draft:") ? given.item_id.slice("draft:".length) : undefined;
      thread.rows = thread.rows.map((row) => {
        if (row.kind !== "draft" || row.id !== draft) return row;
        const how = path.endsWith("dismiss") ? "withdrawn" : given.fixed !== undefined ? "fixed" : given.text !== undefined ? "sent_back" : "approved";
        const words = given.text ?? given.reason;
        return { ...row, ...(given.fixed === undefined ? {} : { text: given.fixed }), settled: { how, ...(words === undefined ? {} : { words }), at: now() } };
      });
      if (draft !== undefined && !path.endsWith("dismiss")) job.turn = { state: "working" };
      return answered();
    }
    case "/api/sessions/fork": {
      const fork = `${id}-fork-${made++}`;
      const title = record.title ?? "Job";
      mac.threads[fork] = {
        session: { ...record, id: fork, started_at: now(), started: { by: { id, title } }, waiting_for: [], hosted: { ...job, turn: { state: "idle" } } },
        rows: thread.rows.filter((row) => row.kind === "message"),
      };
      thread.rows.push({ kind: "started", id: rowId(id), at: now(), session: { id: fork, title }, fork: true });
      const listed = mac.live.find((one) => one.id === id);
      const session: PhoneSession = { ...(listed ?? { kind: "hosted", started_at: now() }), id: fork, title, held: false, waiting: false, working: false, started_at: now(), from: { id, title } };
      delete session.waiting_since;
      mac.live.unshift(session);
      settle(mac, thread);
      return json({ session }, 201);
    }
  }
  return undefined;
}

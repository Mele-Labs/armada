// What crosses from the renderer for a pull request's notes is checked before Fleet is asked, and each call reaches Fleet's route.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, test } from "vitest";

import { noteSaveOf, noteSendOf, pullRequestOf } from "./pull-request-notes-channels";
import { PullRequestNotesHost } from "./pull-request-notes";

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

type Seen = { method: string; url: string; body: string };

async function fleet(body: unknown, into: Seen[]): Promise<PullRequestNotesHost> {
  const server = createServer((request, response) => {
    let text = "";
    request.on("data", (chunk) => (text += chunk));
    request.on("end", () => {
      into.push({ method: request.method ?? "", url: request.url ?? "", body: text });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address() as AddressInfo;
  return new PullRequestNotesHost(() => port);
}

test("a pull request is a manifest id and a positive whole number, and anything else is not", () => {
  expect(pullRequestOf("m", 7)).toEqual({ manifestId: "m", number: 7 });
  expect(pullRequestOf("", 7)).toBeNull();
  expect(pullRequestOf("m", "7")).toBeNull();
  expect(pullRequestOf("m", 0)).toBeNull();
  expect(pullRequestOf("m", 1.5)).toBeNull();
});

test("a note save carries a body and only well-formed places, and anything else is not", () => {
  expect(noteSaveOf({ path: "a.ts", line: 3, side: "left", body: "x" })).toEqual({ path: "a.ts", line: 3, side: "left", body: "x" });
  expect(noteSaveOf({ id: "n1", body: "x", junk: 1 })).toEqual({ id: "n1", body: "x" });
  expect(noteSaveOf({ side: "middle", body: "x" })).toBeNull();
  expect(noteSaveOf({ line: 0, body: "x" })).toBeNull();
  expect(noteSaveOf({ path: "a.ts" })).toBeNull();
  expect(noteSaveOf(null)).toBeNull();
});

test("a send names notes and a Job, and anything else is not", () => {
  expect(noteSendOf({ note_ids: ["n1"], session_id: "s1", title: "T" })).toEqual({ note_ids: ["n1"], session_id: "s1", title: "T" });
  expect(noteSendOf({ note_ids: [], session_id: "s1", title: "T" })).toBeNull();
  expect(noteSendOf({ note_ids: ["n1"], session_id: "", title: "T" })).toBeNull();
  expect(noteSendOf({ note_ids: [1], session_id: "s1", title: "T" })).toBeNull();
});

test("each call reaches its route with the manifest id encoded", async () => {
  const seen: Seen[] = [];
  const host = await fleet({ notes: [] }, seen);
  await expect(host.list("my repo", 7)).resolves.toEqual({ ok: true, value: { notes: [] } });
  await host.save("my repo", 7, { id: "n1", body: "x" });
  await host.remove("my repo", 7, "n 1");
  await host.send("my repo", 7, { note_ids: ["n1"], session_id: "s1", title: "T" });
  expect(seen).toEqual([
    { method: "GET", url: "/pull_request_notes/my%20repo/7", body: "" },
    { method: "POST", url: "/pull_request_notes/my%20repo/7", body: JSON.stringify({ id: "n1", body: "x" }) },
    { method: "POST", url: "/pull_request_notes/my%20repo/7/n%201/remove", body: "" },
    { method: "POST", url: "/pull_request_notes/my%20repo/7/sent", body: JSON.stringify({ note_ids: ["n1"], session_id: "s1", title: "T" }) },
  ]);
});

test("without a connection the answer is not_connected", async () => {
  await expect(new PullRequestNotesHost(() => null).list("m", 1)).resolves.toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
});

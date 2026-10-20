// What crosses from the renderer to run a shell line or reach a repository's flight controls is checked before Fleet is asked, a
// repository travels as an encoded query or in the body whole, and the wait for a shell line outlasts Fleet's own bound.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, test } from "vitest";

import { repositoryOf, repositorySaveOf, shellOf } from "./flight-controls-channels";
import { approvalOf, tileOf } from "./flight-controls-channels";
import { FlightControlsHost, SHELL_MS } from "./flight-controls";

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

type Seen = { method: string; url: string; body: string };

/** A Fleet answering 200 and `body` to every request, recording what arrived. */
async function fleet(body: unknown, into: Seen[]): Promise<FlightControlsHost> {
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
  return new FlightControlsHost(() => port);
}

const CONTROLS = { version: 1, job: [{ id: "my-specs", kind: "shell", name: "Run specs", text: "bin/rspec" }], deck: [] };

test("a Job id and a line are taken, and anything else is not", () => {
  expect(shellOf("s1", "pnpm typecheck")).toEqual({ sessionId: "s1", command: "pnpm typecheck" });
  expect(shellOf("", "ls")).toBeNull();
  expect(shellOf("s1", 3)).toBeNull();
  expect(shellOf(undefined, "ls")).toBeNull();
});

test("Bridge waits longer than Fleet's 120 seconds, so Fleet's timed_out is what arrives", () => {
  expect(SHELL_MS).toBeGreaterThan(120_000);
});

test("a repository is a root, and anything else is not", () => {
  expect(repositoryOf("/Users/user/lendinghome-monolith")).toBe("/Users/user/lendinghome-monolith");
  expect(repositoryOf("")).toBeNull();
  expect(repositoryOf(undefined)).toBeNull();
  expect(repositoryOf(["/Users/user/x"])).toBeNull();
});

test("a repository's save is a root and a file with both lists, and anything else is not", () => {
  expect(repositorySaveOf("/r", CONTROLS)).toEqual({ repository: "/r", controls: CONTROLS });
  expect(repositorySaveOf("", CONTROLS)).toBeNull();
  expect(repositorySaveOf("/r", { version: 1, job: [] })).toBeNull();
  expect(repositorySaveOf("/r", null)).toBeNull();
  expect(repositorySaveOf("/r", "[[job]]")).toBeNull();
});

test("a repository's file is read with its root as an encoded query, and saved whole in the body", async () => {
  const seen: Seen[] = [];
  const answer = { repository: "/Users/user/my repo&x", place: "this_mac", read: { what: "read", controls: CONTROLS } };
  const host = await fleet(answer, seen);
  await expect(host.getRepository("/Users/user/my repo&x")).resolves.toEqual({ ok: true, value: answer });
  await host.saveRepository("/Users/user/my repo&x", CONTROLS as never);
  expect(seen).toEqual([
    { method: "GET", url: "/repositories/flight_controls?repository=%2FUsers%2Fuser%2Fmy%20repo%26x", body: "" },
    { method: "POST", url: "/repositories/flight_controls", body: JSON.stringify({ repository: "/Users/user/my repo&x", controls: CONTROLS }) },
  ]);
});

test("a committed tile a press names is a root and an id, absent is none, and anything else is refused", () => {
  expect(tileOf({ repository: "/r", id: "run-tests" })).toEqual({ repository: "/r", id: "run-tests" });
  expect(tileOf(undefined)).toBeUndefined();
  expect(tileOf({ repository: "", id: "run-tests" })).toBeNull();
  expect(tileOf("run-tests")).toBeNull();
});

test("an approval names the tile and the hash shown, with a Job or without", () => {
  expect(approvalOf({ repository: "/r", tile_id: "t", command_hash: "h" })).toEqual({ repository: "/r", tile_id: "t", command_hash: "h" });
  expect(approvalOf({ repository: "/r", session_id: "s1", tile_id: "t", command_hash: "h" })).toEqual({ repository: "/r", session_id: "s1", tile_id: "t", command_hash: "h" });
  expect(approvalOf({ repository: "/r", tile_id: "t" })).toBeNull();
  expect(approvalOf({ repository: "/r", tile_id: "t", command_hash: "h", session_id: 3 })).toBeNull();
});

test("a read names the Job whose tree holds the committed tiles, a press its tile, and an approval goes whole in the body", async () => {
  const seen: Seen[] = [];
  const host = await fleet({}, seen);
  await host.getRepository("/r", "s 1");
  await host.runShell("s1", "ignored", { repository: "/r", id: "run-tests" });
  await host.approve({ repository: "/r", session_id: "s1", tile_id: "run-tests", command_hash: "h" });
  expect(seen).toEqual([
    { method: "GET", url: "/repositories/flight_controls?repository=%2Fr&session_id=s%201", body: "" },
    { method: "POST", url: "/sessions/shell", body: JSON.stringify({ session_id: "s1", command: "ignored", repository_tile: { repository: "/r", id: "run-tests" } }) },
    { method: "POST", url: "/repositories/flight_controls/approve", body: JSON.stringify({ repository: "/r", session_id: "s1", tile_id: "run-tests", command_hash: "h" }) },
  ]);
});

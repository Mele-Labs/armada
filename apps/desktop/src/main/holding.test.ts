// The slots list as a surface sees it across opening and closing: the last answer stays and is drawn
// at once, the fresh one replaces it, and a read that lands after the surface closed paints nothing.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { HeldWorktrees, WorktreesHeld } from "@armada/protocol";
import { HeldReader } from "./holding";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  server.closeAllConnections();
  await new Promise<void>((done) => server.close(() => done()));
});

const list = (...jobs: string[]): WorktreesHeld => ({ worktrees: jobs.map((job_id) => ({ job_id })) }) as unknown as WorktreesHeld;

type Answer = { status: number; body: unknown };

/** A Fleet whose `/worktrees` answers are released by the test, one per read, in the order asked. */
async function fleet(): Promise<{ port: number; reads: () => number; answer: (answer: Answer) => void }> {
  const waiting: ((answer: Answer) => void)[] = [];
  const early: Answer[] = [];
  let reads = 0;
  const server = createServer((request, response) => {
    reads += 1;
    const send = ({ status, body }: Answer) => {
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    };
    const ready = early.shift();
    if (ready !== undefined) send(ready);
    else waiting.push(send);
    request.resume();
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  return {
    port: (server.address() as AddressInfo).port,
    reads: () => reads,
    answer: (answer) => {
      const send = waiting.shift();
      if (send === undefined) early.push(answer);
      else send(answer);
    },
  };
}

const ok = (...jobs: string[]): Answer => ({ status: 200, body: list(...jobs) });
const broken: Answer = { status: 500, body: { code: "fleet.broken", message: "No." } };

function reader() {
  const published: HeldWorktrees[] = [];
  return { held: new HeldReader((one) => published.push(one)), published };
}

const read = (...jobs: string[]): HeldWorktrees => ({ state: "read", held: list(...jobs) });

const settled = () => new Promise((done) => setTimeout(done, 40));

it("says reading only until there has been an answer, then draws the list", async () => {
  const { port, answer } = await fleet();
  const { held, published } = reader();
  const opening = held.want(port, true);
  expect(published).toEqual([{ state: "reading" }]);
  answer(ok("a"));
  await opening;
  expect(published).toEqual([{ state: "reading" }, read("a")]);
});

it("draws the list read last the moment a surface reopens, before the fresh read answers", async () => {
  const { port, answer } = await fleet();
  const { held, published } = reader();
  answer(ok("a"));
  await held.want(port, true);
  await held.want(port, false);
  published.length = 0;
  const reopening = held.want(port, true);
  // The read is out and unanswered, and the list is already up.
  expect(published).toEqual([read("a")]);
  answer(ok("a", "b"));
  await reopening;
  expect(published).toEqual([read("a"), read("a", "b")]);
});

it("keeps the list when a refresh fails, and says failed only where there never was one", async () => {
  const { port, answer } = await fleet();
  const { held, published } = reader();
  answer(broken);
  await held.want(port, true);
  expect(published.at(-1)?.state).toBe("failed");
  answer(ok("a"));
  await held.again(port);
  published.length = 0;
  answer(broken);
  await held.again(port);
  expect(published).toEqual([read("a")]);
  await held.want(port, false);
  published.length = 0;
  answer(broken);
  await held.want(port, true);
  expect(published).toEqual([read("a"), read("a")]);
});

it("publishes nothing for an answer that lands after the surface closed, and draws it on the next open", async () => {
  const { port, answer } = await fleet();
  const { held, published } = reader();
  answer(ok("a"));
  await held.want(port, true);
  const refreshing = held.again(port);
  await held.want(port, false);
  published.length = 0;
  answer(ok("a", "b"));
  await refreshing;
  expect(published).toEqual([]);
  answer(ok("a", "b"));
  const reopening = held.want(port, true);
  expect(published).toEqual([read("a", "b")]);
  await reopening;
});

it("reads once for nobody so the first open has a list, and reads nothing for it twice", async () => {
  const { port, answer, reads } = await fleet();
  const { held, published } = reader();
  answer(ok("a"));
  await held.warm(port);
  expect(published).toEqual([]);
  await held.warm(port);
  expect(reads()).toBe(1);
  answer(ok("a"));
  const opening = held.want(port, true);
  expect(published).toEqual([read("a")]);
  await opening;
  await settled();
});

it("draws a not-connected failure where nothing is connected and nothing was read", async () => {
  const { held, published } = reader();
  await held.want(null, true);
  expect(published).toEqual([{ state: "failed", outcome: { ok: false, why: "not_connected" } }]);
});

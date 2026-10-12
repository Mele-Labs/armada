// The filed reports as a surface sees them across opening and closing: the last answer stays and is
// drawn at once, the fresh one replaces it, and a read that lands after the surface closed paints nothing.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { ReportList, Reports } from "@armada/protocol";
import { ReportsReader } from "./reports";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  server.closeAllConnections();
  await new Promise<void>((done) => server.close(() => done()));
});

const list = (...ids: string[]): ReportList => ({ reports: ids.map((id) => ({ id })) }) as unknown as ReportList;

type Answer = { status: number; body: unknown };

/** A Fleet whose `/reports` answers are released by the test, one per read, in the order asked. */
async function fleet(): Promise<{ port: number; answer: (answer: Answer) => void }> {
  const waiting: ((answer: Answer) => void)[] = [];
  const early: Answer[] = [];
  const server = createServer((request, response) => {
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
    answer: (answer) => {
      const send = waiting.shift();
      if (send === undefined) early.push(answer);
      else send(answer);
    },
  };
}

const ok = (...ids: string[]): Answer => ({ status: 200, body: list(...ids) });
const broken: Answer = { status: 500, body: { code: "fleet.broken", message: "No." } };

function reader() {
  const published: Reports[] = [];
  return { reports: new ReportsReader((one) => published.push(one)), published };
}

const read = (...ids: string[]): Reports => ({ state: "read", list: list(...ids) });

it("says reading only until there has been an answer, then draws the list", async () => {
  const { port, answer } = await fleet();
  const { reports, published } = reader();
  const opening = reports.want(port, true);
  expect(published).toEqual([{ state: "reading" }]);
  answer(ok("a"));
  await opening;
  expect(published).toEqual([{ state: "reading" }, read("a")]);
});

it("draws the list read last the moment a surface reopens, before the fresh read answers", async () => {
  const { port, answer } = await fleet();
  const { reports, published } = reader();
  answer(ok("a"));
  await reports.want(port, true);
  await reports.want(port, false);
  published.length = 0;
  const reopening = reports.want(port, true);
  expect(published).toEqual([read("a")]);
  answer(ok("a", "b"));
  await reopening;
  expect(published).toEqual([read("a"), read("a", "b")]);
});

it("keeps the list when a refresh fails, and says failed only where there never was one", async () => {
  const { port, answer } = await fleet();
  const { reports, published } = reader();
  answer(broken);
  await reports.want(port, true);
  expect(published.at(-1)?.state).toBe("failed");
  answer(ok("a"));
  await reports.again(port);
  published.length = 0;
  answer(broken);
  await reports.again(port);
  expect(published).toEqual([read("a")]);
});

it("publishes nothing for an answer that lands after the surface closed, and draws it on the next open", async () => {
  const { port, answer } = await fleet();
  const { reports, published } = reader();
  answer(ok("a"));
  await reports.want(port, true);
  const refreshing = reports.again(port);
  await reports.want(port, false);
  published.length = 0;
  answer(ok("a", "b"));
  await refreshing;
  expect(published).toEqual([]);
  answer(ok("a", "b"));
  const reopening = reports.want(port, true);
  expect(published).toEqual([read("a", "b")]);
  await reopening;
});

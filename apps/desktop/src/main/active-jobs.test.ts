// A reading and a Drone list for every Job that has begun and not ended.
//
// **A real server on a loopback port**, `reader.test.ts`'s reason: what is under
// test is which routes were asked, and a stub for `ask` would record what this
// side believed. Every request is counted by path.

import { createServer, type Server } from "node:http";
import { once } from "node:events";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { Active, JobSummary } from "@armada/protocol";
import { ActiveJobs } from "./active-jobs";
import { HOST } from "./runtime-file";

const ONE = "01M21BKVPW002DC0ATD1X9T0VF";
const OTHER = "01M1HQZAKN001AJ5MT3PT09KKY";

const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

function job(id: string, status: string): JobSummary {
  return {
    id,
    handle: `1-${id.slice(-4)}`,
    title: `Job ${id.slice(-4)}`,
    status,
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-10-09T09:00:00Z",
  };
}

/** A Fleet that answers every Job's detail and Drone list, and counts what it was asked. */
async function fleet(): Promise<{ port: number; asked: string[] }> {
  const asked: string[] = [];
  const server: Server = createServer((request, response) => {
    const path = request.url ?? "";
    asked.push(path);
    const id = path.split("/")[2] ?? "";
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify(path.endsWith("/drones") ? { job_id: id, drones: [] } : { job: { id }, steps: [] }));
  });
  server.listen(0, HOST);
  await once(server, "listening");
  opened.push(() => server.close());
  return { port: (server.address() as AddressInfo).port, asked };
}

/** What has been published, and the newest of it. */
function publishing(): { latest: () => Active; publish: (change: { active?: Active }) => void } {
  let latest: Active = {};
  return {
    latest: () => latest,
    publish: (change) => {
      if (change.active !== undefined) latest = change.active;
    },
  };
}

async function settled(held: () => boolean): Promise<void> {
  for (let n = 0; n < 200; n += 1) {
    if (held()) return;
    await new Promise((wake) => setTimeout(wake, 5));
  }
  throw new Error("the readings did not land");
}

it("reads the Job and its Drones for each active Job, and none for one that has not begun or has ended", async () => {
  const { port, asked } = await fleet();
  const out = publishing();
  const active = new ActiveJobs({ publish: out.publish });

  active.follow(true, port, [
    job(ONE, "running"),
    job(OTHER, "awaiting_review"),
    job("01M1AAAAAA001AJ5MT3PT09KKY", "awaiting_approval"),
    job("01M1BBBBBB001AJ5MT3PT09KKY", "completed_success"),
  ]);
  await settled(() => Object.values(out.latest()).filter((one) => one.detail && one.drones).length === 2);

  expect(Object.keys(out.latest()).sort()).toEqual([OTHER, ONE].sort());
  expect(asked.sort()).toEqual([`/jobs/${OTHER}`, `/jobs/${OTHER}/drones`, `/jobs/${ONE}`, `/jobs/${ONE}/drones`].sort());
  active.close();
});

it("re-reads only the Job an event names, and every Job where none is named", async () => {
  const { port, asked } = await fleet();
  const out = publishing();
  const active = new ActiveJobs({ publish: out.publish });
  active.follow(true, port, [job(ONE, "running"), job(OTHER, "running")]);
  await settled(() => asked.length === 4);

  asked.length = 0;
  await active.again(port, ONE);
  expect(asked.sort()).toEqual([`/jobs/${ONE}`, `/jobs/${ONE}/drones`]);

  asked.length = 0;
  await active.again(port, null);
  expect(asked).toHaveLength(4);
  active.close();
});

it("drops a Job that ends, and reads a Job it already holds only once", async () => {
  const { port, asked } = await fleet();
  const out = publishing();
  const active = new ActiveJobs({ publish: out.publish });
  active.follow(true, port, [job(ONE, "running"), job(OTHER, "running")]);
  await settled(() => asked.length === 4);

  active.sync(port, [job(ONE, "running"), job(OTHER, "completed_success")]);
  expect(active.held()).toEqual([ONE]);
  expect(Object.keys(out.latest())).toEqual([ONE]);
  expect(asked).toHaveLength(4);

  asked.length = 0;
  await active.again(port, OTHER);
  expect(asked).toEqual([]);
  active.close();
});

it("keeps the last good reading when a re-read fails", async () => {
  const { port } = await fleet();
  const out = publishing();
  const active = new ActiveJobs({ publish: out.publish });
  active.follow(true, port, [job(ONE, "running")]);
  await settled(() => out.latest()[ONE]?.detail !== undefined && out.latest()[ONE]?.drones !== undefined);

  // Nothing answers on the old port any more.
  await active.again(1, ONE);

  expect(out.latest()[ONE]?.detail).toBeDefined();
  active.close();
});

it("reads nothing until a surface follows, and drops everything when it stops", async () => {
  const { port, asked } = await fleet();
  const out = publishing();
  const active = new ActiveJobs({ publish: out.publish });
  const board = [job(ONE, "running")];

  active.sync(port, board);
  expect(asked).toEqual([]);

  active.follow(true, port, board);
  await settled(() => out.latest()[ONE]?.detail !== undefined && out.latest()[ONE]?.drones !== undefined);

  active.follow(false, port, board);
  expect(active.held()).toEqual([]);
  expect(out.latest()).toEqual({});
});

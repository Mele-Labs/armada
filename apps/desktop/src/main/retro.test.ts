// A Job's retro and the Lessons listing, read over HTTP — `request.ts`. Each is
// sent to a Fleet that records what arrived and answers what it was given.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { JobRetro, Lessons, RepositorySummary } from "@armada/protocol";
import { Picked } from "./picked";
import { lessonsOf, retroOf } from "./request";

const SET_UP: RepositorySummary = {
  root: "/Users/user/store",
  records_root: "/records/store",
  manifest: { id: "store-01", repository: "store", path: "/Users/user/store/armada.yml", records_root: "/records/store", version: 1, checks: [] },
};
const NOT_SET_UP: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };

const RETRO: JobRetro = {
  job_id: "01K6JOB",
  state: "written",
  items: [{ who: "drone", statement: "The armada.yml edit was refused.", evidence: ["refusal:1"] }],
  record: { refusals: [{ cite: "refusal:1", at: "2026-10-02T10:00:00Z", tool: "write_file" }] },
};
const LESSONS: Lessons = {
  lessons: [{ job_id: "01K6JOB", handle: "fix", at: "2026-10-02T11:00:00Z", who: "owner", statement: "Waited.", evidence: [] }],
};

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

/** A Fleet answering `status` with `body`, recording each URL and the caller it named. */
async function fleet(into: string[], body: unknown, status = 200): Promise<number> {
  const server = createServer((request, response) => {
    into.push(`${request.url ?? ""} ${String(request.headers["x-armada-caller"])}`);
    response.writeHead(status, { "content-type": "application/json" });
    response.end(JSON.stringify(body));
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

function pickedAt(root: string | null): Picked {
  const picked = new Picked();
  picked.hold([SET_UP, NOT_SET_UP]);
  picked.pick(root);
  return picked;
}

describe("retro and lessons reads", () => {
  it("reads one Job's retro off its own route, as Bridge", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, RETRO);
    expect(await retroOf(port, "01K6JOB")).toEqual({ ok: true, retro: RETRO });
    expect(asked).toEqual(["/jobs/01K6JOB/retro bridge"]);
  });

  it("answers a refused retro as the refusal", async () => {
    const port = await fleet([], { code: "job_not_found", message: "no such job" }, 404);
    const read = await retroOf(port, "01K6GONE");
    expect(read.ok).toBe(false);
  });

  it("reads every repository's lessons on All, and the pick's alone on a pick", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, LESSONS);
    expect(await lessonsOf(port, pickedAt(null))).toEqual({ ok: true, lessons: LESSONS.lessons });
    expect(await lessonsOf(port, pickedAt(SET_UP.root))).toEqual({ ok: true, lessons: LESSONS.lessons });
    expect(asked).toEqual(["/lessons bridge", "/lessons?manifest_id=store-01 bridge"]);
  });

  it("asks nothing for a repository with no Manifest, which has no Jobs to have lessons", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, LESSONS);
    expect(await lessonsOf(port, pickedAt(NOT_SET_UP.root))).toEqual({ ok: true, lessons: [] });
    expect(asked).toEqual([]);
  });
});

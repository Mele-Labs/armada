// Workspaces, lifecycles and rules, asked of Fleet: the path each of the four reads sends with its id escaped, what
// comes back, and a refusal in Fleet's own words.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { LifecyclesHost } from "./lifecycles";

const servers: Server[] = [];
afterEach(() => {
  while (servers.length > 0) servers.pop()?.close();
});

const EMPTY_JOB = { session_id: "s 1", lifecycles: [], instances: [], moves: [] };

/** A Fleet answering every request with `status` and `body`, remembering what was asked. */
async function serving(status: number, body: unknown): Promise<{ port: number; asked: string[] }> {
  const asked: string[] = [];
  const server = createServer((request, response) => {
    asked.push(`${request.method} ${request.url}`);
    request.resume();
    response.writeHead(status, { "content-type": "application/json" });
    response.end(JSON.stringify(body));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  servers.push(server);
  return { port: (server.address() as AddressInfo).port, asked };
}

describe("Workspaces, lifecycles and rules", () => {
  it("send each read to its path, name what it is about, and answer what Fleet did", async () => {
    const fleet = await serving(200, EMPTY_JOB);
    const host = new LifecyclesHost(() => fleet.port);
    expect(await host.job("s 1")).toEqual({ ok: true, value: EMPTY_JOB });
    await host.workspaces("01MF");
    await host.lifecycles("01 MF");
    await host.rules("01MF");
    expect(fleet.asked).toEqual([
      "GET /sessions/lifecycles?session_id=s%201",
      "GET /workspaces?manifest_id=01MF",
      "GET /lifecycles?manifest_id=01%20MF",
      "GET /lifecycles/rules?manifest_id=01MF",
    ]);
  });

  it("send a piece to save or delete as a POST with the body as it was given", async () => {
    const bodies: string[] = [];
    const server = createServer((request, response) => {
      const chunks: Buffer[] = [];
      request.on("data", (chunk: Buffer) => chunks.push(chunk));
      request.on("end", () => {
        bodies.push(`${request.method} ${request.url} ${Buffer.concat(chunks).toString()}`);
        response.writeHead(200, { "content-type": "application/json" });
        response.end(JSON.stringify({ kind: "rule", id: "progress", scope: { kind: "everywhere" }, file: "/k/rules/progress.toml", replaced: false }));
      });
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    servers.push(server);
    const host = new LifecyclesHost(() => (server.address() as AddressInfo).port);
    const rule = { id: "progress", name: "Start the ticket", scope: { kind: "everywhere" as const }, when: { kind: "job" as const, event: "started" }, conditions: [], does: [], on_failure: { retries: 0, then: "ask_owner" } };
    expect(await host.save({ piece: { kind: "rule", rule }, overwrite: true })).toMatchObject({ ok: true, value: { id: "progress" } });
    await host.remove({ kind: "rule", id: "progress", scope: { kind: "everywhere" } });
    expect(bodies).toEqual([
      `POST /lifecycles/pieces/save ${JSON.stringify({ piece: { kind: "rule", rule }, overwrite: true })}`,
      `POST /lifecycles/pieces/delete ${JSON.stringify({ kind: "rule", id: "progress", scope: { kind: "everywhere" } })}`,
    ]);
  });

  it("send a piece of a Job's copy to keep as a POST, and save nothing", async () => {
    const fleet = await serving(200, { change: 7, kind: "rule", id: "ask_for_review", scope: { kind: "within", id: "01MF" }, replaces: false });
    const host = new LifecyclesHost(() => fleet.port);
    expect(await host.keepPiece({ session_id: "s1", kind: "rule", id: "ask_for_review" })).toMatchObject({ ok: true, value: { change: 7 } });
    expect(fleet.asked).toEqual(["POST /sessions/copy/keep_piece"]);
  });

  it("are refused in Fleet's own words where Fleet serves no such repository", async () => {
    const fleet = await serving(422, { code: "fleet.unacceptable_proposal", message: "This Fleet serves no such repository", run_id: "r", fields: {}, chain: [] });
    const done = await new LifecyclesHost(() => fleet.port).lifecycles("01NOWHERE");
    expect(done).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.unacceptable_proposal" } } });
  });

  it("are not asked where Fleet is not connected", async () => {
    expect(await new LifecyclesHost(() => null).job("s1")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });

  it("tell on the Job a stream event carried", () => {
    const told: unknown[] = [];
    const host = new LifecyclesHost(() => null);
    host.onChanged((held) => told.push(held));
    host.changed(EMPTY_JOB);
    expect(told).toEqual([EMPTY_JOB]);
  });
});

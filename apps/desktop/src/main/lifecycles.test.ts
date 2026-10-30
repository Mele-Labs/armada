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

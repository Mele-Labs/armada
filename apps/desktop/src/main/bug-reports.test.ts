// The bugs agents reported, asked of Fleet: the path each of the three routes sends, what comes back, and a refusal in
// Fleet's own words.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { BugReportsHost } from "./bug-reports";

const servers: Server[] = [];
afterEach(() => {
  while (servers.length > 0) servers.pop()?.close();
});

const REPORT = { id: "r1", session_id: "s1", job_title: "A Job", title: "A fault", what: "It broke.", steps: [], errors: [], logs: [], at: "2026-10-29T10:00:00Z" };

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

describe("the bugs agents reported", () => {
  it("send each route to its path, and answer what Fleet did", async () => {
    const fleet = await serving(200, { reports: [REPORT] });
    const host = new BugReportsHost(() => fleet.port);
    expect(await host.list()).toEqual({ ok: true, value: { reports: [REPORT] } });
    await host.keep("r 1");
    await host.drop("r1");
    expect(fleet.asked).toEqual(["GET /bug_reports", "POST /bug_reports/r%201/keep", "POST /bug_reports/r1/drop"]);
  });

  it("are refused in Fleet's own words where there is no such report", async () => {
    const fleet = await serving(404, { code: "fleet.no_such_bug_report", message: "There is no such bug report", run_id: "r", fields: {}, chain: [] });
    const done = await new BugReportsHost(() => fleet.port).drop("r9");
    expect(done).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.no_such_bug_report" } } });
  });

  it("are not asked where Fleet is not connected", async () => {
    expect(await new BugReportsHost(() => null).list()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });

  it("tell on the list a stream event carried", () => {
    const told: unknown[] = [];
    const host = new BugReportsHost(() => null);
    host.onChanged((reports) => told.push(reports));
    host.changed({ reports: [REPORT] });
    expect(told).toEqual([{ reports: [REPORT] }]);
  });
});

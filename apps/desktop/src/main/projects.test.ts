// A repository's projects, asked of Fleet by its Manifest: the path and body each of the seventeen routes sends, what comes
// back, and a refusal in Fleet's own words. `docs/practices/protocol/projects.md`.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { ProjectsHost } from "./projects";

const servers: Server[] = [];
afterEach(() => {
  while (servers.length > 0) servers.pop()?.close();
});

const PROJECTS = { manifest_id: "armada", projects: [{ id: "p1", name: "Pocket launch", rows: [] }] };

/** A Fleet answering every request with `status` and `body`, remembering what was asked and what each request carried. */
async function serving(status: number, body: unknown): Promise<{ port: number; asked: string[] }> {
  const asked: string[] = [];
  const server = createServer((request, response) => {
    const parts: Buffer[] = [];
    request.on("data", (part: Buffer) => parts.push(part));
    request.on("end", () => {
      const sent = Buffer.concat(parts).toString();
      asked.push(`${request.method} ${request.url}${sent === "" ? "" : ` ${sent}`}`);
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  servers.push(server);
  return { port: (server.address() as AddressInfo).port, asked };
}

describe("a repository's projects", () => {
  it("send each route to its path with its body, and answer the projects whole", async () => {
    const fleet = await serving(200, PROJECTS);
    const host = new ProjectsHost(() => fleet.port);
    const answers = [
      await host.list("armada"),
      await host.add("armada", { name: "Pocket launch" }),
      await host.rename("armada", "p1", { name: "Launch" }),
      await host.delete("armada", "p1"),
      await host.addTask("armada", "p1", { title: "Pair a phone" }),
      await host.addSubtask("armada", "p1", "t1", { title: "Draw the code" }),
      await host.editTask("armada", "p1", "t1", { title: "Pair", notes: "QR" }),
      await host.linkIssue("armada", "p1", { issue: "#7" }),
      await host.linkTaskIssue("armada", "p1", "t1", { issue: "ABC-12" }),
      await host.unlinkTaskIssue("armada", "p1", "t1", { issue: "ABC-12" }),
      await host.linkTaskJob("armada", "p1", "t1", { session_id: "s1" }),
      await host.unlinkTaskJob("armada", "p1", "t1"),
      await host.linkTaskPull("armada", "p1", "t1", { number: 2874 }),
      await host.unlinkTaskPull("armada", "p1", "t1", { number: 2874 }),
      await host.removeRow("armada", "p1", "t1"),
      await host.moveRow("armada", "p1", "t1", { position: 2, parent: "" }),
      await host.tickTask("armada", "p1", "t1", { done: true }),
    ];
    expect(answers).toEqual(answers.map(() => ({ ok: true, value: PROJECTS })));
    expect(fleet.asked).toEqual([
      "GET /projects/armada",
      'POST /projects/armada {"name":"Pocket launch"}',
      'POST /projects/armada/p1/rename {"name":"Launch"}',
      "POST /projects/armada/p1/delete",
      'POST /projects/armada/p1/tasks {"title":"Pair a phone"}',
      'POST /projects/armada/p1/rows/t1/subtasks {"title":"Draw the code"}',
      'POST /projects/armada/p1/rows/t1/edit {"title":"Pair","notes":"QR"}',
      'POST /projects/armada/p1/issues {"issue":"#7"}',
      'POST /projects/armada/p1/rows/t1/issues {"issue":"ABC-12"}',
      'POST /projects/armada/p1/rows/t1/issues/unlink {"issue":"ABC-12"}',
      'POST /projects/armada/p1/rows/t1/job {"session_id":"s1"}',
      "POST /projects/armada/p1/rows/t1/job/unlink",
      'POST /projects/armada/p1/rows/t1/pulls {"number":2874}',
      'POST /projects/armada/p1/rows/t1/pulls/unlink {"number":2874}',
      "POST /projects/armada/p1/rows/t1/remove",
      'POST /projects/armada/p1/rows/t1/move {"position":2,"parent":""}',
      'POST /projects/armada/p1/rows/t1/tick {"done":true}',
    ]);
  });

  it("are refused in Fleet's own words where the project has no such row", async () => {
    const fleet = await serving(404, { code: "fleet.no_such_project_row", message: "There is no row t9", run_id: "r", fields: {}, chain: [] });
    const done = await new ProjectsHost(() => fleet.port).tickTask("armada", "p1", "t9", { done: true });
    expect(done).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.no_such_project_row" } } });
  });

  it("are not asked where Fleet is not connected", async () => {
    expect(await new ProjectsHost(() => null).list("armada")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });

  it("tell on the projects a stream event carried", () => {
    const told: unknown[] = [];
    const host = new ProjectsHost(() => null);
    host.onChanged((projects) => told.push(projects));
    host.changed(PROJECTS);
    expect(told).toEqual([PROJECTS]);
  });
});

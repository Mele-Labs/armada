// Teaching's three calls against a Fleet that answers each route: the body and path that go out, the answer that
// comes back as a value, a refusal as itself, and a Bridge with no Fleet as `not_connected`.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { SaveTeaching, TeachingProposal } from "@armada/protocol";
import { TeachingHost } from "./teaching";

const servers: Server[] = [];
afterEach(() => {
  while (servers.length > 0) servers.pop()?.close();
});

/** A Fleet answering `routes` by method and path, and recording each request body it was sent. */
async function fleet(routes: Record<string, (body: unknown) => [number, unknown]>) {
  const sent: unknown[] = [];
  const server = createServer((request, answer) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => {
      const text = Buffer.concat(chunks).toString();
      const body: unknown = text === "" ? undefined : JSON.parse(text);
      sent.push(body);
      const [status, reply] = (routes[`${request.method} ${request.url}`] ?? (() => [404, { message: "no such route" }]))(body);
      answer.writeHead(status, { "content-type": "application/json" }).end(JSON.stringify(reply));
    });
  });
  servers.push(server);
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  return { port: (server.address() as AddressInfo).port, sent };
}

it("sends what was said to /teachings/read and hands back the proposal as it came", async () => {
  const proposal: TeachingProposal = {
    words: { name: "Jira key", pattern: "[A-Z]+-\\d+", example: "ARM-12" },
    tweak: [{ token: "--bg-raised", was: "#111", now: "#222", used_in: ["--bg-card"] }],
  };
  const served = await fleet({ "POST /teachings/read": () => [200, proposal] });
  const read = { said: "link these", words: "ARM-12", reach: "repository" as const, repository: "/r", source: { kind: "words" as const, text: "ARM-12" } };
  expect(await new TeachingHost(() => served.port).read(read)).toEqual({ ok: true, value: proposal });
  expect(served.sent.at(-1)).toEqual(read);
});

it("keeps a teaching with POST /teachings and answers the entry and the mods", async () => {
  const served = await fleet({ "POST /teachings": () => [200, { entry: "7", mods: ["jira-key"] }] });
  const save: SaveTeaching = { note: { section: "repo", kind: "gotcha", text: "Keys are ARM-n." }, reach: "everywhere", cite: "m3" };
  expect(await new TeachingHost(() => served.port).save(save)).toEqual({ ok: true, value: { entry: "7", mods: ["jira-key"] } });
  expect(served.sent.at(-1)).toEqual(save);
});

it("asks for the rules with only the query it was given, encoded", async () => {
  const rules = { rules: [{ mod_name: "jira-key", name: "Jira key", pattern: "[A-Z]+-\\d+", example: "ARM-12", address: "https://j.example/{match}" }] };
  const served = await fleet({
    "GET /teachings/words": () => [200, { rules: [] }],
    "GET /teachings/words?repository=%2Fa+b%2Fr&session_id=s1": () => [200, rules],
  });
  const host = new TeachingHost(() => served.port);
  expect(await host.rules()).toEqual({ ok: true, value: { rules: [] } });
  expect(await host.rules("/a b/r", "s1")).toEqual({ ok: true, value: rules });
});

it("answers a refusal as itself, and no Fleet as not_connected", async () => {
  const served = await fleet({ "POST /teachings/read": () => [422, { code: "fleet.teaching_nothing_read", message: "nothing read" }] });
  const refused = await new TeachingHost(() => served.port).read({ said: "x", reach: "everywhere" });
  expect(refused.ok).toBe(false);
  const none = new TeachingHost(() => null);
  const outcome = { ok: false, outcome: { ok: false, why: "not_connected" } };
  expect(await none.read({ said: "x", reach: "everywhere" })).toEqual(outcome);
  expect(await none.save({ reach: "everywhere" })).toEqual(outcome);
  expect(await none.rules()).toEqual(outcome);
});

// The Logbook main reads and acts on: every served repository asked by its root beside the habits and this Mac's, and each
// act sent to its own route with the entry's id encoded.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { LogbookEntry } from "@armada/protocol";
import { LogbookHost } from "./logbook";

const entry = (id: string, section: LogbookEntry["section"], repository?: string): LogbookEntry => ({
  id,
  section,
  ...(repository === undefined ? {} : { repository }),
  kind: "convention",
  text: `Entry ${id}.`,
  evidence: [],
  confirmed_by: 2,
  first_seen: "2026-10-01T09:00:00Z",
  last_confirmed: "2026-10-20T09:00:00Z",
  state: "standing",
  locked: false,
  edited: false,
  history: [],
});

let listening: Server | undefined;
afterEach(() => listening?.close());

type Seen = { method: string; url: string; body: string };

/** A Fleet answering each request by what `answer` says of its method and URL, recording what arrived. */
async function fleet(seen: Seen[], answer: (method: string, url: string) => { status: number; body: unknown }): Promise<number> {
  const server = createServer((request, response) => {
    let text = "";
    request.on("data", (chunk) => (text += chunk));
    request.on("end", () => {
      const method = request.method ?? "";
      const url = request.url ?? "";
      seen.push({ method, url, body: text });
      const { status, body } = answer(method, url);
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

const repositories = { repositories: [{ root: "/Users/user/widgets", records_root: "/r/w" }, { root: "/Users/user/store front", records_root: "/r/s" }] };

describe("reading the Logbook", () => {
  it("asks for the habits and this Mac's, then each served repository by its root, and joins the answers", async () => {
    const seen: Seen[] = [];
    const port = await fleet(seen, (_method, url) => {
      if (url === "/repositories") return { status: 200, body: repositories };
      if (url === "/logbook") return { status: 200, body: { entries: [entry("h1", "habits"), entry("machine-ab", "machine")], tidy: { running: true, last: { merged: 3, at: "2026-10-23T10:00:00Z", failed: true } } } };
      if (url.includes("widgets")) return { status: 200, body: { entries: [entry("w1", "repo", "/Users/user/widgets")] } };
      return { status: 200, body: { entries: [entry("s1", "repo", "/Users/user/store front")] } };
    });
    const read = await new LogbookHost(() => port).read();
    expect(seen.map((one) => one.url).sort()).toEqual([
      "/logbook",
      "/logbook?section=repo&repository=%2FUsers%2Fuser%2Fstore%20front",
      "/logbook?section=repo&repository=%2FUsers%2Fuser%2Fwidgets",
      "/repositories",
    ]);
    expect(read).toEqual({
      ok: true,
      entries: [entry("h1", "habits"), entry("machine-ab", "machine"), entry("w1", "repo", "/Users/user/widgets"), entry("s1", "repo", "/Users/user/store front")],
      tidy: { running: true, last: { merged: 3, at: "2026-10-23T10:00:00Z", failed: true } },
    });
  });

  it("reads nothing, and says why, when Fleet is not connected or refuses one repository", async () => {
    expect(await new LogbookHost(() => null).read()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
    const port = await fleet([], (_method, url) =>
      url === "/repositories" ? { status: 200, body: repositories } : url.includes("widgets") ? { status: 500, body: {} } : { status: 200, body: { entries: [] } },
    );
    const read = await new LogbookHost(() => port).read();
    expect(read.ok).toBe(false);
  });
});

describe("the acts on an entry", () => {
  it("edits, locks and forgets at the entry's own routes, the id encoded", async () => {
    const seen: Seen[] = [];
    const port = await fleet(seen, (_method, url) => (url.endsWith("/forget") ? { status: 200, body: { id: "w 1" } } : { status: 200, body: entry("w 1", "repo") }));
    const host = new LogbookHost(() => port);
    expect(await host.edit("w 1", "New words.")).toEqual({ ok: true, entry: entry("w 1", "repo") });
    expect(await host.lock("w 1", true)).toEqual({ ok: true, entry: entry("w 1", "repo") });
    expect(await host.forget("w 1")).toEqual({ ok: true });
    expect(seen.map((one) => [one.method, one.url, one.body])).toEqual([
      ["POST", "/logbook/entries/w%201/edit", JSON.stringify({ text: "New words." })],
      ["POST", "/logbook/entries/w%201/lock", JSON.stringify({ locked: true })],
      ["POST", "/logbook/entries/w%201/forget", "{}"],
    ]);
  });

  it("hands back Fleet's refusal of an entry it does not keep", async () => {
    const port = await fleet([], () => ({
      status: 422,
      body: { code: "fleet.logbook_entry_not_kept", message: "Run memory keeps that entry." },
    }));
    const forgotten = await new LogbookHost(() => port).forget("runs-00000000000000ff");
    expect(forgotten.ok).toBe(false);
    const edited = await new LogbookHost(() => port).edit("runs-00000000000000ff", "x");
    expect(edited.ok === false && edited.outcome.ok === false && edited.outcome.why).toBe("refused");
  });
});

describe("tidying the Logbook", () => {
  it("asks Fleet to tidy every repository and hands back that a pass is running", async () => {
    const seen: Seen[] = [];
    const port = await fleet(seen, () => ({ status: 200, body: { running: true } }));
    expect(await new LogbookHost(() => port).tidy()).toEqual({ ok: true, running: true });
    expect(seen.map((one) => [one.method, one.url, one.body])).toEqual([["POST", "/logbook/tidy", "{}"]]);
  });

  it("says Fleet is not connected with no port, and hands back a refusal", async () => {
    expect(await new LogbookHost(() => null).tidy()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
    const port = await fleet([], () => ({ status: 422, body: { code: "fleet.logbook_tidy_refused", message: "No model." } }));
    expect((await new LogbookHost(() => port).tidy()).ok).toBe(false);
  });
});

describe("organizing the Logbook", () => {
  it("asks Fleet for an action's proposals over every repository and hands them back with whether any look failed", async () => {
    const seen: Seen[] = [];
    const body = { action: "stale", proposals: [{ id: "a", entry_ids: ["a"], text: "T.", note: "60 days" }], failed: true };
    const port = await fleet(seen, () => ({ status: 200, body }));
    expect(await new LogbookHost(() => port).organize("stale")).toEqual({ ok: true, ...body });
    expect(seen.map((one) => [one.method, one.url, one.body])).toEqual([["POST", "/logbook/organize", '{"action":"stale"}']]);
  });

  it("sends the kept proposals to apply and hands back how many entries changed", async () => {
    const seen: Seen[] = [];
    const proposals = [{ id: "a", entry_ids: ["a", "b"], text: "T.", note: "" }];
    const port = await fleet(seen, () => ({ status: 200, body: { action: "merge", applied: 1, reported: [] } }));
    expect(await new LogbookHost(() => port).applyOrganize("merge", proposals)).toEqual({ ok: true, action: "merge", applied: 1, reported: [] });
    expect(seen.map((one) => [one.method, one.url, JSON.parse(one.body)])).toEqual([["POST", "/logbook/organize/apply", { action: "merge", proposals }]]);
  });

  it("says Fleet is not connected with no port", async () => {
    expect(await new LogbookHost(() => null).organize("merge")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});

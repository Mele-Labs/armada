// Starting out, as main sends it: each route, its method and body, the query a path travels in, the refusal in Fleet's words,
// and what the channels let through from a renderer that is not trusted.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { INSTALL_MOD_MS, OnboardingHost } from "./onboarding";
import { dropOf, placeOf } from "./onboarding-channels";

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

type Seen = { method: string; url: string; body: string };

/** A Fleet answering one status and body to every request, recording what arrived. */
async function fleet(status: number, body: unknown, into: Seen[]): Promise<OnboardingHost> {
  const server = createServer((request, response) => {
    let text = "";
    request.on("data", (chunk) => (text += chunk));
    request.on("end", () => {
      into.push({ method: request.method ?? "", url: request.url ?? "", body: text });
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address() as AddressInfo;
  return new OnboardingHost(() => port);
}

describe("reading", () => {
  it("asks for the folders with a GET and no body", async () => {
    const seen: Seen[] = [];
    const answer = await (await fleet(200, { agent: "Claude Code", folders: [] }, seen)).importFolders();
    expect(seen).toEqual([{ method: "GET", url: "/import/folders", body: "" }]);
    expect(answer).toEqual({ ok: true, value: { agent: "Claude Code", folders: [] } });
  });

  it("carries a path and a repository as an encoded query", async () => {
    const seen: Seen[] = [];
    const host = await fleet(200, {}, seen);
    await host.importRuns("/Users/user/my repo&x");
    await host.runMemory("/Users/user/my repo&x");
    await host.runMemoryPlace("/Users/user/my repo&x");
    expect(seen.map((one) => one.url)).toEqual([
      "/import/runs?path=%2FUsers%2Fuser%2Fmy%20repo%26x",
      "/repositories/runs?repository=%2FUsers%2Fuser%2Fmy%20repo%26x",
      "/repositories/runs/place?repository=%2FUsers%2Fuser%2Fmy%20repo%26x",
    ]);
  });
});

describe("acting", () => {
  it("installs with requested true, always", async () => {
    const seen: Seen[] = [];
    await (await fleet(200, { result: "installed", said: "ok" }, seen)).installMod();
    expect(seen).toEqual([{ method: "POST", url: "/import/install_mod", body: JSON.stringify({ requested: true }) }]);
    expect(INSTALL_MOD_MS).toBeGreaterThan(6 * 60_000);
  });

  it("sends a place and a drop whole", async () => {
    const seen: Seen[] = [];
    const host = await fleet(200, {}, seen);
    await host.setRunMemoryPlace({ repository: "/r", place: "synced", synced_folder: "/s" });
    await host.dropRunFact({ repository: "/r", workspace: ".", kind: "tests", command: "pnpm test" });
    expect(seen).toEqual([
      { method: "POST", url: "/repositories/runs/place", body: JSON.stringify({ repository: "/r", place: "synced", synced_folder: "/s" }) },
      { method: "POST", url: "/repositories/runs/drop", body: JSON.stringify({ repository: "/r", workspace: ".", kind: "tests", command: "pnpm test" }) },
    ]);
  });

  it("hands back a refusal in Fleet's code", async () => {
    const refusal = { code: "fleet.run_cloud_unsigned", message: "not yet", run_id: "r", fields: {}, chain: [] };
    const answer = await (await fleet(422, refusal, [])).setRunMemoryPlace({ repository: "/r", place: "cloud" });
    expect(answer).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.run_cloud_unsigned" } } });
  });

  it("sends nothing without a Fleet", async () => {
    expect(await new OnboardingHost(() => null).importFolders()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});

describe("what a renderer may send", () => {
  it("passes a place that names a repository and a known place, leaving an absent folder absent", () => {
    expect(placeOf({ repository: "/r", place: "repository" })).toEqual({ repository: "/r", place: "repository" });
    expect(placeOf({ repository: "/r", place: "synced", synced_folder: "/s" })).toEqual({ repository: "/r", place: "synced", synced_folder: "/s" });
  });

  it("drops anything else", () => {
    expect(placeOf(null)).toBeNull();
    expect(placeOf({ repository: "", place: "this_mac" })).toBeNull();
    expect(placeOf({ repository: "/r", place: "elsewhere" })).toBeNull();
    expect(placeOf({ repository: "/r", place: "synced", synced_folder: 3 })).toBeNull();
    expect(dropOf({ repository: "/r", workspace: ".", kind: "tests" })).toBeNull();
    expect(dropOf({ repository: "/r", workspace: ".", kind: "tests", command: "pnpm test" })).toEqual({
      repository: "/r",
      workspace: ".",
      kind: "tests",
      command: "pnpm test",
    });
  });
});

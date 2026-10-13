// A Job's Commander: its socket, folded into a thread, and the two requests the owner sends into
// it. `helm.test.ts`'s approach, for its reason: what is under test is message handling and the
// requests, through the same listeners the real connection is wired with.

import { once } from "node:events";
import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo, Socket } from "node:net";

import { afterEach, describe, expect, it } from "vitest";
import { WebSocketServer, type WebSocket as Client } from "ws";

import type { CommanderThread, Outcome } from "@armada/protocol";
import { answerCommanderCard, askCommander, CommanderSocket } from "./commander";
import { ask, route } from "./request";

const JOB = "01M1JOB14000000000000000A";
const OPENED = { message: "opened", protocol_id: "0000000000000000", job_id: JOB, replying: false, skipped: 0 };
const WOKE = { message: "woke", ts: "2026-10-13T10:00:00.000Z", why: "Drone on T7 ended with nothing handed in" };
const SAID = { message: "row", ts: "2026-10-13T10:00:01.000Z", event: "said", text: "T7 hit the turn cap." };
const ENDED = { message: "row", ts: "2026-10-13T10:00:02.000Z", event: "ended", turns: 3, cost_micros: 12_000, refusals: 0 };

const opened: (() => void)[] = [];
afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/** A Fleet serving the Commander's socket and answering every request, on one port. */
async function serving(requests: { url: string; method: string; body: string }[] = []) {
  const socket = new WebSocketServer({ noServer: true });
  const server: Server = createServer((request, response) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => {
      requests.push({ url: request.url ?? "", method: request.method ?? "", body: Buffer.concat(chunks).toString("utf8") });
      response.writeHead(202, { "content-type": "application/json" });
      response.end(JSON.stringify({ job_id: JOB, replying: true, resumes: true }));
    });
  });
  server.on("upgrade", (request: IncomingMessage, raw: Socket, head: Buffer) => {
    socket.handleUpgrade(request, raw, head, (client) => socket.emit("connection", client, request));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(() => server.close());
  return {
    port: (server.address() as AddressInfo).port,
    commanderSide: () => once(socket, "connection").then(([client]) => client as Client),
  };
}

function watching() {
  const seen: CommanderThread[] = [];
  const wanted: { holds: (state: CommanderThread) => boolean; keep: (state: CommanderThread) => void }[] = [];
  return {
    seen,
    publish(state: CommanderThread): void {
      seen.push(state);
      for (const [at, want] of [...wanted.entries()].reverse()) {
        if (!want.holds(state)) continue;
        wanted.splice(at, 1);
        want.keep(state);
      }
    },
    until(holds: (state: CommanderThread) => boolean): Promise<CommanderThread> {
      const already = seen.find(holds);
      if (already !== undefined) return Promise.resolve(already);
      return new Promise((keep) => wanted.push({ holds, keep }));
    },
  };
}

describe("a Commander's socket", () => {
  it("folds what woke it and what it said into one thread, replying until a turn ends", async () => {
    const fleet = await serving();
    const published = watching();
    const commander = new CommanderSocket((state) => published.publish(state));
    opened.push(() => commander.close());

    commander.open(fleet.port, JOB);
    const side = await fleet.commanderSide();
    side.send(JSON.stringify(OPENED));
    side.send(JSON.stringify(WOKE));
    side.send(JSON.stringify(SAID));
    const working = await published.until((state) => state.state === "open" && state.items.length === 2);
    expect(working).toMatchObject({ state: "open", jobId: JOB, replying: true });

    side.send(JSON.stringify(ENDED));
    const settled = await published.until((state) => state.state === "open" && !state.replying);
    expect(settled).toMatchObject({
      state: "open",
      items: [{ kind: "woke", why: WOKE.why }, { kind: "row" }, { kind: "row" }],
    });
  });

  it("puts the owner's words on the thread when they are taken, mid-turn or not", async () => {
    const fleet = await serving();
    const published = watching();
    const commander = new CommanderSocket((state) => published.publish(state));
    opened.push(() => commander.close());

    commander.open(fleet.port, JOB);
    const side = await fleet.commanderSide();
    side.send(JSON.stringify({ ...OPENED, replying: true }));
    side.send(JSON.stringify({ message: "asked", ts: "2026-10-13T10:00:03.000Z", text: "why did it stop?" }));

    const held = await published.until((state) => state.state === "open" && state.items.length === 1);
    expect(held).toMatchObject({ replying: true, items: [{ kind: "asked", text: "why did it stop?" }] });
  });

  it("reopens onto the backfill where messages were dropped, rather than drawing a gap", async () => {
    const fleet = await serving();
    const published = watching();
    const commander = new CommanderSocket((state) => published.publish(state));
    opened.push(() => commander.close());

    commander.open(fleet.port, JOB);
    const first = await fleet.commanderSide();
    first.send(JSON.stringify(OPENED));
    first.send(JSON.stringify(WOKE));
    await published.until((state) => state.state === "open" && state.items.length === 1);

    const second = fleet.commanderSide();
    first.send(JSON.stringify({ message: "missed", dropped: 4 }));
    const again = await second;
    again.send(JSON.stringify(OPENED));
    again.send(JSON.stringify(WOKE));
    again.send(JSON.stringify(SAID));
    const whole = await published.until((state) => state.state === "open" && state.items.length === 2);
    expect(whole).toMatchObject({ missed: 0 });
  });

  it("keeps what arrived when the connection breaks, and is not attached", async () => {
    const fleet = await serving();
    const published = watching();
    const commander = new CommanderSocket((state) => published.publish(state));
    opened.push(() => commander.close());

    commander.open(fleet.port, JOB);
    const side = await fleet.commanderSide();
    side.send(JSON.stringify(OPENED));
    side.send(JSON.stringify(WOKE));
    await published.until((state) => state.state === "open" && state.items.length === 1);

    side.send("{ not json");
    const broke = await published.until((state) => state.state === "failed");
    expect("items" in broke && broke.items.length).toBe(1);
    expect(commander.attached()).toBe(false);
  });

  it("says Fleet is not connected without a port, and nothing for no Job", () => {
    const published = watching();
    const commander = new CommanderSocket((state) => published.publish(state));
    commander.open(null, JOB);
    expect(published.seen.at(-1)).toMatchObject({ state: "failed", jobId: JOB });
    commander.open(null, null);
    expect(published.seen.at(-1)).toEqual({ state: "none" });
  });
});

describe("writing to a Commander", () => {
  it("sends the owner's words to the Job's Commander, and refuses blank before sending", async () => {
    const requests: { url: string; method: string; body: string }[] = [];
    const fleet = await serving(requests);

    expect(await askCommander(fleet.port, JOB, "  ")).toEqual({ ok: false, why: "empty_instruction" });
    expect(await askCommander(fleet.port, JOB, "why did it stop?")).toEqual({ ok: true });
    expect(await askCommander(null, JOB, "hello")).toEqual({ ok: false, why: "not_connected" });
    expect(requests.map((one) => [one.method, one.url, JSON.parse(one.body)])).toEqual([
      ["POST", `/jobs/${JOB}/commander/ask`, { text: "why did it stop?" }],
    ]);
  });

  it("answers a card with its id and an option's id and nothing else", async () => {
    const requests: { url: string; method: string; body: string }[] = [];
    const fleet = await serving(requests);
    const guarded = (_jobId: string, send: (port: number) => ReturnType<typeof ask>): Promise<Outcome> =>
      send(fleet.port).then((answer) => (answer.ok === true ? { ok: true } : answer.outcome));

    expect(await answerCommanderCard(guarded, JOB, "01CARD", "2")).toEqual({ ok: true });
    expect(requests.map((one) => [one.method, one.url, JSON.parse(one.body)])).toEqual([
      ["POST", route(JOB, "commander/card/answer"), { card_id: "01CARD", option_id: "2" }],
    ]);
  });
});

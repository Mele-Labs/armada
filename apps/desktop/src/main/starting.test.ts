// Fleet booting, told apart from a Fleet that is up and not answering.
//
// **Fleet binds its port, publishes the runtime file, and only then reconciles
// and serves** (`crates/armada/src/serve.rs`). In between, the kernel accepts a
// connection that nothing reads, so Bridge sees no refusal and no close — the
// upgrade request goes out and no answer comes back. The listener here does the
// same: it takes the upgrade and holds it until the case says Fleet is serving.
//
// The runtime file names this process, so the pid probe passes for real, and
// `now` is the case's own: a process's age is `now` less the start time `ps`
// gave, and a case chooses it rather than waiting for it.

import { once } from "node:events";
import { createServer, type IncomingMessage } from "node:http";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import type { AddressInfo, Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, expect, it } from "vitest";
import { WebSocketServer } from "ws";

import { PROTOCOL_VERSION } from "@armada/protocol";
import type { BridgeState, PickedView } from "../shared/bridge";
import { FleetConnection } from "./connection";
import { holderOf } from "./runtime-file";

const GRACE_MS = 40;
const BOOT_MS = 60_000;

const opened: (() => void | Promise<void>)[] = [];

afterEach(async () => {
  while (opened.length > 0) await opened.pop()?.();
});

/** What this process's own start time says, as `ps` spelled it and as a clock reads it. */
function born(): { text: string; ms: number } {
  const held = holderOf(process.pid);
  if (held.held !== true) throw new Error("this process holds its own pid");
  return { text: held.startedAt, ms: Date.parse(held.startedAt) };
}

/**
 * A Fleet that has bound its port and not begun to serve. `serve()` is the
 * moment it does: every held upgrade is answered, and each one opens with the
 * resync a real connection opens with. `refuse` is the other thing a Fleet can
 * do with a connection, which is to close it.
 */
async function binding(how: "holds" | "refuses" | "answers" = "holds"): Promise<{
  port: number;
  serve: () => void;
}> {
  const sockets = new WebSocketServer({ noServer: true });
  const held: { request: IncomingMessage; socket: Socket; head: Buffer }[] = [];
  const answer = ({ request, socket, head }: (typeof held)[number]): void => {
    sockets.handleUpgrade(request, socket, head, (client) => {
      client.send(
        JSON.stringify({
          message: "resync",
          protocol_version: PROTOCOL_VERSION,
          cursor: 1,
          jobs: { jobs: [], unreadable: [] },
        }),
      );
    });
  };
  const server = createServer((_request, answer) => {
    answer.writeHead(404, { "content-type": "application/json" });
    answer.end("{}");
  });
  server.on("upgrade", (request: IncomingMessage, socket: Socket, head: Buffer) => {
    if (how === "refuses") {
      socket.destroy();
      return;
    }
    const one = { request, socket, head };
    if (how === "answers") answer(one);
    else held.push(one);
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(
    () =>
      new Promise<void>((done) => {
        for (const one of held) one.socket.destroy();
        for (const client of sockets.clients) client.terminate();
        server.close(() => done());
      }),
  );
  return {
    port: (server.address() as AddressInfo).port,
    serve: () => {
      for (const one of held.splice(0)) answer(one);
    },
  };
}

async function runtimeFile(port: number): Promise<string> {
  const home = await mkdtemp(join(tmpdir(), "bridge-"));
  const dir = join(home, "Library", "Application Support", "Armada");
  await mkdir(dir, { recursive: true });
  await writeFile(
    join(dir, "fleet.json"),
    JSON.stringify({
      protocol_version: PROTOCOL_VERSION,
      pid: process.pid,
      port,
      started_at: born().text,
    }),
  );
  return home;
}

/** Every state main published, and a wait for the one a case is about. */
function publishing() {
  const seen: BridgeState[] = [];
  const wanted: { holds: (state: BridgeState) => boolean; keep: () => void }[] = [];
  return {
    seen: () => seen.map((state) => state.connection.state),
    publish(state: BridgeState): void {
      seen.push(state);
      for (const [at, want] of [...wanted.entries()].reverse()) {
        if (!want.holds(state)) continue;
        wanted.splice(at, 1);
        want.keep();
      }
    },
    publishToWindow(_windowId: number, _change: Partial<PickedView>): void {},
    windowIds(): readonly number[] {
      return [];
    },
    until(holds: (state: BridgeState) => boolean): Promise<void> {
      if (seen.some(holds)) return Promise.resolve();
      return new Promise((keep) => wanted.push({ holds, keep }));
    },
    latest(): BridgeState {
      const last = seen[seen.length - 1];
      if (last === undefined) throw new Error("nothing has been published yet");
      return last;
    },
  };
}

/** Bridge, opened against the Fleet a case built, with the clock the case chose. */
async function opening(port: number, clock: { at: number }) {
  const home = await runtimeFile(port);
  const published = publishing();
  const connection = new FleetConnection({
    home,
    publish: (state) => published.publish(state),
    publishToWindow: (id, change) => published.publishToWindow(id, change),
    windowIds: () => published.windowIds(),
    now: () => clock.at,
    timing: { graceMs: GRACE_MS, bootMs: BOOT_MS },
  });
  opened.push(() => connection.stop());
  connection.start();
  return published;
}

it("says a young Fleet is starting while its socket goes unanswered, and connects once it serves", async () => {
  const fleet = await binding();
  const published = await opening(fleet.port, { at: born().ms + 4_000 });

  await published.until((state) => state.connection.state === "starting");
  expect(published.seen()).not.toContain("unreachable");

  fleet.serve();
  await published.until((state) => state.connection.state === "connected");
});

it("says unreachable once a Fleet has been starting for longer than a boot takes", async () => {
  const fleet = await binding();
  const clock = { at: born().ms + BOOT_MS - 150 };
  const published = await opening(fleet.port, clock);

  await published.until((state) => state.connection.state === "starting");
  // The boot window runs out while the socket is still unanswered.
  clock.at = born().ms + BOOT_MS + 5_000;
  await published.until((state) => state.connection.state === "unreachable");

  const now = published.latest().connection;
  if (now.state !== "unreachable") throw new Error("unreachable was just published");
  expect(now.sinceMs).toBe(clock.at);

  // It stays open: a Fleet that does come up late is still connected to.
  fleet.serve();
  await published.until((state) => state.connection.state === "connected");
});

it("never says starting for a Fleet that has been up, however long its socket is silent", async () => {
  const fleet = await binding();
  const published = await opening(fleet.port, { at: born().ms + 3 * BOOT_MS });

  await published.until((state) => state.connection.state === "unreachable");
  expect(published.seen()).not.toContain("starting");
});

it("never says starting for a socket Fleet refused", async () => {
  const fleet = await binding("refuses");
  const published = await opening(fleet.port, { at: born().ms + 4_000 });

  await published.until((state) => state.connection.state === "unreachable");
  expect(published.seen()).not.toContain("starting");
});

it("says only connecting, and then connected, for a Fleet that answers at once", async () => {
  const fleet = await binding("answers");
  const published = await opening(fleet.port, { at: born().ms + 4_000 });
  await published.until((state) => state.connection.state === "connected");

  expect(published.seen()).not.toContain("starting");
  expect(published.seen()).not.toContain("unreachable");
});

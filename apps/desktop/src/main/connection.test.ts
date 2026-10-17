// What a connection publishes once Fleet answers its resync, against a Fleet that is not Fleet.
//
// **One listener, HTTP and the stream, because that is what Fleet is.** The runtime file is written
// for real and the pid in it is this process's, so the connection takes the same path it takes
// against a running daemon: read the file, probe the pid, open `/events`.

import { once } from "node:events";
import { createServer, type IncomingMessage, type Server } from "node:http";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { AddressInfo, Socket } from "node:net";

import { afterEach, expect, it } from "vitest";
import { WebSocketServer, type WebSocket } from "ws";

import { PROTOCOL_ID } from "@armada/protocol";
import type { BridgeState, PickedView } from "../shared/bridge";
import { FleetConnection } from "./connection";
import { holderOf } from "./runtime-file";

/** Everything a case opened, given back in reverse. */
const opened: (() => void | Promise<void>)[] = [];

afterEach(async () => {
  while (opened.length > 0) await opened.pop()?.();
});

/** `readPreferences`' own route, read once per connection like `/limits`. */
const PREFERENCES_ROUTE = "/preferences";

/**
 * What each route answers. **`preferences` is the one answer a case can vary**: it is Fleet-wide and
 * read on every resync, so the one case about a failed read needs the 404 every other unlisted
 * route already gets — passing `undefined` is that, without a second switch arm.
 */
function answering(route: string, preferences: unknown): unknown {
  return route === PREFERENCES_ROUTE ? preferences : undefined;
}

/** Each connection to one route, in arrival order, and a wait for the next. */
function arriving(server: WebSocketServer): {
  all: WebSocket[];
  past: (already: number) => Promise<WebSocket>;
} {
  const all: WebSocket[] = [];
  let waiting: (() => void) | null = null;
  server.on("connection", (socket: WebSocket) => {
    all.push(socket);
    waiting?.();
  });
  return {
    all,
    past: (already: number) =>
      new Promise<WebSocket>((keep) => {
        const look = (): boolean => {
          const found = all[already];
          if (found === undefined) return false;
          keep(found);
          return true;
        };
        if (!look()) waiting = () => void look();
      }),
  };
}

/**
 * A Fleet on one port: `/events`, and the reads between.
 *
 * `preferencesUnavailable` makes `GET /preferences` 404 instead of answering
 * `{ where_things_are_open: true }` — a boolean rather than the body itself,
 * because passing `undefined` for "no override" is indistinguishable from
 * passing it for "this is the failure case" once it is a default parameter.
 */
async function serving(preferencesUnavailable = false): Promise<{
  port: number;
  /** Each connection to `/events`. A reconnection is the second one. */
  stream: (past: number) => Promise<WebSocket>;
  /** How many times one route has been read. */
  read: (route: string) => number;
}> {
  const stream = new WebSocketServer({ noServer: true });
  const events = arriving(stream);

  const reads = new Map<string, number>();
  const server = createServer((request, answer) => {
    const route = request.url ?? "";
    reads.set(route, (reads.get(route) ?? 0) + 1);
    // Anything off the list answers a refusal, which every reader here already
    // renders rather than throwing on.
    const body = answering(route, preferencesUnavailable ? undefined : { where_things_are_open: true });
    answer.writeHead(body === undefined ? 404 : 200, { "content-type": "application/json" });
    answer.end(JSON.stringify(body ?? {}));
  });
  server.on("upgrade", (request: IncomingMessage, socket: Socket, head: Buffer) => {
    stream.handleUpgrade(request, socket, head, (client) => stream.emit("connection", client, request));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(() => closing(server));

  return {
    port: (server.address() as AddressInfo).port,
    stream: events.past,
    read: (route: string) => reads.get(route) ?? 0,
  };
}

/** The first message on every connection, and one more after every drop. */
function resyncing(cursor: number): string {
  return JSON.stringify({
    message: "resync",
    protocol_id: PROTOCOL_ID,
    cursor,
  });
}

/** Every state main published, and a wait for the one a case is about. */
function publishing() {
  const seen: BridgeState[] = [];
  const wanted: { holds: (state: BridgeState) => boolean; keep: () => void }[] = [];
  return {
    publish(state: BridgeState): void {
      seen.push(state);
      for (const [at, want] of [...wanted.entries()].reverse()) {
        if (!want.holds(state)) continue;
        wanted.splice(at, 1);
        want.keep();
      }
    },
    // Nothing here opens a real window, so there is nobody's pick to overlay.
    publishToWindow(_windowId: number, _change: Partial<PickedView>): void {},
    windowIds(): readonly number[] {
      return [];
    },
    until(holds: (state: BridgeState) => boolean): Promise<void> {
      if (seen.some(holds)) return Promise.resolve();
      return new Promise((keep) => wanted.push({ holds, keep }));
    },
    /** The state as last published. Read after an `until` that pinned it. */
    latest(): BridgeState {
      const last = seen[seen.length - 1];
      if (last === undefined) throw new Error("nothing has been published yet");
      return last;
    },
  };
}

/** The runtime file, naming this process, so the pid probe passes for real. */
async function runtimeFile(port: number): Promise<string> {
  const home = await mkdtemp(join(tmpdir(), "bridge-"));
  const dir = join(home, "Library", "Application Support", "Armada");
  await mkdir(dir, { recursive: true });
  const held = holderOf(process.pid);
  await writeFile(
    join(dir, "fleet.json"),
    JSON.stringify({
      protocol_id: PROTOCOL_ID,
      pid: process.pid,
      port,
      started_at: held.held === true ? held.startedAt : "",
    }),
  );
  return home;
}

function closing(server: Server): Promise<void> {
  return new Promise((done) => server.close(() => done()));
}

it("publishes a person's Bridge preferences once Fleet answers the resync", async () => {
  const fleet = await serving();
  const home = await runtimeFile(fleet.port);
  const published = publishing();
  const connection = new FleetConnection({
    home,
    publish: (state) => published.publish(state),
    publishToWindow: (id, change) => published.publishToWindow(id, change),
    windowIds: () => published.windowIds(),
    now: () => 1_756_840_000_000,
  });
  opened.push(() => connection.stop());

  connection.start();
  const stream = await fleet.stream(0);
  stream.send(resyncing(1));

  await published.until((state) => state.preferences.where_things_are_open === true);
  expect(fleet.read(PREFERENCES_ROUTE)).toBe(1);
});

/**
 * **A failed read is never taken as "nothing is saved."** `BridgeState.preferences`
 * is never `null`, unlike `limits` — so the shipped default a person never
 * touched and a person's own choice Fleet could not answer for would draw
 * identically, and only the second one is safe to publish over what is
 * already there.
 */
it("keeps drawing the shipped default when Fleet's preferences read fails", async () => {
  const fleet = await serving(true);
  const home = await runtimeFile(fleet.port);
  const published = publishing();
  const connection = new FleetConnection({
    home,
    publish: (state) => published.publish(state),
    publishToWindow: (id, change) => published.publishToWindow(id, change),
    // One open window, so the per-window reads a resync also fires keep this
    // test's ordering the same as every other read here — an empty set skips
    // them outright, which is correct but not what this race depends on.
    windowIds: () => [1],
    now: () => 1_756_840_000_000,
  });
  opened.push(() => connection.stop());

  connection.start();
  const stream = await fleet.stream(0);
  stream.send(resyncing(1));

  // Nothing publishes *because* the read failed, so what is waited on is a
  // different Fleet-wide read from the same resync — one which
  // always publishes — landing after it. `#507`'s reason: waiting on a state
  // change the failure itself makes would wait forever.
  await published.until(() => fleet.read(PREFERENCES_ROUTE) === 1);
  expect(published.latest().preferences).toEqual({ where_things_are_open: false });
});

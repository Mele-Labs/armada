// What one Job's Observe socket publishes, against a real socket.
//
// **A `ws` server on a loopback port rather than a mock.** What is under test is
// message handling, and every message arrives through the same three listeners
// the real one is wired with — a stub for `ws` would test the wiring this file
// exists to check. The server here sends what `crates/ipc/src/turn.rs` declares;
// `crates/api/src/tests/observing.rs` is the other half, and holds Fleet's.

import { once } from "node:events";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";
import { WebSocketServer, type WebSocket as Socket } from "ws";

import type { Observed } from "@armada/protocol";
import { ObserveSocket } from "./observe";
import { HOST } from "./runtime-file";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";

/** One row, as Fleet spells it: the step and the voice sit beside the kind. */
const A_ROW = {
  message: "row",
  ts: "2026-09-02T19:10:04.000Z",
  step: "implement",
  by: "drone",
  event: "said",
  text: "reading the parser",
};

const OPENED = {
  message: "opened",
  protocol_id: "0000000000000000",
  job_id: A_JOB,
  live: true,
  skipped: 0,
};

/** Everything one case opens, closed in the order it was opened. */
const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/**
 * A Fleet serving this Job's turns, and the socket it serves them down.
 *
 * The connection is awaited by the caller, so a frame is never sent before
 * there is somebody to send it to.
 */
async function serving(): Promise<{ port: number; talking: Promise<Socket> }> {
  const server = new WebSocketServer({ host: HOST, port: 0 });
  await once(server, "listening");
  const talking = once(server, "connection").then(([socket]) => socket as Socket);
  opened.push(() => server.close());
  return { port: (server.address() as AddressInfo).port, talking };
}

/** Every state the socket published, and a wait for the one a case is about. */
function watching() {
  const seen: Observed[] = [];
  const wanted: { holds: (state: Observed) => boolean; keep: (state: Observed) => void }[] = [];
  return {
    seen,
    publish(state: Observed): void {
      seen.push(state);
      for (const [at, want] of [...wanted.entries()].reverse()) {
        if (!want.holds(state)) continue;
        wanted.splice(at, 1);
        want.keep(state);
      }
    },
    /** The first published state that answers this, past or future. */
    until(holds: (state: Observed) => boolean): Promise<Observed> {
      const already = seen.find(holds);
      if (already !== undefined) return Promise.resolve(already);
      return new Promise((keep) => wanted.push({ holds, keep }));
    },
  };
}

describe("a transcript socket that stops", () => {
  it("keeps the rows it already had when it breaks", async () => {
    const fleet = await serving();
    const published = watching();
    const turns = new ObserveSocket((state) => published.publish(state));
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(JSON.stringify(A_ROW));
    await published.until((state) => "turns" in state && state.turns.rows.length === 1);

    // A message this Bridge cannot read. **The defect**: one of these emptied a
    // log that was full a moment before, so a reader lost the step's whole
    // history to a frame that was never about the rows.
    fleetSide.send("{ not json");
    const broke = await published.until((state) => state.state === "failed");

    expect(broke).toMatchObject({ state: "failed", jobId: A_JOB });
    expect("turns" in broke && broke.turns.rows.length).toBe(1);
  });

  it("says why it closed and keeps the rows", async () => {
    const fleet = await serving();
    const published = watching();
    const turns = new ObserveSocket((state) => published.publish(state));
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(JSON.stringify(A_ROW));
    fleetSide.send(JSON.stringify({ message: "closed", because: "drone_ended" }));

    const ended = await published.until((state) => state.state === "ended");
    expect(ended).toMatchObject({ state: "ended", because: "drone_ended" });
    expect("turns" in ended && ended.turns.rows.length).toBe(1);
    // The socket is let go on `closed`, which is what makes the reopen on the
    // next event about this Job the thing that resumes it — `connection.ts`.
    expect(turns.attached()).toBe(false);
  });

  // The same leak `journal.test.ts` covers, on the socket that has the older
  // history of losing what it had read: the connection is let go, not merely
  // dropped, and the server is what says so.
  it("lets the connection go when it cannot read what arrived", async () => {
    const fleet = await serving();
    const published = watching();
    const turns = new ObserveSocket((state) => published.publish(state));
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(JSON.stringify(A_ROW));
    await published.until((state) => "turns" in state && state.turns.rows.length === 1);

    const closed = once(fleetSide, "close");
    fleetSide.send("{ not json");

    await closed;
    expect(turns.attached()).toBe(false);
    const broke = await published.until((state) => state.state === "failed");
    expect("turns" in broke && broke.turns.rows.length).toBe(1);
  });

  // **The handshake is the case that crashed.** A pane closed before Fleet
  // answers leaves ws no connection to close, so it aborts and reports the
  // abort as an `error` a tick later. With every listener already gone, Node
  // threw it, and the main process went down with it.
  it("closes while the handshake is still in flight, and nothing is thrown", async () => {
    const fleet = await serving();
    const turns = new ObserveSocket(() => {});
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    turns.close();

    await new Promise((settle) => setTimeout(settle, 20));
    expect(turns.attached()).toBe(false);
  });
});

describe("a full backfill", () => {
  // Folded a tick at a time rather than a row at a time, and what was
  // published along the way is never written to afterwards.
  it("arrives whole and in order, and leaves what it published as it was", async () => {
    const fleet = await serving();
    const published = watching();
    const lengths: number[] = [];
    const turns = new ObserveSocket((state) => {
      if ("turns" in state) lengths.push(state.turns.rows.length);
      published.publish(state);
    });
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    fleetSide.send(JSON.stringify(OPENED));
    for (let at = 0; at < 2048; at += 1) fleetSide.send(JSON.stringify({ ...A_ROW, text: `row ${at}` }));
    fleetSide.send(JSON.stringify({ message: "closed", because: "drone_ended" }));

    const ended = await published.until((state) => state.state === "ended");
    const rows = "turns" in ended ? ended.turns.rows : [];
    expect(rows.map((row) => row.seq)).toEqual([...Array(2048).keys()]);
    expect(rows.at(-1)?.saw).toEqual({ event: "said", text: "row 2047" });
    expect(published.seen.map((state) => ("turns" in state ? state.turns.rows.length : null)).filter((n) => n !== null)).toEqual(lengths);
  });
});

describe("a Bridge that falls behind a Job's turns", () => {
  // #1759, spike 022's choice: `/events`' resync one socket over. The history
  // is the transcript file, so one bounded backfill redraws what was dropped.
  it("reopens, and the backfill redraws the pane whole without emptying it first", async () => {
    const server = new WebSocketServer({ host: HOST, port: 0 });
    await once(server, "listening");
    opened.push(() => server.close());
    const connections: Promise<Socket>[] = [];
    let next: (socket: Socket) => void = () => {};
    const awaitConnection = () => connections.push(new Promise((keep) => (next = keep)));
    awaitConnection();
    server.on("connection", (socket: Socket) => {
      const keep = next;
      awaitConnection();
      keep(socket);
    });
    const arrived = (n: number) => connections[n - 1] as Promise<Socket>;
    const published = watching();
    const turns = new ObserveSocket((state) => published.publish(state));
    opened.push(() => turns.close());

    turns.open((server.address() as AddressInfo).port, A_JOB);
    const first = await arrived(1);
    first.send(JSON.stringify(OPENED));
    first.send(JSON.stringify({ ...A_ROW, text: "row 0" }));
    first.send(JSON.stringify({ message: "missed", dropped: 2 }));
    first.send(JSON.stringify({ ...A_ROW, text: "row 3" }));

    const second = await arrived(2);
    second.send(JSON.stringify(OPENED));
    for (let at = 0; at < 4; at += 1) second.send(JSON.stringify({ ...A_ROW, text: `row ${at}` }));
    const whole = await published.until((state) => "turns" in state && state.turns.rows.length === 4);

    expect("turns" in whole && whole.turns.rows.map((row) => row.saw)).toEqual(
      [0, 1, 2, 3].map((at) => ({ event: "said", text: `row ${at}` })),
    );
    expect("turns" in whole && whole.turns.missed).toBe(0);
    // Nothing between the two connections read as a pane starting over.
    const after = published.seen.slice(published.seen.findIndex((state) => state.state === "watching"));
    expect(after.every((state) => state.state === "watching" && state.turns.rows.length > 0)).toBe(true);
  });
});

describe("a row Fleet stamped with its Drone", () => {
  // **The frame byte for byte as Fleet sends it**: the string
  // `crates/ipc/src/tests/turns.rs` asserts `TurnMessage::Row` encodes to.
  const STAMPED =
    '{"message":"row","ts":"2026-08-27T14:12:00.000Z","by":"drone","drone_id":"01DRONEAAAAAAAAAAAAAAAAAAA","event":"thinking","estimated_tokens":125}';

  it("carries the Drone beside the kind and the estimate inside it", async () => {
    const fleet = await serving();
    const published = watching();
    const turns = new ObserveSocket((state) => published.publish(state));
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(STAMPED);
    const read = await published.until((state) => "turns" in state && state.turns.rows.length === 1);

    const row = "turns" in read ? read.turns.rows[0] : undefined;
    expect(row?.drone_id).toBe("01DRONEAAAAAAAAAAAAAAAAAAA");
    expect(row?.saw).toEqual({ event: "thinking", estimated_tokens: 125 });
  });
});

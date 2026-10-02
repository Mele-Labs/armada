// What a merge line Check's log socket publishes, against a real socket — `journal.test.ts`'s
// reason for a `ws` server on a loopback port. The server sends what `LandOutputMessage` declares.

import { once } from "node:events";
import type { IncomingMessage } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";
import { WebSocketServer, type WebSocket as Socket } from "ws";

import type { FollowedLandLog } from "@armada/protocol";
import { LandFollowSocket } from "./land-following";
import { HOST } from "./runtime-file";

const AT = { root: "/Users/user/armada", branch: "bridge/check-log-sheet", check: "screens_test" };

const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

async function serving(): Promise<{ port: number; talking: Promise<[Socket, IncomingMessage]> }> {
  const server = new WebSocketServer({ host: HOST, port: 0 });
  await once(server, "listening");
  const talking = once(server, "connection") as Promise<[Socket, IncomingMessage]>;
  opened.push(() => server.close());
  return { port: (server.address() as AddressInfo).port, talking };
}

/** Wait until the published reading says what `until` looks for. */
async function settled(read: () => FollowedLandLog, until: (now: FollowedLandLog) => boolean) {
  for (let tries = 0; tries < 100 && !until(read()); tries += 1) {
    await new Promise((settle) => setTimeout(settle, 10));
  }
  return read();
}

describe("a merge line Check's log", () => {
  it("asks by the line's three names and holds what arrives, then why it ended", async () => {
    const fleet = await serving();
    let now: FollowedLandLog = { state: "none" };
    const log = new LandFollowSocket((landFollowed) => (now = landFollowed));
    opened.push(() => log.close());

    log.open(fleet.port, AT);
    const [socket, request] = await fleet.talking;
    const asked = new URL(request.url ?? "", "ws://fleet.invalid");
    expect(asked.pathname).toBe("/merge_lines/checks/observe");
    expect(Object.fromEntries(asked.searchParams)).toEqual(AT);

    const opening = { message: "opened", protocol_version: { major: 23, minor: 4 }, ...AT, skipped: 0 };
    socket.send(JSON.stringify({ ...opening, name: AT.check }));
    socket.send(JSON.stringify({ message: "lines", lines: ["RUN  v3.2.4"] }));
    socket.send(JSON.stringify({ message: "lines", lines: [" ok src/merge-line.test.ts"] }));
    const growing = await settled(() => now, (one) => one.state === "following" && one.lines.length === 2);
    expect(growing).toMatchObject({ state: "following", ...AT, fromLine: 1 });
    expect(growing.state === "following" ? growing.ended : "not following").toBeUndefined();

    socket.send(JSON.stringify({ message: "closed", because: "finished" }));
    const ended = await settled(() => now, (one) => one.state === "following" && one.ended !== undefined);
    expect(ended).toMatchObject({
      state: "following",
      lines: ["RUN  v3.2.4", " ok src/merge-line.test.ts"],
      ended: "finished",
    });
  });

  it("says Fleet is not connected rather than opening nothing", () => {
    let now: FollowedLandLog = { state: "none" };
    const log = new LandFollowSocket((landFollowed) => (now = landFollowed));
    log.open(null, AT);
    expect(now).toEqual({ state: "failed", ...AT, detail: "Fleet is not connected." });
  });
});

// What the acts that belong to no one surface put on the wire, read off a real listener.
//
// **The subject is the body and nothing else.** A listener rather than a stubbed `fetch`: what is
// under test is what crosses, and a fake that records its arguments records what this side believed
// rather than what was sent.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import { Commands, type Board } from "./command";
import { Picked } from "./picked";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

/** What one request carried: its path, and its body as bytes. */
type Asked = { path: string; body: string };

/** The board an act publishes onto. Nothing here is under test. */
function boardOn(port: number): Board {
  return {
    port: () => port,
    picked: new Picked(),
    publish: () => {},
  };
}

/** A listener that answers a fixed body and records what arrived. */
async function fleetAnswering(body: unknown, into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let text = "";
    request.on("data", (chunk) => (text += String(chunk)));
    request.on("end", () => {
      into.push({ path: request.url ?? "", body: text });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/** `#927`: a preference save names the one it is about, and publishes what
 *  Fleet answers with — `Preferring.save`'s own publish, not a re-read. */
it("saves one preference by name and publishes what Fleet answers with", async () => {
  const asked: Asked[] = [];
  const published: unknown[] = [];
  const port = await fleetAnswering({ where_things_are_open: true }, asked);
  const commands = new Commands({
    ...boardOn(port),
    publish: (change) => published.push(change),
  });

  const answer = await commands.savePreference({ name: "where_things_are_open", value: true });

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe("/preferences/save");
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    name: "where_things_are_open",
    value: true,
  });
  expect(published).toEqual([{ preferences: { where_things_are_open: true } }]);
});

/** Nothing connected is nowhere to send a save to. */
it("refuses a preference save with nothing connected, rather than throwing", async () => {
  const commands = new Commands({ ...boardOn(0), port: () => null });

  const answer = await commands.savePreference({ name: "where_things_are_open", value: true });

  expect(answer).toEqual({ ok: false, why: "not_connected" });
});

/** A listener that answers `search_files` with a fixed list, and records the path asked. */
async function fleetListing(paths: string[], into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    into.push({ path: request.url ?? "", body: "" });
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify({ paths }));
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/**
 * The `@` mention popup's own read. What a person typed rides the querystring,
 * encoded — a bare `@` narrows with an empty `q` and a path or a space in the
 * typed text must survive the trip.
 */
it("asks search_files with the typed query, encoded, and answers its paths", async () => {
  const asked: Asked[] = [];
  const commands = new Commands(
    boardOn(await fleetListing(["src/log.rs", "README.md"], asked)),
  );

  const found = await commands.searchFiles("a query/with slashes");

  expect(found).toEqual(["src/log.rs", "README.md"]);
  expect(asked[0]?.path).toBe(`/manifest/files?q=${encodeURIComponent("a query/with slashes")}`);
});

/** A Session's message box names its own repository, whatever the window has picked. */
it("asks search_files of the repository it is named, not the window's pick", async () => {
  const asked: Asked[] = [];
  const commands = new Commands(boardOn(await fleetListing(["src/cart.rs"], asked)));

  await commands.searchFiles("cart", undefined, "01STORE");

  expect(asked[0]?.path).toBe("/manifest/files?q=cart&manifest_id=01STORE");
});

/** A listener that answers `read_request_repository` with `reading`, and records the path asked. */
async function fleetReading(reading: object, into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    into.push({ path: request.url ?? "", body: "" });
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify(reading));
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/** The words ride the querystring, encoded, and Fleet's pick comes back as the id it named. */
it("asks read_request_repository with the words, encoded, and answers the pick and the ranking", async () => {
  const asked: Asked[] = [];
  const commands = new Commands(boardOn(await fleetReading({ manifest_id: "01STORE", likely: ["01STORE", "01POCKET"] }, asked)));

  const read = await commands.readRequestRepository("fix the cart in @src/cart.rs");

  expect(read).toEqual({ manifest_id: "01STORE", likely: ["01STORE", "01POCKET"] });
  expect(asked[0]?.path).toBe(`/repositories/of_request?q=${encodeURIComponent("fix the cart in @src/cart.rs")}`);
});

/** Fleet leaves `manifest_id` out where it could not tell; the renderer compares against `null`. */
it("answers a null pick where Fleet left manifest_id out", async () => {
  const commands = new Commands(boardOn(await fleetReading({ likely: ["01POCKET", "01STORE"] }, [])));

  const read = await commands.readRequestRepository("make it faster");

  expect(read).toEqual({ manifest_id: null, likely: ["01POCKET", "01STORE"] });
});

/** A read that fires on a pause in typing is never a fault: no pick, no ranking, and the panel asks. */
it("answers no pick and no ranking where there is nowhere to ask or Fleet would not answer", async () => {
  const unconnected = new Commands({ ...boardOn(0), port: () => null });
  expect(await unconnected.readRequestRepository("anything")).toEqual({ manifest_id: null, likely: [] });

  const server = createServer((_request, response) => {
    response.writeHead(500, { "content-type": "application/json" });
    response.end("{}");
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  const refused = new Commands(boardOn((server.address() as AddressInfo).port));
  expect(await refused.readRequestRepository("anything")).toEqual({ manifest_id: null, likely: [] });
});

/**
 * Fires on every keystroke against a form that may not be connected to
 * anything yet — see `Commands.searchFiles`'s own note. Nowhere to ask is
 * answered empty, not as a refusal a popup would have to render.
 */
it("answers no paths at all where there is nowhere to ask", async () => {
  const commands = new Commands({ ...boardOn(0), port: () => null });

  const found = await commands.searchFiles("anything");

  expect(found).toEqual([]);
});

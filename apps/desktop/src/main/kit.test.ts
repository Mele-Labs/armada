// Taking a command out of Kit's allowlist, as it reaches Fleet: the route, the
// method and the body (`docs/concepts/kit.md`, *Kit's allowlist*). Read off a
// real listener, `lesson-acts.test.ts`'s reason.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { KitAllowedCommands } from "@armada/protocol";
import { KitCommands } from "./kit";
import { Picked } from "./picked";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

type Asked = { method: string; path: string; body: string };

async function fleetAnswering(status: number, body: unknown, into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let text = "";
    request.on("data", (chunk) => (text += String(chunk)));
    request.on("end", () => {
      into.push({ method: request.method ?? "", path: request.url ?? "", body: text });
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

const LEFT: KitAllowedCommands = {
  commands: [{ run: "grep -n", source: "retro_item", lesson_id: "01K7LESSON" }],
};

it("removes a command on its own route, by the line as the inventory spelled it, with no repository in the path", async () => {
  const asked: Asked[] = [];
  const port = await fleetAnswering(200, LEFT, asked);

  const answer = await new KitCommands(() => port, new Picked()).removeAllowedCommand("gh issue view");

  expect(answer).toEqual({ ok: true, commands: LEFT });
  expect(asked).toHaveLength(1);
  expect(asked[0]?.method).toBe("POST");
  expect(asked[0]?.path).toBe("/kit/allowed_commands/remove");
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ run: "gh issue view" });
});

it("answers a line Fleet does not hold as the refusal", async () => {
  const port = await fleetAnswering(
    409,
    { code: "fleet.kit_allowlist_refused", message: "no line is spelled that way" },
    [],
  );

  const answer = await new KitCommands(() => port, new Picked()).removeAllowedCommand("gone");

  expect(answer.ok).toBe(false);
});

it("sends nothing where no Fleet is connected", async () => {
  const answer = await new KitCommands(() => null, new Picked()).removeAllowedCommand("gh");
  expect(answer).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
});

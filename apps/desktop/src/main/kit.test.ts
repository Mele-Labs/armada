// Taking a command out of Kit's allowlist, as it reaches Fleet: the route, the
// method and the body (`docs/concepts/kit.md`, *Kit's allowlist*). Read off a
// real listener, `lesson-acts.test.ts`'s reason.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { KitAllowedCommands, KitInventory } from "@armada/protocol";
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

// Add and Remove for a plugin or a skill. Machine-wide like the inventory, so no route names a repository, and a
// 422 `fleet.kit_piece_refused` arrives as the refusal the neighbouring Kit acts give.
// `docs/practices/protocol/kit-add-and-remove-a-plugin-or-a-skill.md`.

const SETUP = { harness: "claude", home: "/Users/user/.claude", kinds: [] } as unknown as KitInventory;

it.each([
  ["lookAtKitSource", "/kit/pieces/look", { source: "acme/widgets" }, { found: "unknown", why: "nothing there" }, { found: { found: "unknown", why: "nothing there" } }],
  ["installKitPiece", "/kit/pieces/install", { source: "acme/widgets", kind: "plugin", manifest: false }, { state: "installed", setup: SETUP }, { installed: { state: "installed", setup: SETUP } }],
  ["removeKitPiece", "/kit/pieces/remove", { kind: "skill", name: "humanizer" }, { setup: SETUP, trashed: "/Users/user/.Trash/humanizer" }, { removed: { setup: SETUP, trashed: "/Users/user/.Trash/humanizer" } }],
  ["restoreKitPiece", "/kit/pieces/restore", { trashed: "/Users/user/.Trash/humanizer" }, SETUP, { setup: SETUP }],
] as const)("%s posts its body to %s, with no repository in the path", async (method, route, body, answered, read) => {
  const asked: Asked[] = [];
  const port = await fleetAnswering(200, answered, asked);

  const kit = new KitCommands(() => port, new Picked()) as unknown as Record<string, (body: unknown) => Promise<unknown>>;
  const answer = await kit[method]!(body);

  expect(answer).toEqual({ ok: true, ...read });
  expect(asked).toHaveLength(1);
  expect(asked[0]?.method).toBe("POST");
  expect(asked[0]?.path).toBe(route);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual(body);
});

it("answers Fleet's 422 refusal of a piece as the refusal, in Fleet's words", async () => {
  const message = "Nothing was installed. Kit's armada.yml has no place for a plugin or a skill yet, so it cannot be added there.";
  const port = await fleetAnswering(422, { code: "fleet.kit_piece_refused", message }, []);

  const answer = await new KitCommands(() => port, new Picked()).installKitPiece({ source: "acme/widgets", kind: "plugin", manifest: true });

  expect(answer.ok).toBe(false);
  expect(JSON.stringify(answer)).toContain("fleet.kit_piece_refused");
});

it("sends no piece act where no Fleet is connected", async () => {
  const answer = await new KitCommands(() => null, new Picked()).removeKitPiece({ kind: "plugin", name: "impeccable@impeccable" });
  expect(answer).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
});

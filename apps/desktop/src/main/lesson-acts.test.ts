// Agreeing and disagreeing with one retro item, as they reach Fleet
// (`docs/concepts/retro.md`). What is asserted is the request off a real
// listener, `command.test.ts`'s reason: the route, the method, the caller and
// that nothing rides the body.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { Lesson } from "@armada/protocol";
import { JobCommands, type Board } from "./command";
import { Picked } from "./picked";

const LESSON: Lesson = {
  id: "01K7LESSON",
  job_id: "01K6JOB",
  handle: "3-retire-guides",
  at: "2026-10-02T22:05:00Z",
  who: "fleet",
  statement: "The gate measured from local main.",
  evidence: [],
  lands_in: "armada",
  state: "agreed",
  job_proposed: "01K7PROPOSED",
};

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

type Asked = { method: string; path: string; body: string; caller: string };

async function fleetAnswering(status: number, body: unknown, into: Asked[], hold?: Promise<void>): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let text = "";
    request.on("data", (chunk) => (text += String(chunk)));
    request.on("end", async () => {
      into.push({
        method: request.method ?? "",
        path: request.url ?? "",
        body: text,
        caller: String(request.headers["x-armada-caller"]),
      });
      if (hold !== undefined) await hold;
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

function boardOn(port: number, rereads: number[] = []): Board {
  return {
    port: () => port,
    picked: new Picked(),
    fold: () => {},
    forget: () => {},
    reread: async (at) => void rereads.push(at),
    refresh: () => {},
    publish: () => {},
    watchProposal: () => {},
    proposalOut: () => null,
    rereadCapacity: async () => {},
  };
}

it("agrees with an item on its own route, as Bridge, with no body, and reads the board again", async () => {
  const asked: Asked[] = [];
  const rereads: number[] = [];
  const port = await fleetAnswering(200, LESSON, asked);
  const commands = new JobCommands(boardOn(port, rereads));

  const answer = await commands.agreeLesson(LESSON.id);

  expect(answer).toEqual({ ok: true, lesson: LESSON });
  expect(asked).toEqual([{ method: "POST", path: "/lessons/01K7LESSON/agree", body: "", caller: "bridge" }]);
  // A proposed Job is a row on the Board that was not there.
  expect(rereads).toEqual([port]);
});

it("disagrees with an item on its own route, with no body", async () => {
  const asked: Asked[] = [];
  const port = await fleetAnswering(200, { ...LESSON, state: "discarded" }, asked);
  const commands = new JobCommands(boardOn(port));

  const answer = await commands.disagreeLesson(LESSON.id);

  expect(answer.ok).toBe(true);
  expect(asked).toEqual([{ method: "POST", path: "/lessons/01K7LESSON/disagree", body: "", caller: "bridge" }]);
});

it("answers a refused agree as the refusal, and reads nothing again", async () => {
  const rereads: number[] = [];
  const port = await fleetAnswering(404, { code: "lesson_not_found", message: "no such lesson" }, []);
  const commands = new JobCommands(boardOn(port, rereads));

  const answer = await commands.agreeLesson("01K7GONE");

  expect(answer.ok).toBe(false);
  expect(rereads).toEqual([]);
});

it("refuses a second press on one item while the first is out, and sends it once", async () => {
  const asked: Asked[] = [];
  let release: () => void = () => {};
  const hold = new Promise<void>((done) => (release = done));
  const port = await fleetAnswering(200, LESSON, asked, hold);
  const commands = new JobCommands(boardOn(port));

  const first = commands.agreeLesson(LESSON.id);
  await expect.poll(() => asked.length).toBe(1);
  const second = await commands.disagreeLesson(LESSON.id);
  expect(second).toEqual({ ok: false, outcome: { ok: false, why: "already_answering_lesson" } });
  release();
  await first;
  expect(asked).toHaveLength(1);
});

it("sends nothing where no Fleet is connected", async () => {
  const board = { ...boardOn(0), port: () => null };
  const answer = await new JobCommands(board).agreeLesson(LESSON.id);
  expect(answer).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
});

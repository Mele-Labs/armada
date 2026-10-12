// The review and the question about one item, as they reach Fleet: the route,
// the caller, the body, and what the renderer's argument is cut to
// (`lesson-acts.test.ts`'s reason: what is asserted is the request off a real listener).

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";
import type { AskTurn } from "@armada/jobs/review-wire";

import { Picked } from "./picked";
import { askOf, reviewOf } from "./request";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

type Asked = { method: string; path: string; body: string; caller: string };

async function fleetAnswering(status: number, body: unknown, into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let text = "";
    request.on("data", (chunk) => (text += String(chunk)));
    request.on("end", () => {
      into.push({ method: request.method ?? "", path: request.url ?? "", body: text, caller: String(request.headers["x-armada-caller"]) });
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

const REVIEW = { model: "sonnet", entries: [{ lesson_id: "a", merged_ids: [], reason: "First." }], set_aside: [] };

it("asks for the review with a POST as Bridge, naming no manifest on All", async () => {
  const asked: Asked[] = [];
  const port = await fleetAnswering(200, REVIEW, asked);

  const answer = await reviewOf(port, new Picked());

  expect(answer).toEqual({ ok: true, review: REVIEW });
  expect(asked).toEqual([{ method: "POST", path: "/lessons/review", body: "{}", caller: "bridge" }]);
});

it("answers Fleet's refusal of a review as the refusal", async () => {
  const port = await fleetAnswering(500, { code: "fleet.lesson_review_failed", message: "the call failed" }, []);
  const answer = await reviewOf(port, new Picked());
  expect(answer.ok).toBe(false);
});

it("puts a question on the item's own route with the thread cut to roles and text", async () => {
  const asked: Asked[] = [];
  const port = await fleetAnswering(200, { answer: "Two Jobs." }, asked);
  const history = [
    { role: "person", text: "Which?", extra: "dropped" },
    { role: "root", text: "not a turn" },
    { role: "fleet", text: "Two." },
  ] as unknown as AskTurn[];

  const answer = await askOf(port, "01K7/ODD", "And then?", history);

  expect(answer).toEqual({ ok: true, answer: { answer: "Two Jobs." } });
  expect(asked).toHaveLength(1);
  expect(asked[0]?.path).toBe("/lessons/01K7%2FODD/ask");
  expect(JSON.parse(asked[0]?.body ?? "")).toEqual({
    question: "And then?",
    history: [
      { role: "person", text: "Which?" },
      { role: "fleet", text: "Two." },
    ],
  });
});

it("answers Fleet's refusal of a question as the refusal", async () => {
  const port = await fleetAnswering(422, { code: "fleet.lesson_question_refused", message: "empty question" }, []);
  expect((await askOf(port, "a", "", [])).ok).toBe(false);
});

// What a restart and a raise put on the wire, read off a real listener.
//
// **The subject is the body and nothing else.** `restart_step` took no body at
// all until #396, and the whole promise of the change is that restarting with
// nothing to say still sends none — so what is asserted here is bytes, not an
// outcome. A version that sent `{"note":""}` for an empty field would look
// identical from the renderer and would earn a 422 from fleet.
//
// A listener rather than a stubbed `fetch`, for `proposing.test.ts`'s reason:
// what is under test is what crosses, and a fake that records its arguments
// records what this side believed rather than what was sent.
//
// **The raise is here for the same reason at a different scale.** It carries a
// figure rather than a sentence, in millionths of a dollar where a person typed
// dollars, and neither side would notice a factor of a million: what it puts on
// the wire is bytes, and bytes are what is asserted.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { JobSummary } from "@armada/protocol";
import { JobCommands, type Board } from "./command";
import { Picked } from "./picked";

const A_JOB: JobSummary = {
  id: "01M1HQZAKN001AJ5MT3PT09KKY",
  handle: "12-a-job",
  title: "Make the parser take it",
  status: "queued",
  workflow_id: "bug",
  owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-09-03T19:00:00Z",
};

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

/** What one request carried: its path, and its body as bytes. */
type Asked = { path: string; body: string };

/** A listener that answers every command with a Job and records what arrived. */
async function fleetRecording(into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let body = "";
    request.on("data", (chunk) => (body += String(chunk)));
    request.on("end", () => {
      into.push({ path: request.url ?? "", body });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(A_JOB));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/** The board an act folds into. Nothing here is under test. */
function boardOn(port: number): Board {
  return {
    port: () => port,
    picked: new Picked(),
    fold: () => {},
    forget: () => {},
    reread: async () => {},
    refresh: () => {},
    publish: () => {},
    watchProposal: () => {},
    proposalOut: () => null,
  };
}

/** A listener that answers a fixed body and records what arrived — `fleetRecording`'s shape, for a case whose answer is not a Job. */
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

/**
 * **The promise the whole change rests on.** Restarting with nothing to say is
 * exactly the request it was, so a fleet built before #396 answers it and a
 * person who never opens the field pays nothing for one existing.
 */
it("sends no body at all when a restart carries no note", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.restartStep(A_JOB.id);

  expect(answer.ok).toBe(true);
  expect(asked).toHaveLength(1);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/restart_step`);
  expect(asked[0]?.body).toBe("");
});

/** The words go over verbatim, on the route the restart already used. */
it("sends the note as the body when a restart carries one", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.restartStep(A_JOB.id, "Delete that test, it tests the old behaviour.");

  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/restart_step`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    note: "Delete that test, it tests the old behaviour.",
  });
});

/**
 * A field somebody opened and typed nothing into is a restart with no note, and
 * **this is the one place it differs from `redirectDrone`.** There the note is
 * the act and blank is refused before anything is sent; here the act is the
 * restart, so the restart happens and fleet is never handed the 422 a blank
 * note earns.
 */
it("drops a blank note rather than refusing the restart", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.restartStep(A_JOB.id, "   \n  ");

  expect(answer.ok).toBe(true);
  expect(asked[0]?.body).toBe("");
});

/**
 * `#1105`. `rerunGate`'s route sends no body either; this is the same act's
 * shape, on the wire it re-runs.
 */
it("sends no body asking a stopped step's checks to run again", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.rerunChecks(A_JOB.id);

  expect(answer.ok).toBe(true);
  expect(asked).toHaveLength(1);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/rerun_checks`);
  expect(asked[0]?.body).toBe("");
});

/** Surrounding whitespace is not part of what a person said. */
it("trims what it sends", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.restartStep(A_JOB.id, "  start from the failing case  ");

  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ note: "start from the failing case" });
});

/**
 * **The reason rides with the refusal**, since protocol 11.5. One act and one
 * body: the reason is in a person's head as they press reject, and carrying it
 * in a second box is what spent it.
 */
it("sends a rejection's note with the answer", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.answerCommand(A_JOB.id, "call_1", "reject", "we are not taking that dependency");

  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/answer_command`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    call: "call_1",
    answer: "reject",
    note: "we are not taking that dependency",
  });
});

/**
 * A field somebody opened and typed nothing into is a bare refusal.
 * **`restart_step`'s rule**: no `note` key at all, which is the body every
 * fleet before 11.5 took and the one a blank string would not be.
 */
it("drops a blank note rather than sending an empty one", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.answerCommand(A_JOB.id, "call_1", "reject", "  \n ");

  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ call: "call_1", answer: "reject" });
});

/**
 * **The rule rides with an always-allow**, since protocol 13.4. What is
 * declared is the rule a person picked off the offer's own candidates, never
 * the whole command.
 */
it("sends an always-allow's rule with the answer", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.answerCommand(A_JOB.id, "call_1", "always_allow", undefined, "gh issue view");

  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    call: "call_1",
    answer: "always_allow",
    rule: "gh issue view",
  });
});

/**
 * **`scope: "kit"` rides with an always-allow**, since protocol 23.35, and with
 * the rule: the whole of a chained command is refused for Kit. A repository
 * always-allow sends no `scope`, which is what it has always meant.
 */
it("sends an always-allow in Kit with its scope and rule, and a repository one with neither", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.answerCommand(A_JOB.id, "call_1", "always_allow", undefined, "gh issue view", "kit");
  await commands.answerCommand(A_JOB.id, "call_2", "always_allow", undefined, "gh issue view");

  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    call: "call_1",
    answer: "always_allow",
    rule: "gh issue view",
    scope: "kit",
  });
  expect(JSON.parse(asked[1]?.body ?? "null")).toEqual({
    call: "call_2",
    answer: "always_allow",
    rule: "gh issue view",
  });
});

/** An allow tells the drone everything it acts on, so it carries no prose. */
it("sends an allow exactly as it did before a note existed", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.answerCommand(A_JOB.id, "call_1", "allow_for_job");

  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    call: "call_1",
    answer: "allow_for_job",
  });
});

/**
 * **The unit crosses as micros.** The renderer hands over millionths of a
 * dollar because `JobSpend` reads in them, and the body carries that integer
 * unchanged — a raise sent in dollars would be a cap below a hundredth of a
 * cent, refused as no raise at all.
 */
it("sends the new cost cap in micros", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseCostCap(A_JOB.id, 20_000_000);

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/raise_cost_cap`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    cost_cap_micros: 20_000_000,
  });
});

/**
 * **Bridge names no raiser.** Fleet places the caller from the connection
 * (`#2165`), so the body carries the figure and nothing that claims who sent it.
 */
it("never claims to be anybody", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.raiseCostCap(A_JOB.id, 12_500_000);

  expect(JSON.parse(asked[0]?.body ?? "null")).not.toHaveProperty("raised_by");
});

/**
 * A figure that cannot be a cap is refused before anything goes out. **Not the
 * comparison against the cap in force** — that one is the dialog's, which has
 * both numbers — this is the floor no ceiling can be under.
 */
it("sends nothing at all for a cap of nothing", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseCostCap(A_JOB.id, 0);

  expect(answer).toEqual({ ok: false, why: "cap_not_raised" });
  expect(asked).toHaveLength(0);
});

/**
 * **The unit does not cross, which is this act's own way of going wrong.** The
 * cost cap converts dollars to micros and the case above pins the factor; a
 * turn cap is a turn count on both sides, so what has to be pinned here is that
 * nothing multiplies it. A raise to 600 sent as 600000000 is a ceiling no job
 * ever reaches, and neither side would notice.
 */
it("sends the new turn cap as a turn count, on its own route", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseTurnCap(A_JOB.id, 600);

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/raise_turn_cap`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    turn_cap: 600,
  });
});

/**
 * A figure that cannot be a cap is refused before anything goes out, and it
 * says which ceiling it was about. **Its own refusal beside `cap_not_raised`**:
 * a message naming money would send somebody to the control that cannot start
 * this job.
 */
it("sends nothing at all for a turn cap of nothing", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseTurnCap(A_JOB.id, 0);

  expect(answer).toEqual({ ok: false, why: "turn_cap_not_raised" });
  expect(asked).toHaveLength(0);
});

/**
 * **The clear is a key with nothing in it, never a missing key.** Fleet refuses
 * a `set_model` body with no `model` rather than reading it as a clear, so a
 * body built with `undefined` — which `JSON.stringify` drops — would earn a 422
 * for the one press that hands the choice back to the workflow.
 */
it("sends a cleared model as null, with the key present", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.setModel(A_JOB.id, null);

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/set_model`);
  expect(asked[0]?.body).toBe('{"model":null}');
});

/** The command goes back exactly as it was allowed: it is what Fleet matches on. */
it("names the allow it takes back by the whole command", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.removeAllowedCommand(A_JOB.id, "pnpm add -D reselect@5.1.1");

  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/remove_allowed_command`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ run: "pnpm add -D reselect@5.1.1" });
});

/** `#927`: a preference save names the one it is about, and publishes what
 *  Fleet answers with — `Preferring.save`'s own publish, not a re-read. */
it("saves one preference by name and publishes what Fleet answers with", async () => {
  const asked: Asked[] = [];
  const published: unknown[] = [];
  const port = await fleetAnswering({ where_things_are_open: true }, asked);
  const commands = new JobCommands({
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

/** Nothing connected is nowhere to send a save to, `raiseCostCap`'s reason. */
it("refuses a preference save with nothing connected, rather than throwing", async () => {
  const commands = new JobCommands({ ...boardOn(0), port: () => null });

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
  const commands = new JobCommands(
    boardOn(await fleetListing(["src/log.rs", "README.md"], asked)),
  );

  const found = await commands.searchFiles("a query/with slashes");

  expect(found).toEqual(["src/log.rs", "README.md"]);
  expect(asked[0]?.path).toBe(`/manifest/files?q=${encodeURIComponent("a query/with slashes")}`);
});

/** A Session's message box names its own repository, whatever the window has picked. */
it("asks search_files of the repository it is named, not the window's pick", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetListing(["src/cart.rs"], asked)));

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
  const commands = new JobCommands(boardOn(await fleetReading({ manifest_id: "01STORE", likely: ["01STORE", "01POCKET"] }, asked)));

  const read = await commands.readRequestRepository("fix the cart in @src/cart.rs");

  expect(read).toEqual({ manifest_id: "01STORE", likely: ["01STORE", "01POCKET"] });
  expect(asked[0]?.path).toBe(`/repositories/of_request?q=${encodeURIComponent("fix the cart in @src/cart.rs")}`);
});

/** Fleet leaves `manifest_id` out where it could not tell; the renderer compares against `null`. */
it("answers a null pick where Fleet left manifest_id out", async () => {
  const commands = new JobCommands(boardOn(await fleetReading({ likely: ["01POCKET", "01STORE"] }, [])));

  const read = await commands.readRequestRepository("make it faster");

  expect(read).toEqual({ manifest_id: null, likely: ["01POCKET", "01STORE"] });
});

/** A read that fires on a pause in typing is never a fault: no pick, no ranking, and the panel asks. */
it("answers no pick and no ranking where there is nowhere to ask or Fleet would not answer", async () => {
  const unconnected = new JobCommands({ ...boardOn(0), port: () => null });
  expect(await unconnected.readRequestRepository("anything")).toEqual({ manifest_id: null, likely: [] });

  const server = createServer((_request, response) => {
    response.writeHead(500, { "content-type": "application/json" });
    response.end("{}");
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  const refused = new JobCommands(boardOn((server.address() as AddressInfo).port));
  expect(await refused.readRequestRepository("anything")).toEqual({ manifest_id: null, likely: [] });
});

/**
 * Fires on every keystroke against a form that may not be connected to
 * anything yet — see `JobCommands.searchFiles`'s own note. Nowhere to ask is
 * answered empty, not as a refusal a popup would have to render.
 */
it("answers no paths at all where there is nowhere to ask", async () => {
  const commands = new JobCommands({ ...boardOn(0), port: () => null });

  const found = await commands.searchFiles("anything");

  expect(found).toEqual([]);
});

/**
 * The running gate's Check acts. **The name rides in the body of one and
 * nothing rides with the other**: `skip_checks` is every unfinished Check.
 */
it("sends the Check a person named to retry_check and skip_check, and nothing to skip_checks", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.retryCheck(A_JOB.id, "lint");
  await commands.skipCheck(A_JOB.id, "unit");
  await commands.skipChecks(A_JOB.id);

  expect(asked.map((one) => one.path)).toEqual([
    `/jobs/${A_JOB.id}/retry_check`,
    `/jobs/${A_JOB.id}/skip_check`,
    `/jobs/${A_JOB.id}/skip_checks`,
  ]);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ check: "lint" });
  expect(JSON.parse(asked[1]?.body ?? "null")).toEqual({ check: "unit" });
  expect(asked[2]?.body).toBe("");
});

/** A mode alone is an answer, so no `chose` key crosses; with a label both do. */
it("sends a question's mode without a label, and beside one", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.answerQuestion(A_JOB.id, "q1", undefined, "quick");
  await commands.answerQuestion(A_JOB.id, "q2", "Keep it", "best");
  const bare = await commands.answerQuestion(A_JOB.id, "q3", undefined);
  const blank = await commands.answerQuestion(A_JOB.id, "q4", "  ", "best");

  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/answer_question`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ question_id: "q1", mode: "quick" });
  expect(JSON.parse(asked[1]?.body ?? "null")).toEqual({ question_id: "q2", chose: "Keep it", mode: "best" });
  expect(bare).toEqual({ ok: false, why: "empty_instruction" });
  expect(blank).toEqual({ ok: false, why: "empty_instruction" });
  expect(asked).toHaveLength(2);
});

/** The command ask and the judge question carry the mode beside their answer. */
it("sends a mode beside a command's answer and a judge's", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.answerCommand(A_JOB.id, "call_1", "allow_for_job", undefined, undefined, undefined, "quick");
  await commands.answerJudge(A_JOB.id, "2026-10-09T10:00:00Z", "agree", undefined, "best");

  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ call: "call_1", answer: "allow_for_job", mode: "quick" });
  expect(JSON.parse(asked[1]?.body ?? "null")).toEqual({
    answer: "agree",
    asked_at: "2026-10-09T10:00:00Z",
    mode: "best",
  });
});

// The Workflows page over the lifecycles and rules Fleet keeps: what is read of them into the editor's layered model, and what a change to the model comes to for Fleet. The mock Fleet's
// slice answers the reads and takes the saves, so the two halves are tried against each other.

import { describe, expect, it, vi } from "vitest";
import { ruleBetween, withNewRule, withRuleAt, withStateAfter, withStateLabelAt, withoutRule, withoutState, withStepLayer, DEFAULT } from "@armada/components";
import type { Workspace } from "@armada/components";
import type { BridgeApi } from "../../../shared/api";
import { lifecycles } from "../mock/slices/lifecycles";
import { storePieces } from "../mock/pieces-seed";
import type { Scenario } from "../mock/moment";
import { readPieces, rootOf, runOps, sent } from "./fleet-pieces";
import type { PiecesApi, Served } from "./fleet-pieces";
import { WORKSPACE } from "./levels";
import { kitPieceAt, opsFrom } from "./write";
import { withPlaces, workspaceOf } from "./read";

const WEB = { id: "01WEB", root: "/Users/user/store-web" };
const API = { id: "01API", root: "/Users/user/store-api" };
const SERVED: Served[] = [
  { manifestId: WEB.id, name: "store-web", root: WEB.root },
  { manifestId: API.id, name: "store-api", root: API.root },
];

/** The mock Fleet over the store's pieces, with the commit a removal goes by recorded. */
function fleet() {
  const commit = vi.fn(async (body: Parameters<BridgeApi["commitToRepository"]>[0]) => ({
    ok: true as const,
    value: { repository: body.repository, session_id: "commit-1", branch: "armada/commit-1", paths: [".armada/rules/close-on-merge.toml"], commit: "4f2c9a1", pull_request: "https://git.example/pull/1" },
  }));
  const api = { ...lifecycles.api({ draft: { pieces: storePieces(WEB, API) } } as Scenario, {} as never), commitToRepository: commit } as unknown as PiecesApi;
  return { api, commit };
}

async function opened() {
  const mock = fleet();
  const answer = await readPieces(mock.api, WEB.id, SERVED);
  if (!answer.ok) throw new Error("not read");
  const workspace = withPlaces(workspaceOf(answer.read)!, ["feature"]);
  return { ...mock, read: answer.read, workspace };
}

const rule = (workspace: Workspace, id: string) => workspace.rules.find((one) => one.id === id)!;

describe("what Fleet keeps, read into the editor's model", () => {
  it("is the Workspace that runs for the repository, with a tab for each of its repositories", async () => {
    const { workspace } = await opened();
    expect(workspace.id).toBe("my-store");
    expect(workspace.repositories).toEqual([
      { id: WEB.id, name: "store-web" },
      { id: API.id, name: "store-api" },
    ]);
    expect(workspace.lifecycles.map((one) => one.id)).toEqual(["ticket", "pull_request", "deploy"]);
  });

  it("draws a repository's own piece as what it changes of the default, and no more", async () => {
    const { workspace } = await opened();
    const ticket = workspace.lifecycles.find((one) => one.id === "ticket")!;
    const review = ticket.states.find((one) => one.id === "in_review")!;
    expect(review.label).toBe("In review");
    expect(review.names).toEqual([{ scope: WEB.id, label: "In QA" }]);
    expect(review.reads).toEqual([
      { source: "Tracker", values: ["In Review"], scope: DEFAULT },
      { source: "Tracker", values: ["QA"], scope: WEB.id },
    ]);
    expect(ticket.states.find((one) => one.id === "todo")!.names).toBeUndefined();
    expect(ticket.kind).toBe("tracker");
    expect(workspace.lifecycles.find((one) => one.id === "pull_request")!.kind).toBe("forge");
  });

  it("says where each rule is kept and the level it applies at", async () => {
    const { workspace } = await opened();
    expect(workspace.rules.map((one) => [one.id, one.kept, one.scope])).toEqual([
      ["progress", "yours", DEFAULT],
      ["close-on-merge", "repository", DEFAULT],
      ["ready", "yours", WORKSPACE],
    ]);
    expect(rule(workspace, "progress")).toMatchObject({ from: { lane: "pull_request", item: "draft" }, to: { lane: "ticket", item: "in_progress" }, autonomy: "acts" });
    expect(rule(workspace, "progress").does).toEqual([{ act: "tracker", text: "Move the ticket to In progress" }]);
  });

  it("draws nothing where Fleet keeps no lifecycle", async () => {
    const none = lifecycles.api({ draft: {} } as Scenario, {} as never) as unknown as PiecesApi;
    const answer = await readPieces(none, WEB.id, SERVED);
    expect(answer.ok && workspaceOf(answer.read)).toBeUndefined();
  });
});

describe("what a change comes to", () => {
  it("is nothing where nothing changed", async () => {
    const { read, workspace } = await opened();
    expect(opsFrom(read, workspace, workspace)).toEqual([]);
  });

  it("saves over the person's own rule, and only the fields that changed", async () => {
    const { read, workspace } = await opened();
    const next = withRuleAt(workspace, "progress", DEFAULT, { autonomy: "asks" });
    const [save, ...rest] = opsFrom(read, workspace, next);
    expect(rest).toEqual([]);
    expect(save).toMatchObject({ op: "save", overwrite: true, piece: { kind: "rule", rule: { id: "progress", scope: { kind: "everywhere" }, autonomy: "asks", name: "Start the ticket" } } });
    expect(save).toMatchObject({ piece: { rule: { does: [{ autonomy: "asks", kind: "tracker_write" }] } } });
  });

  it("saves a repository's rule as the person's own copy, not over the repository's file", async () => {
    const { read, workspace } = await opened();
    const [save] = opsFrom(read, workspace, withRuleAt(workspace, "close-on-merge", DEFAULT, { label: "Close it" }));
    expect(save).toMatchObject({ op: "save", overwrite: false, piece: { rule: { id: "close-on-merge", name: "Close it" } } });
  });

  it("saves what a repository's tab changes as a piece for that repository alone", async () => {
    const { read, workspace } = await opened();
    const [save, ...rest] = opsFrom(read, workspace, withRuleAt(workspace, "progress", WEB.id, { label: "Begin the ticket" }));
    expect(rest).toEqual([]);
    expect(save).toMatchObject({ op: "save", overwrite: false, piece: { rule: { id: "progress", scope: { kind: "within", id: WEB.id }, name: "Begin the ticket" } } });
  });

  it("takes the piece of a level away when the level goes back to the one above", async () => {
    const { read, workspace } = await opened();
    const changed = withRuleAt(workspace, "progress", WEB.id, { label: "Begin the ticket" });
    // Fleet now holds a piece for the repository; reading it and then going back deletes it.
    const mock = fleet();
    await runOps(mock.api, sent(opsFrom(read, workspace, changed)), SERVED);
    const again = await readPieces(mock.api, WEB.id, SERVED);
    if (!again.ok) throw new Error("not read");
    const held = withPlaces(workspaceOf(again.read)!, ["feature"]);
    const back = withRuleAt(held, "progress", WEB.id, { label: "Start the ticket" });
    expect(opsFrom(again.read, held, back)).toEqual([{ op: "delete", id: { kind: "rule", id: "progress", scope: { kind: "within", id: WEB.id } } }]);
  });

  it("names a state at the default by saving the lifecycle there, and one the repository keeps its own copy of too", async () => {
    const { read, workspace } = await opened();
    const ops = opsFrom(read, workspace, withStateLabelAt(workspace, "ticket", "todo", DEFAULT, "Backlog"));
    expect(ops.map((one) => one.op === "save" && [one.piece.kind === "lifecycle" && one.piece.lifecycle.scope, one.overwrite])).toEqual([
      [{ kind: "everywhere" }, false],
      [{ kind: "within", id: WEB.id }, false],
    ]);
  });

  it("adds a state to a tracker's lifecycle at the level that added it, read as its own name", async () => {
    const { read, workspace } = await opened();
    const { workspace: next } = withStateAfter(workspace, "feature", "ticket", "in_progress", "Blocked", WORKSPACE);
    const saved = opsFrom(read, workspace, next).find((one) => one.op === "save" && one.piece.kind === "lifecycle" && one.piece.lifecycle.scope.kind === "within" && one.piece.lifecycle.scope.id === "my-store");
    expect(saved).toMatchObject({ op: "save", overwrite: false });
    const piece = saved?.op === "save" && saved.piece.kind === "lifecycle" ? saved.piece.lifecycle : undefined;
    expect(piece?.states.map((one) => one.id)).toEqual(["todo", "in_progress", "blocked", "in_review", "done"]);
    expect(piece?.states[2]?.read).toEqual({ source: "tracker", words: ["Blocked"] });
    expect(piece?.transitions).toContainEqual({ from: "in_progress", to: "blocked" });
  });

  it("refuses a state added to a lifecycle the forge reads, which has no words to read it by", async () => {
    const { read, workspace } = await opened();
    const { workspace: next } = withStateAfter(workspace, "feature", "pull_request", "draft", "Parked", DEFAULT);
    expect(opsFrom(read, workspace, next)).toEqual([{ op: "refuse", said: "Pull request states are read from forge, so none can be added" }]);
  });

  it("saves a new rule from a connector, by the act Fleet runs for where it lands", async () => {
    const { read, workspace } = await opened();
    const made = ruleBetween(workspace, "feature", { lane: "pull_request", item: "ready" }, { lane: "ticket", item: "in_review" }, { from: "Ready", to: "In review", lane: "Ticket" });
    const [save] = opsFrom(read, workspace, withNewRule(workspace, made));
    expect(save).toMatchObject({
      op: "save",
      overwrite: false,
      piece: {
        kind: "rule",
        rule: {
          when: { kind: "enters", lifecycle: "pull_request", state: "ready" },
          conditions: [{ kind: "workflow", workflow: "feature" }],
          does: [{ kind: "tracker_write", what: { act: "tracker_move", to: "in_review", target: { among: { kind: "held", lifecycle: "ticket" } } } }],
          on_failure: { retries: 2, then: "ask_owner" },
        },
      },
    });
  });

  it("refuses a connector to a state no act brings anything to, and one set off by leaving a state", async () => {
    const { read, workspace } = await opened();
    const toDeploy = ruleBetween(workspace, "feature", { lane: "pull_request", item: "merged" }, { lane: "deploy", item: "healthy" }, { from: "Merged", to: "Healthy", lane: "Deploy" });
    expect(opsFrom(read, workspace, withNewRule(workspace, toDeploy))).toEqual([{ op: "refuse", said: "Fleet has no act that brings Deploy to healthy" }]);
    const leaving = { ...ruleBetween(workspace, "feature", { lane: "ticket", item: "todo" }, { lane: "ticket", item: "in_progress" }, { from: "To do", to: "In progress", lane: "Ticket" }), at: "leaves" as const };
    expect(opsFrom(read, workspace, withNewRule(workspace, leaving))).toEqual([{ op: "refuse", said: "Fleet runs no rule on leaving a state" }]);
  });

  it("deletes the person's own rule and takes a repository's out by a Job of its own", async () => {
    const { read, workspace } = await opened();
    expect(opsFrom(read, workspace, withoutRule(workspace, "progress"))).toEqual([{ op: "delete", id: { kind: "rule", id: "progress", scope: { kind: "everywhere" } } }]);
    expect(opsFrom(read, workspace, withoutRule(workspace, "close-on-merge"))).toEqual([
      { op: "remove", id: { kind: "rule", id: "close-on-merge", scope: { kind: "everywhere" } }, path: `${WEB.root}/.armada/rules/close-on-merge.toml` },
    ]);
  });

  it("takes a state off with the rules that named it", async () => {
    const { read, workspace } = await opened();
    const ops = opsFrom(read, workspace, withoutState(workspace, "pull_request", "draft"));
    expect(ops).toContainEqual({ op: "delete", id: { kind: "rule", id: "progress", scope: { kind: "everywhere" } } });
    expect(ops.some((one) => one.op === "save" && one.piece.kind === "lifecycle" && one.piece.lifecycle.id === "pull_request")).toBe(true);
  });

  it("says what is not saved: the rows of a rule, and a step changed below the default", async () => {
    const { read, workspace } = await opened();
    const rows = { ...workspace, rules: workspace.rules.map((one) => (one.id === "progress" ? { ...one, does: [{ act: "tracker" as const, text: "Move it along" }] } : one)) };
    expect(opsFrom(read, workspace, rows)).toEqual([{ op: "unsaved", said: "If and Do rows are not saved" }]);
    const step = withStepLayer(workspace, "feature", "plan", WEB.id, { label: "Think" }, {});
    expect(opsFrom(read, workspace, step)).toEqual([{ op: "unsaved", said: "What a step ends on, and steps changed below the default, are not saved" }]);
  });
});

describe("doing them", () => {
  it("keeps a saved piece, and reads it back as the person's own", async () => {
    const { api, read, workspace } = await opened();
    const ran = await runOps(api, sent(opsFrom(read, workspace, withRuleAt(workspace, "progress", DEFAULT, { autonomy: "asks" }))), SERVED);
    expect(ran).toEqual({ ok: true });
    const again = await readPieces(api, WEB.id, SERVED);
    if (!again.ok) throw new Error("not read");
    expect(rule(workspaceOf(again.read)!, "progress").autonomy).toBe("asks");
    expect(kitPieceAt(again.read, "rule", "progress", WEB.id)).toEqual({ kind: "rule", id: "progress", scope: { kind: "everywhere" } });
    expect(kitPieceAt(again.read, "rule", "close-on-merge", WEB.id)).toBeUndefined();
  });

  it("stops at what Fleet refuses, in its own words", async () => {
    const { api, read, workspace } = await opened();
    const error = { code: "fleet.piece_unfit", message: "progress names a lifecycle, nowhere, that is not kept", run_id: "r", fields: {}, chain: [] };
    const refusing = { ...api, saveLifecyclePiece: async () => ({ ok: false as const, outcome: { ok: false as const, why: "refused" as const, error } }) } as PiecesApi;
    const ran = await runOps(refusing, sent(opsFrom(read, workspace, withRuleAt(workspace, "progress", DEFAULT, { autonomy: "off" }))), SERVED);
    expect(ran).toEqual({ ok: false, said: "progress names a lifecycle, nowhere, that is not kept" });
  });

  it("takes a repository's piece out by a commit that removes it, from the root of the repository that holds it", async () => {
    const { api, commit, read, workspace } = await opened();
    const ran = await runOps(api, sent(opsFrom(read, workspace, withoutRule(workspace, "close-on-merge"))), SERVED);
    expect(commit).toHaveBeenCalledWith({ repository: WEB.root, piece: { kind: "lifecycle_rule", id: "close-on-merge", scope: { kind: "everywhere" } }, remove: true });
    expect(ran).toEqual({ ok: true, said: "Removed from the repository when its pull request merges" });
  });

  it("is read only, and says no error, on a Fleet that does not serve the route", async () => {
    const { api, read, workspace } = await opened();
    const gone = { ok: false as const, outcome: { ok: false as const, why: "transport" as const, detail: "404", fault: { method: "POST", path: "/lifecycles/pieces/save", why: "unanswerable" as const, status: 404 } } };
    const old = { ...api, saveLifecyclePiece: async () => gone } as PiecesApi;
    const ran = await runOps(old, sent(opsFrom(read, workspace, withRuleAt(workspace, "progress", DEFAULT, { autonomy: "asks" }))), SERVED);
    expect(ran).toEqual({ ok: false, said: "", old: true });
  });

  it("finds the repository whose folder holds a file", () => {
    expect(rootOf(`${API.root}/.armada/rules/x.toml`, SERVED)).toBe(API.root);
    expect(rootOf("/elsewhere/.armada/rules/x.toml", SERVED)).toBeUndefined();
  });
});

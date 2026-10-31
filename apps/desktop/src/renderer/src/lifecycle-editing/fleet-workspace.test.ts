// The Workflows page over the lifecycles and rules Fleet keeps: what is read of them into the editor's layered model, and what a change to the model comes to for Fleet. The mock Fleet's
// slice answers the reads and takes the saves, so the two halves are tried against each other.

import { describe, expect, it, vi } from "vitest";
import { DEFAULT, workspaceLevel, ruleAt, ruleBetween, withLifecycleAdded, withLifecycleLabel, withNewRule, withRuleWireAt, withStateAfter, withStateLabelAt, withStepLayer, withoutLifecycle, withoutRule, withoutState, withoutRuleOverride } from "@armada/components";
import type { Workspace } from "@armada/components";
import type { LifecycleRule } from "@armada/protocol";
import type { BridgeApi } from "../../../shared/api";
import { lifecycles } from "../mock/slices/lifecycles";
import { storePieces } from "../mock/pieces-seed";
import type { Scenario } from "../mock/moment";
import { readPieces, rootOf, runOps } from "./fleet-pieces";
import type { PiecesApi, Served } from "./fleet-pieces";
import { kitPieceAt, opsFrom, withWires } from "./write";
import { landing, namedOf, withDrawn, withPlaces, workspaceOf } from "./read";

/** The level the Workspace `my-store` is kept at. */
const WORKSPACE = workspaceLevel("my-store");

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

/** A rule's piece changed on a tab, the way the editor's form changes it, with every field drawn again from the piece as the page does. */
const edited = (workspace: Workspace, id: string, tab: string, change: (piece: LifecycleRule) => LifecycleRule): Workspace => {
  const now = ruleAt(rule(workspace, id), tab, workspace).wire!;
  return withDrawn(withRuleWireAt(workspace, id, tab, change(now)));
};

describe("what Fleet keeps, read into the editor's model", () => {
  it("is the Workspace that runs for the repository, with a tab for each of its repositories", async () => {
    const { workspace } = await opened();
    expect(workspace.id).toBe("my-store");
    expect(workspace.repositories).toEqual([
      { id: WEB.id, name: "store-web", in: ["my-store"] },
      { id: API.id, name: "store-api", in: ["my-store"] },
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

describe("what a rule's piece is read as", () => {
  it("is the whole piece at the level that keeps it, which the fields are drawn from", async () => {
    const { workspace } = await opened();
    const progress = rule(workspace, "progress");
    expect(progress.wire).toMatchObject({ id: "progress", name: "Start the ticket", when: { kind: "enters", lifecycle: "pull_request", state: "draft" } });
    expect(progress.overrides).toBeUndefined();
  });

  it("is a level's own piece whole where that level keeps one, and the level above's for the rest", async () => {
    const mock = fleet();
    const own = withDrawn(workspaceOf((await readPieces(mock.api, WEB.id, SERVED) as { ok: true; read: never }).read)!);
    const piece = { ...ruleAt(rule(own, "progress"), DEFAULT, own).wire!, name: "Begin the ticket", scope: { kind: "within" as const, id: WEB.id } };
    await mock.api.saveLifecyclePiece({ piece: { kind: "rule", rule: piece } });
    const again = await readPieces(mock.api, WEB.id, SERVED);
    if (!again.ok) throw new Error("not read");
    const held = workspaceOf(again.read)!;
    expect(ruleAt(rule(held, "progress"), WEB.id, held).label).toBe("Begin the ticket");
    expect(ruleAt(rule(held, "progress"), WEB.id, held).wire).toEqual(piece);
    // The default's tab and the Workspace's still read the default's piece.
    expect(ruleAt(rule(held, "progress"), DEFAULT, held).label).toBe("Start the ticket");
    expect(ruleAt(rule(held, "progress"), WORKSPACE, held).label).toBe("Start the ticket");
  });

  it("leaves a state out on a tab whose own piece does not have it", async () => {
    const seed = storePieces(WEB, API);
    const web = seed.lifecycles.find((one) => one.lifecycle.id === "ticket" && one.lifecycle.scope.kind === "within")!;
    web.lifecycle = { ...web.lifecycle, states: web.lifecycle.states.filter((one) => one.id !== "todo"), transitions: web.lifecycle.transitions.filter((one) => one.from !== "todo"), ends: web.lifecycle.ends };
    const api = lifecycles.api({ draft: { pieces: seed } } as Scenario, {} as never) as unknown as PiecesApi;
    const answer = await readPieces(api, WEB.id, SERVED);
    if (!answer.ok) throw new Error("not read");
    const ticket = workspaceOf(answer.read)!.lifecycles.find((one) => one.id === "ticket")!;
    expect(ticket.states.find((one) => one.id === "todo")!.absent).toEqual([WEB.id]);
  });
});

describe("what a change comes to", () => {
  it("is nothing where nothing changed", async () => {
    const { read, workspace } = await opened();
    expect(opsFrom(read, workspace, workspace)).toEqual([]);
  });

  it("saves over the person's own rule when its default tab changes, as the piece whole", async () => {
    const { read, workspace } = await opened();
    const next = edited(workspace, "progress", DEFAULT, (piece) => ({ ...piece, autonomy: "asks", does: piece.does.map((one) => ({ ...one, autonomy: "asks" })) }));
    const [save, ...rest] = opsFrom(read, workspace, next);
    expect(rest).toEqual([]);
    expect(save).toMatchObject({ op: "save", overwrite: true, piece: { kind: "rule", rule: { id: "progress", scope: { kind: "everywhere" }, autonomy: "asks", name: "Start the ticket" } } });
    expect(save).toMatchObject({ piece: { rule: { does: [{ autonomy: "asks", kind: "tracker_write" }] } } });
  });

  it("saves what the pickers change of a rule: when, if, do and how it fails", async () => {
    const { read, workspace } = await opened();
    const next = edited(workspace, "progress", DEFAULT, (piece) => ({
      ...piece,
      when: { kind: "step_ends", step: "plan" },
      conditions: [{ kind: "instances", among: { kind: "held", lifecycle: "ticket" }, quantity: "every", in_state: "todo" }, { kind: "at_step", step: "plan" }],
      does: [...piece.does, { what: { act: "tell", message: "Start on it" }, kind: "tell_agent", autonomy: "acts" }],
      on_failure: { retries: 4, then: "tell_agent" },
    }));
    const [save] = opsFrom(read, workspace, next);
    expect(save).toMatchObject({
      op: "save",
      piece: { rule: { when: { kind: "step_ends", step: "plan" }, on_failure: { retries: 4, then: "tell_agent" }, does: [{ what: { act: "tracker_move" } }, { what: { act: "tell", message: "Start on it" } }] } },
    });
  });

  it("saves a repository's rule as the person's own copy, not over the repository's file", async () => {
    const { read, workspace } = await opened();
    const [save] = opsFrom(read, workspace, edited(workspace, "close-on-merge", DEFAULT, (piece) => ({ ...piece, name: "Close it" })));
    expect(save).toMatchObject({ op: "save", overwrite: false, piece: { rule: { id: "close-on-merge", name: "Close it" } } });
  });

  it("saves what a repository's tab changes as a piece for that repository alone", async () => {
    const { read, workspace } = await opened();
    const [save, ...rest] = opsFrom(read, workspace, edited(workspace, "progress", WEB.id, (piece) => ({ ...piece, name: "Begin the ticket" })));
    expect(rest).toEqual([]);
    expect(save).toMatchObject({ op: "save", overwrite: false, piece: { rule: { id: "progress", scope: { kind: "within", id: WEB.id }, name: "Begin the ticket" } } });
  });

  it("writes the default alone when the default changes, and a repository's own piece as it was", async () => {
    const mock = fleet();
    const first = await readPieces(mock.api, WEB.id, SERVED);
    if (!first.ok) throw new Error("not read");
    const start = withPlaces(workspaceOf(first.read)!, ["feature"]);
    await runOps(mock.api, opsFrom(first.read, start, edited(start, "progress", WEB.id, (piece) => ({ ...piece, name: "Begin the ticket" }))), SERVED);
    const again = await readPieces(mock.api, WEB.id, SERVED);
    if (!again.ok) throw new Error("not read");
    const held = withPlaces(workspaceOf(again.read)!, ["feature"]);
    // The repository keeps its own piece; the default's changes, and only the default's is written.
    const ops = opsFrom(again.read, held, edited(held, "progress", DEFAULT, (piece) => ({ ...piece, autonomy: "off", does: piece.does.map((one) => ({ ...one, autonomy: "off" })) })));
    expect(ops).toHaveLength(1);
    expect(ops[0]).toMatchObject({ op: "save", piece: { rule: { scope: { kind: "everywhere" }, autonomy: "off" } } });
  });

  it("takes the piece of a level away when the level goes back to the one above", async () => {
    const mock = fleet();
    const first = await readPieces(mock.api, WEB.id, SERVED);
    if (!first.ok) throw new Error("not read");
    const start = withPlaces(workspaceOf(first.read)!, ["feature"]);
    await runOps(mock.api, opsFrom(first.read, start, edited(start, "progress", WEB.id, (piece) => ({ ...piece, name: "Begin the ticket" }))), SERVED);
    const again = await readPieces(mock.api, WEB.id, SERVED);
    if (!again.ok) throw new Error("not read");
    const held = withPlaces(workspaceOf(again.read)!, ["feature"]);
    expect(opsFrom(again.read, held, withoutRuleOverride(held, "progress", WEB.id))).toEqual([{ op: "delete", id: { kind: "rule", id: "progress", scope: { kind: "within", id: WEB.id } } }]);
    const back = edited(held, "progress", WEB.id, (piece) => ({ ...piece, name: "Start the ticket" }));
    expect(opsFrom(again.read, held, back)).toEqual([{ op: "delete", id: { kind: "rule", id: "progress", scope: { kind: "within", id: WEB.id } } }]);
  });

  it("names a state at the default by saving the lifecycle there alone, and not the repository's own copy", async () => {
    const { read, workspace } = await opened();
    const ops = opsFrom(read, workspace, withStateLabelAt(workspace, "ticket", "todo", DEFAULT, "Backlog"));
    expect(ops.map((one) => one.op === "save" && [one.piece.kind === "lifecycle" && one.piece.lifecycle.scope, one.overwrite])).toEqual([[{ kind: "everywhere" }, false]]);
  });

  it("writes a repository's piece as it runs it, with its own moves, when its tab names a state", async () => {
    const { read, workspace } = await opened();
    const ops = opsFrom(read, workspace, withStateLabelAt(workspace, "ticket", "todo", WEB.id, "Backlog"));
    expect(ops).toHaveLength(1);
    const piece = ops[0]?.op === "save" && ops[0].piece.kind === "lifecycle" ? ops[0].piece.lifecycle : undefined;
    expect(piece?.scope).toEqual({ kind: "within", id: WEB.id });
    expect(piece?.states.find((one) => one.id === "in_review")).toMatchObject({ label: "In QA", read: { source: "tracker", words: ["QA"] } });
    expect(piece?.states.find((one) => one.id === "todo")?.label).toBe("Backlog");
    expect(piece?.transitions).toEqual([
      { from: "todo", to: "in_progress" },
      { from: "in_progress", to: "in_review" },
      { from: "in_review", to: "done" },
    ]);
    expect(piece?.ends).toEqual(["done"]);
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

  it("saves a new rule from a connector, by the act Fleet runs for where it lands", async () => {
    const { read, workspace } = await opened();
    const made = ruleBetween(workspace, "feature", { lane: "pull_request", item: "ready" }, { lane: "ticket", item: "in_review" }, { from: "Ready", to: "In review", lane: "Ticket" });
    const next = withDrawn(withWires(workspace, withNewRule(workspace, made)));
    const [save] = opsFrom(read, workspace, next);
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

  it("draws no rule from a connector Fleet has no act for", async () => {
    const { workspace } = await opened();
    const toDeploy = ruleBetween(workspace, "feature", { lane: "pull_request", item: "merged" }, { lane: "deploy", item: "healthy" }, { from: "Merged", to: "Healthy", lane: "Deploy" });
    expect(withWires(workspace, withNewRule(workspace, toDeploy)).rules.map((one) => one.id)).not.toContain(toDeploy.id);
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

  it("keeps nothing of a step changed below the default, which no piece holds", async () => {
    const { read, workspace } = await opened();
    expect(opsFrom(read, workspace, withStepLayer(workspace, "feature", "plan", WEB.id, { label: "Think" }, {}))).toEqual([]);
  });
});

describe("a whole lifecycle", () => {
  it.each([
    [DEFAULT, { kind: "everywhere" }],
    [WORKSPACE, { kind: "within", id: "my-store" }],
    [WEB.id, { kind: "within", id: WEB.id }],
  ])("is added on the tab %s as a piece of that tab's scope, read from the tracker", async (tab, scope) => {
    const { read, workspace } = await opened();
    const { workspace: next, id } = withLifecycleAdded(workspace, "Incident", tab);
    const ops = opsFrom(read, workspace, next);
    expect(ops).toHaveLength(1);
    expect(ops[0]).toMatchObject({
      op: "save",
      overwrite: false,
      piece: { kind: "lifecycle", lifecycle: { id, name: "Incident", scope, transitions: [{ from: "open", to: "done" }], ends: ["done"] } },
    });
    expect(ops[0]?.op === "save" && ops[0].piece.kind === "lifecycle" ? ops[0].piece.lifecycle.states.map((one) => one.read) : []).toEqual([
      { source: "tracker", words: [] },
      { source: "tracker", words: [] },
    ]);
  });

  it("is read back as the person's own, renamed in the piece that holds its name, and deleted with its file", async () => {
    const { api, read, workspace } = await opened();
    const added = withLifecycleAdded(workspace, "Incident", DEFAULT);
    await runOps(api, opsFrom(read, workspace, added.workspace), SERVED);
    const again = await readPieces(api, WEB.id, SERVED);
    if (!again.ok) throw new Error("not read");
    const held = withPlaces(workspaceOf(again.read)!, ["feature"]);
    expect(held.lifecycles.find((one) => one.id === added.id)).toMatchObject({ label: "Incident", kept: "yours" });
    const [renamed] = opsFrom(again.read, held, withLifecycleLabel(held, added.id, "Outage"));
    expect(renamed).toMatchObject({ op: "save", overwrite: true, piece: { lifecycle: { id: added.id, name: "Outage", scope: { kind: "everywhere" } } } });
    expect(opsFrom(again.read, held, withoutLifecycle(held, added.id))).toEqual([{ op: "delete", id: { kind: "lifecycle", id: added.id, scope: { kind: "everywhere" } } }]);
  });

  it("is a file of the repository's removed by a Job, and Armada's own are no files at all", async () => {
    const { read, workspace } = await opened();
    expect(opsFrom(read, workspace, withoutLifecycle(workspace, "deploy"))).toEqual([]);
    expect(workspace.lifecycles.find((one) => one.id === "deploy")?.kept).toBe("armada");
  });
});

describe("where a rule lands", () => {
  it("is the state a ticket moves to, or the step the Job is on where nothing else lands", async () => {
    const { workspace } = await opened();
    const defs = namedOf(workspace.lifecycles);
    const wire = rule(workspace, "progress").wire!;
    expect(landing(wire, defs)).toEqual({ lane: "ticket", item: "in_progress" });
    const tell: LifecycleRule = { ...wire, does: [{ what: { act: "tell", message: "Go" }, kind: "tell_agent", autonomy: "acts" }] };
    expect(landing(tell, defs)).toBeUndefined();
    expect(landing({ ...tell, conditions: [{ kind: "at_step", step: "plan" }] }, defs)).toEqual({ lane: "workflow", item: "plan" });
  });
});

describe("doing them", () => {
  it("keeps a saved piece, and reads it back as the person's own", async () => {
    const { api, read, workspace } = await opened();
    const next = edited(workspace, "progress", DEFAULT, (piece) => ({ ...piece, autonomy: "asks", does: piece.does.map((one) => ({ ...one, autonomy: "asks" })) }));
    const ran = await runOps(api, opsFrom(read, workspace, next), SERVED);
    expect(ran).toEqual({ ok: true, removing: [] });
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
    const next = edited(workspace, "progress", DEFAULT, (piece) => ({ ...piece, autonomy: "off" }));
    const ran = await runOps(refusing, opsFrom(read, workspace, next), SERVED);
    expect(ran).toEqual({ ok: false, said: "progress names a lifecycle, nowhere, that is not kept" });
  });

  it("takes a repository's piece out by a commit that removes it, from the root of the repository that holds it", async () => {
    const { api, commit, read, workspace } = await opened();
    const ran = await runOps(api, opsFrom(read, workspace, withoutRule(workspace, "close-on-merge")), SERVED);
    expect(commit).toHaveBeenCalledWith({ repository: WEB.root, piece: { kind: "lifecycle_rule", id: "close-on-merge", scope: { kind: "everywhere" } }, remove: true });
    expect(ran).toEqual({ ok: true, removing: [{ kind: "rule", id: "close-on-merge", scope: { kind: "everywhere" } }] });
  });

  it("is read only, and says no error, on a Fleet that does not serve the route", async () => {
    const { api, read, workspace } = await opened();
    const gone = { ok: false as const, outcome: { ok: false as const, why: "transport" as const, detail: "404", fault: { method: "POST", path: "/lifecycles/pieces/save", why: "unanswerable" as const, status: 404 } } };
    const old = { ...api, saveLifecyclePiece: async () => gone } as PiecesApi;
    const ran = await runOps(old, opsFrom(read, workspace, edited(workspace, "progress", DEFAULT, (piece) => ({ ...piece, autonomy: "asks" }))), SERVED);
    expect(ran).toEqual({ ok: false, said: "", old: true });
  });

  it("finds the repository whose folder holds a file", () => {
    expect(rootOf(`${API.root}/.armada/rules/x.toml`, SERVED)).toBe(API.root);
    expect(rootOf("/elsewhere/.armada/rules/x.toml", SERVED)).toBeUndefined();
  });
});

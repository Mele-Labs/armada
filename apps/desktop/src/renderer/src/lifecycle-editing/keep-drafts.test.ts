// The Keep for every Job drafts the Workflows page reads of Fleet: read for the repository it is on, merged with the removals a pull request has open into the clock every piece waiting on
// something wears, read again when Fleet says the drafts changed, and a lifecycle only a draft holds drawn with nothing to rename or delete. The mock Fleet's slice answers the reads.

import { describe, expect, it, vi } from "vitest";
import { lifecycles } from "../mock/slices/lifecycles";
import { workspacesLanes } from "../mock/scenarios/workspaces-lanes";
import { workspacesPiecesKeeps } from "../mock/scenarios/workspaces-pieces-keeps";
import { followKeepDrafts, readKeepDrafts, readPieces } from "./fleet-pieces";
import type { PiecesApi, Served } from "./fleet-pieces";
import { pendingMarks, workspaceDrawn } from "./keep-drafts";

const [web, api] = workspacesLanes.state.holds.repositories ?? [];
const WEB = { id: web!.manifest!.id, root: web!.root };
const API = { id: api!.manifest!.id, root: api!.root };
const SERVED: Served[] = [
  { manifestId: WEB.id, name: "store-web", root: WEB.root },
  { manifestId: API.id, name: "store-api", root: API.root },
];

/** The mock Fleet with the four drafts of `workspaces-pieces-keeps` open for store-web. */
const fleet = () => lifecycles.api(workspacesPiecesKeeps, {} as never) as unknown as PiecesApi;

describe("the drafts of a repository", () => {
  it("are read for the repository the page is on, each as the file its approval would save", async () => {
    const drafts = await readKeepDrafts(fleet(), WEB.id);
    expect(drafts.map((one) => `${one.kind}:${one.id}`)).toEqual(["rule:progress", "lifecycle:ticket", "rule:to-review", "lifecycle:qa_sign_off"]);
    expect(drafts.every((one) => one.text.length > 0)).toBe(true);
  });

  it("are none for a repository that has none, and none where the read is refused", async () => {
    expect(await readKeepDrafts(fleet(), API.id)).toEqual([]);
    const refused = { listKeepDrafts: async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) } as unknown as PiecesApi;
    expect(await readKeepDrafts(refused, WEB.id)).toEqual([]);
  });
});

describe("the clock on a piece that waits", () => {
  it("is a keep for each draft, beside the removal of each piece a pull request has open", async () => {
    const drafts = await readKeepDrafts(fleet(), WEB.id);
    const marks = pendingMarks(drafts, ["rule:close-on-merge", "lifecycle:deploy"]);
    expect(Object.fromEntries(marks)).toEqual({
      "rule:progress": "keep",
      "lifecycle:ticket": "keep",
      "rule:to-review": "keep",
      "lifecycle:qa_sign_off": "keep",
      "rule:close-on-merge": "removal",
      "lifecycle:deploy": "removal",
    });
  });

  it("is the removal where one piece has both, since the piece is going", async () => {
    const drafts = await readKeepDrafts(fleet(), WEB.id);
    expect(pendingMarks(drafts, ["rule:progress"]).get("rule:progress")).toBe("removal");
  });

  it("is none where nothing waits", () => {
    expect(pendingMarks([], []).size).toBe(0);
  });
});

describe("reading the drafts again", () => {
  /** Fleet's `keep_drafts.changed`, told by hand to every one that listens. */
  const told = () => {
    const listeners = new Set<(manifestId: string) => void>();
    const api = { onKeepDraftsChanged: (on: (manifestId: string) => void) => (listeners.add(on), () => void listeners.delete(on)) } as unknown as PiecesApi;
    return { api, say: (manifestId: string) => listeners.forEach((on) => on(manifestId)), listening: () => listeners.size };
  };

  it("happens when Fleet says the drafts of this repository changed, and for no other repository's", () => {
    const { api, say } = told();
    const again = vi.fn();
    followKeepDrafts(api, WEB.id, again);
    say(API.id);
    expect(again).not.toHaveBeenCalled();
    say(WEB.id);
    say(WEB.id);
    expect(again).toHaveBeenCalledTimes(2);
  });

  it("stops when the page leaves", () => {
    const { api, say, listening } = told();
    const again = vi.fn();
    const stop = followKeepDrafts(api, WEB.id, again);
    expect(listening()).toBe(1);
    stop();
    say(WEB.id);
    expect(listening()).toBe(0);
    expect(again).not.toHaveBeenCalled();
  });
});

describe("a lifecycle only a draft holds", () => {
  const read = async () => {
    const mock = fleet();
    const answer = await readPieces(mock, WEB.id, SERVED);
    if (!answer.ok) throw new Error("not read");
    return { read: answer.read, drafts: await readKeepDrafts(mock, WEB.id) };
  };

  it("is drawn with no file that keeps it, so it has no name to change and nothing to delete", async () => {
    const { read: held, drafts } = await read();
    const workspace = workspaceDrawn(held, drafts, WEB.id)!;
    const drawn = workspace.lifecycles.find((one) => one.id === "qa_sign_off");
    expect(drawn?.label).toBe("QA sign-off");
    expect(drawn?.kept).toBeUndefined();
  });

  it("leaves a lifecycle the repository holds as it was, whatever draft waits on it", async () => {
    const { read: held, drafts } = await read();
    const drawn = workspaceDrawn(held, drafts, WEB.id)!.lifecycles.find((one) => one.id === "ticket");
    const without = workspaceDrawn(held, [], WEB.id)!.lifecycles.find((one) => one.id === "ticket");
    expect(drawn?.kept).toBeDefined();
    expect(drawn).toEqual(without);
  });

  it("is not drawn once its draft is approved or dismissed and the drafts are read again", async () => {
    const { read: held } = await read();
    expect(workspaceDrawn(held, [], WEB.id)!.lifecycles.some((one) => one.id === "qa_sign_off")).toBe(false);
  });
});

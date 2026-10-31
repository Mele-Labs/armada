// A repository in two Workspaces: the Workflows page reads both, draws a tab for each, writes an edit on a tab under that Workspace's own scope, and reads a rule both change as neither's. The mock
// Fleet's slice answers the reads and takes the saves, as it sets aside a rule two Workspaces change alike (`mobilePieces`: api is in Platform and Mobile, web in Platform, ios in Mobile).

import { describe, expect, it, vi } from "vitest";
import { DEFAULT, clashAt, levelName, levelsOf, ownersOf, ruleAt, withRuleWireAt, withStateLabelAt, workspaceLevel } from "@armada/components";
import type { Workspace } from "@armada/components";
import type { LifecycleRule } from "@armada/protocol";
import { lifecycles } from "../mock/slices/lifecycles";
import { mobilePieces } from "../mock/pieces-seed";
import type { PiecesSeed } from "../mock/pieces-seed";
import type { Scenario } from "../mock/moment";
import { readPieces, runOps } from "./fleet-pieces";
import type { PiecesApi, Served } from "./fleet-pieces";
import { levelOf, scopeOf } from "./levels";
import { opsFrom } from "./write";
import { withDrawn, withPlaces, workspaceOf } from "./read";

const API = { id: "01API", root: "/Users/user/api" };
const WEB = { id: "01WEB", root: "/Users/user/web" };
const IOS = { id: "01IOS", root: "/Users/user/ios" };
const SERVED: Served[] = [
  { manifestId: API.id, name: "api", root: API.root },
  { manifestId: WEB.id, name: "web", root: WEB.root },
  { manifestId: IOS.id, name: "ios", root: IOS.root },
];

const PLATFORM = workspaceLevel("platform");
const MOBILE = workspaceLevel("mobile");

/** The mock Fleet over `mobilePieces`, or over `seed` where a test takes some of them away. */
const fleet = (seed: PiecesSeed = mobilePieces(API, WEB, IOS)) => lifecycles.api({ draft: { pieces: seed } } as Scenario, {} as never) as unknown as PiecesApi;

/** The pieces `mobilePieces` keeps, less the rules kept at the files named. */
const without = (...files: string[]): PiecesSeed => {
  const seed = mobilePieces(API, WEB, IOS);
  return { ...seed, rules: seed.rules.filter((one) => !files.some((file) => one.kept.path?.endsWith(`/${file}`))) };
};

async function opened(api: PiecesApi = fleet(), on: string = API.id) {
  const answer = await readPieces(api, on, SERVED);
  if (!answer.ok) throw new Error("not read");
  const workspace = withPlaces(workspaceOf(answer.read)!, ["feature"]);
  return { api, read: answer.read, workspace };
}

const rule = (workspace: Workspace, id: string) => workspace.rules.find((one) => one.id === id)!;

/** A rule's piece changed on a tab, the way the editor's form changes it. */
const edited = (workspace: Workspace, id: string, tab: string, change: (piece: LifecycleRule) => LifecycleRule): Workspace =>
  withDrawn(withRuleWireAt(workspace, id, tab, change(ruleAt(rule(workspace, id), tab, workspace).wire!)));

describe("what is read for a repository in two Workspaces", () => {
  it("reads both Workspaces and the union of their repositories, the one the page is on first and none twice", async () => {
    const { read } = await opened();
    expect(read.workspaces.map((one) => one.id)).toEqual(["platform", "mobile"]);
    expect(read.repositories.map((one) => one.id)).toEqual([API.id, WEB.id, IOS.id]);
  });

  it("names the Workspaces each repository is in, so a tab per Workspace stands over the repositories it holds", async () => {
    const { workspace } = await opened();
    expect(workspace.workspaces).toEqual([
      { id: "platform", name: "Platform" },
      { id: "mobile", name: "Mobile" },
    ]);
    expect(workspace.repositories).toEqual([
      { id: API.id, name: "api", in: ["platform", "mobile"] },
      { id: WEB.id, name: "web", in: ["platform"] },
      { id: IOS.id, name: "ios", in: ["mobile"] },
    ]);
  });

  it("reads the repository first from either of its Workspaces, and the other's repository follows it", async () => {
    const { read } = await opened(fleet(), IOS.id);
    expect(read.workspaces.map((one) => one.id)).toEqual(["mobile"]);
    expect(read.repositories.map((one) => one.id)).toEqual([IOS.id, API.id]);
  });

  it("leaves out a repository of a Workspace that this window does not serve", async () => {
    const answer = await readPieces(fleet(), API.id, SERVED.slice(0, 2));
    if (!answer.ok) throw new Error("not read");
    expect(answer.read.workspaces.map((one) => one.id)).toEqual(["platform", "mobile"]);
    expect(answer.read.repositories.map((one) => one.id)).toEqual([API.id, WEB.id]);
  });

  it("has no Workspace level for a repository in none, whose tabs are the default's and its own", async () => {
    const alone = { id: "01ALONE", root: "/Users/user/alone" };
    const answer = await readPieces(fleet(), alone.id, [...SERVED, { manifestId: alone.id, name: "alone", root: alone.root }]);
    if (!answer.ok) throw new Error("not read");
    expect(answer.read.workspaces).toEqual([]);
    expect(answer.read.repositories.map((one) => one.id)).toEqual([alone.id]);
    const workspace = workspaceOf(answer.read)!;
    expect(workspace.workspaces).toEqual([]);
    expect(levelsOf(alone.id, workspace)).toEqual([alone.id, DEFAULT]);
  });
});

describe("the scope an edit on a tab is saved under", () => {
  it("is the Workspace's own for a tab, and the repository's for a repository", () => {
    const read = { workspaces: [{ id: "platform" }, { id: "mobile" }] } as never;
    expect(scopeOf(PLATFORM)).toEqual({ kind: "within", id: "platform" });
    expect(scopeOf(MOBILE)).toEqual({ kind: "within", id: "mobile" });
    expect(scopeOf(API.id)).toEqual({ kind: "within", id: API.id });
    expect(scopeOf(DEFAULT)).toEqual({ kind: "everywhere" });
    expect([levelOf({ kind: "within", id: "platform" }, read), levelOf({ kind: "within", id: API.id }, read), levelOf({ kind: "everywhere" }, read)]).toEqual([PLATFORM, API.id, DEFAULT]);
  });

  it("is Platform's for a rule changed on the Platform tab, and Mobile's on the Mobile tab, each over its own piece", async () => {
    const { read, workspace } = await opened();
    const onPlatform = opsFrom(read, workspace, edited(workspace, "ready", PLATFORM, (piece) => ({ ...piece, name: "Mark it ready" })));
    expect(onPlatform).toHaveLength(1);
    expect(onPlatform[0]).toMatchObject({ op: "save", overwrite: true, piece: { rule: { id: "ready", name: "Mark it ready", scope: { kind: "within", id: "platform" } } } });
    const onMobile = opsFrom(read, workspace, edited(workspace, "ready", MOBILE, (piece) => ({ ...piece, name: "Ready it" })));
    expect(onMobile).toHaveLength(1);
    expect(onMobile[0]).toMatchObject({ op: "save", overwrite: true, piece: { rule: { id: "ready", name: "Ready it", scope: { kind: "within", id: "mobile" } } } });
  });

  it("is a new piece of the Workspace's where it keeps none of that rule yet, and no other Workspace gets one", async () => {
    const { read, workspace } = await opened();
    const onMobile = opsFrom(read, workspace, edited(workspace, "progress", MOBILE, (piece) => ({ ...piece, name: "Begin the ticket" })));
    expect(onMobile).toHaveLength(1);
    expect(onMobile[0]).toMatchObject({ op: "save", overwrite: false, piece: { rule: { id: "progress", scope: { kind: "within", id: "mobile" } } } });
  });

  it("is the Workspace's for a state named on its tab, and for no repository under it", async () => {
    const { read, workspace } = await opened();
    for (const [tab, id] of [[PLATFORM, "platform"], [MOBILE, "mobile"]] as const) {
      const ops = opsFrom(read, workspace, withStateLabelAt(workspace, "ticket", "todo", tab, "Backlog"));
      expect(ops).toHaveLength(1);
      expect(ops[0]).toMatchObject({ op: "save", overwrite: false, piece: { kind: "lifecycle", lifecycle: { id: "ticket", scope: { kind: "within", id } } } });
    }
  });

  it("is kept by Fleet at the Workspace it names, which a read of the tab then gives back", async () => {
    const { api, read, workspace } = await opened();
    const saved = vi.spyOn(api, "saveLifecyclePiece");
    const ran = await runOps(api, opsFrom(read, workspace, edited(workspace, "progress", PLATFORM, (piece) => ({ ...piece, name: "Begin the ticket" }))), SERVED);
    expect(ran.ok).toBe(true);
    expect(saved).toHaveBeenCalledTimes(1);
    expect(saved.mock.calls[0]![0]).toMatchObject({ piece: { rule: { scope: { kind: "within", id: "platform" } } } });
    const again = (await opened(api)).workspace;
    expect(ruleAt(rule(again, "progress"), PLATFORM, again).label).toBe("Begin the ticket");
    expect(ruleAt(rule(again, "progress"), MOBILE, again).label).toBe("Start the ticket");
  });
});

describe("what a repository tab runs of a rule the Workspaces change", () => {
  it("is the one Workspace's where only it changes the rule, for api and for the repository only it holds", async () => {
    const { workspace } = await opened(fleet(without("ready-mobile.toml")));
    const ready = rule(workspace, "ready");
    expect(ready.clash).toBeUndefined();
    expect(ruleAt(ready, API.id, workspace).autonomy).toBe("asks");
    expect(ruleAt(ready, WEB.id, workspace).autonomy).toBe("asks");
    // Mobile does not change it, so ios reads the default's.
    expect(ruleAt(ready, IOS.id, workspace).autonomy).toBe("off");
    expect(clashAt(ready.clash, API.id, workspace, ownersOf(ready))).toEqual([]);
  });

  it("is Mobile's where only Mobile changes it", async () => {
    const { workspace } = await opened(fleet(without("ready-platform.toml")));
    const ready = rule(workspace, "ready");
    expect(ruleAt(ready, API.id, workspace).autonomy).toBe("acts");
    expect(ruleAt(ready, IOS.id, workspace).autonomy).toBe("acts");
    expect(ruleAt(ready, WEB.id, workspace).autonomy).toBe("off");
  });

  it("is the default's for api where both change it, with both named as set aside, while web and ios keep their one Workspace's", async () => {
    const { workspace } = await opened();
    const ready = rule(workspace, "ready");
    expect(ready.clash).toEqual([[PLATFORM, MOBILE]]);
    expect(ruleAt(ready, API.id, workspace).autonomy).toBe("off");
    expect(clashAt(ready.clash, API.id, workspace, ownersOf(ready))).toEqual([PLATFORM, MOBILE]);
    expect(clashAt(ready.clash, API.id, workspace, ownersOf(ready)).map((one) => levelName(workspace, one))).toEqual(["Platform", "Mobile"]);
    expect(ruleAt(ready, WEB.id, workspace).autonomy).toBe("asks");
    expect(ruleAt(ready, IOS.id, workspace).autonomy).toBe("acts");
    expect(clashAt(ready.clash, WEB.id, workspace, ownersOf(ready))).toEqual([]);
    expect(clashAt(ready.clash, IOS.id, workspace, ownersOf(ready))).toEqual([]);
  });

  it("is a clash on no Workspace's own tab, which reads its own piece", async () => {
    const { workspace } = await opened();
    const ready = rule(workspace, "ready");
    expect(ruleAt(ready, PLATFORM, workspace).autonomy).toBe("asks");
    expect(ruleAt(ready, MOBILE, workspace).autonomy).toBe("acts");
    expect(clashAt(ready.clash, PLATFORM, workspace, ownersOf(ready))).toEqual([]);
  });

  it("is no clash once api keeps a piece of its own, which it runs in place of either Workspace's", async () => {
    const { read, workspace } = await opened();
    const ready = rule(workspace, "ready");
    const mobile = ruleAt(ready, MOBILE, workspace).wire!;
    const [save] = opsFrom(read, workspace, withDrawn(withRuleWireAt(workspace, "ready", API.id, mobile)));
    expect(save).toMatchObject({ op: "save", piece: { rule: { id: "ready", autonomy: "acts", scope: { kind: "within", id: API.id } } } });
    const api = fleet();
    await runOps(api, [save!], SERVED);
    const again = (await opened(api)).workspace;
    const after = rule(again, "ready");
    expect(ruleAt(after, API.id, again).autonomy).toBe("acts");
    expect(clashAt(after.clash, API.id, again, ownersOf(after))).toEqual([]);
  });
});

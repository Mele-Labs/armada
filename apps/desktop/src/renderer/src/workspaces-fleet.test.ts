// System > Workspaces over what Fleet keeps (armada-cwj): the id a new Workspace is saved under, the Workspaces read across the repositories one window serves, the layers laid into the rows the
// page draws, and the saves and deletes a press comes to. The mock Fleet's slice answers the reads and takes the saves, so the halves are tried against each other.

import { describe, expect, it } from "vitest";
import { autonomyRows } from "@armada/components";
import type { AutonomyLayer as WireLayer, RepositorySummary, WorkspaceOfRepositories } from "@armada/protocol";

import type { Scenario } from "./mock/moment";
import { lifecycles } from "./mock/slices/lifecycles";
import { autonomyActs, filesOf, layersOf, membersAfter, perform, readAutonomy, readWorkspaces, removalActs, rowOf, servedOf, uniqueWorkspaces, workspaceAct, workspaceIdFrom } from "./workspaces-fleet";
import type { LayerFile, WorkspacesApi } from "./workspaces-fleet";

const repository = (id: string, name: string): RepositorySummary => ({
  root: `/Users/user/${name}`,
  records_root: `/records/${name}`,
  manifest: { id, repository: name, path: `${name}/armada.yml` } as RepositorySummary["manifest"],
});
const REPOSITORIES = [repository("01WEB", "store-web"), repository("01API", "store-api"), repository("01DOCS", "store-docs"), repository("01NONE", "scratch")];
const SERVED = servedOf(REPOSITORIES);

/** The mock Fleet over two Workspaces: my-store, kept in Kit, and platform, committed in store-docs. store-api is in both. */
function fleet(): WorkspacesApi {
  const scenario = {
    state: { holds: { repositories: REPOSITORIES } },
    draft: {
      workspace: {
        workspace: {},
        proposals: [],
        workspaces: {
          served: [],
          records: [
            { id: "my-store", name: "my-store", repositories: ["store-web", "store-api"], kept: "mac" },
            { id: "platform", name: "platform", repositories: ["store-docs", "store-api"], kept: "repository", home: "store-docs" },
          ],
          layers: { everywhere: { tell_agent: "off" }, "workspace:my-store": { run_command: "asks" }, "workspace:platform": { run_command: "off" } },
        },
      },
    },
  } as unknown as Scenario;
  return lifecycles.api(scenario, {} as never) as unknown as WorkspacesApi;
}

const reads = async (api: WorkspacesApi) => {
  const answer = await readAutonomy(api, SERVED);
  if (!answer.ok) throw new Error("not read");
  return answer.files;
};

const file = (level: string, id: string, source: string, defaults: LayerFile["defaults"]): LayerFile => ({ level, id, source, defaults });

describe("the id a new Workspace is saved under", () => {
  it("is its name in lowercase with each run of spaces and marks as one dash", () => {
    expect(workspaceIdFrom("My Store", [], [])).toBe("my-store");
    expect(workspaceIdFrom("  Store  &  Docs! ", [], [])).toBe("store-docs");
  });

  it("is made unique against the ids already kept", () => {
    expect(workspaceIdFrom("my-store", ["my-store"], [])).toBe("my-store-2");
    expect(workspaceIdFrom("my-store", ["my-store", "my-store-2"], [])).toBe("my-store-3");
  });

  it("is never the id of a repository, which Fleet refuses", () => {
    expect(workspaceIdFrom("01WEB", [], ["01WEB"])).toBe("01web-2");
  });

  it("is a word of its own where the name has no letter or digit in it", () => {
    expect(workspaceIdFrom("!!!", [], [])).toBe("workspace");
  });
});

describe("the Workspaces read across the repositories", () => {
  const one = (id: string, repositories: string[], more: Partial<WorkspaceOfRepositories> = {}): WorkspaceOfRepositories => ({ id, name: id, repositories, source: "kit", path: `/kit/${id}.toml`, ...more });

  it("keeps each id once, and the one that runs where Fleet answered a replaced one too", () => {
    const replaced = one("platform", ["01DOCS"], { source: "repository", overridden_by: "kit" });
    const running = one("platform", ["01DOCS", "01API"], { hides: "repository" });
    expect(uniqueWorkspaces([[replaced], [running], [one("my-store", ["01WEB"])]]).map((each) => [each.id, each.overridden_by])).toEqual([
      ["platform", undefined],
      ["my-store", undefined],
    ]);
    expect(uniqueWorkspaces([[running], [replaced]])[0]).toBe(running);
  });

  it("is each Workspace once, though store-api is asked about and is in both", async () => {
    const answer = await readWorkspaces(fleet(), SERVED);
    expect(answer.ok && answer.workspaces.map((each) => each.id)).toEqual(["my-store", "platform"]);
  });

  it("is the refusal where Fleet refuses one of the reads", async () => {
    const error = { code: "fleet.piece_unfit", message: "no", run_id: "r", fields: {}, chain: [] };
    const api = { ...fleet(), listRepositoryWorkspaces: async () => ({ ok: false as const, outcome: { ok: false as const, why: "refused" as const, error } }) };
    const answer = await readWorkspaces(api, SERVED);
    expect(answer.ok).toBe(false);
  });

  it("lists in a row only the repositories the window serves, and keeps the rest to be saved back", () => {
    const held = one("platform", ["01DOCS", "01ELSEWHERE"], { home: "01DOCS" });
    expect(rowOf(held, SERVED)).toEqual({ id: "platform", name: "platform", repositories: ["01DOCS"], home: "01DOCS" });
    expect(membersAfter(held.repositories, ["01API"], SERVED)).toEqual(["01API", "01ELSEWHERE"]);
  });
});

describe("the layers laid into the rows the page draws", () => {
  const layer = (id: string, scope: WireLayer["scope"], reach: string, defaults: WireLayer["defaults"], more: Partial<WireLayer["kept"]> = {}): WireLayer => ({ id, scope, kept: { rank: { reach, source: "kit" }, ...more }, defaults });
  const everywhere = layer("defaults", { kind: "everywhere" }, "everywhere", [{ kind: "tell_agent", autonomy: "off" }]);
  const mine = layer("defaults", { kind: "within", id: "my-store" }, "workspace", [{ kind: "run_command", autonomy: "asks" }]);
  const platform = layer("defaults", { kind: "within", id: "platform" }, "workspace", [{ kind: "run_command", autonomy: "off" }]);

  it("keys each by its level, widest first, and each file of one level once however many repositories answered it", () => {
    const files = filesOf([[everywhere, mine], [everywhere, platform, mine]]);
    expect(files.map((each) => each.level)).toEqual(["everywhere", "workspace:my-store", "workspace:platform"]);
  });

  it("leaves out a file another of its id runs instead of", () => {
    const replaced = layer("defaults", { kind: "within", id: "my-store" }, "workspace", [{ kind: "run_command", autonomy: "off" }], { overridden_by: { reach: "workspace", source: "kit" } });
    expect(filesOf([[replaced]])).toEqual([]);
  });

  it("draws a kind a level set as set by that level, and the rest as Armada's", () => {
    const layers = layersOf(filesOf([[everywhere, mine]]));
    const rows = autonomyRows(layers, [[{ key: "workspace:my-store", name: "my-store" }], [{ key: "everywhere", name: "Everywhere" }]]);
    const row = (kind: string) => rows.find((each) => each.kind === kind)!;
    expect(row("run_command")).toMatchObject({ value: "asks", setBy: "my-store", own: true });
    expect(row("tell_agent")).toMatchObject({ value: "off", setBy: "Everywhere", own: false });
    expect(row("draft")).toMatchObject({ value: "asks", setBy: "Armada" });
  });

  it("draws the stricter value and a clash for a repository in two Workspaces that set one kind differently", () => {
    const layers = layersOf(filesOf([[mine, platform]]));
    const rows = autonomyRows(layers, [
      [{ key: "repository:01API", name: "store-api" }],
      [
        { key: "workspace:my-store", name: "my-store" },
        { key: "workspace:platform", name: "platform" },
      ],
      [{ key: "everywhere", name: "Everywhere" }],
    ]);
    expect(rows.find((each) => each.kind === "run_command")).toMatchObject({ value: "off", setBy: "platform", clash: "my-store and platform differ" });
  });

  it("lays the files of one level over each other, the last to name a kind setting it", () => {
    const layers = layersOf([file("everywhere", "a", "kit", { tell_agent: "off", draft: "acts" }), file("everywhere", "b", "kit", { tell_agent: "asks" })]);
    expect(layers["everywhere"]).toEqual({ tell_agent: "asks", draft: "acts" });
  });

  it("leaves out a kind or a value Bridge does not draw", () => {
    const odd = layer("defaults", { kind: "everywhere" }, "everywhere", [
      { kind: "tell_agent", autonomy: "sometimes" },
      { kind: "new_kind", autonomy: "acts" },
      { kind: "draft", autonomy: "acts" },
    ]);
    expect(filesOf([[odd]])[0]!.defaults).toEqual({ draft: "acts" });
  });
});

describe("what a press on the autonomy table comes to", () => {
  it("is a save over the file that runs at the level, with every kind it already set", () => {
    const acts = autonomyActs([file("workspace:my-store", "strict", "kit", { run_command: "asks" })], "workspace:my-store", "tracker_write", "off");
    expect(acts).toEqual([{ act: "save", overwrite: true, piece: { kind: "autonomy", autonomy: { id: "strict", scope: { kind: "within", id: "my-store" }, defaults: { run_command: "asks", tracker_write: "off" } } } }]);
  });

  it("is a new file called defaults, with no overwrite, where the level has none", () => {
    expect(autonomyActs([], "everywhere", "draft", "acts")).toEqual([{ act: "save", overwrite: false, piece: { kind: "autonomy", autonomy: { id: "defaults", defaults: { draft: "acts" } } } }]);
  });

  it("is a Kit copy that carries what a repository's file set, where the level's file is committed", () => {
    const [save] = autonomyActs([file("repository:01API", "team", "repository", { draft: "off" })], "repository:01API", "run_agent", "asks");
    expect(save).toMatchObject({ overwrite: false, piece: { autonomy: { id: "team", defaults: { draft: "off", run_agent: "asks" } } } });
  });

  it("is a save without the kind on a reset, where the file sets others", () => {
    const acts = autonomyActs([file("everywhere", "defaults", "kit", { tell_agent: "off", draft: "acts" })], "everywhere", "tell_agent", undefined);
    expect(acts).toEqual([{ act: "save", overwrite: true, piece: { kind: "autonomy", autonomy: { id: "defaults", defaults: { draft: "acts" } } } }]);
  });

  it("is a delete of the file on a reset of its last kind", () => {
    const acts = autonomyActs([file("workspace:my-store", "defaults", "kit", { run_command: "asks" })], "workspace:my-store", "run_command", undefined);
    expect(acts).toEqual([{ act: "delete", kind: "autonomy", id: "defaults", scope: { kind: "within", id: "my-store" } }]);
  });

  it("is nothing on a reset of a kind the level never set", () => {
    expect(autonomyActs([file("everywhere", "defaults", "kit", { draft: "acts" })], "everywhere", "tell_agent", undefined)).toEqual([]);
  });

  it("removes a Workspace and the layers Kit holds for it", () => {
    const files = [file("workspace:my-store", "defaults", "kit", { run_command: "asks" }), file("workspace:platform", "defaults", "kit", { run_command: "off" })];
    expect(removalActs(files, "my-store")).toEqual([
      { act: "delete", kind: "workspace", id: "my-store" },
      { act: "delete", kind: "autonomy", id: "defaults", scope: { kind: "within", id: "my-store" } },
    ]);
  });
});

describe("the acts sent to the mock Fleet and read back", () => {
  it("sets a kind at a level, which the next read lays at that level, and a reset takes the layer away", async () => {
    const api = fleet();
    expect(await perform(api, autonomyActs(await reads(api), "workspace:my-store", "tracker_write", "off"))).toEqual({ ok: true });
    let layers = layersOf(await reads(api));
    expect(layers["workspace:my-store"]).toEqual({ run_command: "asks", tracker_write: "off" });
    expect(await perform(api, autonomyActs(await reads(api), "workspace:my-store", "tracker_write", undefined))).toEqual({ ok: true });
    expect(await perform(api, autonomyActs(await reads(api), "workspace:my-store", "run_command", undefined))).toEqual({ ok: true });
    layers = layersOf(await reads(api));
    expect(layers["workspace:my-store"]).toBeUndefined();
  });

  it("creates a Workspace, which every repository in it then reads, and saves it again over itself", async () => {
    const api = fleet();
    expect(await perform(api, [workspaceAct({ id: "mobile", name: "mobile", repositories: ["01NONE", "01WEB"] }, false)])).toEqual({ ok: true });
    const first = await readWorkspaces(api, SERVED);
    expect(first.ok && first.workspaces.find((each) => each.id === "mobile")?.repositories).toEqual(["01NONE", "01WEB"]);
    expect(await perform(api, [workspaceAct({ id: "mobile", name: "mobile apps", repositories: ["01NONE"] }, true)])).toEqual({ ok: true });
    const second = await readWorkspaces(api, SERVED);
    expect(second.ok && second.workspaces.find((each) => each.id === "mobile")).toMatchObject({ name: "mobile apps", repositories: ["01NONE"] });
  });

  it("says Fleet's words for a Workspace named like a repository it holds, and writes nothing", async () => {
    const api = fleet();
    const done = await perform(api, [workspaceAct({ id: "01WEB", name: "web", repositories: ["01WEB"] }, false)]);
    expect(done).toEqual({ ok: false, said: expect.stringContaining("a scope could not say which it meant") });
    const read = await readWorkspaces(api, SERVED);
    expect(read.ok && read.workspaces.map((each) => each.id)).toEqual(["my-store", "platform"]);
  });

  it("stops at the first act Fleet refuses", async () => {
    const api = fleet();
    const done = await perform(api, [{ act: "delete", kind: "workspace", id: "platform" }, workspaceAct({ id: "late", name: "late", repositories: ["01WEB"] }, false)]);
    expect(done.ok).toBe(false);
    const read = await readWorkspaces(api, SERVED);
    expect(read.ok && read.workspaces.map((each) => each.id)).not.toContain("late");
  });

  it("removes a Workspace Kit holds, and not one a repository committed", async () => {
    const api = fleet();
    expect(await perform(api, removalActs(await reads(api), "my-store"))).toEqual({ ok: true });
    const read = await readWorkspaces(api, SERVED);
    expect(read.ok && read.workspaces.map((each) => each.id)).toEqual(["platform"]);
    expect(layersOf(await reads(api))["workspace:my-store"]).toBeUndefined();
  });
});

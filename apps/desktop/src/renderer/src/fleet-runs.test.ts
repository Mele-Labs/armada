// The Fleet-backed source of runs, against a fake Fleet: a repository's runs mapped onto what the run control and the running panel
// draw, `app_run.changed` replacing one run whole, the acts reaching Fleet's routes by name, and a process's output read only while
// its pane is open. What the surfaces do with them is the walks' (`mock/walks-4of4.test.tsx`, `run-an-app` and `run-link`).

import { describe, expect, it } from "vitest";

import type { AppRun, AppRunDraft, Outcome, ServerMessage } from "@armada/protocol";

import type { AppRunsOf } from "../../shared/api/app-runs";
import { createFleetRuns, draftOf, type FleetRuns } from "./fleet-runs";

const ROOT = "/Users/nick/code/shop";
const settled = () => new Promise((done) => setTimeout(done, 0));

const run = (over: Partial<AppRun> = {}): AppRun => ({
  name: "web",
  label: "Web",
  opens_in: "browser",
  manifest_id: "m-shop",
  checkout: { kind: "main", path: ROOT },
  processes: [
    { name: "api", state: "not_started" },
    { name: "web", state: "not_started" },
  ],
  ...over,
});

const UP: AppRun["processes"] = [
  { name: "api", state: "ready", port: 41210, pinned_port: 8731, server_id: "srv-api-1", links: [{ url: "http://localhost:8731" }] },
  { name: "web", state: "starting", port: 41211, server_id: "srv-web-1" },
];

/** A Fleet serving one repository, and a record of what Bridge asked of it, in order. */
function fake(listed: AppRun[] = [run()], connected = true) {
  const asked: string[] = [];
  const told: Outcome[] = [];
  let hear: (run: AppRun) => void = () => undefined;
  let print: (serverId: string, message: ServerMessage) => void = () => undefined;
  let jobRuns: AppRun[] = [];
  const answer = (value: AppRun) => ({ ok: true as const, value });
  const fleet: FleetRuns = {
    state: async () =>
      ({
        connection: { state: connected ? "connected" : "connecting" },
        readAt: 1,
        holds: { repositories: [{ root: ROOT, records_root: "", manifest: { id: "m-shop" } }] },
      }) as never,
    subscribe: () => () => undefined,
    listAppRuns: async (of: AppRunsOf) => {
      asked.push("manifestId" in of ? `list ${of.manifestId}` : `list job ${of.jobId}`);
      return { ok: true, value: { runs: "manifestId" in of ? listed : jobRuns } };
    },
    getAppRunDraft: async (manifestId) => {
      asked.push(`draft ${manifestId}`);
      const draft: AppRunDraft = {
        root: ROOT,
        place: "repository",
        processes: [
          { name: "api", dir: "api", command: "uvicorn app:app --port ${port}", port_env: "PORT" },
          { name: "web", command: "pnpm exec vite --port ${port}" },
        ],
        links: [{ from: "web", to: "api", port: 8731, found_in: "vite.config.ts", brief: "Make web read the api's port" }],
      };
      return { ok: true, value: draft };
    },
    actOnAppRun: async (act, manifestId, named) => {
      asked.push(`${act} ${manifestId} ${JSON.stringify(named)}`);
      if (named.name === "refused") return { ok: false, outcome: { ok: false, why: "not_connected" } };
      return answer(run({ name: named.name, processes: UP, ...(named.job_id === undefined ? {} : { checkout: { kind: "job", path: "/slots/3", job_id: named.job_id, branch: "fix/cart", bay: 3 } }) }));
    },
    saveAppRun: async (manifestId, save) => {
      asked.push(`save ${manifestId} ${JSON.stringify(save)}`);
      return { ok: true, value: { place: "repository", file: "armada.yml", run: run({ name: "storybook", label: save.name, processes: UP }) } };
    },
    pinAppRunPort: async (manifestId, pin) => (asked.push(`pin ${manifestId} ${JSON.stringify(pin)}`), answer(run({ processes: UP }))),
    unpinAppRunPort: async (manifestId, unpin) => (asked.push(`unpin ${manifestId} ${JSON.stringify(unpin)}`), answer(run({ processes: UP }))),
    onAppRunChanged: (on) => ((hear = on), () => undefined),
    followServer: async (serverId) => (asked.push(`follow ${serverId}`), { ok: true }),
    unfollowServer: (serverId) => void asked.push(`unfollow ${serverId}`),
    onServerOutput: (on) => ((print = on), () => undefined),
  };
  const source = createFleetRuns(fleet, (outcome) => told.push(outcome));
  source.subscribe(() => undefined);
  return {
    source,
    asked,
    told,
    push: (one: AppRun) => hear(one),
    print: (serverId: string, message: ServerMessage) => print(serverId, message),
    jobHas: (runs: AppRun[]) => (jobRuns = runs),
  };
}

describe("a repository's runs, as Fleet lists them", () => {
  it("are the picker's saved runs once Fleet is connected, the one used last named, and nothing is running", async () => {
    const { source, asked } = fake([run(), run({ name: "storybook", label: "Storybook", opens_in: "app", last_used: "2026-10-26T09:00:00Z" })]);
    await settled();
    expect(asked).toEqual(["list m-shop"]);
    const saved = source.get().saved[ROOT];
    expect(saved?.last).toBe("storybook");
    expect(saved?.runs.map((one) => [one.id, one.name, one.kind])).toEqual([
      ["web", "Web", "browser"],
      ["storybook", "Storybook", "app"],
    ]);
    expect(source.get().apps).toEqual([]);
  });

  it("are read for nobody before Fleet is connected", async () => {
    const { source, asked } = fake([run()], false);
    await settled();
    expect(asked).toEqual([]);
    expect(source.get().saved).toEqual({});
  });

  it("are an empty list where the repository saves none, so Run asks first", async () => {
    const { source } = fake([]);
    await settled();
    expect(source.get().saved[ROOT]).toEqual({ runs: [] });
  });
});

describe("a run that was started", () => {
  it("is a running app with each process's phase, port, pin and address, and the links Fleet found", async () => {
    const { source, push } = fake();
    await settled();
    push(run({ processes: UP, links: [{ from: "web", to: "api", port: 8731, found_in: "log", brief: "Make web read the api's port" }] }));
    const app = source.get().apps[0];
    expect(app).toEqual({
      id: "m-shop:main:web",
      run: "Web",
      kind: "browser",
      repository: ROOT,
      checkout: { kind: "main", branch: "main" },
      processes: [
        { id: "api", name: "api", phase: "ready", port: 41210, pinned: true, url: "http://localhost:8731", log: [] },
        { id: "web", name: "web", phase: "starting", port: 41211, log: [] },
      ],
      links: [{ from: "web", to: "api", port: 8731, found: "log" }],
    });
  });

  it("is replaced whole by `app_run.changed`, and a process that ended on its own is failed", async () => {
    const { source, push } = fake();
    await settled();
    push(run({ processes: UP }));
    push(run({ processes: [{ name: "api", state: "failed", ended: "exited with 1" }, { name: "web", state: "exited" }] }));
    expect(source.get().apps).toHaveLength(1);
    expect(source.get().apps[0]?.processes.map((one) => one.phase)).toEqual(["failed", "exited"]);
  });

  it("on a Job's bay is named by the Job the Cockpit drew, on the bay Fleet says", async () => {
    const { source, asked, jobHas } = fake();
    await settled();
    jobHas([run({ processes: UP, checkout: { kind: "job", path: "/slots/3", job_id: "job-7", branch: "fix/cart", bay: 3 } })]);
    source.seen({ repository: ROOT, job: "job-7", checkout: { kind: "job", title: "Fix the cart", branch: "fix/cart", bay: 3 } });
    await settled();
    expect(asked).toContain("list job job-7");
    expect(source.get().apps[0]).toMatchObject({ id: "m-shop:job-7:web", job: "job-7", checkout: { kind: "job", title: "Fix the cart", branch: "fix/cart", bay: 3 } });
    // A Job's own run is not one of main's saved runs.
    expect(source.get().saved[ROOT]?.runs.map((one) => one.id)).toEqual(["web"]);
  });
});

describe("the acts, as Fleet takes them", () => {
  it("start a saved run by name on its checkout, and answer the app's id", async () => {
    const { source, asked } = fake();
    await settled();
    const id = await source.start({ repository: ROOT, job: "job-7", checkout: { kind: "job", title: "Fix the cart", branch: "fix/cart", bay: 3 } }, source.get().saved[ROOT]!.runs[0]!);
    expect(asked).toContain(`start m-shop {"name":"web","job_id":"job-7"}`);
    expect(id).toBe("m-shop:job-7:web");
    expect(source.get().apps[0]?.checkout).toEqual({ kind: "job", title: "Fix the cart", branch: "fix/cart", bay: 3 });
  });

  it("tell Fleet's refusal and open nothing", async () => {
    const { source, told } = fake();
    await settled();
    const id = await source.start({ repository: ROOT, checkout: { kind: "main", branch: "main" } }, { id: "refused", name: "Refused", kind: "browser", processes: [] });
    expect(id).toBeUndefined();
    expect(told).toEqual([{ ok: false, why: "not_connected" }]);
  });

  it("draft with `~` for the home directory, the file it saves in, and a link by process", async () => {
    const { source } = fake();
    await settled();
    expect(await source.draft(ROOT)).toEqual({
      root: "~/code/shop",
      processes: [
        { id: "api", name: "api", folder: "api", command: "uvicorn app:app --port ${port}" },
        { id: "web", name: "web", folder: ".", command: "pnpm exec vite --port ${port}" },
      ],
      savedIn: "armada.yml",
      links: [{ from: "web", to: "api", port: 8731, found: "vite.config.ts" }],
    });
  });

  it("say each place a run can be saved in, cloud storage among them", () => {
    const saved = (place: string) => draftOf([], ROOT, place).savedIn;
    expect(["repository", "this_mac", "synced", "cloud"].map(saved)).toEqual(["armada.yml", "this Mac", "a synced folder", "cloud storage"]);
  });

  it("save what was confirmed with the draft's port variable handed back, then pin what was pinned in the confirm and start again", async () => {
    const { source, asked } = fake();
    await settled();
    const draft = (await source.draft(ROOT))!;
    const [api, web] = draft.processes;
    const id = await source.save({ repository: ROOT, checkout: { kind: "main", branch: "main" } }, "Storybook", [{ ...api!, pin: 8731 }, web!]);
    await settled();
    expect(id).toBe("m-shop:main:storybook");
    expect(asked.slice(-3)).toEqual([
      `save m-shop {"name":"Storybook","processes":[{"name":"api","command":"uvicorn app:app --port \${port}","dir":"api","port_env":"PORT"},{"name":"web","command":"pnpm exec vite --port \${port}"}]}`,
      `pin m-shop {"process":"api","port":8731,"name":"storybook"}`,
      `restart m-shop {"name":"storybook"}`,
    ]);
  });

  it("pin and unpin a process of a running app by its run, and start it again", async () => {
    const { source, asked, push } = fake();
    await settled();
    push(run({ processes: UP }));
    source.pin("m-shop:main:web", "api", 8731);
    await settled();
    source.pin("m-shop:main:web", "api", undefined);
    await settled();
    expect(asked.slice(-4)).toEqual([`pin m-shop {"process":"api","name":"web","port":8731}`, `restart m-shop {"name":"web"}`, `unpin m-shop {"process":"api","name":"web"}`, `restart m-shop {"name":"web"}`]);
  });

  it("stop and restart a running app by its run", async () => {
    const { source, asked, push } = fake();
    await settled();
    push(run({ processes: UP }));
    source.stop("m-shop:main:web");
    source.restart("m-shop:main:web");
    await settled();
    expect(asked.slice(-2)).toEqual([`stop m-shop {"name":"web"}`, `restart m-shop {"name":"web"}`]);
  });
});

describe("a process's output", () => {
  it("is read from its server while the pane is open, and let go when it closes", async () => {
    const { source, asked, push, print } = fake();
    await settled();
    push(run({ processes: UP }));
    const close = source.reading("m-shop:main:web");
    expect(asked.slice(-2)).toEqual(["follow srv-api-1", "follow srv-web-1"]);
    print("srv-api-1", { message: "opened", protocol_id: "1.0" as never, id: "srv-api-1", name: "api", path: ROOT, live: true, skipped: 0 });
    print("srv-api-1", { message: "lines", lines: ["INFO:     Application startup complete."] });
    print("srv-api-1", { message: "lines", lines: ["INFO:     Uvicorn running on http://127.0.0.1:41210"] });
    expect(source.get().apps[0]?.processes[0]?.log).toEqual(["INFO:     Application startup complete.", "INFO:     Uvicorn running on http://127.0.0.1:41210"]);
    close();
    expect(asked.slice(-2)).toEqual(["unfollow srv-api-1", "unfollow srv-web-1"]);
    expect(source.get().apps[0]?.processes[0]?.log).toEqual([]);
  });

  it("is read from the new server after a restart", async () => {
    const { source, asked, push } = fake();
    await settled();
    push(run({ processes: UP }));
    source.reading("m-shop:main:web");
    push(run({ processes: [{ ...UP[0]!, server_id: "srv-api-2" }, UP[1]!] }));
    expect(asked.slice(-2)).toEqual(["follow srv-api-2", "unfollow srv-api-1"]);
  });

  it("is not read where no pane is open", async () => {
    const { asked, push } = fake();
    await settled();
    push(run({ processes: UP }));
    expect(asked.filter((one) => one.startsWith("follow"))).toEqual([]);
  });
});

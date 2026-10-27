// What the page showing offers the palette, and what choosing one of its rows and the acts on a Job's row run: each is the
// code the Cockpit and the Projects page already run, so a test hands those a spy and reads what was asked of it.

import { SURFACE } from "@armada/shell";
import type { Session } from "@armada/screens/src/draft/sessions";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

const focusAddTask = vi.hoisted(() => vi.fn(() => true));
vi.mock("./projects-tab/keys", () => ({ focusAddTask }));
vi.mock("./cockpit/deck/store", () => ({ setView: vi.fn() }));

import { setView } from "./cockpit/deck/store";
import { pageOf } from "./palette-page";
import { carryOutRow, type RowWorld } from "./palette-rows";

const job = (over: Partial<Session> = {}): Session => ({ id: "j1", title: "Cache key", turn: { state: "idle" }, standing: "idle", rows: [], attachments: [], ...over }) as Session;

describe("what the page offers", () => {
  test("the Cockpit heads its results with its own acts", () => {
    const page = pageOf(SURFACE.overview, undefined);
    expect(page?.title).toBe("Cockpit");
    expect(page?.rows.map((one) => one.id)).toEqual(["dispatch", "map", "timeline", "needing-you"]);
  });

  test("Projects offers a task and a project", () => {
    const page = pageOf(SURFACE.projects, undefined);
    expect(page?.title).toBe("Projects");
    expect(page?.rows.map((one) => one.id)).toEqual(["add-task", "new-project"]);
  });

  test("a page with nothing of its own offers nothing", () => {
    expect(pageOf(SURFACE.manifest, undefined)).toBeUndefined();
    expect(pageOf("", undefined)).toBeUndefined();
  });

  test("a Job open on the deck takes the block, titled with it, and offers Resume where it is paused", () => {
    expect(pageOf(SURFACE.overview, job())?.title).toBe("Cache key");
    expect(pageOf(SURFACE.overview, job())?.rows.map((one) => one.label)).toEqual(["Pause", "Stop", "Fork", "Open pull request"]);
    expect(pageOf(SURFACE.overview, job({ paused: true }))?.rows[0]?.label).toBe("Resume");
    expect(pageOf(SURFACE.overview, job())?.rows.find((one) => one.label === "Stop")?.destructive).toBe(true);
  });
});

const pull = { kind: "pull_request", number: 1899, title: "t", branch: "b", address: "https://forge.example/armada/pull/1899", checks: { state: "passed" }, state: "open", auto: false } as Session["attachments"][number];

function world(over: Partial<RowWorld> = {}) {
  const draft = { pause: vi.fn(async () => true), resume: vi.fn(async () => true), stop: vi.fn(), fork: vi.fn(async () => "forked"), act: vi.fn(async () => true) };
  const hands = { openSession: vi.fn(), compose: vi.fn(), surface: vi.fn(), openLink: vi.fn(() => true), startJob: vi.fn(), task: vi.fn() };
  const projects = { name: vi.fn(), tick: vi.fn() };
  const held = job({ attachments: [pull] });
  const all = { fixtures: { pulls: [], issues: [], commands: [] }, jobs: [held], draft, job: undefined, projects, taskTitle: (id: string) => (id === "t1" ? "Tick a task" : undefined), ...hands, ...over } as unknown as RowWorld;
  return { all, draft, hands, projects };
}

beforeEach(() => {
  vi.stubGlobal("window", { requestAnimationFrame: (run: () => void) => (run(), 0), open: vi.fn() });
  focusAddTask.mockClear();
});
afterEach(() => vi.unstubAllGlobals());

describe("choosing a page's row", () => {
  test("Add a task goes to the add field the page's own key goes to", () => {
    carryOutRow({ of: "page", id: "add-task" }, world().all);
    expect(focusAddTask).toHaveBeenCalledTimes(1);
  });

  test("New project opens the name box through his projects", () => {
    const { all, projects } = world();
    carryOutRow({ of: "page", id: "new-project" }, all);
    expect(projects.name).toHaveBeenCalledWith(true);
  });

  test("the Cockpit's rows start a Job, show the map and the timeline, and go to Jobs needing him", () => {
    const { all, hands } = world();
    carryOutRow({ of: "page", id: "dispatch" }, all);
    expect(hands.compose).toHaveBeenCalledTimes(1);
    carryOutRow({ of: "page", id: "timeline" }, all);
    expect(setView).toHaveBeenCalledWith("timeline");
    carryOutRow({ of: "page", id: "needing-you" }, all);
    expect(hands.surface).toHaveBeenCalledWith(SURFACE.overview);
  });

  test("an open Job's rows act on that Job", () => {
    const { all, draft } = world({ job: job() });
    carryOutRow({ of: "page", id: "job-pause" }, all);
    expect(draft.pause).toHaveBeenCalledWith("j1");
  });
});

describe("the acts on a Job's row, which are the row menu's and the flight deck's", () => {
  const on = (action: string, w = world()) => (carryOutRow({ of: "on", target: "session:j1", action }, w.all), w);

  test("Pause and Resume", () => {
    expect(on("pause").draft.pause).toHaveBeenCalledWith("j1");
    expect(on("resume").draft.resume).toHaveBeenCalledWith("j1");
  });

  test("Stop", () => {
    expect(on("stop").draft.stop).toHaveBeenCalledWith("j1");
  });

  test("Merge merges the pull request the Job holds, by its number", () => {
    expect(on("merge-pull").draft.act).toHaveBeenCalledWith("j1", 1899, "merge");
  });

  test("Open pull request opens the address the Job holds it at, where a link opens", () => {
    expect(on("open-pull").hands.openLink).toHaveBeenCalledWith("https://forge.example/armada/pull/1899");
  });

  test("a Job with no pull request has nothing to merge or open", () => {
    const w = world({ jobs: [job()] });
    on("merge-pull", w);
    on("open-pull", w);
    expect(w.draft.act).not.toHaveBeenCalled();
    expect(w.hands.openLink).not.toHaveBeenCalled();
  });

  test("Fork opens the Job it started", async () => {
    const w = on("fork");
    await Promise.resolve();
    await Promise.resolve();
    expect(w.draft.fork).toHaveBeenCalledWith("j1");
    expect(w.hands.openSession).toHaveBeenCalledWith("forked");
  });
});

describe("the acts on a task's row", () => {
  test("Open shows it on Projects, Tick ticks it and Dispatch starts a Job with its title", () => {
    const w = world();
    carryOutRow({ of: "on", target: "task:t1", action: "open" }, w.all);
    expect(w.hands.task).toHaveBeenCalledWith("t1");
    carryOutRow({ of: "on", target: "task:t1", action: "tick" }, w.all);
    expect(w.projects.tick).toHaveBeenCalledWith("t1");
    carryOutRow({ of: "on", target: "task:t1", action: "dispatch" }, w.all);
    expect(w.hands.startJob).toHaveBeenCalledWith("Tick a task");
  });

  test("choosing the row itself shows the task", () => {
    const w = world();
    carryOutRow({ of: "task", id: "t1" }, w.all);
    expect(w.hands.task).toHaveBeenCalledWith("t1");
  });
});

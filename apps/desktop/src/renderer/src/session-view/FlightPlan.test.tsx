// How a plan group and its tasks are read in the Job's story: an open group lists its tasks and draws no track, a folded one draws the one slim track, and a task
// the agent set in progress and has not touched in half an hour dims and says how long. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { openInPanel } from "../shared-panel/store";
import { FlightRows } from "./FlightPlan";
import { TaskLine } from "./PlanGroup";
import type { ViewGroup } from "./plan-marks";

// The Jobs the window holds: a group handed to one draws it, found by the parent's ledger.
const held: { sessions: Session[] } = { sessions: [] };
vi.mock("../sessions-draft", async (original) => ({ ...(await original<typeof import("../sessions-draft")>()), useSessions: () => held.sessions }));
vi.mock("../shared-panel/store", async (original) => ({ ...(await original<typeof import("../shared-panel/store")>()), openInPanel: vi.fn(() => true) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  held.sessions = [];
  vi.mocked(openInPanel).mockClear();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

function show(node: React.ReactNode): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(node));
  roots.push({ root, host });
  return host;
}

const session = { id: "s1", attachments: [], rows: [], turn: { state: "idle" } } as unknown as Session;
const group = (id: string, ...states: string[]): ViewGroup => ({ id, title: `Group ${id}`, agent: "rust-engineer", paths: [], tasks: states.map((state, at) => ({ id: `${id}-t${at}`, text: `Task ${id} ${at}`, state })) }) as unknown as ViewGroup;

const TRACK = '[role="img"][aria-label="Tasks in this group, done and to do"]';
const rowOf = (host: HTMLElement, id: string) => host.querySelector<HTMLElement>(`[data-group="${id}"]`)!;

test("an open group lists its tasks and draws no track, and a folded group draws the track and lists nothing", () => {
  const host = show(<FlightRows groups={[group("one", "done", "working", "open"), group("two", "open", "open")]} workingGroup="one" now={undefined} session={session} manifestId={undefined} onOpenSession={() => undefined} />);
  const open = rowOf(host, "one");
  expect(open.dataset["open"]).toBe("true");
  expect(open.querySelectorAll(TRACK)).toHaveLength(0);
  expect(open.querySelectorAll('ul[aria-label="Tasks"] > li')).toHaveLength(3);

  const folded = rowOf(host, "two");
  expect(folded.dataset["open"]).toBeUndefined();
  expect(folded.querySelectorAll(TRACK)).toHaveLength(1);
  expect(folded.querySelectorAll(`${TRACK} > .sv-track__seg`)).toHaveLength(2);
  expect(folded.querySelector('ul[aria-label="Tasks"]')).toBeNull();
});

test("pressing an open group's row folds it and brings the track in", () => {
  const host = show(<FlightRows groups={[group("one", "done", "working")]} workingGroup="one" now={undefined} session={session} manifestId={undefined} onOpenSession={() => undefined} />);
  act(() => rowOf(host, "one").querySelector<HTMLElement>("button.sv-flight__row")!.click());
  expect(rowOf(host, "one").querySelectorAll(TRACK)).toHaveLength(1);
  expect(rowOf(host, "one").querySelector('ul[aria-label="Tasks"]')).toBeNull();
});

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const task = (state: string, minutes: number) => ({ id: "t", text: "Retry once", state, changedAt: ago(minutes) }) as unknown as ViewGroup["tasks"][number];

function line(minutes: number): HTMLElement {
  const host = show(
    <ul>
      <TaskLine task={task("working", minutes)} here={false} now={undefined} />
    </ul>,
  );
  return host.querySelector<HTMLElement>("li.sv-group__task")!;
}

test("a mark in progress untouched for more than 30 minutes dims, and reads its age in the largest unit", () => {
  const stale = line(90);
  expect(stale.dataset["stale"]).toBe("true");
  expect(stale.querySelector(".sv-group__task-age")?.firstChild?.textContent).toBe("1 h");
});

test("a mark touched inside 30 minutes keeps its colour and reads its age in minutes", () => {
  const fresh = line(5.5);
  expect(fresh.dataset["stale"]).toBeUndefined();
  expect(fresh.querySelector(".sv-group__task-age")?.firstChild?.textContent).toBe("5 m");
});

test("the age names what it is in its tooltip", () => {
  const age = line(90).querySelector<HTMLElement>(".sv-group__task-age")!;
  const bubble = document.getElementById(age.getAttribute("aria-describedby")!.split(" ").pop()!)!;
  expect(bubble.textContent).toBe("Since the agent last updated this task");
});

test("a task that is not in progress carries no age", () => {
  const host = show(
    <ul>
      <TaskLine task={task("done", 600)} here={false} now={undefined} />
    </ul>,
  );
  expect(host.querySelector(".sv-group__task-age")).toBeNull();
  expect(host.querySelector<HTMLElement>("li.sv-group__task")!.dataset["stale"]).toBeUndefined();
});

const handed = { id: "s1", attachments: [{ kind: "started_to", id: "c1", group: "one" }], rows: [], turn: { state: "idle" } } as unknown as Session;
const child = {
  id: "c1",
  title: "Migrations get names",
  attachments: [{ kind: "pull_request", number: 2412, title: "Name the migrations", branch: "armada/names", address: "https://forge/pull/2412", checks: { state: "pending" }, state: "open", auto: false }],
  rows: [],
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(1),
} as unknown as Session;

test("a group handed to a Job is the Job's card, titled with the Job and the group under it, with its pull request, and has no toggle; each press opens its own", () => {
  held.sessions = [handed, child];
  const opened: string[] = [];
  const host = show(<FlightRows calm groups={[group("one", "open"), group("two", "open")]} workingGroup={undefined} now={undefined} session={handed} manifestId="armada" onOpenSession={(id) => opened.push(id)} />);
  const row = rowOf(host, "one");
  expect(row.querySelector("button.sv-flight__row")).toBeNull();
  const card = row.querySelector<HTMLElement>("button.armada-job-card")!;
  expect(card.getAttribute("aria-label")).toBe("Job Migrations get names");
  expect(card.querySelector(".armada-job-card__says")?.textContent).toBe("Group one");
  expect(card.querySelector('[role="img"]')?.getAttribute("aria-label")).toBe("Working");
  act(() => card.click());
  expect(opened).toEqual(["c1"]);
  act(() => row.querySelector<HTMLElement>("button.armada-pull-chip")!.click());
  expect(vi.mocked(openInPanel)).toHaveBeenCalledWith({ kind: "pr", number: 2412, address: "https://forge/pull/2412" });
});

test("a group handed to a Job is the card whether its row would have been open or shut", () => {
  held.sessions = [handed, child];
  const host = show(<FlightRows all groups={[group("one", "open")]} workingGroup={undefined} now={undefined} session={handed} manifestId="armada" onOpenSession={() => undefined} />);
  expect(rowOf(host, "one").querySelector("button.armada-job-card")).not.toBeNull();
  expect(rowOf(host, "one").querySelector('ul[aria-label="Tasks"]')).toBeNull();
});

test("a group handed to no Job keeps its toggle and its track, and draws no card and no pull request", () => {
  held.sessions = [handed, child];
  const host = show(<FlightRows calm groups={[group("one", "open"), group("two", "open")]} workingGroup={undefined} now={undefined} session={handed} manifestId="armada" onOpenSession={() => undefined} />);
  const none = rowOf(host, "two");
  expect(none.querySelector(".armada-job-card, .armada-pull-chip")).toBeNull();
  expect(none.querySelector('button.sv-flight__row[aria-expanded="false"]')).not.toBeNull();
  expect(none.querySelectorAll(TRACK)).toHaveLength(1);
});

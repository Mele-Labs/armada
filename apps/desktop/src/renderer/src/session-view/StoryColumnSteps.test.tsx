// The top of the Job's story as the owner approved it (walk `spine-top-brief`): the workflow's header holds its name alone, the Brief is the first zone and folds to a line,
// and a Job at a step of a workflow draws one zone for each step, finished things under the step whose visit covers their time. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session, SessionWork, SessionWorkActs, WorkGroup } from "@armada/screens/src/draft/sessions";

import { StoryColumn } from "./StoryColumn";
import { pressBand, resetFolds } from "./story";

// The mod list is Fleet's, read through `window.armada`; no Job here made a mod.
vi.mock("../mod-list", () => ({ useModList: () => [] }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ID = "story-steps-s1";
const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  resetFolds(ID);
  localStorage.clear();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const STEPS = ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"] as const;
// Discuss and Approve plan are each visited twice: the Job went back from Approve plan to Discuss.
const VISITS = [
  { step: 0, since: "2026-10-27T09:02:00Z" },
  { step: 1, since: "2026-10-27T09:10:00Z" },
  { step: 2, since: "2026-10-27T09:40:00Z" },
  { step: 1, since: "2026-10-27T09:55:00Z" },
  { step: 2, since: "2026-10-27T10:20:00Z" },
  { step: 3, since: "2026-10-27T10:40:00Z" },
];

const group = (id: string, title: string, changedAt: string | undefined, state: "done" | "open" = "done"): WorkGroup =>
  ({ id, title, agent: "rust-engineer", paths: [], tasks: [{ id: `${id}-t`, text: `Task of ${title}`, state, ...(changedAt === undefined ? {} : { changedAt }) }] }) as unknown as WorkGroup;

type Own = { at?: number; visits?: typeof VISITS | undefined; groups?: WorkGroup[]; railOff?: true; workflow?: boolean; attachments?: unknown[]; trail?: unknown; started?: Session["started"]; extra?: Partial<Session>; acts?: SessionWorkActs; proposal?: SessionWork["proposal"] };

function build(own: Own = {}): { session: Session; work: SessionWork } {
  const groups = own.groups ?? [group("g1", "Alpha", "2026-10-27T09:30:00Z"), group("g2", "Live work", undefined, "open")];
  const plan = { scope: [], approved: true, groups };
  const work = {
    plan,
    ...(own.workflow === false ? {} : { workflow: { name: "Product", steps: STEPS, at: own.at ?? 3, ...("visits" in own ? (own.visits === undefined ? {} : { visits: own.visits }) : { visits: VISITS }) } }),
    ...(own.railOff === true ? { railOff: true } : {}),
    ...(own.proposal === undefined ? {} : { proposal: own.proposal }),
  } as unknown as SessionWork;
  const session = {
    id: ID,
    title: "Issues tab",
    attachments: own.attachments ?? [],
    turn: { state: "idle" },
    work,
    trail: own.trail,
    ...(own.started === undefined ? {} : { started: own.started }),
    rows: [{ id: "m1", at: "2026-10-27T09:02:00Z", kind: "message", from: { kind: "you" }, text: "Add an Issues tab beside Reviews" }],
    ...own.extra,
  } as unknown as Session;
  return { session, work };
}

function mount(own: Own = {}, opened: (id: string) => void = () => undefined): HTMLElement {
  const { session, work } = build(own);
  const ed = { id: ID, plan: work.plan!, marks: [], set: () => undefined, send: () => undefined };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() =>
    root.render(
      <StoryColumn session={session} entries={[]} work={work} id={ID} manifestId={undefined} acts={own.acts} ed={ed} onOpenSketch={() => undefined} onOpenSession={opened} live={undefined} />,
    ),
  );
  roots.push({ root, host });
  return host;
}

const zone = (host: HTMLElement, label: string) => host.querySelector<HTMLElement>(`section[aria-label="${label}"]`)!;
const zones = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>("section.sv-panel")].map((one) => one.getAttribute("aria-label"));
const shut = (one: HTMLElement) => one.getAttribute("data-collapsed") === "true";
const band = (one: HTMLElement) => one.querySelector<HTMLElement>(".sv-panel__band")!;

test("the header holds the workflow's name alone, with no step rail in it", () => {
  const host = mount();
  const head = host.querySelector<HTMLElement>(".sv-workflow")!;
  expect(head.querySelector(".sv-workflow__title")!.textContent).toBe("Product");
  expect(head.querySelector(".sv-rail, ol, li")).toBeNull();
});

test("the Brief is the first zone, folded to one line", () => {
  const host = mount();
  expect(zones(host)[0]).toBe("Brief");
  const press = zone(host, "Brief").querySelector<HTMLButtonElement>("button[aria-expanded]")!;
  expect(press.getAttribute("aria-expanded")).toBe("false");
  expect(press.textContent).toContain("Add an Issues tab beside Reviews");
  expect(zone(host, "Brief").querySelector("dl")).toBeNull();
});

test("a Job at a step of a workflow has one zone for each step, in the workflow's order, and no Completed, Now or Next", () => {
  const host = mount();
  expect(zones(host)).toEqual(["Brief", ...STEPS]);
});

test("a finished step is folded with a Done marker, the current step is open, and steps to come are dashed", () => {
  const host = mount();
  for (const name of ["Propose", "Discuss", "Approve plan"]) {
    expect(shut(zone(host, name)), name).toBe(true);
    expect(zone(host, name).querySelector('[role="img"][aria-label="Done"]'), name).not.toBeNull();
    expect(zone(host, name).getAttribute("data-area")).toBe("step-done");
  }
  expect(zone(host, "Walk").getAttribute("data-area")).toBe("step-now");
  expect(shut(zone(host, "Walk"))).toBe(false);
  for (const name of ["Refine", "Deliver"]) expect(zone(host, name).getAttribute("data-area"), name).toBe("step-next");
});

test("the current step holds Now, with the plan groups not yet started beneath the live work", () => {
  const host = mount({ groups: [group("g1", "Alpha", "2026-10-27T09:30:00Z"), group("g2", "Live work", undefined, "open"), group("g3", "Later work", undefined, "open")] });
  const walk = zone(host, "Walk");
  expect(walk.querySelector('ul[aria-label="Now"]')).not.toBeNull();
  expect(walk.textContent).toContain("Live work");
  expect(walk.textContent).toContain("Later work");
  expect(walk.textContent!.indexOf("Live work")).toBeLessThan(walk.textContent!.indexOf("Later work"));
});

test("the last step holds the landing that Next held", () => {
  const host = mount();
  const ahead = zone(host, "Deliver").querySelector('ol[aria-label="Ahead"]')!;
  expect(ahead.textContent).toContain("Pull request");
  expect(ahead.textContent).toContain("Merge queue");
  expect(zone(host, "Refine").querySelector('ol[aria-label="Ahead"]')).toBeNull();
  expect(host.querySelector('section[aria-label="Next"]')).toBeNull();
});

test("a finished group stands under the step whose visit covers its time, and a step visited twice is one zone in time order", () => {
  const host = mount({
    groups: [
      // Listed newest first to show the zone orders by time, not by the plan.
      group("g3", "Third", "2026-10-27T10:30:00Z"),
      group("g2", "Second", "2026-10-27T10:05:00Z"),
      group("g1", "First", "2026-10-27T09:30:00Z"),
      group("g4", "Proposing", "2026-10-27T09:05:00Z"),
      group("gw", "Live work", undefined, "open"),
    ],
  });
  expect(zone(host, "Propose").textContent).toContain("Proposing");
  // 09:30 is in Discuss's first visit; 10:05 is in its second (it began 09:55, after Approve plan's first).
  const discuss = zone(host, "Discuss").textContent!;
  expect(discuss).toContain("First");
  expect(discuss).toContain("Second");
  expect(discuss.indexOf("First")).toBeLessThan(discuss.indexOf("Second"));
  expect(discuss).not.toContain("Third");
  expect(zone(host, "Approve plan").textContent).toContain("Third");
  expect(zone(host, "Approve plan").textContent).not.toContain("First");
  // A step visited twice is one zone.
  expect(zones(host).filter((one) => one === "Discuss")).toHaveLength(1);
});

test("a finished group with no time stands under the last done step", () => {
  const host = mount({ groups: [group("g1", "Untimed", undefined), group("gw", "Live work", undefined, "open")] });
  expect(zone(host, "Approve plan").textContent).toContain("Untimed");
  expect(zone(host, "Discuss").textContent).not.toContain("Untimed");
});

test("with no visits at all, everything finished stands under the last done step", () => {
  const host = mount({ visits: undefined, groups: [group("g1", "Early", "2026-10-27T09:05:00Z"), group("gw", "Live work", undefined, "open")] });
  expect(zone(host, "Approve plan").textContent).toContain("Early");
  expect(zone(host, "Propose").textContent).not.toContain("Early");
});

test("a time after the current step began stands under the last done step, never under the current one", () => {
  const host = mount({ groups: [group("g1", "Late", "2026-10-27T11:00:00Z"), group("gw", "Live work", undefined, "open")] });
  expect(zone(host, "Approve plan").textContent).toContain("Late");
});

test("a finished pull request stands under the step its opening time falls in", () => {
  const pr = { kind: "pull_request", number: 3012, title: "Search the issues", branch: "armada/issues", address: "https://forge/3012", checks: { state: "passed" }, state: "open", auto: false };
  const host = mount({ attachments: [pr], trail: { started: "2026-10-27T09:02:00Z", opened: { 3012: "2026-10-27T10:10:00Z" } } });
  // 10:10 falls in Discuss's second visit (from 09:55), before Approve plan's second (10:20).
  expect(zone(host, "Discuss").textContent).toContain("Search the issues");
  expect(zone(host, "Approve plan").textContent).not.toContain("Search the issues");
});

test("a Job with no workflow keeps Brief, Completed, Now and Next", () => {
  const host = mount({ workflow: false });
  expect(zones(host)).toEqual(["Brief", "Completed", "Now", "Next"]);
  expect(host.querySelector(".sv-workflow")).toBeNull();
});

test("a Job whose workflows are off keeps Brief, Completed, Now and Next", () => {
  const host = mount({ railOff: true });
  expect(zones(host)).toEqual(["Brief", "Completed", "Now", "Next"]);
});

test("the Brief sits above Completed and is not drawn inside it", () => {
  const host = mount({ workflow: false });
  expect(zone(host, "Completed").textContent).not.toContain("Add an Issues tab beside Reviews");
  expect(zone(host, "Completed").querySelector('ol[aria-label="By step"]')).not.toBeNull();
});

test("⌥1 folds and unfolds all the done steps, ⌥2 the current step, ⌥3 the steps to come", () => {
  const host = mount();
  const { session } = build();
  const done = ["Propose", "Discuss", "Approve plan"];
  const coming = ["Refine", "Deliver"];

  act(() => pressBand(session, "done"));
  for (const name of done) expect(shut(zone(host, name)), `${name} opened`).toBe(false);
  act(() => pressBand(session, "done"));
  for (const name of done) expect(shut(zone(host, name)), `${name} folded`).toBe(true);

  act(() => pressBand(session, "now"));
  expect(shut(zone(host, "Walk"))).toBe(true);
  act(() => pressBand(session, "now"));
  expect(shut(zone(host, "Walk"))).toBe(false);

  act(() => pressBand(session, "next"));
  for (const name of coming) expect(shut(zone(host, name)), `${name} folded`).toBe(true);
  act(() => pressBand(session, "next"));
  for (const name of coming) expect(shut(zone(host, name)), `${name} opened`).toBe(false);
});

test("pressing one done step opens it alone, and ⌥1 afterwards sets them all", () => {
  const host = mount();
  const { session } = build();
  act(() => band(zone(host, "Discuss")).click());
  expect(shut(zone(host, "Discuss"))).toBe(false);
  expect(shut(zone(host, "Propose"))).toBe(true);
  act(() => pressBand(session, "done"));
  for (const name of ["Propose", "Discuss", "Approve plan"]) expect(shut(zone(host, name)), name).toBe(false);
});

// ---- The Brief ----

const brief = (host: HTMLElement) => zone(host, "Brief");
const press = (host: HTMLElement) => brief(host).querySelector<HTMLButtonElement>("button[aria-expanded]")!;
const rows = (host: HTMLElement) => Object.fromEntries([...brief(host).querySelectorAll("dl > div")].map((one) => [one.querySelector("dt")!.textContent, one.querySelector("dd")!.textContent]));

test("pressing the Brief shows the whole text and the start settings", () => {
  const host = mount({
    started: { workflow: "Product", delivery: "open_pr", model: "opus", effort: "high", mode: "auto" },
  });
  act(() => press(host).click());
  expect(press(host).getAttribute("aria-expanded")).toBe("true");
  expect(brief(host).querySelector(".sv-brief__text")!.textContent).toBe("Add an Issues tab beside Reviews");
  expect(rows(host)).toEqual({ Workflow: "Product", Delivery: "Open a pull request", Model: "opus", Effort: "high", Mode: "Auto" });
  act(() => press(host).click());
  expect(press(host).getAttribute("aria-expanded")).toBe("false");
});

test("what the Job started with wins over what it holds now", () => {
  const host = mount({
    started: { workflow: "Bug fix", delivery: "keep_local", model: "sonnet", effort: "low", mode: "plan" },
    extra: { model: "opus", effort: "high", mode: "auto", start: { picked: { choice: "auto_land" } } } as unknown as Partial<Session>,
  });
  act(() => press(host).click());
  expect(rows(host)).toEqual({ Workflow: "Bug fix", Delivery: "Keep local", Model: "sonnet", Effort: "low", Mode: "Plan" });
});

test("without `started`, the grid falls back to the picked delivery, model, effort, mode and the workflow's name", () => {
  const host = mount({ extra: { model: "opus", effort: "high", mode: "accept_edits", start: { picked: { choice: "auto_land" } } } as unknown as Partial<Session> });
  act(() => press(host).click());
  expect(rows(host)).toEqual({ Workflow: "Product", Delivery: "Auto land", Model: "opus", Effort: "high", Mode: "Accept edits" });
});

test("a setting with no value is left out of the grid", () => {
  const host = mount({ workflow: false, started: { model: "opus" } });
  act(() => press(host).click());
  expect(rows(host)).toEqual({ Model: "opus" });
});

test("a Job with nothing to say about how it started draws no grid", () => {
  const host = mount({ workflow: false });
  act(() => press(host).click());
  expect(brief(host).querySelector("dl")).toBeNull();
});

test("Started by is the Job it came from, and pressing it opens that Job", () => {
  const opened = vi.fn();
  const host = mount({ started: { model: "opus", by: { job: "one-view-1", title: "Retry the slot lease" } } }, opened);
  act(() => press(host).click());
  expect(rows(host)["Started by"]).toBe("Retry the slot lease");
  act(() => brief(host).querySelector<HTMLButtonElement>(".sv-brief__by")!.click());
  expect(opened).toHaveBeenCalledWith("one-view-1");
});

test("a Job with no one it was started by draws no Started by row", () => {
  const host = mount({ started: { model: "opus" } });
  act(() => press(host).click());
  expect(rows(host)["Started by"]).toBeUndefined();
});

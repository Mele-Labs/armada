// Progress: the workflow as one line of glyphs, each named by its hover card, and the plan's groups under it. The step Fleet names is lit,
// the ones before it are done, and with none named every step is to come. Browser: a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import type { Session, SessionWork, SessionWorkActs, WorkPlan } from "@armada/screens/src/draft/sessions";

import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom } from "../sessions-draft";
import type { PlanEditing } from "./PlanEdit";
import { ProgressPanel } from "./ProgressPanel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => localStorage.clear());
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  localStorage.clear();
});

const plan = { scope: [], approved: false, groups: [{ id: "g1", title: "Retry", agent: "rust-engineer", paths: [], tasks: [{ id: "t1", text: "Retry the lease", state: "open" }] }] } as unknown as WorkPlan;
const ed: PlanEditing = { id: "p1", plan, marks: [], set: () => undefined, send: () => undefined };
const session = { id: "s1", attachments: [], rows: [], turn: { state: "idle" } } as unknown as Session;

const subagents = (...tasks: [string, "running" | "done"][]) =>
  ({ id: "s1", attachments: tasks.map(([task, state], i) => ({ kind: "subagent", id: `sa-${i}`, task, state, agentType: "bridge-engineer" })), rows: [], turn: { state: "idle" } }) as unknown as Session;

function mount(work: SessionWork, editing: PlanEditing | undefined = undefined, held: Session = session, acts: SessionWorkActs | undefined = undefined): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<ProgressPanel work={work} place="side" id="s1" session={held} acts={acts} ed={editing} onOpenSketch={() => undefined} onOpenSession={() => undefined} />));
  roots.push({ root, host });
  return host;
}

const states = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>(".sv-rail__step")].map((one) => one.dataset["state"]);

test("the step Fleet names is lit, the ones before it are done and the ones after are to come", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan", "Build", "Pull request"], at: 1 } });
  expect(states(host)).toEqual(["done", "now", "next"]);
  expect(host.querySelector('[aria-current="step"] [aria-label="Build · current"]')).not.toBeNull();
});

test("with no step named every step is to come and none is lit", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan", "Build"] } });
  expect(states(host)).toEqual(["next", "next"]);
  expect(host.querySelector('[aria-current="step"]')).toBeNull();
});

test("a step is a glyph with its name in its hover card, and no name is written beside it", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan", "Build"], at: 0 } });
  expect(host.querySelector('[aria-label="Plan · current"]')).not.toBeNull();
  expect(host.querySelector('[aria-label="Build · next"]')).not.toBeNull();
  expect(host.querySelectorAll(".sv-rail__step [role='img']")).toHaveLength(2);
  expect(host.querySelector(".sv-step__name")).toBeNull();
});

test("it is one panel, Progress, and neither a Status nor a Plan panel is drawn beside it", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan"], at: 0 }, plan }, ed);
  expect(host.querySelectorAll('section[aria-label="Progress"]')).toHaveLength(1);
  const panel = host.querySelector('section[aria-label="Progress"]')!;
  expect(host.querySelectorAll('section[aria-label="Status"]')).toHaveLength(0);
  // Plan is a zone inside Progress, never a panel beside it.
  expect([...host.querySelectorAll('section[aria-label="Plan"]')].every((one) => panel.contains(one))).toBe(true);
  expect(host.querySelector('section[aria-label="Retry"]')).not.toBeNull();
});

test("the group being worked stands open and the others fold to a line", () => {
  const task = (id: string, state: "open" | "working" | "done") => ({ id, text: `Task ${id}`, state });
  const group = (id: string, title: string, state: "open" | "working" | "done") => ({ id, title, agent: "rust-engineer", paths: [], tasks: [task(`${id}-1`, state)] });
  const three = { scope: [], approved: true, groups: [group("g1", "Schema", "done"), group("g2", "Retry", "working"), group("g3", "Docs", "open")] } as unknown as WorkPlan;
  const host = mount({ plan: three }, { ...ed, plan: three });
  const opened = (title: string) => host.querySelector<HTMLElement>(`section[aria-label="${title}"]`)!.dataset["open"] !== undefined;
  expect([opened("Schema"), opened("Retry"), opened("Docs")]).toEqual([false, true, false]);
});

test("Ask for an update is a button in the band, and pressing it leaves the panel open", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan"], at: 0 }, plan }, ed);
  const section = host.querySelector<HTMLElement>('section[aria-label="Progress"]')!;
  const ask = section.querySelector<HTMLButtonElement>('header button[aria-label="Ask for an update"]');
  expect(ask).not.toBeNull();
  act(() => ask!.click());
  expect(section.dataset["collapsed"]).toBeUndefined();
});

test("a Session with no plan is its band alone", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan"], at: 0 } });
  expect(host.querySelector(".sv-plan")).toBeNull();
});

test("with no plan, the subagents at work are listed, each its working mark and its title, and a finished one is left to the layer", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan"], at: 0 } }, undefined, subagents(["Harden the restart scripts", "running"], ["Failed refresh toast", "running"], ["Old", "done"]));
  const rows = [...host.querySelectorAll(".sv-subagents__row")];
  expect(rows.map((one) => one.querySelector(".sv-subagents__task")?.textContent)).toEqual(["Harden the restart scripts", "Failed refresh toast"]);
  expect(rows[0]!.querySelector('[role="img"][aria-label="Working"]')).not.toBeNull();
});

test("with a plan the subagents are not listed beside it", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan"], at: 0 }, plan }, ed, subagents(["Harden the restart scripts", "running"]));
  expect(host.querySelector(".sv-subagents")).toBeNull();
});

test("with a plan the agent's own tasks are not listed beside it, and with none they are, above the subagents", () => {
  const tasks = [{ id: "a", subject: "Rename the files", state: "working" as const }];
  const planned = mount({ workflow: { name: "Feature", steps: ["Plan"], at: 0 }, plan, tasks }, ed);
  expect(planned.querySelector(".sv-agent-tasks")).toBeNull();
  const open = mount({ workflow: { name: "Feature", steps: ["Plan"], at: 0 }, tasks }, undefined, subagents(["Harden the restart scripts", "running"]));
  expect(open.querySelector(".sv-agent-tasks__subject")?.textContent).toBe("Rename the files");
  expect(open.querySelector(".sv-agent-tasks + .sv-subagents")).not.toBeNull();
});

test("a bare None select stands folded to the band, and gives way to a subagent at work", () => {
  const acts = { workflows: [], choose: () => undefined } as unknown as SessionWorkActs;
  const work: SessionWork = { proposal: { workflow: null } };
  const bare = mount(work, undefined, session, acts);
  expect(bare.querySelector<HTMLElement>('section[aria-label="Progress"]')!.dataset["collapsed"]).toBe("true");
  const busy = mount(work, undefined, subagents(["Harden the restart scripts", "running"]), acts);
  expect(busy.querySelector<HTMLElement>('section[aria-label="Progress"]')!.dataset["collapsed"]).toBeUndefined();
});

test("a Drop on a Later row asks Fleet to drop that follow-up by its id, and the row stays until Fleet's row loses it", () => {
  const dropped: [string, string][] = [];
  const none: Session[] = [];
  const draft = { get: () => none, subscribe: () => () => undefined, dropLater: (id: string, later: string) => dropped.push([id, later]) } as unknown as SessionsDraft;
  const held = { ...session, later: [{ id: "later-2", text: "The Studio icon tooltip is still owed.", at: "2026-10-21T10:00:00Z" }] } as unknown as Session;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() =>
    root.render(
      <SessionsFrom held={draft}>
        <ProgressPanel work={{ proposal: { workflow: null } }} place="side" id="s1" session={held} acts={undefined} ed={undefined} onOpenSketch={() => undefined} onOpenSession={() => undefined} />
      </SessionsFrom>,
    ),
  );
  roots.push({ root, host });
  act(() => host.querySelector<HTMLButtonElement>(".sv-later__row")!.click());
  act(() => host.querySelector<HTMLButtonElement>('button[aria-label^="Drop"]')!.click());
  expect(dropped).toEqual([["s1", "later-2"]]);
  expect(host.querySelector(".sv-later__item")).not.toBeNull();
});

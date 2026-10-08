// A Trigger that holds the Job, from the rows Fleet serves: which firings hold, where each is drawn on the
// canvas and in the list, and what Rerun and Skip send. A hold at a step is on the line it holds, so the
// edge stays one edge with one arrowhead; at `pr_opened` it hangs beside the delivering step.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { AddedStep, JobAlert, JobTrigger } from "@armada/protocol";
import { FiredTriggers, HoldNode, holdsOf, JobAlertMark, triggerAlert } from "@armada/components";
import type { WorkflowCanvasEdge } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

import { repairBranches, withHolds } from "./repair-branch";

afterEach(() => unmount());

/** What the tooltip says. Its bubble is hidden from the accessibility tree, so it is read off the page. */
const tipped = () => [...document.querySelectorAll(".armada-tooltip__label")].map((one) => one.textContent).join("|");

const AT = "2026-10-07T10:00:00.000Z";

const fired = (state: JobTrigger["state"], over: Partial<JobTrigger> = {}): JobTrigger => ({
  name: "deploy_qa",
  when: "pr_opened",
  step: "handoff",
  level: "machine",
  state,
  exit_code: 1,
  started_at: AT,
  blocks: true,
  ...over,
});

const ADDED: AddedStep = {
  id: "a1",
  runs: { kind: "script", command: "smoke" },
  when: "step_starts",
  step: "verify",
  block: true,
  repair: false,
  placed: "running",
  added_at: AT,
  state: "held",
};

test("a blocking firing holds the Job while it is held or being repaired, and a passed one does not", () => {
  expect(holdsOf([fired("held")]).map((one) => one.by)).toEqual([{ trigger: "deploy_qa" }]);
  for (const state of ["repairing", "rerunning", "fix_ready"] as const) expect(holdsOf([fired(state)])).toHaveLength(1);
  expect(holdsOf([fired("passed")])).toEqual([]);
  expect(holdsOf([fired("skipped")])).toEqual([]);
  // A Trigger that does not block is not a hold while it repairs.
  expect(holdsOf([fired("repairing", { blocks: false })])).toEqual([]);
  // The latest firing is the one that counts.
  expect(holdsOf([fired("held"), fired("passed")])).toEqual([]);
});

test("an added step holds by its id, and the Job has an alert while a hold stands", () => {
  expect(holdsOf([], [ADDED])).toMatchObject([{ name: "smoke", by: { addition: "a1" }, when: "step_starts", step: "verify" }]);
  expect(holdsOf([], [{ ...ADDED, state: "passed" }])).toEqual([]);
  expect(triggerAlert([fired("held")])).toBe(true);
  expect(triggerAlert([], [ADDED])).toBe(true);
  expect(triggerAlert([fired("passed")])).toBe(false);
});

const SPINE: WorkflowCanvasEdge[] = [
  { id: "plan>verify", source: "plan", target: "verify", kind: "leads" },
  { id: "verify>handoff", source: "verify", target: "handoff", kind: "leads" },
];
const anchorOf = (at: { step: string }) => ({ id: at.step === "" ? "pr" : at.step, x: 0, y: 100, width: 260 });
const drawn = (triggers: JobTrigger[], additions: AddedStep[] = []) =>
  repairBranches(triggers, "job-1", anchorOf, () => undefined, undefined, { holds: holdsOf(triggers, additions), spine: SPINE, act: undefined });

test("a hold before a step is on the line into it, and after a step on the line out of it, and neither adds an edge", () => {
  const before = drawn([fired("held", { when: "step_starts", step: "handoff" })]);
  expect([...before.onLine.keys()]).toEqual(["verify>handoff"]);
  expect(before.nodes).toEqual([]);
  expect(before.edges).toEqual([]);
  const after = drawn([fired("held", { when: "step_passes", step: "plan" })]);
  expect([...after.onLine.keys()]).toEqual(["plan>verify"]);
  const edges = withHolds(SPINE, after.onLine);
  expect(edges).toHaveLength(SPINE.length);
  expect(edges.filter((one) => one.add !== undefined).map((one) => one.id)).toEqual(["plan>verify"]);
});

test("a hold at the pull request hangs beside the delivering step, and a repair under way stands under it", () => {
  const { nodes, edges, onLine } = drawn([fired("repairing", { repair: { attempt: 1, branch: "armada/repair-deploy_qa-1" } })]);
  expect(onLine.size).toBe(0);
  expect(nodes.map((one) => one.position.y)).toEqual([100, 260]);
  expect(edges).toMatchObject([{ source: "handoff", across: true }, { source: "handoff", across: true }]);
});

test("the hold names the Trigger and its moment, and Rerun and Skip send the Trigger's name", async () => {
  const act = vi.fn().mockResolvedValue({ ok: true });
  const [held] = holdsOf([fired("held")]);
  mount(<HoldNode held={held!} onAct={act} />);
  await expect.element(page.getByRole("group", { name: "deploy_qa, PR opened, handoff" })).toBeVisible();
  await page.getByRole("img", { name: /Held, deploy_qa/ }).hover();
  expect(tipped()).toContain("deploy_qa, PR opened, handoff");
  await page.getByRole("button", { name: "Rerun" }).click();
  expect(act).toHaveBeenCalledWith("rerun", { trigger: "deploy_qa" });
});

test("a hold on an added step is skipped by its id", async () => {
  const act = vi.fn().mockResolvedValue({ ok: true });
  const [held] = holdsOf([], [ADDED]);
  mount(<HoldNode held={held!} onAct={act} />);
  await page.getByRole("button", { name: "Skip" }).click();
  expect(act).toHaveBeenCalledWith("skip", { addition: "a1" });
});

test("a hold's act is sent once, and the buttons come back where Fleet refused it", async () => {
  const act = vi.fn().mockResolvedValueOnce({ ok: false }).mockResolvedValueOnce({ ok: true });
  const [held] = holdsOf([fired("held")]);
  mount(<HoldNode held={held!} onAct={act} />);
  await page.getByRole("button", { name: "Rerun" }).click();
  await page.getByRole("button", { name: "Skip" }).click();
  expect(act).toHaveBeenLastCalledWith("skip", { trigger: "deploy_qa" });
  await expect.element(page.getByRole("button", { name: "Skip" })).toBeDisabled();
  expect(act).toHaveBeenCalledTimes(2);
});

test("a hold under repair takes neither act, and a fix waiting on its choice can only be skipped", async () => {
  const [repairing] = holdsOf([fired("repairing")]);
  mount(<HoldNode held={repairing!} onAct={vi.fn()} />);
  await expect.element(page.getByRole("button", { name: "Rerun" })).toBeDisabled();
  await expect.element(page.getByRole("button", { name: "Skip" })).toBeDisabled();
  unmount();
  const [waiting] = holdsOf([fired("fix_ready")]);
  mount(<HoldNode held={waiting!} onAct={vi.fn()} />);
  await expect.element(page.getByRole("button", { name: "Rerun" })).toBeDisabled();
  await expect.element(page.getByRole("button", { name: "Skip" })).toBeEnabled();
});

test("the Triggers list offers a hold the same two acts, and a held added step too", async () => {
  const act = vi.fn().mockResolvedValue({ ok: true });
  mount(<FiredTriggers triggers={[fired("held"), fired("passed", { name: "fmt", blocks: false })]} additions={[ADDED]} onHoldAct={act} />);
  const rows = page.getByRole("list", { name: "Triggers" });
  await expect.element(rows.getByRole("button", { name: "Rerun" }).first()).toBeVisible();
  expect(rows.getByRole("button", { name: "Rerun" }).elements()).toHaveLength(2);
  await rows.getByRole("button", { name: "Rerun" }).first().click();
  expect(act).toHaveBeenCalledWith("rerun", { trigger: "deploy_qa" });
  await rows.getByRole("button", { name: "Skip" }).last().click();
  expect(act).toHaveBeenLastCalledWith("skip", { addition: "a1" });
});

test("a Board row's bell for a hold says the Trigger, the moment and the step, and draws no count", async () => {
  const alert: JobAlert = { kind: "held", trigger: "deploy_qa", when: "pr_opened", step: "handoff" };
  mount(<JobAlertMark alert={alert} />);
  const bell = page.getByRole("img", { name: "Held, deploy_qa, PR opened, handoff" });
  await expect.element(bell).toBeVisible();
  await bell.hover();
  expect(tipped()).toBe("Held, deploy_qa, PR opened, handoff");
  expect(document.body.textContent).not.toMatch(/\d/);
});

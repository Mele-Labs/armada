// A failed Trigger's repair branch, from the rows Fleet serves: where it starts, what each end of it
// draws, and what the choice does. The three ends are a fix placed on this branch, a fix that became a
// pull request, and a repair that found none.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { JobTrigger } from "@armada/protocol";
import { additionBranches, fixOf, RepairNode, triggerAlert } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

import { leavesOf, leafRoom, repairBranches, repairNodeId } from "./repair-branch";

afterEach(() => unmount());

const AT = "2026-10-07T10:00:00.000Z";

/** `deploy_qa` after the pull request opened, with the repair as `state` and `repair` say. */
const deploy = (state: JobTrigger["state"], repair: JobTrigger["repair"]): JobTrigger => ({
  name: "deploy_qa",
  when: "pr_opened",
  step: "handoff",
  level: "machine",
  state,
  exit_code: 1,
  started_at: AT,
  ...(repair === undefined ? {} : { repair }),
});

const FIX = { attempt: 1, branch: "armada/repair-deploy_qa-1", files: ["deploy/qa.sh", ".armada/qa.env"] };
const ANCHOR = { id: "pr", x: 0, y: 100, width: 260 };
const branches = (triggers: JobTrigger[]) => repairBranches(triggers, "job-1", () => ANCHOR, () => "land", undefined);

test("a branch starts at the node the Trigger fired at, beside it, and the edge leaves it sideways", () => {
  const { nodes, edges } = branches([deploy("repairing", { attempt: 1, branch: FIX.branch })]);
  expect(nodes.map((one) => one.position)).toEqual([{ x: 316, y: 100 }]);
  expect(edges).toMatchObject([{ source: "pr", kind: "leads", across: true, flowing: true }]);
});

test("a fix placed on this branch joins back into the spine, and draws no pull request", () => {
  const { nodes, edges } = branches([deploy("passed", { ...FIX, choice: "this_branch" })]);
  expect(nodes).toHaveLength(1);
  expect(edges.map((one) => [one.source === "pr" ? "pr" : "branch", one.target === "land" ? "land" : "branch"])).toEqual([
    ["pr", "branch"],
    ["branch", "land"],
  ]);
});

test("a fix that became a pull request ends in a mark of its own, under the branch", () => {
  const trigger = deploy("passed", { ...FIX, choice: "new_pr", pull_request: { url: "https://forge.test/pull/1751", number: 1751 } });
  const { nodes, edges } = branches([trigger]);
  expect(nodes.map((one) => one.id)).toEqual([repairNodeId(trigger), `${repairNodeId(trigger)}:pr`]);
  expect(edges.some((one) => one.target === "land")).toBe(false);
  expect(edges).toHaveLength(2);
});

test("a repair that found no fix ends where it stands: no pull request, and no way back into the spine", () => {
  const { nodes, edges } = branches([deploy("failed", { attempt: 2, branch: FIX.branch })]);
  expect(nodes).toHaveLength(1);
  expect(edges).toHaveLength(1);
});

test("a Trigger no repair Drone was put on is a mark off its step, with no branch under it", () => {
  const { nodes, edges } = branches([deploy("failed", undefined)]);
  expect(nodes.map((one) => one.id)).toEqual(["leaf:pr_opened|handoff|deploy_qa"]);
  expect(edges).toMatchObject([{ source: "pr", kind: "leads", across: true }]);
});

test("a fix held for the owner asks, and the Job has an alert until he chooses", () => {
  const held = deploy("fix_ready", FIX);
  expect(branches([held]).asking).toBe(repairNodeId(held));
  expect(triggerAlert([held])).toBe(true);
  expect(triggerAlert([deploy("fix_ready", { ...FIX, choice: "this_branch" })])).toBe(false);
  expect(triggerAlert([deploy("failed", { attempt: 2 })])).toBe(true);
  // A failure with no repair is not an alert, and a later pass clears one.
  expect(triggerAlert([deploy("failed", undefined)])).toBe(false);
  expect(triggerAlert([deploy("failed", { attempt: 2 }), deploy("passed", { attempt: 2 })])).toBe(false);
});

test("the choice is sent once, and the buttons come back where Fleet refused it", async () => {
  const choose = vi.fn().mockResolvedValueOnce({ ok: false }).mockResolvedValueOnce({ ok: true });
  const held = deploy("fix_ready", FIX);
  mount(<RepairNode trigger={held} onChoose={choose} />);
  await expect.element(page.getByRole("list", { name: "The fix" })).toBeVisible();
  await page.getByRole("button", { name: "New PR" }).click();
  expect(choose).toHaveBeenCalledWith(held, "new_pr");
  // Refused: asked again.
  await page.getByRole("button", { name: "This branch" }).click();
  expect(choose).toHaveBeenLastCalledWith(held, "this_branch");
  // Taken: the buttons stay down until the row moves.
  await expect.element(page.getByRole("button", { name: "This branch" })).toBeDisabled();
  expect(choose).toHaveBeenCalledTimes(2);
});

test("a Skill's run draws a branch while its Drone works, named for a Drone", async () => {
  const running = { ...deploy("running", { attempt: 1, branch: FIX.branch }), drone: true };
  const { nodes, edges } = branches([running]);
  expect(nodes).toHaveLength(1);
  expect(edges).toMatchObject([{ flowing: true }]);
  // A Command that is running has no branch: it is a mark.
  expect(branches([deploy("running", undefined)]).nodes.map((one) => one.id)).toEqual(["leaf:pr_opened|handoff|deploy_qa"]);
  mount(<RepairNode trigger={running} />);
  await expect.element(page.getByRole("img", { name: "Drone branch" })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Running" })).toBeVisible();
});

test("a Drone that changed nothing leaves no branch, and an added step's fix is chosen by its id", () => {
  expect(branches([deploy("passed", { attempt: 1, branch: FIX.branch })]).nodes.map((one) => one.id)).toEqual(["leaf:pr_opened|handoff|deploy_qa"]);
  const step = additionBranches([
    {
      id: "a1",
      runs: { kind: "drone", brief: "Add a line" },
      when: "step_passes",
      step: "implement",
      block: false,
      repair: false,
      placed: "approval",
      added_at: AT,
      state: "fix_ready",
      repair_record: FIX,
    },
  ]);
  expect(step.map((one) => [one.drone, fixOf(one)])).toEqual([[true, { addition: "a1" }]]);
  expect(branches(step).asking).toBe(repairNodeId(step[0]!));
});

test("a Script added to the Job that waits on him to run it is a leaf with Run and Skip, named to Fleet by its id", () => {
  const waiting = additionBranches([
    { id: "a1", runs: { kind: "script", command: "wipe_qa" }, when: "pr_opened", step: "handoff", block: true, repair: false, placed: "running", added_at: AT, state: "awaiting_owner" },
  ]);
  expect(waiting.map((one) => fixOf(one))).toEqual([{ addition: "a1" }]);
  expect(branches(waiting).nodes.map((one) => one.id)).toEqual(["leaf:addition|a1"]);
  expect(triggerAlert([], [{ id: "a1", runs: { kind: "script", command: "wipe_qa" }, when: "pr_opened", step: "handoff", block: false, repair: false, placed: "running", added_at: AT, state: "awaiting_owner" }])).toBe(true);
});

test("a leaf stands in the column its lane made, and a second stands under the first with room for its files", () => {
  const first = deploy("fix_ready", FIX);
  const second = { ...deploy("passed", undefined), name: "fmt" };
  const { nodes } = repairBranches([first, second], "job-1", () => ({ ...ANCHOR, leafX: 400 }), () => undefined, undefined);
  expect(nodes.map((one) => one.position.x)).toEqual([400, 400]);
  // The repair's head, its two files and its choice, then the gap.
  expect(nodes[1]!.position.y - nodes[0]!.position.y).toBe(64 + 2 * 22 + 52 + 12);
});

test("the room a step's leaves ask of its lane is the stack they make, and a step with none asks nothing", () => {
  const leaves = leavesOf([deploy("fix_ready", FIX), { ...deploy("passed", undefined), name: "fmt" }], "job-1", (at) => (at.step === "handoff" ? "pr" : undefined), undefined);
  expect([...leafRoom(leaves)]).toEqual([["pr", 64 + 2 * 22 + 52 + 12 + 44]]);
  expect(leafRoom(leavesOf([deploy("passed", undefined)], "job-1", () => undefined, undefined)).size).toBe(0);
});

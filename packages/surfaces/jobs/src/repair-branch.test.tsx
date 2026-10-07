// A failed Trigger's repair branch, from the rows Fleet serves: where it starts, what each end of it
// draws, and what the choice does. The three ends are a fix placed on this branch, a fix that became a
// pull request, and a repair that found none.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { JobTrigger } from "@armada/protocol";
import { RepairNode, triggerAlert } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

import { repairBranches, repairNodeId } from "./repair-branch";

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

test("a Trigger no repair Drone was put on draws no branch", () => {
  expect(branches([deploy("failed", undefined)]).nodes).toEqual([]);
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

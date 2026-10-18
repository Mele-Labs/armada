// A form edit's pull request the forge would not auto-merge, as a real window acts on it: no Sessions fixtures in the scenario, so
// Merge and Review go over `WiredSessions` to the fake's `window.armada`, which is the call main makes to Fleet by the repository's
// Manifest and the number (#2396). The hub row's `merge_by_hand` is what draws every place; nothing here is mock state.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { s261MergeByHand } from "./scenarios/merge-by-hand";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const wired = { ...s261MergeByHand, draft: { ...s261MergeByHand.draft, sessions: undefined } };
const merged = { ok: true, value: { manifest_id: "armada", number: 2405, state: "merged", auto_merge: false, checks: { state: "passed" }, title: "", branch: "", address: "" } } as const;

async function started() {
  const app = mount(wired);
  const press = vi.spyOn(app.api, "pressRepositoryPullRequest").mockResolvedValue(merged as never);
  await onScreen();
  return { app, press };
}

const chip = () => page.getByRole("region", { name: "Merge queue" }).getByRole("button", { name: /^#2405/ });
const marked = () => page.getByRole("img", { name: "Needs merging by hand" });

test("the merge queue chip is drawn from the hub row and its pane's Merge asks Fleet by the repository's Manifest and the number", async () => {
  const { press } = await started();
  await chip().click();
  await page.getByRole("region", { name: "Pull request #2405" }).getByRole("button", { name: "Merge", exact: true }).click();
  await expect.poll(() => press.mock.calls.length).toBe(1);
  const [manifest, number, act] = press.mock.calls[0]!;
  expect(typeof manifest).toBe("string");
  expect([number, act]).toEqual([2405, "merge"]);
});

test("the form's line under Save has Merge, and it is the same press", async () => {
  const { press } = await started();
  await page.getByRole("button", { name: "Open the Manifest", exact: true }).click();
  await page.getByRole("button", { name: "Choose a repository" }).click();
  await page.getByRole("menuitem", { name: "armada" }).click();
  await page.getByRole("tab", { name: "Edit" }).click();
  const form = page.getByRole("tabpanel");
  await expect.element(form.getByRole("button", { name: "#2405", exact: true })).toBeVisible();
  await form.getByRole("button", { name: "Merge" }).click();
  await expect.poll(() => press.mock.calls.length).toBe(1);
  expect(press.mock.calls[0]!.slice(1)).toEqual([2405, "merge"]);
});

test("the same pull request without the hub's mark has no glyph", async () => {
  const lines = wired.state.mergeLines!.lines.map((line) => ({
    ...line,
    hub: { ...line.hub, pull_requests: (line.hub?.pull_requests ?? []).map(({ merge_by_hand: _marked, ...row }) => row) },
  }));
  mount({ ...wired, state: { ...wired.state, mergeLines: { lines } } } as never);
  await onScreen();
  await expect.element(marked()).not.toBeInTheDocument();
});

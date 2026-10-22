// Where a Session's pull request opens, which `links.openPullRequestsIn` decides: Armada's panel by
// default, and the forge in the browser where the setting says so, from the same press.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

async function artifacts() {
  const app = mount("session-pr-panel");
  await onScreen();
  await toSession(/Store migration spike/);
  await userEvent.click(page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Artifacts", exact: true }));
  return { app, pulls: page.getByRole("region", { name: "Artifacts", exact: true }).getByRole("region", { name: "Pull requests", exact: true }) };
}

test("PR place: Armada's panel by default", async () => {
  const { app, pulls } = await artifacts();
  const link = vi.spyOn(app.api, "openLink").mockResolvedValue({ ok: true });
  await userEvent.click(pulls.getByRole("button", { name: /^#2210 / }));
  await expect.element(page.getByRole("dialog", { name: "Pull request #2210" })).toBeVisible();
  expect(link).not.toHaveBeenCalled();
});

test("PR place: the browser where the setting says browser, and no panel", async () => {
  const { app, pulls } = await artifacts();
  const link = vi.spyOn(app.api, "openLink").mockResolvedValue({ ok: true });
  await app.api.saveSettings({ "links.openPullRequestsIn": "browser" });
  await userEvent.click(pulls.getByRole("button", { name: /^#2210 / }));
  await expect.poll(() => link.mock.calls.length).toBe(1);
  expect(link).toHaveBeenLastCalledWith("https://github.com/mele-labs/armada/pull/2210");
  await expect.element(page.getByRole("dialog", { name: "Pull request #2210" })).not.toBeInTheDocument();
});

// Set up with a job, through `App`: the press on the Set up workspaces tab starts a Session in
// the repository being set up, sends it onboard-repo's instruction and Setup's reading, and opens it.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/repository";

import { openSessionIn } from "../open-session";
import { s237SetupWithASession } from "./scenarios/setup-with-a-session";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Set up with a job starts one on the repository, with the reading, and opens it", async () => {
  const { api } = mount(s237SetupWithASession);
  const started = vi.spyOn(api, "startSession");
  const sent = vi.spyOn(api, "sendSessionMessage");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Open the Manifest", exact: true }));
  await userEvent.click(page.getByRole("tab", { name: "Set up workspaces" }));
  await expect.element(page.getByRole("region", { name: "Workspaces" })).toBeVisible();

  await userEvent.click(page.getByRole("button", { name: "Set up with a job" }));

  await expect.poll(() => sent.mock.calls.length).toBe(1);
  expect(started).toHaveBeenCalledWith("Set up armada", repository().root);
  const text = sent.mock.calls[0]![0].text;
  expect(text).toContain("onboard-repo");
  expect(text).toContain("one question at a time");
  for (const line of [
    "- `.` (read package.json, pnpm-lock.yaml): already set up.",
    "- `apps/web` (read apps/web/package.json): ready to write. Checks: test `pnpm test`",
    "- `docs` (read docs/package.json): no checks proposed.",
    "- `test/bats` (no files read): no checks proposed. Nothing here could be read.",
  ]) {
    expect(text).toContain(line);
  }
  // The Session's own page, with the message in its thread.
  await expect.poll(() => openSessionIn(document)).toBe(sent.mock.calls[0]![0].session_id);
  await expect.element(page.getByText("Nothing here could be read.").last()).toBeVisible();
});

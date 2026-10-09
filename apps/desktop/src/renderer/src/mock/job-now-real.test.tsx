// The Now panel on a Job that carries no draft: its rows come from the detail and the Drones Fleet
// serves, so a real Job is never a panel-less one. The walks `job-now-*` hold the draft's rows.

import { describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const now = () => page.getByRole("region", { name: "Now" });

// One name, so the whole file runs as one Check: `armada check app_smoke "job now real"`.
describe("job now real", () => {
  test("a gate running its Checks lists the one that started", async () => {
    mount("job/gateChecksStreaming");
    await onScreen();
    await expect.element(now().getByText("cargo_nextest")).toBeVisible();
    // Queued behind it and not started: not running, so not listed.
    expect(now().element().textContent).not.toContain("cargo_build");
    expect(now().element().textContent).not.toContain("Nothing is actively running");
  });

  test("pressing a running Check opens its log", async () => {
    mount("job/gateChecksStreaming");
    await onScreen();
    await now().getByText("cargo_nextest").click();
    await expect.element(page.getByRole("dialog", { name: /Check log/ })).toBeVisible();
  });

  test("a Drone held on a command is asking, by the step it works", async () => {
    mount("job/runningWaitingOnACommand");
    await onScreen();
    await expect.element(now().getByText("pnpm add", { exact: false })).toBeVisible();
  });

  test("a queued Job waits on the reason Fleet gives, and does not read as idle", async () => {
    mount("job/queued");
    await onScreen();
    await expect.element(now().getByText("Waiting on resources")).toBeVisible();
    expect(now().element().textContent).not.toContain("Nothing is actively running");
  });

  test("a live Job with nothing running says so", async () => {
    mount("job/awaitingRepair");
    await onScreen();
    await expect.element(now().getByText("Nothing is actively running on this job")).toBeVisible();
  });

  test("a Job that is over draws no panel", async () => {
    mount("job/completedSuccess");
    await onScreen();
    await expect.element(page.getByRole("region", { name: "Now" })).not.toBeInTheDocument();
  });
});

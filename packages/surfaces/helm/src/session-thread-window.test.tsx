// A Session that ran for days is thousands of rows. The thread draws the newest items and
// reveals older ones on a press, so typing beside it stays quick.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { SessionThread } from "@armada/components";
import type { SessionThreadRow } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const rows = (count: number): SessionThreadRow[] =>
  Array.from({ length: count }, (_, at) => ({ id: `r${at}`, at: "10:00:00", kind: "message", from: "agent", text: `row ${at} said` }));

const thread = (sessionId: string, count: number) => <SessionThread sessionId={sessionId} rows={rows(count)} onAnswer={() => {}} onOpenSession={() => {}} />;

test("draws the newest 150 rows, and Show earlier adds 150 more", async () => {
  mount(thread("a", 400));

  await expect.element(page.getByText("row 399 said")).toBeVisible();
  expect(document.querySelectorAll(".armada-helm-thread__rows > li").length).toBe(151);
  expect(page.getByText("row 249 said").query()).toBeNull();
  expect(page.getByText("row 250 said").query()).not.toBeNull();

  await page.getByRole("button", { name: "Show earlier" }).click();
  expect(document.querySelectorAll(".armada-helm-thread__rows > li").length).toBe(301);
  expect(page.getByText("row 100 said").query()).not.toBeNull();
});

test("a thread that fits draws no Show earlier", async () => {
  mount(thread("a", 20));

  await expect.element(page.getByText("row 19 said")).toBeVisible();
  expect(page.getByRole("button", { name: "Show earlier" }).query()).toBeNull();
});

// The mock's page asking, which plays the app's: a request waits on the tab that asked, and goes with it where that tab is closed.

import { expect, test, vi } from "vitest";

import { peekJobBrowser, reportTabs, requestPermission, seedJobBrowser } from "./store";

vi.mock("../job-panes/store", () => ({
  jobOpens: vi.fn(),
  look: vi.fn(),
  peek: vi.fn(),
}));
vi.mock("../cockpit/viewed", () => ({
  viewingNow: vi.fn(() => false),
}));

function asking(job: string) {
  seedJobBrowser(job, {
    visited: [
      "http://localhost:5173/cart",
    ],
    page: () => null,
  });
  requestPermission(job, "camera", "t1");
}

test("closing the Browser tab that asked ends the ask, and nothing is kept for the site", () => {
  asking("closed");
  expect(peekJobBrowser("closed")?.site.pending).toEqual({
    permission: "camera",
    host: "localhost:5173",
  });
  reportTabs("closed", []);
  const after = peekJobBrowser("closed");
  expect(after?.site.pending).toBeNull();
  expect(after?.asked).toBeUndefined();
  expect(after?.site.sites).toEqual({});
});

test("closing a tab that did not ask leaves the ask standing", () => {
  asking("other");
  reportTabs("other", [
    "t1",
  ]);
  expect(peekJobBrowser("other")?.site.pending).not.toBeNull();
});

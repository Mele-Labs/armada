// A row re-sorting travels; a new row, an unchanged poll and reduced motion move nothing.
// `useTravel` in `ActiveJobsList`, driven through Overview's lists — the list the app draws, since
// the Job Board went.
//
// **The re-sort is inside one section.** Each section is its own `ActiveJobsList`, so a Job that
// changes section unmounts from one and mounts in the other: that is a new row rather than a
// travel, and it is what the second test already says about a row arriving.

import type { JobSummary } from "@armada/protocol";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { OverviewLists } from "./OverviewLists";
import { job, workflow } from "@armada/screens/src/fixtures/build/base";
import { motion, mount, rerender, unmount } from "@armada/screens/src/mounted";

// Travel is the claim, and reduced motion — which this project runs under — moves nothing, so
// "nothing travelled" would hold whatever `useTravel` did. The test that takes the preference takes
// it back itself.
beforeEach(motion);

const noop = () => {};
let travelled: string[] = [];

beforeEach(() => {
  travelled = [];
  const animate = Element.prototype.animate;
  vi.spyOn(Element.prototype, "animate").mockImplementation(function (this: Element, ...args) {
    const id = (this as HTMLElement).dataset?.jobId;
    if (id !== undefined) travelled.push(id);
    return animate.apply(this, args);
  });
});

afterEach(() => {
  unmount();
  vi.restoreAllMocks();
});

function board(jobs: readonly JobSummary[]): React.ReactElement {
  return (
    <OverviewLists
      jobs={jobs}
      stale={false}
      now={Date.parse("2026-09-10T21:00:00Z")}
      workflows={[workflow()]}
      repositories={[]}
      picked={null}
      disconnected={null}
      selected={null}
      onOpen={noop}
      onKill={noop}
      onRedispatch={noop}
      onClear={noop}
      onCompose={noop}
      onCopied={noop}
    />
  );
}

function drawn(): string[] {
  return Array.from(document.querySelectorAll<HTMLElement>("[data-job-id]"), (row) => row.dataset.jobId ?? "");
}

const older = job("running", { id: "a", title: "Older", created_at: "2026-09-10T14:00:00Z" });
const newer = job("running", { id: "b", title: "Newer", created_at: "2026-09-10T15:00:00Z" });

test("a row that re-sorts inside its section travels to its new place", async () => {
  mount(board([older, newer]));
  await expect.poll(drawn).toEqual(["a", "b"]);

  // Both stay under Running, so this is one list reordering rather than a Job
  // changing section — `newer` becomes the older of the two.
  rerender(board([older, { ...newer, created_at: "2026-09-10T13:00:00Z" }]));
  await expect.poll(drawn).toEqual(["b", "a"]);
  expect(travelled).toContain("b");
});

test("a new row does not travel, and nothing makes room for it by moving", async () => {
  mount(board([older]));
  await expect.poll(drawn).toEqual(["a"]);

  rerender(board([older, job("awaiting_review", { id: "c", title: "Arrived" })]));
  await expect.poll(drawn).toEqual(["c", "a"]);
  expect(travelled).toEqual([]);
});

test("a poll that changes no order moves nothing", async () => {
  mount(board([older, newer]));
  await expect.poll(drawn).toEqual(["a", "b"]);

  rerender(board([{ ...older }, { ...newer }]));
  await new Promise((settle) => setTimeout(settle, 50));
  expect(drawn()).toEqual(["a", "b"]);
  expect(travelled).toEqual([]);
});

test("under reduced motion a re-sort moves nothing", async () => {
  const matchMedia = window.matchMedia.bind(window);
  // A real list that always matches, not `{ matches: true }`: the hold on a
  // row's Kill subscribes to changes, and a bare object has no addEventListener.
  vi.spyOn(window, "matchMedia").mockImplementation((query) =>
    matchMedia(query === "(prefers-reduced-motion: reduce)" ? "all" : query),
  );
  mount(board([older, newer]));
  await expect.poll(drawn).toEqual(["a", "b"]);

  rerender(board([older, { ...newer, status: "awaiting_review" }]));
  await expect.poll(drawn).toEqual(["b", "a"]);
  expect(travelled).toEqual([]);
});

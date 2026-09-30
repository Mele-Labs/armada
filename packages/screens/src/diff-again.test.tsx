// The diff a person opens is a reading taken for the press, not for the Job.
//
// # Why this is a browser test
//
// The call is made by an effect inside `JobDetail`, off the sheet's open state
// and off the live footprint — three things a function test cannot see. What
// this pins is what a person did: they opened a Job, waited while its Drone
// wrote, and opened the diff.
//
// **Opened by `f` since 29 Sep 2026.** The Produced chapter's own `Open the
// diff` control came off with the Overview reframe; `actions.toml`'s
// `open_diff`, scope `detail`, is the route that survived. What is pinned is
// unchanged — `useDiffAgain` is live and this is the only thing reading it.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { JobDetail } from "./JobDetail";
import { running } from "./fixtures/build";
import { propsFor } from "./fixtures/props";
import type { JobFixture } from "./fixtures/fixture";
import { mount, rerender, unmount } from "./mounted";

afterEach(unmount);

/** The same fixture with one more reading of what the Drone has written. */
/**
 * `f` — `open_diff`, scope `detail`. Dispatched on `document`, where
 * `detail-keys` listens (`window`, not the focused element), so it works under
 * fake timers.
 */
async function openDiff(): Promise<void> {
  // The listener is registered in an effect, so the mount has to have settled
  // before the press can land.
  await vi.waitFor(() => expect(document.querySelector(".armada-detail-tab")).not.toBeNull());
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "f", bubbles: true }));
  await vi.waitFor(() => expect(document.querySelector(".armada-sheet")).not.toBeNull());
}

function alsoWrote(fixture: JobFixture, paths: string[]): JobFixture {
  const observed = fixture.observed;
  if (!("turns" in observed)) throw new Error("the fixture is not watching");
  const wrote = {
    ts: "2026-09-10T14:31:00Z",
    seq: 9_000,
    step: "fix",
    by: "fleet" as const,
    saw: {
      event: "produced" as const,
      files: paths.map((path) => ({ path, change: "modified" as const, outside_plan: false })),
    },
  };
  return {
    ...fixture,
    observed: { ...observed, turns: { ...observed.turns, rows: [...observed.turns.rows, wrote] } },
  };
}

test("`f` takes the reading again, and so does the file list moving", async () => {
  const fixture = running();
  const onReadDiff = vi.fn<(jobId: string | null) => void>();
  mount(<JobDetail {...propsFor(fixture)} onReadDiff={onReadDiff} />);

  // Opening the Job reads it once, which is what it has always done.
  await vi.waitFor(() => expect(onReadDiff.mock.calls).toEqual([[fixture.job.id]]));

  // **The press.** Ten minutes of a Drone writing sit between this and the read
  // above, and what the sheet drew was the worktree as it was before any of it.
  onReadDiff.mockClear();
  await userEvent.keyboard("f");
  await expect.element(page.getByText("Job diff")).toBeVisible();
  expect(onReadDiff).toHaveBeenCalledWith(fixture.job.id);

  // And the Drone writes again while the sheet is open, which is the Produced
  // chapter moving under it — one worktree, and now one answer.
  onReadDiff.mockClear();
  rerender(
    <JobDetail {...propsFor(alsoWrote(fixture, ["src/late.ts"]))} onReadDiff={onReadDiff} />,
  );
  await vi.waitFor(() => expect(onReadDiff).toHaveBeenCalledWith(fixture.job.id));
});

test("an open sheet keeps reading while a drone is still writing", async () => {
  vi.useFakeTimers();
  try {
    const fixture = running();
    const onReadDiff = vi.fn<(jobId: string | null) => void>();
    mount(<JobDetail {...propsFor(fixture)} onReadDiff={onReadDiff} />);
    // The press, then the clock. **Fleet republishes a footprint only where the
    // file list changed**, so a Drone editing the same seven files leaves the
    // event silent and the hunks on screen are what this takes again.
    //
    // **Dispatched rather than typed.** The clock is frozen before the mount,
    // because the interval this is about is made there; `userEvent` is driven
    // by the browser and stalls against a frozen clock, which the button this
    // used to press did not expose.
    await openDiff();
    onReadDiff.mockClear();

    await vi.advanceTimersByTimeAsync(11_000);
    expect(onReadDiff.mock.calls.length).toBeGreaterThanOrEqual(2);
    expect(onReadDiff).toHaveBeenCalledWith(fixture.job.id);
  } finally {
    vi.useRealTimers();
  }
});

// **A worktree nobody is writing to cannot go stale.** The clock is the one
// thing here that spends without being asked, so it is bounded by the two
// facts that make a reading able to move: somebody is looking, and a Drone
// holds the pen.
test("and pays nothing on a job with no drone on it", async () => {
  vi.useFakeTimers();
  try {
    const fixture = running();
    // Both readings agree the drone is gone — `JobDetail` now reads
    // `assigned_drone` off the fetched detail once it has arrived, `whole.job`'s
    // own terms, so a fixture that dropped it from the board row alone would
    // still read as a job with a drone.
    const stopped: JobFixture = {
      ...fixture,
      job: { ...fixture.job, assigned_drone: undefined },
      watched:
        fixture.watched.state === "read"
          ? {
              ...fixture.watched,
              detail: {
                ...fixture.watched.detail,
                job: { ...fixture.watched.detail.job, assigned_drone: undefined },
              },
            }
          : fixture.watched,
    };
    const onReadDiff = vi.fn<(jobId: string | null) => void>();
    mount(<JobDetail {...propsFor(stopped)} onReadDiff={onReadDiff} />);
    await userEvent.keyboard("f");
    onReadDiff.mockClear();

    await vi.advanceTimersByTimeAsync(30_000);
    expect(onReadDiff).not.toHaveBeenCalled();
  } finally {
    vi.useRealTimers();
  }
});

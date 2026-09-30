// A job dispatched from a Studio, and the one press back to it, through `App`.
// #1362.
//
// **The defect was navigational, so the test is.** The owner dispatched from a
// Studio, watched the job, and could not get back to the notes and the draft
// it came from — nothing on the job even said a Studio existed. What is pinned
// is the sentence on the header, the named Studio under *Where things are*,
// and that one press leaves the job and lands on that Studio with the job's
// own node picked.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { running } from "@armada/screens/src/fixtures/build/index";
import { watchedRead } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { onJob } from "./scenario";
import type { Scenario } from "./scenario";
import { EVERY_KIND_NAME, EVERY_KIND_STUDIO, everyKind } from "./studio-fleet";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The job's own node on that Studio — `everyKind()` draws it as `every-job`. */
const NODE = "every-job";

/**
 * `running`, dispatched off a Studio: the origin a person's press writes, and
 * the read that names which Studio. `from_studio` absent is the deleted case.
 */
function offAStudio({ still = true }: { still?: boolean } = {}): JobFixture {
  const fixture = running();
  const job = { ...fixture.job, origin: "studio_dispatched" };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  return {
    ...fixture,
    job,
    watched: watchedRead({
      ...fixture.watched.detail,
      job,
      ...(still
        ? { from_studio: { studio_id: EVERY_KIND_STUDIO, name: EVERY_KIND_NAME, node_id: NODE } }
        : {}),
    }),
  };
}

/** That job open, on a Fleet keeping the Studio it names. */
function dispatchedFromAStudio(options?: { still?: boolean }): Scenario {
  const fixture = offAStudio(options);
  const base = onJob(fixture, { whereOpen: true });
  return {
    ...base,
    // This repository picked, because a Studio belongs to one and the surface
    // asks for one on All — the press must land on the whiteboard, not a picker.
    state: { ...base.state, repository: repository().root },
    studios: [everyKind(fixture.job.id)],
  };
}

test("the header says where the job came from and who pressed, in one sentence", async () => {
  mount(dispatchedFromAStudio());
  await expect.element(page.getByText("From a Studio, by you").first()).toBeVisible();
});

// **The route back to the Studio went with *Where things are*** on 29 Sep
// 2026. Two claims stood here: the region naming the Studio beside the
// worktree, and one press landing on that Studio with this Job's node picked.
// The header still says *From a Studio, by you*, which is the claim above —
// what it does not carry is the press, so there is no way back from a Job to
// the canvas it was dispatched from.

test("a job whose Studio has been deleted says so, and offers no press", async () => {
  mount(dispatchedFromAStudio({ still: false }));
  await expect.element(page.getByText("That Studio has been deleted").first()).toBeVisible();
  expect(page.getByRole("button", { name: `Open ${EVERY_KIND_NAME}` }).query()).toBeNull();
});

test("a job nothing dispatched from a Studio says nothing extra", async () => {
  mount(onJob(running(), { whereOpen: true }));
  await expect.element(page.getByRole("button", { name: running().job.handle })).toBeVisible();
  expect(page.getByText("That Studio has been deleted").query()).toBeNull();
  expect(page.getByRole("button", { name: `Open ${EVERY_KIND_NAME}` }).query()).toBeNull();
});

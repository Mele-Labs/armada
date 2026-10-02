// A job dispatched from a Studio, and the one press back to it, through `App`.
// #1362.
//
// **The defect was navigational, so the test is.** The owner dispatched from a
// Studio, watched the job, and could not get back to the notes and the draft
// it came from — nothing on the job even said a Studio existed. What is pinned
// is the sentence on the header, and that one press on it leaves the job and
// lands on that Studio with the job's own node picked — #1674.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { running } from "@armada/screens/src/fixtures/build/index";
import { watchedRead } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { originReading } from "@armada/screens/src/origin";

import { onJob } from "./scenario";
import type { Scenario } from "./scenario";
import { EVERY_KIND_NAME, EVERY_KIND_STUDIO, everyKind } from "./studio-fleet";
import { mount, openHelm, unmountAfterEach } from "./testing";

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

/** The header's origin sentence, as a control. Exact: the Board's row says it too, as text. */
const fromTheHeader = () => page.getByRole("button", { name: "From a Studio, by you", exact: true });

// **The route back went with *Where things are*** on 29 Sep 2026, and #1674
// put it on the header's own sentence: it already said where the Job came
// from, so pressing it goes there. The tooltip names the Studio, because the
// registry's sentence has no slot for it.
test("the header's sentence names that Studio on its tooltip", async () => {
  mount(dispatchedFromAStudio());
  await expect.element(fromTheHeader()).toHaveAccessibleDescription(`Open ${EVERY_KIND_NAME}`);
});

test("one press on the header's sentence leaves the job and lands on that Studio, with the job's node picked", async () => {
  mount(dispatchedFromAStudio());
  await fromTheHeader().click();
  await openHelm();
  // **Helm's footer is what says where a person is**, and it names the node as
  // well as the Studio — the same reading `studios.test.tsx` asserts a pick by.
  // The job itself is gone: `goTo` clears it the way every destination does.
  await expect
    .element(page.getByText(`Studios · ${EVERY_KIND_NAME} · Job ${running().job.title} selected`))
    .toBeVisible();
});

/** Overview's Brief card. */
const theBrief = () => page.getByRole("region", { name: "Brief", exact: true });

/** The Studio's name in the head of Overview's Brief card, as a control. */
const fromTheBrief = () => theBrief().getByRole("button", { name: EVERY_KIND_NAME });

/**
 * Overview's Brief once the Job's own read has drawn it — the moment a Studio
 * could be named there, so an absence read before it would prove nothing.
 */
async function briefRead(): Promise<void> {
  await expect.element(theBrief().getByText("selectColumnOrder")).toBeVisible();
}

// The owner's second place, 2 Oct 2026: the Brief card names the Studio, in
// one line and with one press, and the press is the header's.
test("one press on the Studio named in the Brief leaves the job and lands on that Studio, with the job's node picked", async () => {
  mount(dispatchedFromAStudio());
  await expect.element(fromTheBrief()).toHaveAccessibleDescription(`Open ${EVERY_KIND_NAME}`);
  await fromTheBrief().click();
  await openHelm();
  await expect
    .element(page.getByText(`Studios · ${EVERY_KIND_NAME} · Job ${running().job.title} selected`))
    .toBeVisible();
});

test("a job whose Studio has been deleted says so, and offers no press", async () => {
  mount(dispatchedFromAStudio({ still: false }));
  await expect.element(page.getByText("That Studio has been deleted").first()).toBeVisible();
  expect(fromTheHeader().query()).toBeNull();
  await briefRead();
  expect(theBrief().getByRole("button").query()).toBeNull();
});

test("a job nothing dispatched from a Studio says nothing extra, and draws no way to one", async () => {
  mount(onJob(running(), { whereOpen: true }));
  await expect.element(page.getByRole("button", { name: running().job.handle })).toBeVisible();
  expect(page.getByText("That Studio has been deleted").query()).toBeNull();
  // Its own origin sentence is drawn, and is not a press: there is no Studio behind it.
  const said = originReading(running().job)!;
  await expect.element(page.getByText(said, { exact: true }).first()).toBeVisible();
  expect(page.getByRole("button", { name: said, exact: true }).query()).toBeNull();
  // And the Brief's head draws nothing in the Studio's place.
  await briefRead();
  expect(theBrief().getByRole("button").query()).toBeNull();
});

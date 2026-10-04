// The Brief on Overview, through `App`, on a Job whose request is an issue
// body Fleet pasted in. The owner's Job 1 of 1 Oct 2026 drew `## What
// happened`, its inline code and its line breaks as one run-on paragraph.
// `Prose` is the renderer the Judge's words already use, so the Brief reads
// the request the same way. On the canvas's Brief node since the canvas became
// the whole Overview (the owner, 4 Oct 2026).

import { expect, test } from "vitest";
import { running } from "@armada/screens/src/fixtures/build/index";
import { watchedRead } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { onJob } from "./scenario";
import { mount, openNode, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The owner's own request, cut to the three constructs it was flattened on. */
const FACTS = [
  "Guide 8 is in the catalogue and openable from Guides, and nothing on a Job opens it.",
  "",
  "## What happened",
  "",
  "`GuideMark guide={GUIDE_STEP_BAR}` is drawn in `packages/screens/src/InsideAJob.tsx:379`.",
].join("\n");

function withFacts(fixture: JobFixture, facts: string): JobFixture {
  if (fixture.watched.state !== "read") throw new Error("the fixture is read");
  return { ...fixture, watched: watchedRead({ ...fixture.watched.detail, facts }) };
}

test("the Brief draws a request's heading, paragraphs and code as themselves", async () => {
  mount(onJob(withFacts(running(), FACTS)));
  const card = await openNode("Brief");
  await expect.element(card.getByText("What happened", { exact: true })).toBeVisible();
  // The request's own prose: the card holds Done when's criteria as prose too.
  const brief = (card.getByText("What happened", { exact: true }).element() as HTMLElement).closest(".armada-prose");
  expect(brief?.querySelector(".armada-prose__said")?.textContent).toBe("What happened");
  expect(brief?.querySelectorAll(".armada-prose__paragraph")).toHaveLength(2);
  const code = [...(brief?.querySelectorAll("code") ?? [])].map((one) => one.textContent);
  expect(code).toEqual(["GuideMark guide={GUIDE_STEP_BAR}", "packages/screens/src/InsideAJob.tsx:379"]);
});

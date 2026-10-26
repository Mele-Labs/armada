// A Job's document is a panel floating over the Job, not a layer covering it: Contents and the Notes stand as two cards, one under the
// other at the left, and the page takes the rest of the width. Read by where each lands on screen, as a person sees it. Through `App`
// over the `job-story` scenario, played by the walk `job-document` to the press that opens the report.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "../mock/testing";
import { walkThrough } from "../mock/walk";
import { WALKS } from "../mock/walks";
import { LEASE_DOC_TITLE } from "../mock/document-pages";

unmountAfterEach();

async function opened(): Promise<void> {
  const script = WALKS.get("job-document")!;
  mount(script.scenario);
  await onScreen();
  await walkThrough(script.steps.slice(0, script.steps.findIndex((one) => one.say.startsWith("A press opens the page")) + 1));
}

test("the document floats over the Job, with Contents over the Notes at its left and the page beside them", async () => {
  await opened();
  const reading = page.getByRole("region", { name: "Document", exact: true });
  const contents = reading.getByRole("navigation", { name: "Contents" });
  const notes = reading.getByRole("complementary", { name: "Notes on the page" });
  const article = reading.getByRole("article", { name: LEASE_DOC_TITLE, exact: true });
  await expect.element(contents).toBeVisible();
  await expect.element(notes).toBeVisible();
  await expect.element(article).toBeVisible();

  // Held off the window's edges by a floating sheet's margin, with the Job showing to its left.
  const panel = (reading.element().closest('[role="dialog"]') ?? reading.element()).getBoundingClientRect();
  const margin = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--space-4"));
  const job = page.getByRole("region", { name: /^Job: / }).element().getBoundingClientRect();
  expect.soft({ right: window.innerWidth - panel.right >= margin, bottom: window.innerHeight - panel.bottom >= margin, jobShows: job.left < panel.left }, "the panel floats over the Job rather than covering it").toEqual({ right: true, bottom: true, jobShows: true });

  const card = contents.element().getBoundingClientRect();
  const under = notes.element().getBoundingClientRect();
  expect.soft(under.top, "the Notes card's top is below the Contents card's bottom").toBeGreaterThanOrEqual(card.bottom);
  expect.soft(under.left, "the Notes card's left edge is the Contents card's").toBe(card.left);

  const words = article.element().getBoundingClientRect();
  expect.soft(words.left, "the page starts right of both cards").toBeGreaterThanOrEqual(Math.max(card.right, under.right));
}, 60_000);

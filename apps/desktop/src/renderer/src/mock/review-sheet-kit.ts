// What a browser test needs to reach a Job's review sheet the way a person does. Questions, commands and pull requests are rows of the Needs you card; what he reads
// (a plan, sketch, draft, document) is one Review row on it, or the band's own press where the Job has nothing else. The sheet lists those items, and a row's press picks
// the item whose body the right side draws.

import { expect } from "vitest";
import { page, userEvent } from "vitest/browser";

/** The band on the composer's top edge. */
export const band = () =>
  page.getByRole("region", {
    name: "Needs you",
    exact: true,
  });

/** The press on the band: it folds the card, or opens the sheet where the Job has only review items. */
export const bandKey = () =>
  band().getByRole("button", {
    name: "Needs you",
    exact: true,
  });

/** The one row of the card for what he reads, under "To review". */
export const reviewRow = () =>
  band().getByRole("button", {
    name: "Review",
    exact: true,
  });

/** The sheet, named by its Job. */
export const sheet = (job: string | RegExp) =>
  page.getByRole("dialog", {
    name: job,
    exact: typeof job === "string",
  });

/** The list of everything to review, in the sheet. */
export const list = (job: string | RegExp) =>
  sheet(job).getByRole("group", {
    name: "Waiting on you",
  });

/** One row of the list, by the words of its item. */
export const row = (job: string | RegExp, name: string | RegExp) =>
  list(job).getByRole("button", {
    name,
  });

/** The body drawn for the picked item: a region named for it. */
export const detail = (job: string | RegExp, name: string | RegExp) =>
  sheet(job).getByRole("region", {
    name,
  });

/** The names of the rows, in order, as the list reads them. */
export const names = (job: string | RegExp): string[] =>
  [
    ...list(job).element().querySelectorAll(".armada-job-review__name"),
  ].map((one) => one.textContent ?? "");

/** Opens the sheet through the card's Review row, pressing the band first when the card is folded, and waits for the sheet to be drawn. */
export async function openSheet(job: string | RegExp) {
  if (reviewRow().query() === null) await userEvent.click(bandKey());
  await userEvent.click(reviewRow());
  await expect.element(sheet(job)).toBeVisible();
}

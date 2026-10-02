// The pull request card at Job 2's review gate, through `App` and nothing else.
//
// Its title and its comment count are Fleet's, off `delivery` since protocol
// 23.5. Where no title is kept, the live read's stands in. **Absent draws nothing**: no stand-in title and no count of none.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const TITLE = "Retire guides 8 and 20, add validation that every guide's piece is drawn somewhere";

describe("the pull request card", () => {
  test("real/job-2-at-review: a 23.5 Fleet's title and comment count are on the card", async () => {
    mount("real/job-2-at-review");
    const card = page.getByRole("link", { name: `Pull request #1750, ${TITLE}` });

    await expect.element(card).toBeVisible();
    await expect.element(card.getByText(TITLE)).toBeVisible();
    await expect.element(card.getByText("0 comments")).toBeVisible();
  });

  test(
    "real/job-2-at-review-before-23-5: a pull request no read has named draws no title and " +
      "no count, and nothing in their place",
    async () => {
      mount("real/job-2-at-review-before-23-5");
      const card = page.getByRole("link", { name: "Pull request #1750", exact: true });

      await expect.element(card).toBeVisible();
      // The review's own comments have been read, so a count taken from them would be drawn by now.
      await expect.element(page.getByRole("region", { name: "Comments on the pull request" })).toBeVisible();
      expect(card.element().textContent).not.toMatch(/comment/);
      expect(card.element().textContent).not.toContain("Retire guides");
    },
  );

  test("real/job-2-at-review-live-title: with no title kept, the live read's title is on the card", async () => {
    mount("real/job-2-at-review-live-title");
    const card = page.getByRole("link", { name: `Pull request #1750, ${TITLE}` });

    await expect.element(card).toBeVisible();
    await expect.element(card.getByText(TITLE)).toBeVisible();
  });
});

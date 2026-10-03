// The pull request card at Job 2's review gate, through `App` and nothing else.
//
// Its title and its comment count are Fleet's, off `delivery` since protocol
// 23.5. Where no title is kept, the live read's stands in. **Absent draws nothing**: no stand-in title and no count of none.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import type { BridgeApi } from "../../../shared/api";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * Resolves once Fleet's answer to the gate's comments read has been served.
 * Wrapped before the first render, because `window.armada` is this object and
 * the read goes out from an effect.
 */
function remarksAnswered(api: BridgeApi): Promise<void> {
  const read = api.readRemarks;
  return new Promise((resolve) => {
    api.readRemarks = async (jobId) => {
      await read(jobId);
      if (jobId !== null) resolve();
    };
  });
}

/** Two frames: whatever the answer changed has been committed and painted. */
async function drawn(): Promise<void> {
  for (let frame = 0; frame < 2; frame++) await new Promise((done) => requestAnimationFrame(done));
}

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
      const app = mount("real/job-2-at-review-before-23-5");
      const answered = remarksAnswered(app.api);
      const card = page.getByRole("link", { name: "Pull request #1750", exact: true });

      await expect.element(card).toBeVisible();
      // The review's own comments have been read and drawn, so a count taken
      // from them — #1750 had none, which would read `0 comments` — is on the
      // card by now if it is ever going to be.
      await answered;
      await drawn();
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

// Focus through `App`, on the `pr-focus-review` scenario: lines picked in its gutter go into his pending review, the same one
// the Review tab lists, and nothing reaches the forge until he submits it.

import type { WritePullRequest } from "@armada/protocol";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { sPrFocusReview } from "./scenarios/pr-focus-review";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const reader = () =>
  page.getByRole("region", {
    name: "Pull request #2417",
  });
const focusPane = () =>
  reader().getByRole("region", {
    name: "Pane: Focus",
  });

async function comment(line: number, words: string) {
  await focusPane()
    .getByRole("button", {
      name: new RegExp(`^Comment on line ${line}: `),
    })
    .click();
  await focusPane()
    .getByRole("textbox", {
      name: "Comment",
    })
    .fill(words);
}

test("Add to review puts a second comment in the pending review, and sends nothing", async () => {
  const app = mount(sPrFocusReview);
  const wrote: WritePullRequest[] = [];
  const write = app.api.writeReview;
  app.api.writeReview = async (manifestId, one) => {
    wrote.push(one);
    return write(manifestId, one);
  };
  await onScreen();
  await page
    .getByRole("navigation", {
      name: "Places",
    })
    .getByRole("tab", {
      name: "Reviews",
    })
    .click();
  await page
    .getByRole("region", {
      name: "Pull requests",
    })
    .getByRole("button", {
      name: /^#2417 /,
    })
    .click();
  await reader()
    .getByRole("region", {
      name: "Pane: Overview",
    })
    .getByRole("button", {
      name: "Add tab",
    })
    .click();
  await page
    .getByRole("menuitem", {
      name: "Focus",
    })
    .click();

  await comment(8, "Lock the payment row before reading what is left.");
  await focusPane()
    .getByRole("button", {
      name: "Start a review",
    })
    .click();
  await comment(15, "Pass an idempotency key, so a retry cannot refund twice.");
  await focusPane()
    .getByRole("button", {
      name: "Add to review (1)",
    })
    .click();

  // The Review tab reads the same pending review.
  await reader()
    .getByRole("tab", {
      name: "Review",
      exact: true,
    })
    .click();
  const pending = page
    .getByRole("region", {
      name: "Your review",
    })
    .getByRole("list", {
      name: "Pending comments",
    });
  await expect
    .element(
      pending.getByRole("button", {
        name: "Go to your comment on app/services/refund_issuer.rb:15",
      }),
    )
    .toBeVisible();
  await expect.element(pending.getByRole("listitem").nth(1)).toBeInTheDocument();
  expect(wrote).toEqual([]);
});

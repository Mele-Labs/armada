// The Review tab through `App`, on the `pr-review-by-file` scenario: lines picked in its gutter go into his pending review, which
// its submit bar lists, and nothing reaches the forge until he submits it. A layout kept while the tab was called Focus opens it.

import type { WritePullRequest } from "@armada/protocol";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { sPrReviewByFile } from "./scenarios/pr-review-by-file";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const reader = () =>
  page.getByRole("region", {
    name: "Pull request #2417",
  });
const reviewPane = () =>
  reader().getByRole("region", {
    name: "Pane: Review",
  });

async function openThePullRequest() {
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
}

/** Opening Review asks which tools to run. Run, and Review steps by Tour's stops, Next to the issuer's, as the walk does. */
async function runTheTools() {
  await page
    .getByRole("dialog", {
      name: "Run on opening",
    })
    .getByRole("button", {
      name: "Run",
      exact: true,
    })
    .click();
  await expect.element(reviewPane().getByText("Stop 1 of 6")).toBeVisible();
  await reviewPane()
    .getByRole("button", {
      name: "Next stop",
    })
    .click();
  await expect.element(reviewPane().getByText("Stop 2 of 6")).toBeVisible();
}

async function comment(line: number, words: string) {
  await reviewPane()
    .getByRole("button", {
      name: new RegExp(`^Comment on line ${line}: `),
    })
    .click();
  await reviewPane()
    .getByRole("textbox", {
      name: "Comment",
    })
    .fill(words);
}

test("Add to review puts a second comment in the pending review, and sends nothing", async () => {
  const app = mount(sPrReviewByFile);
  const wrote: WritePullRequest[] = [];
  const write = app.api.writeReview;
  app.api.writeReview = async (manifestId, one) => {
    wrote.push(one);
    return write(manifestId, one);
  };
  await openThePullRequest();
  await reader()
    .getByRole("tab", {
      name: "Review",
      exact: true,
    })
    .click();
  await runTheTools();

  await comment(8, "Lock the payment row before reading what is left.");
  await reviewPane()
    .getByRole("button", {
      name: "Start a review",
    })
    .click();
  await comment(15, "Pass an idempotency key, so a retry cannot refund twice.");
  await reviewPane()
    .getByRole("button", {
      name: "Add to review (1)",
    })
    .click();

  // The submit bar lists the same pending review.
  await reviewPane()
    .getByRole("button", {
      name: "Submit review",
    })
    .click();
  const pending = reviewPane()
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

// A pull request's panes kept before Focus became Review name the kind `pr-focus`, which no tab has any more.
test("a layout kept with a Focus tab opens the Review tab in its place", async () => {
  // After the mount, which starts the review surface with nothing kept.
  mount(sPrReviewByFile);
  localStorage.setItem(
    "armada:kept:pr-panes:pr:fernhill/tally#2417",
    JSON.stringify({
      arrangement: {
        root: {
          type: "split",
          id: "s0",
          dir: "row",
          ratio: 0.62,
          a: {
            type: "pane",
            id: "p1",
            tabs: [
              {
                id: "t3",
                kind: "pr-overview",
              },
              {
                id: "t4",
                kind: "pr-focus",
              },
            ],
            active: "t4",
          },
          b: {
            type: "pane",
            id: "p2",
            tabs: [],
          },
        },
        focus: "p1",
        next: 5,
      },
      layout: "Default",
      edited: true,
      chosen: true,
      version: 0,
    }),
  );
  await openThePullRequest();
  await expect
    .element(
      reviewPane().getByRole("group", {
        name: "Files",
      }),
    )
    .toBeVisible();
  // The layout is the one kept, not Default: Ask is not docked under it.
  await expect
    .element(
      reader().getByRole("tab", {
        name: "Ask",
      }),
    )
    .not.toBeInTheDocument();
});

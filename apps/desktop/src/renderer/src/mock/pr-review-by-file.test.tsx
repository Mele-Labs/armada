// The Review tab through `App`, on the `pr-review-by-file` scenario: lines picked in its gutter, or a finding's fix taken as a
// suggestion, go into his pending review, which its submit bar lists, and nothing reaches the forge until he submits it. A
// layout kept while the tab was called Focus opens it.

import type { WritePullRequest } from "@armada/protocol";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { refunds } from "./pr-review-by-file-pages";
import { fixResult } from "./review-run-fixtures";
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

/**
 * Run, then the issuer's stop, whichever stop Review opens on: the stop read is kept for the window, so a test after
 * `runTheTools` opens on the second.
 */
async function toTheIssuer() {
  await page
    .getByRole("dialog", {
      name: "Run on opening",
    })
    .getByRole("button", {
      name: "Run",
      exact: true,
    })
    .click();
  await expect.element(reviewPane().getByText(/^Stop \d of 6$/)).toBeVisible();
  const issuer = reviewPane().getByText("Stop 2 of 6");
  if (issuer.query() === null)
    await reviewPane()
      .getByRole("button", {
        name: "Next stop",
      })
      .click();
  await expect.element(issuer).toBeVisible();
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

test("the fixture's fix replaces the page's own lines at the finding's place", () => {
  const result = fixResult(refunds.pr, {
    title: "Two refunds can pass the same check",
    body: "",
    severity: "high",
    at: {
      path: "app/services/refund_issuer.rb",
      side: "right",
      start: 8,
      end: 10,
    },
  });
  expect(result).toMatchObject({
    shape: "fix",
    before: [
      "    raise Refund::TooLarge unless @payment.refundable?(amount_cents)",
      "",
      '    refund = @payment.refunds.create!(amount_cents: amount_cents, state: "pending")',
    ].join("\n"),
  });
});

test("a fix asked for a finding is drawn as its lines before and after, and Add as suggestion posts it on the finding's range", async () => {
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
  await toTheIssuer();

  const findings = reviewPane().getByRole("complementary", {
    name: "Findings",
  });
  await findings
    .getByRole("button", {
      name: "Ask for a fix: Two refunds can pass the same check",
    })
    .click();
  await page
    .getByRole("dialog", {
      name: "Ask for a fix",
    })
    .getByRole("button", {
      name: "Run",
      exact: true,
    })
    .click();
  const fix = findings.getByRole("region", {
    name: "Fix for Two refunds can pass the same check",
  });
  const lines = fix.getByRole("group", {
    name: "app/services/refund_issuer.rb",
  });
  await expect
    .element(lines, {
      timeout: 6000,
    })
    .toBeVisible();
  await expect.element(lines).toHaveTextContent("@@ -8,3 +8,5 @@");
  await expect.element(lines).toHaveTextContent("@payment.with_lock do");

  await fix
    .getByRole("button", {
      name: "Add as suggestion: Two refunds can pass the same check",
    })
    .click();
  const suggestion = [
    "```suggestion",
    "    refund = @payment.with_lock do",
    "      raise Refund::TooLarge unless @payment.refundable?(amount_cents)",
    "",
    '      @payment.refunds.create!(amount_cents: amount_cents, state: "pending")',
    "    end",
    "```",
  ].join("\n");
  await expect
    .element(
      reviewPane().getByRole("textbox", {
        name: "Comment",
      }),
    )
    .toHaveValue(suggestion);
  await reviewPane()
    .getByRole("button", {
      name: "Start a review",
    })
    .click();
  await expect.element(reviewPane().getByText("Lines 8 to 10")).toBeVisible();
  expect(wrote).toEqual([]);

  await reviewPane()
    .getByRole("button", {
      name: "Submit review",
    })
    .click();
  const yours = reviewPane().getByRole("region", {
    name: "Your review",
  });
  await yours
    .getByRole("tab", {
      name: "Approve",
    })
    .click();
  await yours
    .getByRole("button", {
      name: "Submit review",
    })
    .click();
  // Findings with no answer are said once, and the second press sends.
  await expect.element(yours.getByRole("alert")).toBeVisible();
  await yours
    .getByRole("button", {
      name: "Submit anyway",
    })
    .click();
  await expect.poll(() => wrote.length).toBe(1);
  expect(wrote[0]!.comments).toEqual([
    {
      kind: "line",
      path: "app/services/refund_issuer.rb",
      side: "right",
      start_line: 8,
      line: 10,
      body: suggestion,
    },
  ]);
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

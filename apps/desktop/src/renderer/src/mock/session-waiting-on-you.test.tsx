// A Session that waits on the person, through `App`: its row in the Cockpit's list has a hand, the band at the
// foot of its thread opens to a list, the question first, the page item is a walk and so is the Review row, which opens the sheet that plays it,
// and the question's own row opens the card that answers it.
// The walk `session-waiting-on-you` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { detail, reviewRow, sheet } from "./review-sheet-kit";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Waiting on you: row hand, the band opens, the page item is on the Review row, the card answers", async () => {
  mount("session-waiting-on-you");
  await onScreen();
  const list = page.getByRole("region", {
    name: "Jobs",
    exact: true,
  });
  await expect
    .element(
      list.getByRole("img", {
        name: /^Waiting on you/,
      }),
    )
    .toBeVisible();
  await toSession(/Pin the store clock/);

  const band = page.getByRole("region", {
    name: "Needs you",
  });
  // The panel is open on the question when he comes in. The question is the first row of the list, and the page item is a walk, so it is the Review row and not a row of its own.
  await expect
    .element(
      band.getByRole("button", {
        name: "Open Which clock?",
      }),
    )
    .toBeVisible();
  expect(
    band
      .getByRole("button", {
        name: "Open Look at the findings page",
      })
      .query(),
  ).toBeNull();
  await userEvent.click(reviewRow());
  await expect.element(sheet("Pin the store clock")).toBeVisible();
  await expect
    .element(
      detail("Pin the store clock", "Look at the findings page").getByRole("button", {
        name: /^Approve/,
      }),
    )
    .toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(sheet("Pin the store clock")).not.toBeInTheDocument();
  await expect
    .element(
      page.getByRole("tab", {
        name: "Look at the findings page",
      }),
    )
    .toBeVisible();

  await userEvent.click(
    band.getByRole("button", {
      name: "Open Which clock?",
    }),
  );
  const card = page.getByRole("article", {
    name: "Waiting on you",
  });
  await userEvent.click(
    card.getByRole("radio", {
      name: "Frozen",
    }),
  );
  await userEvent.click(
    card.getByRole("button", {
      name: "Answer",
    }),
  );
  await expect
    .element(
      page
        .getByRole("region", {
          name: "Thread",
        })
        .getByText("Going with Frozen."),
    )
    .toBeVisible();
  await expect.element(card).not.toBeInTheDocument();
});

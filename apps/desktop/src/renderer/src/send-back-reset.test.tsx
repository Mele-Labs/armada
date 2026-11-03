// A sketch sent back is the same card when the Job shows its next revision, so the note he sent must not still be open on it.
// `ApprovalAnswers` in `@armada/components`, mounted in a browser because the box and the keys are the claim.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { ApprovalAnswers, forgetAllKept } from "@armada/components";
import { mount, rerender, unmount } from "@armada/screens/src/mounted";

beforeEach(forgetAllKept);
afterEach(unmount);

test("after Send back, the next revision asks with the note box shut and empty", async () => {
  const onAnswer = vi.fn();
  const card = () => <ApprovalAnswers id="s1" keep="sketch:s1" open onAnswer={onAnswer} />;
  mount(card());

  await userEvent.click(
    page.getByRole("button", {
      name: "Send back",
    }),
  );
  await userEvent.type(
    page.getByRole("textbox", {
      name: "Note",
    }),
    "Make the arrow clearer",
  );
  await userEvent.click(
    page.getByRole("button", {
      name: "Send back",
    }),
  );
  expect(onAnswer).toHaveBeenCalledWith("s1", {
    text: "Make the arrow clearer",
  });

  // The Job shows revision 2: the same card, asking again.
  rerender(card());

  await expect
    .element(
      page.getByRole("button", {
        name: "Approve",
      }),
    )
    .toBeVisible();
  expect(
    page
      .getByRole("textbox", {
        name: "Note",
      })
      .query(),
  ).toBeNull();
});

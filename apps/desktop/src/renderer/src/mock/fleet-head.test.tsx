// Fleet's state, in the left column's panel head — the owner, 28 Sep 2026:
// *"Delete this row and just put the status dot next to the 'Fleet' title in
// the panel header."* Through `App`, because the claim is about what the
// column draws beside a running window rather than about one component's props.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

/** The panel's head, named by its label and the state its dot now carries. */
const head = () => page.getByRole("button", { name: "Fleet Running", exact: true });

unmountAfterEach();

test("the dot sits inside the Fleet panel's head, and the head says the state", async () => {
  mount("every-state");
  await onScreen();

  await expect.element(head()).toBeVisible();
  // The panel's own dot, not the title row's, which carries the whole sentence
  // because it stands there with no label beside it.
  const dot = head().getByRole("img", { name: "Running", exact: true });
  await expect.element(dot).toBeVisible();
  expect(dot.element().closest(".armada-panel__head")).not.toBeNull();
});

test("the state row under the head is gone, and the figures it sat above are not", async () => {
  mount("every-state");
  await onScreen();

  // Open by default, so nothing is pressed: pid and port still read. The row
  // that went said in words what the dot already says in colour, and nothing
  // else left with it.
  await expect.element(page.getByText("pid")).toBeVisible();
  await expect.element(page.getByText("port").first()).toBeVisible();
  expect(document.querySelector(".armada-fleet-panel__state")).toBeNull();
});

test("a Fleet with no figure, no sentence and no Doctor reading draws its head and nothing to press", async () => {
  // `fleet-not-running` names a runtime file, so it has a sentence and a body.
  // The bodyless case is `reading` — Bridge before it has read the file at all.
  mount("first-launch");
  await onScreen();
  await expect.element(page.getByRole("button", { name: /^Fleet/ })).toBeVisible();
});

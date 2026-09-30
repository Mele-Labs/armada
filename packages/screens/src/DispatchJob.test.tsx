// One request out of this surface, and one way through it.
//
// # Why the guard is tested and not just written
//
// There is no in-flight guard on `proposeFromRequest`, matching `proposeJob`.
// Two presses are two model calls and two drafted plans — two of everything at
// the gate, and somebody deleting one by hand.
//
// **The press leaves the composer** since 30 Sep 2026, so the app unmounting
// this card is most of what stops a second one. The ref is the rest, and it is
// what these press: the unmount is a render, and a key repeat or a synthetic
// click in the same task reaches the handler with the button still there.
//
// **The wait, the answer and both refusals are not here any more.** They are
// `ProposalPage`'s, in `packages/components`, where its own stories agree them.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { DispatchJob } from "./DispatchJob";
import type { DispatchJobProps } from "./DispatchJob";
import { mount, unmount } from "./mounted";

afterEach(unmount);

/** What the form is holding when the guard has to work. */
const REQUEST = "The board flickers every time an event lands.";

/**
 * What this screen is handed beside the request: where the work starts, and
 * what the settings block may offer.
 */
const HELD = {
  landing: {
    target: "main",
    from_ref: "main",
    prs: "job",
    branching: "job",
    pr_mode: "ready",
    complete_when: "delivered",
    land_together: [],
  },
  // Nothing has listed the repository's branches either, which is every
  // dispatch against a real Fleet — so the two ref fields draw plain.
  branches: null,
  workflows: [],
  models: [],
  machineCap: null,
} satisfies Partial<DispatchJobProps>;

/** Mount it, and hand back every request that reached the caller. */
function opened(): { sent: string[] } {
  const sent: string[] = [];
  mount(
    <DispatchJob
      onPropose={(request) => sent.push(request)}
      onStage={() => Promise.resolve({ path: "/tmp/staged" })}
      onSearchFiles={() => Promise.resolve([])}
      {...HELD}
      disabled={false}
    />,
  );
  return { sent };
}

function field() {
  return page.getByRole("textbox", { name: "Request" });
}

const dispatch = () => page.getByRole("button", { name: "Dispatch", exact: true });

test("two presses in one task are one call", async () => {
  const { sent } = opened();
  await userEvent.fill(field(), REQUEST);

  // **Both presses in one task, on purpose.** React has not re-rendered between
  // them and the app has not had a chance to unmount this card, so the second
  // reaches the handler with the button still enabled in the DOM — the whole
  // case the ref exists for, and one a click helper that waits for the control
  // to settle can never produce.
  const button = dispatch().element() as HTMLButtonElement;
  button.click();
  button.click();

  expect(sent, "a second press fired a second model call").toEqual([REQUEST]);
});

/**
 * And it does not release. **The card is what the app takes away**, so a second
 * press a render later is still the same request — where before the guard let go
 * on the answer, because the answer was drawn here.
 */
test("a press a render later is still one call", async () => {
  const { sent } = opened();
  await userEvent.fill(field(), REQUEST);
  await userEvent.click(dispatch());
  expect(sent).toEqual([REQUEST]);

  await userEvent.click(dispatch());
  expect(sent, "the surface sent the same request twice").toEqual([REQUEST]);
});

/** Nothing is sent for a field holding only spaces, by the button or otherwise. */
test("whitespace is not a request", async () => {
  const { sent } = opened();
  await userEvent.fill(field(), "   \t ");
  await expect.element(dispatch()).toBeDisabled();
  expect(sent).toEqual([]);
});

/**
 * There is one way through this surface and no way off it.
 *
 * **The owner took hand entry out on 2026-09-23**, once Settings carried every
 * decision the form did, and the footer has carried one control since.
 */
test("nothing offers a second way to make a Job", async () => {
  opened();
  await userEvent.fill(field(), REQUEST);

  await expect.element(page.getByRole("button", { name: "Enter by hand" })).not.toBeInTheDocument();
  await expect
    .element(page.getByRole("button", { name: "Describe the work instead" }))
    .not.toBeInTheDocument();
  // Live rather than pending: the press leaves the card, so there is no state of
  // it in which Fleet has been asked and has not answered.
  await expect.element(dispatch()).toBeEnabled();
  await expect
    .element(page.getByRole("button", { name: "Reading the request" }))
    .not.toBeInTheDocument();
});

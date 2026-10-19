// A time on screen is a button: it reads as a clock, and a press reads it as an age. `Stamp` and
// `StampSurface` in `@armada/components`, mounted in a browser because focus and the keys are the claim.
//
// Stamps are told apart by how long ago they are, so a test says which one turned.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { DEFAULT_TIME_FORMAT, Stamp, StampSurface, setTimeFormat } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

const MINUTE = 60_000;

beforeEach(() => setTimeFormat(DEFAULT_TIME_FORMAT));
afterEach(unmount);

/** Four minutes ago, and two hours ago: each reads differently as a clock and as an age. */
const FOUR_MINUTES = () => Date.now() - 4 * MINUTE - 10_000;
const TWO_HOURS = () => Date.now() - 120 * MINUTE - 10_000;

const CLOCK = /^\d{2}:\d{2}:\d{2}$/;

test("a stamp reads as the clock, and as the age when it is asked to start there", async () => {
  mount(
    <>
      <Stamp at={FOUR_MINUTES()} />
      <Stamp at={TWO_HOURS()} first="age" />
    </>,
  );

  await expect.element(page.getByRole("button").nth(0)).toHaveAccessibleName(CLOCK);
  await expect.element(page.getByRole("button").nth(1)).toHaveAccessibleName("2h ago");
});

test("a stamp takes an RFC 3339 instant as well as milliseconds", async () => {
  mount(<Stamp at={new Date(FOUR_MINUTES()).toISOString()} first="age" />);

  await expect.element(page.getByRole("button")).toHaveAccessibleName("4m ago");
});

test("a press turns the clock into the age and a second press turns it back", async () => {
  mount(<Stamp at={FOUR_MINUTES()} />);
  const stamp = page.getByRole("button");

  await userEvent.click(stamp);
  await expect.element(stamp).toHaveAccessibleName("4m ago");

  await userEvent.click(stamp);
  await expect.element(stamp).toHaveAccessibleName(CLOCK);
});

test("a press turns an age that began as one into the clock", async () => {
  mount(<Stamp at={FOUR_MINUTES()} first="age" />);
  const stamp = page.getByRole("button");

  await expect.element(stamp).toHaveAccessibleName("4m ago");
  await userEvent.click(stamp);
  await expect.element(stamp).toHaveAccessibleName(CLOCK);
});

test("Enter on the focused stamp turns it", async () => {
  mount(<Stamp at={FOUR_MINUTES()} />);
  const stamp = page.getByRole("button");

  await userEvent.tab();
  await expect.element(stamp).toHaveFocus();
  await userEvent.keyboard("{Enter}");

  await expect.element(stamp).toHaveAccessibleName("4m ago");
});

test("Space on the focused stamp turns it", async () => {
  mount(<Stamp at={FOUR_MINUTES()} />);
  const stamp = page.getByRole("button");

  await userEvent.tab();
  await expect.element(stamp).toHaveFocus();
  await userEvent.keyboard(" ");

  await expect.element(stamp).toHaveAccessibleName("4m ago");
});

test("the tooltip is the other reading, and follows the stamp when it turns", async () => {
  mount(<Stamp at={FOUR_MINUTES()} />);
  const stamp = page.getByRole("button");

  // The clock is drawn, so what a press gives is the age.
  await expect.element(stamp).toHaveAccessibleDescription("4m ago");

  await userEvent.click(stamp);
  await expect.element(stamp).toHaveAccessibleName("4m ago");
  await expect.element(stamp).toHaveAccessibleDescription(CLOCK);
});

test("two stamps in one surface turn together, from a press on either", async () => {
  mount(
    <StampSurface>
      <Stamp at={FOUR_MINUTES()} />
      <Stamp at={TWO_HOURS()} />
    </StampSurface>,
  );
  const [first, second] = [page.getByRole("button").nth(0), page.getByRole("button").nth(1)];

  await userEvent.click(second);
  await expect.element(first).toHaveAccessibleName("4m ago");
  await expect.element(second).toHaveAccessibleName("2h ago");

  await userEvent.click(first);
  await expect.element(first).toHaveAccessibleName(CLOCK);
  await expect.element(second).toHaveAccessibleName(CLOCK);
});

test("a stamp in another surface keeps its own reading", async () => {
  mount(
    <>
      <StampSurface>
        <Stamp at={FOUR_MINUTES()} />
      </StampSurface>
      <StampSurface>
        <Stamp at={TWO_HOURS()} />
      </StampSurface>
    </>,
  );
  const [here, there] = [page.getByRole("button").nth(0), page.getByRole("button").nth(1)];

  await userEvent.click(here);

  await expect.element(here).toHaveAccessibleName("4m ago");
  await expect.element(there).toHaveAccessibleName(CLOCK);
});

test("a stamp outside a surface turns only itself", async () => {
  mount(
    <>
      <Stamp at={FOUR_MINUTES()} />
      <Stamp at={TWO_HOURS()} />
    </>,
  );

  await userEvent.click(page.getByRole("button").nth(0));

  await expect.element(page.getByRole("button").nth(0)).toHaveAccessibleName("4m ago");
  await expect.element(page.getByRole("button").nth(1)).toHaveAccessibleName(CLOCK);
});

test("text that is not a time is drawn as it came, and is not a button", async () => {
  mount(<Stamp at="a while back" />);

  await expect.element(page.getByText("a while back")).toBeVisible();
  await expect.element(page.getByRole("button")).not.toBeInTheDocument();
});
